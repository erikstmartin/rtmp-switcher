use crate::gst_create_element;
use crate::Result;
use gst::prelude::*;
use gstreamer as gst;

pub struct Auto {
    pub name: String,
    pipeline: Option<gst::Pipeline>,
    audioqueue: gst::Element,
    videoqueue: gst::Element,
    video_convert: gst::Element,
    video_scale: gst::Element,
    video_rate: gst::Element,
    video_capsfilter: gst::Element,
    videosink: gst::Element,
    videosink_queue: gst::Element,
    audiosink: gst::Element,
}

impl Auto {
    pub fn create(name: &str) -> Result<Self> {
        let videoqueue =
            gst_create_element("queue", format!("output_{}_video_queue", name).as_str())?;
        let video_convert = gst_create_element(
            "videoconvert",
            format!("output_{}_video_convert", name).as_str(),
        )?;
        let video_scale = gst_create_element(
            "videoscale",
            format!("output_{}_video_scale", name).as_str(),
        )?;
        let video_rate =
            gst_create_element("videorate", format!("output_{}_video_rate", name).as_str())?;
        let video_capsfilter = gst_create_element(
            "capsfilter",
            format!("output_{}_video_capsfilter", name).as_str(),
        )?;

        let video_caps = gst::Caps::builder("video/x-raw")
            .field("framerate", &gst::Fraction::new(30, 1))
            .field("format", &"I420")
            .field("profile", &"high")
            .build();
        video_capsfilter.set_property("caps", &video_caps).unwrap();

        let videosink_queue =
            gst_create_element("queue", format!("output_{}_videosink_queue", name).as_str())?;
        let videosink = gst_create_element(
            "autovideosink",
            format!("output_{}_video_sink", name).as_str(),
        )?;

        let audioqueue =
            gst_create_element("queue", format!("output_{}_audio_queue", name).as_str())?;
        let audiosink = gst_create_element(
            "autoaudiosink",
            format!("output_{}_audio_sink", name).as_str(),
        )?;

        Ok(Self {
            name: name.to_string(),
            pipeline: None,
            audioqueue,
            audiosink,
            videoqueue,
            video_convert,
            video_rate,
            video_scale,
            video_capsfilter,
            videosink_queue,
            videosink,
        })
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
            &self.audioqueue,
            &self.audiosink,
            &self.videoqueue,
            &self.video_convert,
            &self.video_scale,
            &self.video_rate,
            &self.video_capsfilter,
            &self.videosink_queue,
            &self.videosink,
        ])?;

        self.pipeline = Some(pipeline);

        gst::Element::link_many(&[&audio, &self.audioqueue, &self.audiosink])?;
        gst::Element::link_many(&[
            &video,
            &self.videoqueue,
            &self.video_convert,
            &self.video_scale,
            &self.video_rate,
            &self.video_capsfilter,
            &self.videosink_queue,
            &self.videosink,
        ])?;

        Ok(())
    }

    pub fn unlink(&self) -> Result<()> {
        super::release_request_pad(&self.audioqueue)?;
        super::release_request_pad(&self.videoqueue)?;

        self.pipeline.as_ref().unwrap().remove_many(&[
            &self.audioqueue,
            &self.audiosink,
            &self.videoqueue,
            &self.video_convert,
            &self.video_scale,
            &self.video_rate,
            &self.video_capsfilter,
            &self.videosink_queue,
            &self.videosink,
        ])?;

        Ok(())
    }

    pub fn set_state(&mut self, state: gst::State) -> Result<()> {
        self.videoqueue.set_state(state)?;
        self.video_convert.set_state(state)?;
        self.video_scale.set_state(state)?;
        self.video_rate.set_state(state)?;
        self.video_capsfilter.set_state(state)?;
        self.videosink_queue.set_state(state)?;
        self.videosink.set_state(state)?;
        self.audioqueue.set_state(state)?;
        self.audiosink.set_state(state)?;
        Ok(())
    }
}
