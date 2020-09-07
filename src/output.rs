extern crate gstreamer as gst;
use gst::prelude::*;

type Error = Box<dyn std::error::Error>;

pub trait Output {
    fn name(&self) -> String;
    fn link(
        &mut self,
        pipeline: gst::Pipeline,
        audio: gst::Element,
        video: gst::Element,
    ) -> Result<(), Error>;
    fn unlink(&self) -> Result<(), Error>;
}

pub struct Auto {
    pub name: String,
    pipeline: Option<gst::Pipeline>,
    videosink: gst::Element,
    audiosink: gst::Element,
}

impl Auto {
    pub fn new(name: &str) -> Result<Box<dyn Output>, Box<dyn std::error::Error>> {
        let videosink = gst::ElementFactory::make(
            "autovideosink",
            Some(format!("{}_video_sink", name).as_str()),
        )?;

        let audiosink = gst::ElementFactory::make(
            "autoaudiosink",
            Some(format!("{}_audio_sink", name).as_str()),
        )?;

        Ok(Box::new(Self {
            name: name.to_string(),
            pipeline: None,
            audiosink,
            videosink,
        }))
    }
}

impl Output for Auto {
    fn name(&self) -> String {
        self.name.clone()
    }
    fn link(
        &mut self,
        pipeline: gst::Pipeline,
        audio: gst::Element,
        video: gst::Element,
    ) -> Result<(), Error> {
        pipeline.add_many(&[&self.audiosink, &self.videosink])?;
        self.pipeline = Some(pipeline);
        gst::Element::link_many(&[&audio, &self.audiosink])?;
        gst::Element::link_many(&[&video, &self.videosink])?;
        Ok(())
    }

    fn unlink(&self) -> Result<(), Error> {
        self.pipeline
            .as_ref()
            .unwrap()
            .remove_many(&[&self.audiosink, &self.videosink])?;

        Ok(())
    }
}

pub struct RTMP {
    pub name: String,
    pipeline: Option<gst::Pipeline>,
    pub video_convert: gst::Element,
    pub video_scale: gst::Element,
    pub video_rate: gst::Element,
    pub video_capsfilter: gst::Element,
    pub x264enc: gst::Element,
    pub h264parse: gst::Element,
    pub flvmux: gst::Element,
    pub video_queue: gst::Element,
    pub queue_sink: gst::Element,
    pub video_sink: gst::Element,

    pub audio_convert: gst::Element,
    pub audio_resample: gst::Element,
    pub audioenc: gst::Element,
    pub aacparse: gst::Element,
    pub audio_queue: gst::Element,
}

impl RTMP {
    pub fn new(name: &str, uri: &str) -> Result<Box<dyn Output>, Box<dyn std::error::Error>> {
        // Video stream
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

        let x264enc =
            gst::ElementFactory::make("x264enc", Some(format!("{}_x264enc", name).as_str()))?;
        x264enc.set_property("key-int-max", &60u32)?;
        // TODO: We probably want to set (or have configurable) the bitrate and Preset

        let h264parse =
            gst::ElementFactory::make("h264parse", Some(format!("{}_h264parse", name).as_str()))?;
        let video_queue =
            gst::ElementFactory::make("queue", Some(format!("{}_video_queue", name).as_str()))?;
        let flvmux =
            gst::ElementFactory::make("flvmux", Some(format!("{}_flvmux", name).as_str()))?;
        flvmux.set_property_from_str("streamable", "true");
        let queue_sink =
            gst::ElementFactory::make("queue", Some(format!("{}_queuesink", name).as_str()))?;
        let video_sink =
            gst::ElementFactory::make("rtmpsink", Some(format!("{}_video_sink", name).as_str()))?;
        video_sink.set_property("location", &uri)?;

        // Audio stream
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
        let audio_queue =
            gst::ElementFactory::make("queue", Some(format!("{}_audio_queue", name).as_str()))?;

        Ok(Box::new(Self {
            name: name.to_string(),
            pipeline: None,
            video_convert,
            video_scale,
            video_rate,
            video_capsfilter,
            x264enc,
            h264parse,
            video_queue,
            flvmux,
            queue_sink,
            video_sink,
            audioenc,
            aacparse,
            audio_convert,
            audio_resample,
            audio_queue,
        }))
    }
}

impl Output for RTMP {
    fn name(&self) -> String {
        self.name.clone()
    }
    fn link(
        &mut self,
        pipeline: gst::Pipeline,
        audio: gst::Element,
        video: gst::Element,
    ) -> Result<(), Error> {
        // Video
        pipeline.add_many(&[
            &self.video_convert,
            &self.video_scale,
            &self.video_rate,
            &self.video_capsfilter,
            &self.x264enc,
            &self.h264parse,
            &self.video_queue,
            &self.flvmux,
            &self.queue_sink,
            &self.video_sink,
        ])?;

        gst::Element::link_many(&[
            &video,
            &self.video_convert,
            &self.video_scale,
            &self.video_rate,
            &self.video_capsfilter,
            &self.x264enc,
            &self.h264parse,
            &self.video_queue,
            &self.flvmux,
            &self.queue_sink,
            &self.video_sink,
        ])?;

        // Audio
        pipeline.add_many(&[
            &self.audio_convert,
            &self.audio_resample,
            &self.audioenc,
            &self.aacparse,
            &self.audio_queue,
        ])?;

        gst::Element::link_many(&[
            &audio,
            &self.audio_convert,
            &self.audio_resample,
            &self.audioenc,
            &self.aacparse,
            &self.audio_queue,
            &self.flvmux,
        ])?;

        self.pipeline = Some(pipeline);

        Ok(())
    }

    fn unlink(&self) -> Result<(), Error> {
        let pipeline = self.pipeline.as_ref().unwrap();
        pipeline.remove_many(&[
            &self.video_convert,
            &self.video_scale,
            &self.video_rate,
            &self.video_capsfilter,
            &self.x264enc,
            &self.h264parse,
            &self.video_queue,
            &self.flvmux,
            &self.queue_sink,
            &self.video_sink,
        ])?;

        pipeline.remove_many(&[
            &self.audio_convert,
            &self.audio_resample,
            &self.audioenc,
            &self.aacparse,
            &self.audio_queue,
        ])?;

        Ok(())
    }
}
