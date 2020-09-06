extern crate gstreamer as gst;
use super::input::Input;
use gst::prelude::*;
use std::collections::HashMap;
use std::error::Error;
use std::fmt;

// TODO:
// - We need to ensure we have queues before N-1 elements and after 1-N elements.
// - Inputs and Outputs may be audio only or video only.
// - autoaudiosink does not play audio, even though it's in a playing state,
// when used with RTMP sink
// - Artifacting/discontinuity on RTMP feed but not autovideosink
// - Refactor Input, Output, Mixer into different modules.
// - Create separate bins for inputs and outputs (intervideosrc, intervideosink)
// - Handle dynamically changing pipeline while running
//   - Use Idle PadProbe's in order to ensure we don't unlink elements during negotiations, etc.
//   - Block src pads until ready.
//   - Synchronize state between bins/elements before linking.
// - remove_input
// - Figure out why some input videos work and others fail
// - Background video test src is bleeding into input (do we need the compositor?)
// - Have mixer enforce consistent codec on output. So we can perform them only 1 time, before
// the tee
// - Better comments
// - Tests (eeeeek!)

// TODO: Current issue - We have weird artifacts/discontinuity only on RTMP sinks.
// - This does not happen when our input is in the main pipeline. It only happens when the input is
// in its own bin.
// - The video that comes of the autovideosink is perfect..., even when input is in its own bin.

pub struct Mux {
    audio: gst::Element,
    video: gst::Element,
}

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

// Mixer (a channel)
// - Inputs
// - Outputs
/// This becomes docs?
pub struct Mixer {
    name: String,
    pipeline: gst::Pipeline,
    audio_mixer: gst::Element,
    video_mixer: gst::Element,
    inputs: HashMap<String, Input>,
    outputs: HashMap<String, Mux>,
    audio_out: gst::Element,
    video_out: gst::Element,
}

// TODO:
// - We need some sort constant src to be played. The reason for this is that the pipeline will end
// when the video completes. So we need a black screen or something to always be underneath all of
// our streams. So that we can swap them out.
impl Mixer {
    pub fn new(name: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let audio_mixer = gst::ElementFactory::make("audiomixer", Some("audiomixer"))?;
        let video_mixer = gst::ElementFactory::make("videomixer", Some("videomixer"))?;

        let audio_tee = gst::ElementFactory::make("tee", Some("audiotee"))?;
        audio_tee.set_property("allow-not-linked", &true)?;

        let video_tee = gst::ElementFactory::make("tee", Some("videotee"))?;
        video_tee.set_property("allow-not-linked", &true)?;

        let pipeline = gst::Pipeline::new(Some(name));
        pipeline.add_many(&[&audio_mixer, &video_mixer, &audio_tee, &video_tee])?;

        gst::Element::link_many(&[&audio_mixer, &audio_tee])?;
        gst::Element::link_many(&[&video_mixer, &video_tee])?;

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

    // Assume this is always RTMP for now?
    pub fn add_output(&mut self, name: &str, uri: &str) -> Result<(), Box<dyn std::error::Error>> {
        if self.outputs.contains_key(name) {
            return Err(MixerError::new(
                format!("Output with name '{}' already exists.", name).as_str(),
            )
            .into());
        }

        // Video stream
        let video_queue =
            gst::ElementFactory::make("queue", Some(format!("{}_video_queue", name).as_str()))?;
        let video_convert = gst::ElementFactory::make(
            "videoconvert",
            Some(format!("{}_videoconvert", name).as_str()),
        )?;
        let x264enc =
            gst::ElementFactory::make("x264enc", Some(format!("{}_x264enc", name).as_str()))?;
        let h264parse =
            gst::ElementFactory::make("h264parse", Some(format!("{}_h264parse", name).as_str()))?;
        let flvmux =
            gst::ElementFactory::make("flvmux", Some(format!("{}_flvmux", name).as_str()))?;
        let queue_sink =
            gst::ElementFactory::make("queue", Some(format!("{}_queuesink", name).as_str()))?;
        let video_sink =
            gst::ElementFactory::make("rtmpsink", Some(format!("{}_video_sink", name).as_str()))?;

        // Audio stream
        let audio_queue =
            gst::ElementFactory::make("queue", Some(format!("{}_audio_queue", name).as_str()))?;
        let audioenc =
            gst::ElementFactory::make("fdkaacenc", Some(format!("{}_fdkaacenc", name).as_str()))?;

        video_sink.set_property("location", &uri)?;
        flvmux.set_property_from_str("streamable", "true");

        // Add elements to pipeline
        self.pipeline.add_many(&[
            &video_queue,
            &audio_queue,
            //&video_convert,
            &x264enc,
            &h264parse,
            &flvmux,
            &queue_sink,
            &video_sink,
            &audioenc,
        ])?;

        // Link video elements
        gst::Element::link_many(&[
            &self.video_out,
            &video_queue,
            // &video_convert,
            &x264enc,
            &h264parse,
            &flvmux,
            &queue_sink,
            &video_sink,
        ])?;

        // Link audio elements
        gst::Element::link_many(&[&self.audio_out, &audio_queue, &audioenc, &flvmux])?;

        self.outputs.insert(
            name.to_string(),
            Mux {
                audio: audioenc.clone(),
                video: video_convert.clone(),
            },
        );

        Ok(())
    }

    pub fn remove_output(&mut self, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let output = self
            .outputs
            .get(&name.to_string())
            .ok_or(MixerError::new("output not found"))?;

        // Detach audio
        let sink_pad = output
            .audio
            .get_static_pad("sink")
            .expect("Failed to get sink pad from audio");

        self.audio_out
            .release_request_pad(&sink_pad.get_peer().unwrap());
        self.remove_output_elements(&output.audio);

        // Detach video
        let sink_pad = output
            .video
            .get_static_pad("sink")
            .expect("Failed to get sink pad from video");

        self.video_out
            .release_request_pad(&sink_pad.get_peer().unwrap());
        self.remove_output_elements(&output.video);

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
