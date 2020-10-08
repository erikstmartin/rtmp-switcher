use crate::gst_create_element;
use crate::mixer;
use crate::Result;

use gst::prelude::*;
use gstreamer as gst;

pub struct Test {
    pub name: String,
    pipeline: Option<gst::Pipeline>,
    config: mixer::Config,
    audio: gst::Element,
    audio_convert: gst::Element,
    audio_resample: gst::Element,
    audio_queue: gst::Element,
    video: gst::Element,
    video_convert: gst::Element,
    video_scale: gst::Element,
    video_rate: gst::Element,
    video_capsfilter: gst::Element,
}

impl Test {
    // TODO: Change element names to use name from config
    pub fn new(config: mixer::Config) -> Result<super::Input> {
        let video = gst_create_element(
            "videotestsrc",
            format!("input_{}_videotestsrc", config.name).as_str(),
        )?;
        video.set_property_from_str("pattern", "black");
        video.set_property("is-live", &true)?;

        let video_convert = gst_create_element(
            "videoconvert",
            format!("input_{}_video_convert", config.name).as_str(),
        )?;
        let video_scale = gst_create_element(
            "videoscale",
            format!("input_{}_video_scale", config.name).as_str(),
        )?;
        let video_rate = gst_create_element(
            "videorate",
            format!("input_{}_video_rate", config.name).as_str(),
        )?;
        let video_capsfilter = gst_create_element(
            "capsfilter",
            format!("input_{}_video_capsfilter", config.name).as_str(),
        )?;
        let video_caps = gst::Caps::builder("video/x-raw")
            .field(
                "framerate",
                &gst::Fraction::new(config.video.framerate.unwrap(), 1),
            )
            .field("width", &config.video.width.unwrap())
            .field("height", &config.video.height.unwrap())
            .field("format", &config.video.format.clone().unwrap().as_str())
            .build();
        video_capsfilter.set_property("caps", &video_caps).unwrap();

        let audio = gst_create_element(
            "audiotestsrc",
            format!("input_{}_audiotestsrc", config.name).as_str(),
        )?;
        audio.set_property("volume", &config.audio.volume.unwrap())?;
        audio.set_property("is-live", &true)?;
        let audio_queue = gst_create_element(
            "queue",
            format!("input_{}_audio_queue", config.name).as_str(),
        )?;
        let audio_convert = gst_create_element(
            "audioconvert",
            format!("input_{}_audio_convert", config.name).as_str(),
        )?;
        let audio_resample = gst_create_element(
            "audioresample",
            format!("input_{}_audio_resample", config.name).as_str(),
        )?;

        Ok(super::Input::Test(Test {
            name: config.name.clone(),
            pipeline: None,
            config,
            audio,
            audio_queue,
            audio_resample,
            audio_convert,
            video,
            video_convert,
            video_rate,
            video_scale,
            video_capsfilter,
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
        pipeline.add_many(&[
            &self.video,
            &self.video_convert,
            &self.video_scale,
            &self.video_rate,
            &self.video_capsfilter,
            &self.audio,
            &self.audio_convert,
            &self.audio_resample,
            &self.audio_queue,
        ])?;

        self.pipeline = Some(pipeline);

        // Link video elements
        gst::Element::link_many(&[
            &self.video,
            &self.video_convert,
            &self.video_scale,
            &self.video_rate,
            &self.video_capsfilter,
            &video,
        ])?;

        // Link audio elements
        gst::Element::link_many(&[
            &self.audio,
            &self.audio_convert,
            &self.audio_resample,
            &self.audio_queue,
            &audio,
        ])?;

        Ok(())
    }

    pub fn unlink(&self) -> Result<()> {
        super::release_request_pad(&self.audio)?;
        super::release_request_pad(&self.video)?;

        self.pipeline.as_ref().unwrap().remove_many(&[
            &self.video,
            &self.video_convert,
            &self.video_scale,
            &self.video_rate,
            &self.video_capsfilter,
            &self.audio,
            &self.audio_convert,
            &self.audio_resample,
            &self.audio_queue,
        ])?;
        Ok(())
    }

    pub fn set_state(&mut self, state: gst::State) -> Result<()> {
        self.audio.set_state(state)?;
        self.audio_convert.set_state(state)?;
        self.audio_resample.set_state(state)?;
        self.audio_queue.set_state(state)?;
        self.video.set_state(state)?;
        self.video_convert.set_state(state)?;
        self.video_scale.set_state(state)?;
        self.video_rate.set_state(state)?;
        self.video_capsfilter.set_state(state)?;
        Ok(())
    }

    pub fn set_volume(&mut self, _volume: f64) -> Result<()> {
        Ok(())
    }

    pub fn set_zorder(&mut self, zorder: u32) -> Result<()> {
        super::set_peer_pad_property(
            &self.video_capsfilter.get_static_pad("src").unwrap(),
            "zorder",
            &zorder,
        )?;

        Ok(())
    }

    pub fn set_width(&mut self, width: i32) -> Result<()> {
        todo!()
    }

    pub fn set_height(&mut self, height: i32) -> Result<()> {
        todo!()
    }

    pub fn set_xpos(&mut self, xpos: i32) -> Result<()> {
        todo!()
    }

    pub fn set_ypos(&mut self, ypos: i32) -> Result<()> {
        todo!()
    }

    pub fn set_alpha(&mut self, ypos: f64) -> Result<()> {
        todo!()
    }

    pub fn config(&self) -> mixer::Config {
        self.config.clone()
    }
}
