mod error;
pub mod input;
pub mod output;

use crate::Result;
pub use error::Error;
use gst::prelude::*;
pub use input::Input;
pub use output::Output;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct VideoConfig {
    pub framerate: Option<i32>,
    pub format: Option<String>,
    pub width: Option<i32>,
    pub height: Option<i32>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AudioConfig {
    pub volume: Option<f64>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    pub name: String,
    pub video: VideoConfig,
    pub audio: AudioConfig,
}

pub struct Mixer {
    config: Config,
    pipeline: gst::Pipeline,
    audio_mixer: gst::Element,
    video_mixer: gst::Element,
    pub inputs: HashMap<String, Input>,
    pub outputs: HashMap<String, Output>,
    audio_out: gst::Element,
    video_out: gst::Element,
    join_handle: Option<std::thread::JoinHandle<()>>,
}

impl Drop for Mixer {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

impl Mixer {
    pub fn new(config: Config) -> Result<Self> {
        let background_enabled = true;
        let pipeline = gst::Pipeline::new(Some(config.name.as_str()));

        // Create Video Channel
        let video_capsfilter = gst::ElementFactory::make("capsfilter", Some("video_capsfilter"))?;
        let video_mixer = gst::ElementFactory::make("compositor", Some("videomixer"))?;
        let video_caps = gst::Caps::builder("video/x-raw")
            .field(
                "framerate",
                &gst::Fraction::new(config.video.framerate.unwrap(), 1),
            )
            .field("format", &config.video.format.clone().unwrap().as_str())
            .field("width", &config.video.width.unwrap())
            .field("height", &config.video.height.unwrap())
            .build();
        video_capsfilter.set_property("caps", &video_caps).unwrap();

        let video_queue = gst::ElementFactory::make("queue", Some("videomixer_queue"))?;
        let video_tee = gst::ElementFactory::make("tee", Some("videotee"))?;
        video_tee.set_property("allow-not-linked", &true)?;

        pipeline.add_many(&[&video_mixer, &video_capsfilter, &video_queue, &video_tee])?;
        gst::Element::link_many(&[&video_mixer, &video_capsfilter, &video_queue, &video_tee])?;

        let audio_mixer = gst::ElementFactory::make("audiomixer", Some("audiomixer"))?;
        let volume = gst::ElementFactory::make("volume", Some("audio_volume"))?;
        volume.set_property("volume", &config.audio.volume.unwrap())?;
        let audio_capsfilter = gst::ElementFactory::make("capsfilter", Some("audio_capsfilter"))?;
        let audio_caps = gst::Caps::builder("audio/x-raw")
            .field("channels", &2)
            .field("layout", &"interleaved")
            .field("format", &"S32LE")
            .build();
        audio_capsfilter.set_property("caps", &audio_caps).unwrap();

        let audio_tee = gst::ElementFactory::make("tee", Some("audiotee"))?;
        audio_tee.set_property("allow-not-linked", &true)?;

        pipeline.add_many(&[&audio_mixer, &volume, &audio_capsfilter, &audio_tee])?;
        gst::Element::link_many(&[&audio_mixer, &volume, &audio_capsfilter, &audio_tee])?;

        let mut mixer = Mixer {
            config: config.clone(),
            pipeline,
            join_handle: None,
            audio_mixer,
            video_mixer,
            inputs: HashMap::new(),
            outputs: HashMap::new(),
            audio_out: audio_tee,
            video_out: video_tee,
        };

        let config = Config {
            name: "background".to_string(),
            audio: AudioConfig { volume: Some(0.0) },
            video: config.video.clone(),
        };
        let mut background = input::Test::new(config)?;
        if background_enabled {
            background.link(
                mixer.pipeline.clone(),
                mixer.audio_mixer.clone(),
                mixer.video_mixer.clone(),
            )?;
        }

        Ok(mixer)
    }

    pub fn input_count(&self) -> usize {
        self.inputs.len()
    }

    pub fn input_add(&mut self, mut input: Input) -> Result<()> {
        if self.inputs.contains_key(&input.name()) {
            return Err(Error::Exists("input".to_string(), input.name()));
        }

        // TODO: Handle pending states
        let state = self.pipeline.get_state(gst::ClockTime::from_seconds(15)).1;
        input.set_state(state)?;
        input.link(
            self.pipeline.clone(),
            self.audio_mixer.clone(),
            self.video_mixer.clone(),
        )?;

        self.inputs.insert(input.name(), input);

        Ok(())
    }

    pub fn input_remove(&mut self, name: &str) -> Result<()> {
        if !self.inputs.contains_key(name) {
            return Err(Error::NotFound("input".to_string(), name.to_string()));
        }

        let input = self.inputs.get_mut(name).unwrap();
        input.set_state(gst::State::Null)?;
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

        // TODO: Handle pending states
        let state = self.pipeline.get_state(gst::ClockTime::from_seconds(15)).1;
        output.set_state(state)?;
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
        output.set_state(gst::State::Null)?;
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

        /* TODO: Fix me
        if self.join_handle.is_some() {
            self.join_handle.take().unwrap().join().unwrap();
        }
        */

        Ok(())
    }

    pub fn generate_dot(&self) -> String {
        self.pipeline
            .debug_to_dot_data(gst::DebugGraphDetails::ALL)
            .to_string()
    }

    pub fn name(&self) -> String {
        self.config.name.clone()
    }

    pub fn config(&self) -> Config {
        self.config.clone()
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
                    "{}: Error received from element {:?} {}",
                    pipeline.get_name(),
                    err.get_src().map(|s| s.get_path_string()),
                    err.get_error()
                );
                eprintln!(
                    "{}: Debugging information: {:?}",
                    pipeline.get_name(),
                    err.get_debug()
                );
                break;
            }
            MessageView::StateChanged(state_changed) => {
                if state_changed
                    .get_src()
                    .map(|s| s == pipeline)
                    .unwrap_or(false)
                {
                    println!(
                        "{}: Pipeline state changed from {:?} to {:?}",
                        pipeline.get_name(),
                        state_changed.get_old(),
                        state_changed.get_current()
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

pub fn default_config() -> Config {
    Config {
        name: "".to_string(),
        audio: default_audio_config(),
        video: default_video_config(),
    }
}

pub fn default_audio_config() -> AudioConfig {
    AudioConfig { volume: Some(1.0) }
}

pub fn default_video_config() -> VideoConfig {
    VideoConfig {
        framerate: Some(30),
        width: Some(1920),
        height: Some(1080),
        format: Some("RGBA".to_string()),
    }
}
