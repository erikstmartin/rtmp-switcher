pub mod fake;
pub mod test;
pub mod uri;

use crate::mixer;
use crate::Result;

pub use fake::Fake;
pub use test::Test;
pub use uri::URI;

use gst::prelude::*;
use gstreamer as gst;

pub enum Input {
    URI(URI),
    Test(Test),
    Fake(Fake),
}

impl Input {
    pub fn from_uri(config: mixer::Config, uri: &str) -> Input {
        uri::URI::new(config, uri).unwrap()
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

    pub fn set_volume(&mut self, volume: f64) -> Result<()> {
        match self {
            Input::URI(input) => input.set_volume(volume),
            Input::Test(input) => input.set_volume(volume),
            Input::Fake(input) => input.set_volume(volume),
        }
    }

    pub fn set_zorder(&mut self, zorder: u32) -> Result<()> {
        match self {
            Input::URI(input) => input.set_zorder(zorder),
            Input::Test(input) => input.set_zorder(zorder),
            Input::Fake(input) => input.set_zorder(zorder),
        }
    }

    pub fn set_width(&mut self, width: i32) -> Result<()> {
        match self {
            Input::URI(input) => input.set_width(width),
            Input::Test(input) => input.set_width(width),
            Input::Fake(input) => input.set_width(width),
        }
    }

    pub fn set_height(&mut self, height: i32) -> Result<()> {
        match self {
            Input::URI(input) => input.set_height(height),
            Input::Test(input) => input.set_height(height),
            Input::Fake(input) => input.set_height(height),
        }
    }

    pub fn set_xpos(&mut self, xpos: i32) -> Result<()> {
        match self {
            Input::URI(input) => input.set_xpos(xpos),
            Input::Test(input) => input.set_xpos(xpos),
            Input::Fake(input) => input.set_xpos(xpos),
        }
    }

    pub fn set_ypos(&mut self, ypos: i32) -> Result<()> {
        match self {
            Input::URI(input) => input.set_ypos(ypos),
            Input::Test(input) => input.set_ypos(ypos),
            Input::Fake(input) => input.set_ypos(ypos),
        }
    }

    pub fn set_alpha(&mut self, alpha: f64) -> Result<()> {
        match self {
            Input::URI(input) => input.set_alpha(alpha),
            Input::Test(input) => input.set_alpha(alpha),
            Input::Fake(input) => input.set_alpha(alpha),
        }
    }

    pub fn config(&self) -> mixer::Config {
        match self {
            Input::URI(input) => input.config(),
            Input::Test(input) => input.config(),
            Input::Fake(input) => input.config(),
        }
    }
}

fn set_peer_pad_property(pad: &gst::Pad, property: &str, value: &dyn ToValue) -> Result<()> {
    let peer_pad = pad.get_peer().unwrap();

    peer_pad.set_property(property, value)?;
    Ok(())
}

fn release_request_pad(elem: &gst::Element) -> Result<()> {
    let pad = elem.get_static_pad("src").unwrap();
    if pad.is_linked() {
        let peer_pad = pad.get_peer().unwrap();
        peer_pad
            .get_parent_element()
            .unwrap()
            .release_request_pad(&peer_pad);
    }

    Ok(())
}
