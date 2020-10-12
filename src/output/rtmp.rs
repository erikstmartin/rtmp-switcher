use super::Config;
use crate::gst_create_element;
use crate::Result;
use gst::prelude::*;
use gstreamer as gst;

pub struct RTMP {
    pub name: String,
    pub location: String,
    pipeline: Option<gst::Pipeline>,
    video_queue: gst::Element,
    video_convert: gst::Element,
    video_scale: gst::Element,
    video_rate: gst::Element,
    video_capsfilter: gst::Element,
    x264enc: gst::Element,
    h264parse: gst::Element,
    flvqueue: gst::Element,
    flvmux: gst::Element,
    queue_sink: gst::Element,
    video_sink: gst::Element,

    audio_queue: gst::Element,
    audio_convert: gst::Element,
    audio_resample: gst::Element,
    audioenc: gst::Element,
}

impl RTMP {
    pub fn create(config: Config, uri: &str) -> Result<Self> {
        let name = &config.name;

        // Video stream
        let video_queue = gst_create_element("queue", &format!("output_{}_video_queue", name))?;

        let video_convert =
            gst_create_element("videoconvert", &format!("output_{}_video_convert", name))?;
        let video_scale =
            gst_create_element("videoscale", &format!("output_{}_video_scale", name))?;
        let video_rate = gst_create_element("videorate", &format!("output_{}_video_rate", name))?;
        let video_capsfilter =
            gst_create_element("capsfilter", &format!("output_{}_video_capsfilter", name))?;

        let video_caps = gst::Caps::builder("video/x-raw")
            .field("framerate", &gst::Fraction::new(30, 1))
            .field("format", &"I420")
            .field("profile", &"high")
            .build();
        video_capsfilter.set_property("caps", &video_caps).unwrap();

        let x264enc = gst_create_element("nvh264enc", &format!("output_{}_video_x264enc", name))?;
        let h264parse =
            gst_create_element("h264parse", &format!("output_{}_video_h264parse", name))?;

        let flvqueue = gst_create_element("queue", &format!("output_{}_video_flvqueue", name))?;
        let flvmux = gst_create_element("flvmux", &format!("output_{}_video_flvmux", name))?;
        flvmux.set_property_from_str("streamable", "true");

        let queue_sink = gst_create_element("queue", &format!("output_{}_rtmp_queuesink", name))?;
        let video_sink = gst_create_element("rtmpsink", &format!("output_{}_rtmp_sink", name))?;
        video_sink.set_property("location", &uri)?;

        // Audio stream
        let audio_queue = gst_create_element("queue", &format!("output_{}_audio_queue", name))?;
        let audio_convert =
            gst_create_element("audioconvert", &format!("output_{}_audio_convert", name))?;
        let audio_resample =
            gst_create_element("audioresample", &format!("output_{}_audio_resample", name))?;
        let audioenc =
            gst_create_element("fdkaacenc", &format!("output_{}_audio_fdkaacenc", name))?;

        Ok(Self {
            name: name.to_string(),
            location: uri.to_string(),
            pipeline: None,
            video_queue,
            video_convert,
            video_scale,
            video_rate,
            video_capsfilter,
            x264enc,
            h264parse,
            flvqueue,
            flvmux,
            queue_sink,
            video_sink,
            audio_queue,
            audio_convert,
            audio_resample,
            audioenc,
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
        // Video
        pipeline.add_many(&[
            &self.video_queue,
            &self.video_convert,
            &self.video_scale,
            &self.video_rate,
            &self.video_capsfilter,
            &self.x264enc,
            &self.h264parse,
            &self.flvqueue,
            &self.flvmux,
            &self.queue_sink,
            &self.video_sink,
        ])?;

        gst::Element::link_many(&[
            &video,
            &self.video_queue,
            &self.video_convert,
            &self.video_scale,
            &self.video_rate,
            &self.video_capsfilter,
            &self.x264enc,
            &self.h264parse,
            &self.flvqueue,
            &self.flvmux,
            &self.queue_sink,
            &self.video_sink,
        ])?;

        // Audio
        pipeline.add_many(&[
            &self.audio_queue,
            &self.audio_convert,
            &self.audio_resample,
            &self.audioenc,
        ])?;

        gst::Element::link_many(&[
            &audio,
            &self.audio_queue,
            &self.audio_convert,
            &self.audio_resample,
            &self.audioenc,
            &self.flvmux,
        ])?;

        self.pipeline = Some(pipeline);

        Ok(())
    }

    pub fn unlink(&self) -> Result<()> {
        super::release_request_pad(&self.audio_queue)?;
        super::release_request_pad(&self.video_queue)?;

        let pipeline = self.pipeline.as_ref().unwrap();
        pipeline.remove_many(&[
            &self.video_queue,
            &self.video_convert,
            &self.video_scale,
            &self.video_rate,
            &self.video_capsfilter,
            &self.x264enc,
            &self.h264parse,
            &self.flvqueue,
            &self.flvmux,
            &self.queue_sink,
            &self.video_sink,
        ])?;

        pipeline.remove_many(&[
            &self.audio_queue,
            &self.audio_convert,
            &self.audio_resample,
            &self.audioenc,
        ])?;

        Ok(())
    }

    pub fn set_state(&mut self, state: gst::State) -> Result<()> {
        self.video_queue.set_state(state)?;
        self.video_convert.set_state(state)?;
        self.video_scale.set_state(state)?;
        self.video_rate.set_state(state)?;
        self.video_capsfilter.set_state(state)?;
        self.x264enc.set_state(state)?;
        self.h264parse.set_state(state)?;
        self.flvqueue.set_state(state)?;
        self.flvmux.set_state(state)?;
        self.queue_sink.set_state(state)?;
        self.video_sink.set_state(state)?;

        self.audio_queue.set_state(state)?;
        self.audio_convert.set_state(state)?;
        self.audio_resample.set_state(state)?;
        self.audioenc.set_state(state)?;
        Ok(())
    }
}
