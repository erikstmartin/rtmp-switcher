pub mod fake;
pub mod test;
pub mod uri;

use crate::{mixer::Error as MixerError, AudioConfig, Result, VideoConfig};
pub use fake::Fake;
use serde::{Deserialize, Serialize};
pub use test::Test;
pub use uri::URI;

use gst::prelude::*;
use gstreamer as gst;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    pub name: String,
    pub video: VideoConfig,
    pub audio: AudioConfig,
    pub record: bool,
}

pub enum Input {
    URI(URI),
    Test(Test),
    Fake(Fake),
}

impl Input {
    pub fn create_uri(config: Config, uri: &str) -> Result<Input> {
        URI::create(config, uri).map(Self::URI)
    }

    pub fn create_test(config: Config) -> Result<Self> {
        Test::create(config).map(Self::Test)
    }

    pub fn create_fake(config: Config) -> Result<Self> {
        Fake::create(config).map(Self::Fake)
    }

    pub fn name(&self) -> String {
        match self {
            Input::URI(input) => input.name(),
            Input::Test(input) => input.name(),
            Input::Fake(input) => input.name(),
        }
    }

    pub fn location(&self) -> String {
        match self {
            Input::URI(input) => input.location.clone(),
            Input::Test(_) => "".to_string(),
            Input::Fake(_) => "".to_string(),
        }
    }

    pub fn input_type(&self) -> String {
        match self {
            Input::URI(_) => "URI".to_string(),
            Input::Test(_) => "Test".to_string(),
            Input::Fake(_) => "Fake".to_string(),
        }
    }

    pub fn link(
        &mut self,
        pipeline: gst::Pipeline,
        audio: gst::Element,
        video: gst::Element,
    ) -> Result<()> {
        match self {
            Input::URI(input) => input.link(pipeline, audio, video),
            Input::Test(input) => input.link(pipeline, audio, video),
            Input::Fake(input) => input.link(pipeline, audio, video),
        }
    }

    pub fn unlink(&self) -> Result<()> {
        match self {
            Input::URI(input) => input.unlink(),
            Input::Test(input) => input.unlink(),
            Input::Fake(input) => input.unlink(),
        }
    }

    pub fn set_state(&mut self, state: gst::State) -> Result<()> {
        match self {
            Input::URI(input) => input.set_state(state),
            Input::Test(input) => input.set_state(state),
            Input::Fake(input) => input.set_state(state),
        }
    }

    pub fn set_volume(&mut self, volume: f64, update_config: bool) -> Result<()> {
        match self {
            Input::URI(input) => input.set_volume(volume, update_config),
            Input::Test(input) => input.set_volume(volume, update_config),
            Input::Fake(input) => input.set_volume(volume, update_config),
        }
    }

    pub fn set_zorder(&mut self, zorder: u32, update_config: bool) -> Result<()> {
        match self {
            Input::URI(input) => input.set_zorder(zorder, update_config),
            Input::Test(input) => input.set_zorder(zorder, update_config),
            Input::Fake(input) => input.set_zorder(zorder, update_config),
        }
    }

    pub fn set_width(&mut self, width: i32, update_config: bool) -> Result<()> {
        match self {
            Input::URI(input) => input.set_width(width, update_config),
            Input::Test(input) => input.set_width(width, update_config),
            Input::Fake(input) => input.set_width(width, update_config),
        }
    }

    pub fn set_height(&mut self, height: i32, update_config: bool) -> Result<()> {
        match self {
            Input::URI(input) => input.set_height(height, update_config),
            Input::Test(input) => input.set_height(height, update_config),
            Input::Fake(input) => input.set_height(height, update_config),
        }
    }

    pub fn set_xpos(&mut self, xpos: i32, update_config: bool) -> Result<()> {
        match self {
            Input::URI(input) => input.set_xpos(xpos, update_config),
            Input::Test(input) => input.set_xpos(xpos, update_config),
            Input::Fake(input) => input.set_xpos(xpos, update_config),
        }
    }

    pub fn set_ypos(&mut self, ypos: i32, update_config: bool) -> Result<()> {
        match self {
            Input::URI(input) => input.set_ypos(ypos, update_config),
            Input::Test(input) => input.set_ypos(ypos, update_config),
            Input::Fake(input) => input.set_ypos(ypos, update_config),
        }
    }

    pub fn set_alpha(&mut self, alpha: f64, update_config: bool) -> Result<()> {
        match self {
            Input::URI(input) => input.set_alpha(alpha, update_config),
            Input::Test(input) => input.set_alpha(alpha, update_config),
            Input::Fake(input) => input.set_alpha(alpha, update_config),
        }
    }

    pub fn config(&self) -> Config {
        match self {
            Input::URI(input) => input.config(),
            Input::Test(input) => input.config(),
            Input::Fake(input) => input.config(),
        }
    }
}

fn set_peer_pad_property(pad: &gst::Pad, property: &str, value: &dyn ToValue) -> Result<()> {
    let peer_pad = pad
        .get_peer()
        .ok_or_else(|| MixerError::Gstreamer("Could not retrieve peer pad".to_string()))?;

    peer_pad.set_property(property, value)?;
    Ok(())
}

fn release_request_pad(elem: &gst::Element) -> Result<()> {
    let pad = elem.get_static_pad("src").ok_or_else(|| {
        MixerError::Gstreamer("Failed to get static src pad for element".to_string())
    })?;
    if pad.is_linked() {
        let peer_pad = pad.get_peer().ok_or_else(|| {
            MixerError::Gstreamer("Could not retrieve peer pad for src element".to_string())
        })?;
        peer_pad
            .get_parent_element()
            .ok_or_else(|| {
                MixerError::Gstreamer("Failed to get parent element for peer pad".to_string())
            })?
            .release_request_pad(&peer_pad);
    }

    Ok(())
}
