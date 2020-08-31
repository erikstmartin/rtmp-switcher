extern crate gstreamer as gst;
use gst::prelude::*;
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::sync::{Arc, Mutex};

pub struct Output {
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
    inputs: Vec<gst::Element>,
    outputs: HashMap<String, Output>,
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
        let video_tee = gst::ElementFactory::make("tee", Some("videotee"))?;

        let pipeline = gst::Pipeline::new(Some(name));
        pipeline.debug_to_dot_file(gst::DebugGraphDetails::all(), name);
        pipeline.add_many(&[&audio_mixer, &video_mixer, &audio_tee, &video_tee])?;
        gst::Element::link_many(&[&audio_mixer, &audio_tee]);
        gst::Element::link_many(&[&video_mixer, &video_tee]);

        Ok(Mixer {
            name: name.to_string(),
            pipeline: pipeline,
            audio_mixer: audio_mixer,
            video_mixer: video_mixer,
            inputs: vec![],
            outputs: HashMap::new(),
            audio_out: audio_tee,
            video_out: video_tee,
        })
    }

    pub fn add_input(&mut self, uri: &str) -> Result<(), Box<dyn std::error::Error>> {
        let source = gst::ElementFactory::make("uridecodebin", Some("source"))?;
        source.set_property("uri", &uri)?;
        self.pipeline.add(&source);
        self.inputs.push(source.clone());

        // Connect the pad-added signal
        let audio_mixer = self.audio_mixer.clone();
        let video_mixer = self.video_mixer.clone();
        source.connect_pad_added(move |src, src_pad| {
            println!(
                "Received new pad {} from {}",
                src_pad.get_name(),
                src.get_name()
            );

            let new_pad_caps = src_pad
                .get_current_caps()
                .expect("Failed to get caps of new pad.");
            let new_pad_struct = new_pad_caps
                .get_structure(0)
                .expect("Failed to get first structure of caps.");
            let new_pad_type = new_pad_struct.get_name();

            if new_pad_type.starts_with("audio/x-raw") {
                let sink_pad = audio_mixer
                    .get_request_pad("sink_%u")
                    .expect("Failed to get sink pad from audio mixer");
                if sink_pad.is_linked() {
                    println!("We are already linked. Ignoring.");
                    return;
                }

                let res = src_pad.link(&sink_pad);
                if res.is_err() {
                    println!("Type is {} but link failed.", new_pad_type);
                } else {
                    println!("Link succeeded (type {}).", new_pad_type);
                }
            } else if new_pad_type.starts_with("video/x-raw") {
                let sink_pad = video_mixer
                    .get_request_pad("sink_%u")
                    .expect("Failed to get static sink pad from video_mixer");
                if sink_pad.is_linked() {
                    println!("We are already linked. Ignoring.");
                    return;
                }

                let res = src_pad.link(&sink_pad);
                if res.is_err() {
                    println!("Type is {} but link failed.", new_pad_type);
                } else {
                    println!("Link succeeded (type {}).", new_pad_type);
                }
            }
        });

        Ok(())
    }

    // Assume this is always RTMP for now?
    pub fn add_output(&mut self, uri: &str) -> Result<(), Box<dyn std::error::Error>> {
        // TODO: Ensure that this output uri doesn't already exist in the map. Throw error.

        // Video stream
        let video_convert = gst::ElementFactory::make("videoconvert", Some("videoconvert"))?;
        let x264enc = gst::ElementFactory::make("x264enc", Some("x264enc"))?;
        let flvmux = gst::ElementFactory::make("flvmux", Some("flvmux"))?;
        let queue_sink = gst::ElementFactory::make("queue", Some("queuesink"))?;
        let video_sink = gst::ElementFactory::make("rtmpsink", Some("video_sink"))?;

        // Audio stream
        let audioenc = gst::ElementFactory::make("fdkaacenc", Some("fdkaacenc"))?;
        let convert = gst::ElementFactory::make("audioconvert", Some("convert"))?;
        let resample = gst::ElementFactory::make("audioresample", Some("resample"))?;

        video_sink.set_property("location", &uri)?;
        flvmux.set_property_from_str("streamable", "true");

        // Add elements to pipeline
        self.pipeline.add_many(&[
            &video_convert,
            &x264enc,
            &flvmux,
            &queue_sink,
            &video_sink,
            &audioenc,
            &convert,
            &resample,
        ])?;

        // Link video elements
        gst::Element::link_many(&[
            &self.video_out,
            &video_convert,
            &x264enc,
            &flvmux,
            &queue_sink,
            &video_sink,
        ])?;

        // Link audio elements
        gst::Element::link_many(&[&self.audio_out, &audioenc, &flvmux])?;

        self.outputs.insert(
            uri.to_string(),
            Output {
                audio: audioenc.clone(),
                video: video_convert.clone(),
            },
        );

        Ok(())
    }

    pub fn remove_output(&self, uri: &str) -> Result<(), Box<dyn std::error::Error>> {
        let output = self
            .outputs
            .get(&uri.to_string())
            .ok_or(MixerError::new("output not found"))?;

        // Detach audio
        let sink_pad = output
            .audio
            .get_static_pad("sink")
            .expect("Failed to get sink pad from audio");
        sink_pad.get_peer().unwrap().unlink(&sink_pad);

        // Detach video
        let sink_pad = output
            .video
            .get_static_pad("sink")
            .expect("Failed to get sink pad from video");
        sink_pad.get_peer().unwrap().unlink(&sink_pad);

        // TODO: Remove elements from the pipeline itself?
        // TODO: Is there any cleanup that needs to be done on gst::Element
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
                    }
                }
                MessageView::Eos(..) => break,
                _ => (),
            }
        }

        self.pipeline.set_state(gst::State::Null)?;
        Ok(())
    }
}
