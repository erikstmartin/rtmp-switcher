extern crate gstreamer as gst;
extern crate gstreamer_video as gst_video;
use super::input::Input;
use super::output::Output;
use gst::prelude::*;
use std::collections::HashMap;
use std::error::Error;
use std::fmt;

// TODO:
// - Inputs and Outputs may be audio only or video only.
// - autoaudiosink does not play audio, even though it's in a playing state,
// when used with RTMP sink
// - Artifacting/discontinuity on RTMP feed but not autovideosink
// - Handle dynamically changing pipeline while running
//   - Use Idle PadProbe's in order to ensure we don't unlink elements during negotiations, etc.
//   - Block src pads until ready.
//   - Synchronize state between bins/elements before linking.
// - remove_input
// - fix remove_ouput
// - Figure out why some input videos work and others fail (mismatch between sample rate of audio)
// - Is there a way to manually connect our inter(audio|video)src and sink?
// - Better comments
// - Tests (eeeeek!)

// TODO: Current issue - We have weird artifacts/discontinuity only on RTMP sinks.
// - This does not happen when our input is in the main pipeline. It only happens when the input is
// in its own bin.
// - The video that comes of the autovideosink is perfect..., even when input is in its own bin.

#[derive(Debug)]
pub struct MixerError {
    details: String,
}

impl MixerError {
    fn new(msg: impl Into<String>) -> MixerError {
        MixerError {
            details: msg.into(),
        }
    }
}

impl fmt::Display for MixerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.details)
    }
}

impl Error for MixerError {
    fn description(&self) -> &str {
        &self.details
    }
}

pub struct Mixer {
    name: String,
    pipeline: gst::Pipeline,
    audio_mixer: gst::Element,
    video_mixer: gst::Element,
    inputs: HashMap<String, Input>,
    outputs: HashMap<String, Output>,
    audio_out: gst::Element,
    video_out: gst::Element,
}

impl Mixer {
    pub fn new(name: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let background_enabled = true;
        let pipeline = gst::Pipeline::new(Some(name));

        // Create Video Channel
        let video_capsfilter = gst::ElementFactory::make("capsfilter", Some("video_capsfilter"))?;
        let video_mixer = gst::ElementFactory::make("compositor", Some("videomixer"))?;
        let video_caps = gst::Caps::builder("video/x-raw")
            // TODO:.field("format", &gst_video::VideoFormat::Rgba.to_str())
            .field("framerate", &gst::Fraction::new(60, 1))
            .build();
        video_capsfilter.set_property("caps", &video_caps).unwrap();

        let video_queue = gst::ElementFactory::make("queue", Some("videotestsrc_queue"))?;
        let video_tee = gst::ElementFactory::make("tee", Some("videotee"))?;
        video_tee.set_property("allow-not-linked", &true)?;

        pipeline.add_many(&[&video_mixer, &video_capsfilter, &video_queue, &video_tee])?;
        gst::Element::link_many(&[&video_mixer, &video_capsfilter, &video_queue, &video_tee])?;

        let audio_mixer = gst::ElementFactory::make("audiomixer", Some("audiomixer"))?;
        let audio_capsfilter = gst::ElementFactory::make("capsfilter", Some("audio_capsfilter"))?;
        let audio_caps = gst::Caps::builder("audio/x-raw")
            .field("channels", &2)
            .field("layout", &"interleaved")
            .field("format", &"S32LE")
            .build();
        audio_capsfilter.set_property("caps", &audio_caps).unwrap();

        let audio_tee = gst::ElementFactory::make("tee", Some("audiotee"))?;
        audio_tee.set_property("allow-not-linked", &true)?;

        pipeline.add_many(&[&audio_mixer, &audio_capsfilter, &audio_tee])?;
        gst::Element::link_many(&[&audio_mixer, &audio_capsfilter, &audio_tee])?;

        if background_enabled {
            let video_background = gst::ElementFactory::make("videotestsrc", Some("videotestsrc"))?;
            video_background.set_property_from_str("pattern", "ball");
            video_background.set_property("is-live", &true)?;
            let video_convert = gst::ElementFactory::make("videoconvert", Some("videoconvert"))?;
            let video_scale = gst::ElementFactory::make("videoscale", Some("videoscale"))?;

            let audio_background = gst::ElementFactory::make("audiotestsrc", Some("audiotestsrc"))?;
            audio_background.set_property("volume", &0.0)?;
            audio_background.set_property("is-live", &true)?;
            let audio_convert = gst::ElementFactory::make("audioconvert", Some("audioconvert"))?;
            let audio_resample = gst::ElementFactory::make("audioresample", Some("audioresample"))?;
            let audio_queue = gst::ElementFactory::make("queue", Some("audiotestsrc_queue"))?;

            pipeline.add_many(&[
                &video_background,
                &video_convert,
                &video_scale,
                &audio_background,
                &audio_convert,
                &audio_resample,
                &audio_queue,
            ])?;
            // Link video elements
            gst::Element::link_many(&[
                &video_background,
                &video_convert,
                &video_scale,
                &video_mixer,
            ])?;
            // Link audio elements
            gst::Element::link_many(&[
                &audio_background,
                &audio_convert,
                &audio_resample,
                &audio_queue,
                &audio_mixer,
            ])?;
        }

        Ok(Mixer {
            name: name.to_string(),
            pipeline: pipeline,
            audio_mixer: audio_mixer,
            video_mixer: video_mixer,
            inputs: HashMap::new(),
            outputs: HashMap::new(),
            audio_out: audio_tee,
            video_out: video_tee,
        })
    }

    pub fn add_input(&mut self, input: Input) -> Result<(), Box<dyn std::error::Error>> {
        if self.inputs.contains_key(&input.name) {
            return Err(MixerError::new(
                format!("Input with name '{}' already exists.", input.name).as_str(),
            )
            .into());
        }

        let intervideosrc = gst::ElementFactory::make(
            "intervideosrc",
            Some(format!("{}_intervideosrc", input.name).as_str()),
        )?;
        intervideosrc.set_property("channel", &format!("{}_video_channel", input.name))?;

        let interaudiosrc = gst::ElementFactory::make(
            "interaudiosrc",
            Some(format!("{}_interaudiosrc", input.name).as_str()),
        )?;
        interaudiosrc.set_property("channel", &format!("{}_audio_channel", input.name))?;

        self.pipeline.add_many(&[&input.pipeline])?;
        self.pipeline.add_many(&[&intervideosrc, &interaudiosrc])?;
        gst::Element::link_many(&[&interaudiosrc, &self.audio_mixer])?;
        gst::Element::link_many(&[&intervideosrc, &self.video_mixer])?;

        self.inputs.insert(input.name.to_string(), input);

        Ok(())
    }

    // TODO:  remove_input
    // traverse pads->peers until we hit audio or video mixer.
    // Don't remove mixer element
    // release pad from mixer

    pub fn add_output(&mut self, output: Output) -> Result<(), Box<dyn std::error::Error>> {
        if self.outputs.contains_key(&output.name) {
            return Err(MixerError::new(
                format!("Output with name '{}' already exists.", output.name).as_str(),
            )
            .into());
        }

        let intervideosink = gst::ElementFactory::make(
            "intervideosink",
            Some(format!("{}_intervideosink", output.name).as_str()),
        )?;
        intervideosink.set_property("channel", &format!("{}_video_channel", output.name))?;
        let intervideoqueue = gst::ElementFactory::make(
            "queue",
            Some(format!("{}_intervideoqueue", output.name).as_str()),
        )?;

        let interaudiosink = gst::ElementFactory::make(
            "interaudiosink",
            Some(format!("{}_interaudiosink", output.name).as_str()),
        )?;
        interaudiosink.set_property("channel", &format!("{}_audio_channel", output.name))?;
        let interaudioqueue = gst::ElementFactory::make(
            "queue",
            Some(format!("{}_interaudioqueue", output.name).as_str()),
        )?;

        self.pipeline.add_many(&[
            &interaudioqueue,
            &intervideoqueue,
            &intervideosink,
            &interaudiosink,
        ])?;

        // TODO: Add queue after tee
        gst::Element::link_many(&[&self.audio_out, &interaudioqueue, &interaudiosink])?;
        gst::Element::link_many(&[&self.video_out, &intervideoqueue, &intervideosink])?;
        self.pipeline.add_many(&[&output.pipeline])?;

        self.outputs.insert(output.name.to_string(), output);

        Ok(())
    }

    pub fn remove_output(&mut self, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let output = self
            .outputs
            .get(&name.to_string())
            .ok_or(MixerError::new("output not found"))?;

        // Detach audio
        let src_pad = output
            .audio
            .get_static_pad("src")
            .expect("Failed to get src pad from audio");

        //        self.audio_out
        //            .release_request_pad(&src_pad.get_peer().unwrap());

        // Detach video
        let src_pad = output
            .video
            .get_static_pad("src")
            .expect("Failed to get src pad from video");

        //        self.video_out
        //           .release_request_pad(&src_pad.get_peer().unwrap());

        // Ask the Output for its audio/video (which represent the interaudiosrc/intervideosrc)
        // Ask for the src pad
        // As the src pad for its peer
        // unlink
        // After doing this for both audio and video. Then we can remove the output.pipeline from
        // our mixer pipeline

        self.outputs.remove(name);

        Ok(())
    }

    pub fn play(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.pipeline.set_state(gst::State::Playing)?;

        // Wait until error or EOS
        let bus = self.pipeline.get_bus().unwrap();
        for msg in bus.iter_timed(gst::CLOCK_TIME_NONE) {
            use gst::MessageView;
            match msg.view() {
                MessageView::Error(err) => {
                    eprintln!(
                        "Error received from element {:?} {}",
                        err.get_src().map(|s| s.get_path_string()),
                        err.get_error()
                    );
                    eprintln!("Debugging information: {:?}", err.get_debug());
                    break;
                }
                MessageView::StateChanged(state_changed) => {
                    if state_changed
                        .get_src()
                        .map(|s| s == self.pipeline)
                        .unwrap_or(false)
                    {
                        println!(
                            "Pipeline state changed from {:?} to {:?}",
                            state_changed.get_old(),
                            state_changed.get_current()
                        );

                        self.pipeline.debug_to_dot_file(
                            gst::DebugGraphDetails::all(),
                            format!("{:?}", state_changed.get_current()),
                        );
                    }
                }
                MessageView::Eos(..) => break,
                _ => (),
            }
        }

        self.pipeline.set_state(gst::State::Null)?;
        Ok(())
    }

    fn remove_output_elements(&self, elem: &gst::Element) -> Result<(), Box<dyn Error>> {
        elem.foreach_src_pad(|e, p| {
            if let Some(peer) = p.get_peer() {
                self.remove_output_elements(&peer.get_parent_element().unwrap())
                    .expect("expected elements to be removed");
            }
            true
        });

        self.pipeline
            .remove(elem)
            .expect("Expected element to be removed from pipeline.");
        Ok(())
    }
}
