extern crate gstreamer as gst;
use gst::prelude::*;

type Error = Box<dyn std::error::Error>;

pub trait Input {
    fn name(&self) -> String;
    fn link(
        &mut self,
        pipeline: gst::Pipeline,
        audio: gst::Element,
        video: gst::Element,
    ) -> Result<(), Error>;
    fn unlink(&self) -> Result<(), Error>;
}

pub struct URI {
    pub name: String,
    source: gst::Element,
    audioconvert: gst::Element,
    audioresample: gst::Element,
    audioqueue: gst::Element,
    videoconvert: gst::Element,
    videoqueue: gst::Element,
}

impl URI {
    pub fn new(name: &str, uri: &str) -> Result<Box<dyn Input>, Box<dyn std::error::Error>> {
        let source =
            gst::ElementFactory::make("uridecodebin", Some(format!("{}_source", name).as_str()))?;
        source.set_property("uri", &uri)?;

        let videoconvert = gst::ElementFactory::make(
            "videoconvert",
            Some(format!("{}_videoconvert", name).as_str()),
        )?;
        let videoqueue =
            gst::ElementFactory::make("queue", Some(format!("{}_videoqueue", name).as_str()))?;

        let audioconvert = gst::ElementFactory::make(
            "audioconvert",
            Some(format!("{}_audioconvert", name).as_str()),
        )?;
        let audioresample = gst::ElementFactory::make(
            "audioresample",
            Some(format!("{}_audioresample", name).as_str()),
        )?;
        let audioqueue =
            gst::ElementFactory::make("queue", Some(format!("{}_audioqueue", name).as_str()))?;

        let audio = audioconvert.clone();
        let video = videoconvert.clone();
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
                let sink_pad = audio
                    .get_static_pad("sink")
                    .expect("Failed to get sink pad from audio mixer");
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
            } else if new_pad_type.starts_with("video/x-raw") {
                let sink_pad = video
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

        Ok(Box::new(Self {
            name: name.to_string(),
            source,
            audioconvert,
            audioresample,
            audioqueue,
            videoconvert,
            videoqueue,
        }))
    }
}

impl Input for URI {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn link(
        &mut self,
        pipeline: gst::Pipeline,
        audio: gst::Element,
        video: gst::Element,
    ) -> Result<(), Box<dyn std::error::Error>> {
        pipeline.add_many(&[
            &self.source,
            &self.audioconvert,
            &self.audioresample,
            &self.audioqueue,
            &self.videoconvert,
            &self.videoqueue,
        ])?;

        gst::Element::link_many(&[
            &self.audioconvert,
            &self.audioresample,
            &self.audioqueue,
            &audio,
        ])?;
        gst::Element::link_many(&[&self.videoconvert, &self.videoqueue, &video])?;

        return Ok(());
    }

    fn unlink(&self) -> Result<(), Error> {
        unimplemented!()
    }
}
