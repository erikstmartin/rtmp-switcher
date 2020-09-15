use crate::Result;
use gst::prelude::*;
use gstreamer as gst;

pub enum Output {
    RTMP(RTMP),
    Auto(Auto),
}

impl Output {
    pub fn name(&mut self) -> String {
        match self {
            Output::RTMP(output) => output.name(),
            Output::Auto(output) => output.name(),
        }
    }

    pub fn link(
        &mut self,
        pipeline: gst::Pipeline,
        audio: gst::Element,
        video: gst::Element,
    ) -> Result<()> {
        match self {
            Output::RTMP(output) => output.link(pipeline, audio, video),
            Output::Auto(output) => output.link(pipeline, audio, video),
        }
    }

    pub fn unlink(&self) -> Result<()> {
        match self {
            Output::RTMP(output) => output.unlink(),
            Output::Auto(output) => output.unlink(),
        }
    }
}

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
    pub fn new(name: &str) -> Result<Output> {
        let videoqueue =
            gst::ElementFactory::make("queue", Some(format!("{}_video_queue", name).as_str()))?;
        let video_convert = gst::ElementFactory::make(
            "videoconvert",
            Some(format!("{}_videoconvert", name).as_str()),
        )?;
        let video_scale =
            gst::ElementFactory::make("videoscale", Some(format!("{}_videoscale", name).as_str()))?;
        let video_rate =
            gst::ElementFactory::make("videorate", Some(format!("{}_videorate", name).as_str()))?;
        let video_capsfilter = gst::ElementFactory::make(
            "capsfilter",
            Some(format!("{}_video_capsfilter", name).as_str()),
        )?;
        let video_caps = gst::Caps::builder("video/x-raw")
            .field("framerate", &gst::Fraction::new(30, 1))
            .build();
        video_capsfilter.set_property("caps", &video_caps).unwrap();
        let videosink_queue =
            gst::ElementFactory::make("queue", Some(format!("{}_videosink_queue", name).as_str()))?;
        let videosink = gst::ElementFactory::make(
            "autovideosink",
            Some(format!("{}_video_sink", name).as_str()),
        )?;

        let audioqueue =
            gst::ElementFactory::make("queue", Some(format!("{}_audio_queue", name).as_str()))?;
        let audiosink = gst::ElementFactory::make(
            "autoaudiosink",
            Some(format!("{}_audio_sink", name).as_str()),
        )?;

        Ok(Output::Auto(Self {
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
        }))
    }

    fn name(&self) -> String {
        self.name.clone()
    }
    fn link(
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

    fn unlink(&self) -> Result<()> {
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
}

pub struct RTMP {
    pub name: String,
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
    aacparse: gst::Element,
}

impl RTMP {
    pub fn new(name: &str, uri: &str) -> Result<Output> {
        // Video stream
        let video_queue =
            gst::ElementFactory::make("queue", Some(format!("{}_video_queue", name).as_str()))?;
        let video_convert = gst::ElementFactory::make(
            "videoconvert",
            Some(format!("{}_videoconvert", name).as_str()),
        )?;
        let video_scale =
            gst::ElementFactory::make("videoscale", Some(format!("{}_videoscale", name).as_str()))?;
        let video_rate =
            gst::ElementFactory::make("videorate", Some(format!("{}_videorate", name).as_str()))?;
        let video_capsfilter = gst::ElementFactory::make(
            "capsfilter",
            Some(format!("{}_video_capsfilter", name).as_str()),
        )?;
        let video_caps = gst::Caps::builder("video/x-raw")
            .field("framerate", &gst::Fraction::new(60, 1))
            .build();
        video_capsfilter.set_property("caps", &video_caps).unwrap();

        let x264enc =
            gst::ElementFactory::make("x264enc", Some(format!("{}_x264enc", name).as_str()))?;
        x264enc.set_property("key-int-max", &60u32)?;

        let h264parse =
            gst::ElementFactory::make("h264parse", Some(format!("{}_h264parse", name).as_str()))?;
        let flvqueue = gst::ElementFactory::make("queue", Some(format!("{}_flv", name).as_str()))?;
        let flvmux =
            gst::ElementFactory::make("flvmux", Some(format!("{}_flvmux", name).as_str()))?;
        flvmux.set_property_from_str("streamable", "true");
        let queue_sink =
            gst::ElementFactory::make("queue2", Some(format!("{}_queuesink", name).as_str()))?;
        let video_sink =
            gst::ElementFactory::make("rtmpsink", Some(format!("{}_video_sink", name).as_str()))?;
        video_sink.set_property("location", &uri)?;

        // Audio stream
        let audio_queue =
            gst::ElementFactory::make("queue", Some(format!("{}_audio_queue", name).as_str()))?;
        let audio_convert = gst::ElementFactory::make(
            "audioconvert",
            Some(format!("{}_audioconvert", name).as_str()),
        )?;
        let audio_resample = gst::ElementFactory::make(
            "audioresample",
            Some(format!("{}_audioresample", name).as_str()),
        )?;
        let audioenc =
            gst::ElementFactory::make("fdkaacenc", Some(format!("{}_fdkaacenc", name).as_str()))?;
        let aacparse =
            gst::ElementFactory::make("aacparse", Some(format!("{}_aacparse", name).as_str()))?;

        Ok(Output::RTMP(Self {
            name: name.to_string(),
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
            aacparse,
        }))
    }

    fn name(&self) -> String {
        self.name.clone()
    }
    fn link(
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
            &self.aacparse,
        ])?;

        gst::Element::link_many(&[
            &audio,
            &self.audio_queue,
            &self.audio_convert,
            &self.audio_resample,
            &self.audioenc,
            &self.aacparse,
            &self.flvmux,
        ])?;

        self.pipeline = Some(pipeline);

        Ok(())
    }

    fn unlink(&self) -> Result<()> {
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
            &self.aacparse,
        ])?;

        Ok(())
    }
}
