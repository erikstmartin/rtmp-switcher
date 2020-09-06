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
        let video_queue =
            gst::ElementFactory::make("queue", Some(format!("{}_video_queue", name).as_str()))?;
        let video_convert = gst::ElementFactory::make(
            "videoconvert",
            Some(format!("{}_videoconvert", name).as_str()),
        )?;
        let x264enc =
            gst::ElementFactory::make("x264enc", Some(format!("{}_x264enc", name).as_str()))?;
        let h264parse =
            gst::ElementFactory::make("h264parse", Some(format!("{}_h264parse", name).as_str()))?;
        let flvmux =
            gst::ElementFactory::make("flvmux", Some(format!("{}_flvmux", name).as_str()))?;
        let queue_sink =
            gst::ElementFactory::make("queue", Some(format!("{}_queuesink", name).as_str()))?;
        let video_sink =
            gst::ElementFactory::make("rtmpsink", Some(format!("{}_video_sink", name).as_str()))?;
        video_sink.set_property("location", &uri)?;
        flvmux.set_property_from_str("streamable", "true");

        let video_intersrc = gst::ElementFactory::make(
            "intervideosrc",
            Some(format!("{}_intervideosrc", name).as_str()),
        )?;
        video_intersrc.set_property("channel", &format!("{}_video_channel", name))?;

        // Audio stream
        let audio_queue =
            gst::ElementFactory::make("queue", Some(format!("{}_audio_queue", name).as_str()))?;
        let audioenc =
            gst::ElementFactory::make("fdkaacenc", Some(format!("{}_fdkaacenc", name).as_str()))?;
        let audio_intersrc = gst::ElementFactory::make(
            "interaudiosrc",
            Some(format!("{}_interaudiosrc", name).as_str()),
        )?;
        audio_intersrc.set_property("channel", &format!("{}_audio_channel", name))?;

        // Add elements to pipeline
        pipeline.add_many(&[
            &video_queue,
            &audio_queue,
            &video_convert,
            &x264enc,
            &h264parse,
            &flvmux,
            &queue_sink,
            &video_sink,
            &audioenc,
            &video_intersrc,
            &audio_intersrc,
        ])?;

        // Link video elements
        gst::Element::link_many(&[
            &video_intersrc,
            &video_queue,
            &video_convert,
            &x264enc,
            &h264parse,
            &flvmux,
            &queue_sink,
            &video_sink,
        ])?;

        // Link audio elements
        gst::Element::link_many(&[&audio_intersrc, &audio_queue, &audioenc, &flvmux])?;

        Ok(Self {
            name: name.to_string(),
            pipeline,
            audio: audio_intersrc,
            video: video_intersrc,
        })
    }
}
