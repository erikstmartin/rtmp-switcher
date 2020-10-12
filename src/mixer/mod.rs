mod error;

use crate::gst_create_element;
pub use crate::input;
pub use crate::output;
use crate::Format;
use crate::Result;
pub use error::Error;
use gst::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(default)]
pub struct VideoConfig {
    #[serde(default = "VideoConfig::framerate_default")]
    pub framerate: i32,
    #[serde(default = "VideoConfig::format_default")]
    pub format: Format,
    #[serde(default = "VideoConfig::width_default")]
    pub width: i32,
    #[serde(default = "VideoConfig::height_default")]
    pub height: i32,
    pub xpos: i32,
    pub ypos: i32,
    pub zorder: Option<u32>,
    #[serde(default = "VideoConfig::alpha_default")]
    pub alpha: f64,
    pub repeat: bool,
}

impl VideoConfig {
    fn framerate_default() -> i32 {
        30
    }

    fn format_default() -> Format {
        Format::RGBA
    }

    fn height_default() -> i32 {
        1920
    }

    fn width_default() -> i32 {
        1080
    }

    fn alpha_default() -> f64 {
        1.0
    }
}

impl Default for VideoConfig {
    fn default() -> Self {
        Self {
            framerate: Self::framerate_default(),
            width: Self::width_default(),
            height: Self::height_default(),
            zorder: None,
            xpos: 0,
            ypos: 0,
            alpha: Self::alpha_default(),
            repeat: false,
            format: Self::format_default(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(default)]
pub struct AudioConfig {
    #[serde(default = "AudioConfig::volume_default")]
    pub volume: f64,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            volume: Self::volume_default(),
        }
    }
}

impl AudioConfig {
    fn volume_default() -> f64 {
        1.0
    }
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
    pub inputs: HashMap<String, input::Input>,
    pub outputs: HashMap<String, output::Output>,
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
        let video_capsfilter = gst_create_element(
            "capsfilter",
            format!("mixer_{}_video_capsfilter", config.name).as_str(),
        )?;
        let video_mixer = gst_create_element(
            "compositor",
            format!("mixer_{}_video_compositor", config.name).as_str(),
        )?;
        let video_caps = gst::Caps::builder("video/x-raw")
            .field("framerate", &gst::Fraction::new(config.video.framerate, 1))
            .field("format", &format!("{}", config.video.format))
            .field("width", &config.video.width)
            .field("height", &config.video.height)
            .build();
        video_capsfilter.set_property("caps", &video_caps).unwrap();

        let video_queue = gst_create_element(
            "queue",
            format!("mixer_{}_video_queue", config.name).as_str(),
        )?;
        let video_tee =
            gst_create_element("tee", format!("mixer_{}_video_tee", config.name).as_str())?;
        video_tee.set_property("allow-not-linked", &true)?;

        pipeline.add_many(&[&video_mixer, &video_capsfilter, &video_queue, &video_tee])?;
        gst::Element::link_many(&[&video_mixer, &video_capsfilter, &video_queue, &video_tee])?;

        let audio_mixer = gst_create_element(
            "audiomixer",
            format!("mixer_{}_audio_mixer", config.name).as_str(),
        )?;
        let volume = gst_create_element(
            "volume",
            format!("mixer_{}_audio_volume", config.name).as_str(),
        )?;
        volume.set_property("volume", &config.audio.volume)?;
        let audio_capsfilter = gst_create_element(
            "capsfilter",
            format!("mixer_{}_audio_capsfilter", config.name).as_str(),
        )?;
        let audio_caps = gst::Caps::builder("audio/x-raw")
            .field("channels", &2)
            .field("layout", &"interleaved")
            .field("format", &"S32LE")
            .build();
        audio_capsfilter.set_property("caps", &audio_caps).unwrap();

        let audio_tee =
            gst_create_element("tee", format!("mixer_{}_audio_tee", config.name).as_str())?;
        audio_tee.set_property("allow-not-linked", &true)?;

        pipeline.add_many(&[&audio_mixer, &volume, &audio_capsfilter, &audio_tee])?;
        gst::Element::link_many(&[&audio_mixer, &volume, &audio_capsfilter, &audio_tee])?;

        let mixer = Mixer {
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

        let config = input::Config {
            name: "background".to_string(),
            audio: AudioConfig { volume: 0.0 },
            video: config.video.clone(),
            record: false,
        };

        let mut background = input::Input::create_test(config)?;
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

    pub fn input_add(&mut self, mut input: input::Input) -> Result<()> {
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

    pub fn output_add(&mut self, mut output: output::Output) -> Result<()> {
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

    pub fn input_set_active(&mut self, name: &str) -> Result<()> {
        if !self.inputs.contains_key(name) {
            return Err(Error::NotFound("input".to_string(), name.to_string()));
        }

        // TODO: Fix this!!
        let input = self.inputs.get_mut(name).unwrap();

        input.set_zorder(1000)?;
        input.set_xpos(0)?;
        input.set_ypos(0)?;
        input.set_width(self.config.video.width)?;
        input.set_height(self.config.video.height)?;

        let input_config = input.config();
        input.set_volume(input_config.audio.volume)?;

        // Decrease volume and restore zorder of all other inputs
        for (n, input) in self.inputs.iter_mut() {
            if n != name {
                input.set_volume(0.0)?;
                input.set_zorder(input_config.video.zorder.unwrap())?;
            }
        }

        Ok(())
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
