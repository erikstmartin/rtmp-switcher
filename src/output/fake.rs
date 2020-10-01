use crate::Result;
use gst::prelude::*;
use gstreamer as gst;

pub struct Fake {
    pub name: String,
    pipeline: Option<gst::Pipeline>,
    audio: gst::Element,
    video: gst::Element,
}

impl Fake {
    pub fn new(name: &str) -> Result<super::Output> {
        let audio =
            gst::ElementFactory::make("fakesink", Some(format!("{}_audio_sink", name).as_str()))?;

        let video =
            gst::ElementFactory::make("fakesink", Some(format!("{}_video_sink", name).as_str()))?;

        Ok(super::Output::Fake(Fake {
            name: name.to_string(),
            pipeline: None,
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

        gst::Element::link_many(&[&audio, &self.audio])?;
        gst::Element::link_many(&[&video, &self.video])?;

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
}
