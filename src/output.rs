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

        let videosink = gst::ElementFactory::make(
            "autovideosink",
            Some(format!("{}_video_sink", name).as_str()),
        )?;
        let intervideosrc = gst::ElementFactory::make(
            "intervideosrc",
            Some(format!("{}_intervideosrc", name).as_str()),
        )?;
        intervideosrc.set_property("channel", &format!("{}_video_channel", name))?;

        let audiosink = gst::ElementFactory::make(
            "autoaudiosink",
            Some(format!("{}_audio_sink", name).as_str()),
        )?;

        let interaudiosrc = gst::ElementFactory::make(
            "interaudiosrc",
            Some(format!("{}_interaudiosrc", name).as_str()),
        )?;
        interaudiosrc.set_property("channel", &format!("{}_audio_channel", name))?;

        // Add elements to pipeline
        pipeline.add_many(&[&audiosink, &interaudiosrc, &videosink, &intervideosrc])?;
        gst::Element::link_many(&[&interaudiosrc, &audiosink])?;
        gst::Element::link_many(&[&intervideosrc, &videosink])?;

        Ok(Self {
            name: name.to_string(),
            pipeline,
            audio: interaudiosrc,
            video: intervideosrc,
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

        let intervideosrc = gst::ElementFactory::make(
            "intervideosrc",
            Some(format!("{}_intervideosrc", name).as_str()),
        )?;
        intervideosrc.set_property("channel", &format!("{}_video_channel", name))?;

        // Audio stream
        let audio_queue =
            gst::ElementFactory::make("queue", Some(format!("{}_audio_queue", name).as_str()))?;
        let audioenc =
            gst::ElementFactory::make("fdkaacenc", Some(format!("{}_fdkaacenc", name).as_str()))?;
        let interaudiosrc = gst::ElementFactory::make(
            "interaudiosrc",
            Some(format!("{}_interaudiosrc", name).as_str()),
        )?;
        interaudiosrc.set_property("channel", &format!("{}_audio_channel", name))?;

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
            &intervideosrc,
            &interaudiosrc,
        ])?;

        // Link video elements
        gst::Element::link_many(&[
            &intervideosrc,
            &video_queue,
            &video_convert,
            &x264enc,
            &h264parse,
            &flvmux,
            &queue_sink,
            &video_sink,
        ])?;

        // Link audio elements
        gst::Element::link_many(&[&interaudiosrc, &audio_queue, &audioenc, &flvmux])?;

        Ok(Self {
            name: name.to_string(),
            pipeline,
            audio: interaudiosrc,
            video: intervideosrc,
        })
    }
}
