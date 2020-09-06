extern crate gstreamer as gst;
use gst::prelude::*;

pub struct Output {
    pub name: String,
    pub pipeline: gst::Pipeline,
    pub audio: gst::Element,
    pub video: gst::Element,
}

impl Output {
    pub fn autosink(name: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let pipeline = gst::Pipeline::new(Some(name));

        let video_sink = gst::ElementFactory::make(
            "autovideosink",
            Some(format!("{}_video_sink", name).as_str()),
        )?;
        let video_intersrc = gst::ElementFactory::make(
            "intervideosrc",
            Some(format!("{}_intervideosrc", name).as_str()),
        )?;
        video_intersrc.set_property("channel", &format!("{}_video_channel", name))?;

        let audio_sink = gst::ElementFactory::make(
            "autoaudiosink",
            Some(format!("{}_audio_sink", name).as_str()),
        )?;

        let audio_intersrc = gst::ElementFactory::make(
            "interaudiosrc",
            Some(format!("{}_interaudiosrc", name).as_str()),
        )?;
        audio_intersrc.set_property("channel", &format!("{}_audio_channel", name))?;

        // Add elements to pipeline
        pipeline.add_many(&[&audio_sink, &audio_intersrc, &video_sink, &video_intersrc])?;
        gst::Element::link_many(&[&audio_intersrc, &audio_sink])?;
        gst::Element::link_many(&[&video_intersrc, &video_sink])?;

        Ok(Self {
            name: name.to_string(),
            pipeline,
            audio: audio_intersrc,
            video: video_intersrc,
        })
    }

    pub fn rtmp(name: &str, uri: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let pipeline = gst::Pipeline::new(Some(name));

        // Video stream
        let video_intersrc = gst::ElementFactory::make(
            "intervideosrc",
            Some(format!("{}_intervideosrc", name).as_str()),
        )?;
        video_intersrc.set_property("channel", &format!("{}_video_channel", name))?;
        let video_convert = gst::ElementFactory::make(
            "videoconvert",
            Some(format!("{}_videoconvert", name).as_str()),
        )?;
        let video_scale =
            gst::ElementFactory::make("videoscale", Some(format!("{}_videoscale", name).as_str()))?;
        let video_rate =
            gst::ElementFactory::make("videorate", Some(format!("{}_videorate", name).as_str()))?;
        let video_capsfilter = gst::ElementFactory::make("capsfilter", Some("video_capsfilter"))?;

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

        pipeline.add_many(&[
            &video_intersrc,
            &video_convert,
            &video_scale,
            &video_rate,
            &video_capsfilter,
            &x264enc,
            &h264parse,
            &video_queue,
            &flvmux,
            &queue_sink,
            &video_sink,
        ])?;

        // Link video elements
        gst::Element::link_many(&[
            &video_intersrc,
            &video_convert,
            &video_scale,
            &video_rate,
            &video_capsfilter,
            &x264enc,
            &h264parse,
            &video_queue,
            &flvmux,
            &queue_sink,
            &video_sink,
        ])?;

        // Audio stream
        // interaudiosrc -> audioconvert -> audioresample
        let audio_intersrc = gst::ElementFactory::make(
            "interaudiosrc",
            Some(format!("{}_interaudiosrc", name).as_str()),
        )?;
        audio_intersrc.set_property("channel", &format!("{}_audio_channel", name))?;
        let audio_convert = gst::ElementFactory::make("audioconvert", Some("audioconvert"))?;
        let audio_resample = gst::ElementFactory::make("audioresample", Some("audioresample"))?;
        let audioenc =
            gst::ElementFactory::make("fdkaacenc", Some(format!("{}_fdkaacenc", name).as_str()))?;
        let aacparse =
            gst::ElementFactory::make("aacparse", Some(format!("{}_aacparse", name).as_str()))?;
        let audio_queue =
            gst::ElementFactory::make("queue", Some(format!("{}_audio_queue", name).as_str()))?;

        // Add elements to pipeline
        pipeline.add_many(&[
            &audio_intersrc,
            &audio_convert,
            &audio_resample,
            &audioenc,
            &aacparse,
            &audio_queue,
        ])?;

        // Link audio elements
        gst::Element::link_many(&[
            &audio_intersrc,
            &audio_convert,
            &audio_resample,
            &audioenc,
            &aacparse,
            &audio_queue,
            &flvmux,
        ])?;

        Ok(Self {
            name: name.to_string(),
            pipeline,
            audio: audio_intersrc,
            video: video_intersrc,
        })
    }
}
