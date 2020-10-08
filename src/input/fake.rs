use crate::gst_create_element;
use crate::mixer;
use crate::Result;

use gst::prelude::*;
use gstreamer as gst;

pub struct Fake {
    pub name: String,
    pipeline: Option<gst::Pipeline>,
    config: mixer::Config,
    audio: gst::Element,
    video: gst::Element,
}

impl Fake {
    pub fn new(config: mixer::Config) -> Result<super::Input> {
        let audio = gst_create_element(
            "fakesrc",
            format!("input_{}_audio_src", config.name).as_str(),
        )?;
        audio.set_property("is-live", &true)?;

        let video = gst_create_element(
            "fakesrc",
            format!("input_{}_video_src", config.name).as_str(),
        )?;
        video.set_property("is-live", &true)?;

        Ok(super::Input::Fake(Fake {
            name: config.name.to_string(),
            pipeline: None,
            config,
            audio,
            video,
        }))
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn link(
        &mut self,
        pipeline: gst::Pipeline,
        audio: gst::Element,
        video: gst::Element,
    ) -> Result<()> {
        pipeline.add_many(&[&self.audio, &self.video])?;

        self.pipeline = Some(pipeline);

        gst::Element::link_many(&[&self.audio, &audio])?;
        gst::Element::link_many(&[&self.video, &video])?;
        Ok(())
    }

    pub fn unlink(&self) -> Result<()> {
        super::release_request_pad(&self.audio)?;
        super::release_request_pad(&self.video)?;

        self.pipeline
            .as_ref()
            .unwrap()
            .remove_many(&[&self.audio, &self.video])?;
        Ok(())
    }

    pub fn set_state(&mut self, state: gst::State) -> Result<()> {
        self.audio.set_state(state)?;
        self.video.set_state(state)?;
        Ok(())
    }

    pub fn set_volume(&mut self, _volume: f64) -> Result<()> {
        Ok(())
    }

    pub fn set_zorder(&mut self, zorder: u32) -> Result<()> {
        super::set_peer_pad_property(
            &self.video.get_static_pad("src").unwrap(),
            "zorder",
            &zorder,
        )?;

        Ok(())
    }

    pub fn set_width(&mut self, _width: i32) -> Result<()> {
        Ok(())
    }

    pub fn set_height(&mut self, _height: i32) -> Result<()> {
        Ok(())
    }

    pub fn set_xpos(&mut self, _xpos: i32) -> Result<()> {
        Ok(())
    }

    pub fn set_ypos(&mut self, _ypos: i32) -> Result<()> {
        Ok(())
    }

    pub fn set_alpha(&mut self, _alpha: f64) -> Result<()> {
        Ok(())
    }

    pub fn config(&self) -> mixer::Config {
        self.config.clone()
    }
}
