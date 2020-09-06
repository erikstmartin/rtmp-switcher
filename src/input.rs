extern crate gstreamer as gst;
use gst::prelude::*;

pub struct Input {
    pub name: String,
    pub pipeline: gst::Pipeline,
    audio: gst::Element,
    video: gst::Element,
}

impl Input {
    pub fn from_uri(name: &str, uri: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let pipeline = gst::Pipeline::new(Some(name));
        let source =
            gst::ElementFactory::make("uridecodebin", Some(format!("{}_source", name).as_str()))?;
        source.set_property("uri", &uri)?;

        let videoconvert = gst::ElementFactory::make(
            "videoconvert",
            Some(format!("{}_videoconvert", name).as_str()),
        )?;
        let intervideosink = gst::ElementFactory::make(
            "intervideosink",
            Some(format!("{}_intervideosink", name).as_str()),
        )?;
        intervideosink.set_property("channel", &format!("{}_video_channel", name))?;
        let videoqueue =
            gst::ElementFactory::make("queue", Some(format!("{}_videoqueue", name).as_str()))?;

        let audioconvert = gst::ElementFactory::make(
            "audioconvert",
            Some(format!("{}_audioconvert", name).as_str()),
        )?;
        let interaudiosink = gst::ElementFactory::make(
            "interaudiosink",
            Some(format!("{}_interaudiosink", name).as_str()),
        )?;
        interaudiosink.set_property("channel", &format!("{}_audio_channel", name))?;
        let audioqueue =
            gst::ElementFactory::make("queue", Some(format!("{}_audioqueue", name).as_str()))?;

        pipeline.add_many(&[
            &source,
            &videoconvert,
            &intervideosink,
            &videoqueue,
            &audioconvert,
            &interaudiosink,
            &audioqueue,
        ])?;

        gst::Element::link_many(&[&audioconvert, &audioqueue, &interaudiosink])?;
        gst::Element::link_many(&[&videoconvert, &videoqueue, &intervideosink])?;

        source.connect_pad_added(move |src, src_pad| {
            println!(
                "Received new pad {} from {}",
                src_pad.get_name(),
                src.get_name()
            );

            let new_pad_caps = src_pad
                .get_current_caps()
                .expect("Failed to get caps of new pad.");
            let new_pad_struct = new_pad_caps
                .get_structure(0)
                .expect("Failed to get first structure of caps.");
            let new_pad_type = new_pad_struct.get_name();

            if new_pad_type.starts_with("audio/x-raw") {
                let sink_pad = audioconvert
                    .get_static_pad("sink")
                    .expect("Failed to get sink pad from audio mixer");
                if sink_pad.is_linked() {
                    println!("We are already linked. Ignoring.");
                    return;
                }

                let res = src_pad.link(&sink_pad);
                if res.is_err() {
                    dbg!(res);
                    println!("Type is {} but link failed.", new_pad_type);
                } else {
                    println!("Link succeeded (type {}).", new_pad_type);
                }
            } else if new_pad_type.starts_with("video/x-raw") {
                let sink_pad = videoconvert
                    .get_static_pad("sink")
                    .expect("Failed to get static sink pad from video_mixer");
                if sink_pad.is_linked() {
                    println!("We are already linked. Ignoring.");
                    return;
                }

                let res = src_pad.link(&sink_pad);
                if res.is_err() {
                    println!("Type is {} but link failed.", new_pad_type);
                } else {
                    println!("Link succeeded (type {}).", new_pad_type);
                }
            }
        });

        Ok(Self {
            name: name.to_string(),
            pipeline,
            audio: interaudiosink,
            video: intervideosink,
        })
    }
}
