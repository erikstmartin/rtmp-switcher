mod error;
pub mod input;
pub mod output;

use crate::Result;
pub use error::Error;
use gst::prelude::*;
pub use input::Input;
pub use output::Output;
use std::collections::HashMap;

// TODO:
// - Inputs and Outputs may be audio only or video only.
// - Auto and RTMP sinks can't be used together or they get stuck in the Paused state.
// - autoaudiosink does not play audio, even though it's in a playing state,
// when used with RTMP sink
// - Handle dynamically changing pipeline while running
//   - Use Idle PadProbe's in order to ensure we don't unlink elements during negotiations, etc.
//   - Block src pads until ready.
//   - Synchronize state between bins/elements before linking.
// - Figure out why some input videos work and others fail (mismatch between sample rate of audio)
// - Better comments
// - Tests (eeeeek!)
//
// - Network resilience (need to reset from paused to play)
// https://gstreamer.freedesktop.org/documentation/tutorials/basic/streaming.html?gi-language=c

pub struct Mixer {
    pub name: String,
    pipeline: gst::Pipeline,
    audio_mixer: gst::Element,
    video_mixer: gst::Element,
    pub inputs: HashMap<String, Input>,
    pub outputs: HashMap<String, Output>,
    audio_out: gst::Element,
    video_out: gst::Element,
    join_handle: Option<std::thread::JoinHandle<()>>,
}

impl Mixer {
    pub fn new(name: &str) -> Result<Self> {
        let background_enabled = true;
        let pipeline = gst::Pipeline::new(Some(name));

        // Create Video Channel
        let video_capsfilter = gst::ElementFactory::make("capsfilter", Some("video_capsfilter"))?;
        let video_mixer = gst::ElementFactory::make("compositor", Some("videomixer"))?;
        let video_caps = gst::Caps::builder("video/x-raw")
            // TODO:.field("format", &gst_video::VideoFormat::Rgba.to_str())
            .field("framerate", &gst::Fraction::new(30, 1))
            .build();
        video_capsfilter.set_property("caps", &video_caps).unwrap();

        let video_queue = gst::ElementFactory::make("queue", Some("videomixer_queue"))?;
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
            video_background.set_property_from_str("pattern", "black");
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
            pipeline,
            join_handle: None,
            audio_mixer,
            video_mixer,
            inputs: HashMap::new(),
            outputs: HashMap::new(),
            audio_out: audio_tee,
            video_out: video_tee,
        })
    }

    pub fn input_count(&self) -> usize {
        self.inputs.len()
    }

    pub fn input_add(&mut self, mut input: Input) -> Result<()> {
        if self.inputs.contains_key(&input.name()) {
            return Err(Error::Exists("input".to_string(), input.name()));
        }

        input.link(
            self.pipeline.clone(),
            self.audio_mixer.clone(),
            self.video_mixer.clone(),
        )?;

        self.inputs.insert(input.name(), input);

        Ok(())
    }

    // traverse pads->peers until we hit audio or video mixer.
    // Don't remove mixer element
    // release pad from mixer
    pub fn input_remove(&mut self, name: &str) -> Result<()> {
        if !self.inputs.contains_key(name) {
            return Err(Error::NotFound("input".to_string(), name.to_string()));
        }

        let input = self.inputs.get_mut(name).unwrap();
        input.unlink()?;
        self.inputs.remove(name);

        Ok(())
    }

    pub fn output_count(&self) -> usize {
        self.outputs.len()
    }

    pub fn output_add(&mut self, mut output: Output) -> Result<()> {
        if self.outputs.contains_key(&output.name()) {
            return Err(Error::Exists("output".to_string(), output.name()));
        }

        output.link(
            self.pipeline.clone(),
            self.audio_out.clone(),
            self.video_out.clone(),
        )?;

        self.outputs.insert(output.name(), output);

        Ok(())
    }

    pub fn output_remove(&mut self, name: &str) -> Result<()> {
        if !self.outputs.contains_key(name) {
            return Err(Error::NotFound("output".to_string(), name.to_string()));
        }

        let output = self.outputs.get_mut(name).unwrap();
        output.unlink()?;
        self.outputs.remove(name);

        Ok(())
    }

    pub fn play(&mut self) -> Result<()> {
        let p = self.pipeline.clone();
        self.join_handle = Some(std::thread::spawn(move || watch_bus(p)));

        self.pipeline.set_state(gst::State::Playing)?;
        Ok(())
    }

    pub fn stop(&mut self) -> Result<()> {
        self.pipeline.set_state(gst::State::Null)?;
        self.join_handle.take().unwrap().join().unwrap();

        Ok(())
    }

    pub fn generate_dot(&self) -> String {
        self.pipeline
            .debug_to_dot_data(gst::DebugGraphDetails::ALL)
            .to_string()
    }
}

fn watch_bus(pipeline: gst::Pipeline) {
    // Wait until error or EOS
    let bus = pipeline.get_bus().unwrap();
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
                    .map(|s| s == pipeline)
                    .unwrap_or(false)
                {
                    println!(
                        "Pipeline state changed from {:?} to {:?}",
                        state_changed.get_old(),
                        state_changed.get_current()
                    );

                    pipeline.debug_to_dot_file(
                        gst::DebugGraphDetails::VERBOSE,
                        format!("{:?}", state_changed.get_current()),
                    );

                    match state_changed.get_current() {
                        gst::State::Null => break,
                        _ => continue,
                    }
                }
            }
            MessageView::Eos(..) => break,
            _ => (),
        }
    }
}
