use crate::Result;
use gst::prelude::*;
use gstreamer as gst;

pub enum Input {
    URI(URI),
    Test(Test),
    Fake(Fake),
}

impl Input {
    pub fn from_uri(name: &str, uri: &str) -> Input {
        URI::new(name, uri).unwrap()
    }

    pub fn name(&self) -> String {
        match self {
            Input::URI(input) => input.name(),
            Input::Test(input) => input.name(),
            Input::Fake(input) => input.name(),
        }
    }

    pub fn location(&self) -> String {
        match self {
            Input::URI(input) => input.location.clone(),
            Input::Test(_) => "".to_string(),
            Input::Fake(_) => "".to_string(),
        }
    }

    pub fn input_type(&self) -> String {
        match self {
            Input::URI(_) => "URI".to_string(),
            Input::Test(_) => "Test".to_string(),
            Input::Fake(_) => "Fake".to_string(),
        }
    }

    pub fn link(
        &mut self,
        pipeline: gst::Pipeline,
        audio: gst::Element,
        video: gst::Element,
    ) -> Result<()> {
        match self {
            Input::URI(input) => input.link(pipeline, audio, video),
            Input::Test(input) => input.link(pipeline, audio, video),
            Input::Fake(input) => input.link(pipeline, audio, video),
        }
    }

    pub fn unlink(&self) -> Result<()> {
        match self {
            Input::URI(input) => input.unlink(),
            Input::Test(input) => input.unlink(),
            Input::Fake(input) => input.unlink(),
        }
    }
}

pub struct URI {
    pub name: String,
    pub location: String,
    pipeline: Option<gst::Pipeline>,
    source: gst::Element,
    audioconvert: gst::Element,
    audioresample: gst::Element,
    audioqueue: gst::Element,
    videoconvert: gst::Element,
    videoqueue: gst::Element,
}

impl URI {
    pub fn new(name: &str, uri: &str) -> Result<Input> {
        let source =
            gst::ElementFactory::make("uridecodebin", Some(format!("{}_source", name).as_str()))?;
        source.set_property("uri", &uri)?;

        let videoconvert = gst::ElementFactory::make(
            "videoconvert",
            Some(format!("{}_videoconvert", name).as_str()),
        )?;
        let videoqueue =
            gst::ElementFactory::make("queue2", Some(format!("{}_videoqueue", name).as_str()))?;

        let audioconvert = gst::ElementFactory::make(
            "audioconvert",
            Some(format!("{}_audioconvert", name).as_str()),
        )?;
        let audioresample = gst::ElementFactory::make(
            "audioresample",
            Some(format!("{}_audioresample", name).as_str()),
        )?;
        let audioqueue =
            gst::ElementFactory::make("queue2", Some(format!("{}_audioqueue", name).as_str()))?;

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

        Ok(Input::URI(Self {
            name: name.to_string(),
            location: name.to_string(),
            pipeline: None,
            source,
            audioconvert,
            audioresample,
            audioqueue,
            videoconvert,
            videoqueue,
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
            &self.source,
            &self.audioconvert,
            &self.audioresample,
            &self.audioqueue,
            &self.videoconvert,
            &self.videoqueue,
        ])?;

        self.pipeline = Some(pipeline);

        gst::Element::link_many(&[
            &self.audioconvert,
            &self.audioresample,
            &self.audioqueue,
            &audio,
        ])?;
        gst::Element::link_many(&[&self.videoconvert, &self.videoqueue, &video])?;

        Ok(())
    }

    fn unlink(&self) -> Result<()> {
        self.pipeline.as_ref().unwrap().remove_many(&[
            &self.source,
            &self.audioconvert,
            &self.audioresample,
            &self.audioqueue,
            &self.videoconvert,
            &self.videoqueue,
        ])?;

        Ok(())
    }
}

pub struct Test {
    pub name: String,
    pipeline: Option<gst::Pipeline>,
    audio: gst::Element,
    video: gst::Element,
}

impl Test {
    pub fn new(name: &str) -> Result<Input> {
        let audio = gst::ElementFactory::make(
            "audiotestsrc",
            Some(format!("{}_audio_source", name).as_str()),
        )?;

        let video = gst::ElementFactory::make(
            "videotestsrc",
            Some(format!("{}_video_source", name).as_str()),
        )?;

        Ok(Input::Test(Test {
            name: name.to_string(),
            pipeline: None,
            audio,
            video,
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
        pipeline.add_many(&[&self.audio, &self.video])?;

        self.pipeline = Some(pipeline);

        gst::Element::link_many(&[&self.audio, &audio])?;
        gst::Element::link_many(&[&self.video, &video])?;

        Ok(())
    }

    fn unlink(&self) -> Result<()> {
        self.pipeline
            .as_ref()
            .unwrap()
            .remove_many(&[&self.audio, &self.video])?;
        Ok(())
    }
}

pub struct Fake {
    pub name: String,
    pipeline: Option<gst::Pipeline>,
    audio: gst::Element,
    video: gst::Element,
}

impl Fake {
    pub fn new(name: &str) -> Result<Input> {
        let audio =
            gst::ElementFactory::make("fakesrc", Some(format!("{}_audio_source", name).as_str()))?;
        audio.set_property("is-live", &true)?;

        let video =
            gst::ElementFactory::make("fakesrc", Some(format!("{}_video_source", name).as_str()))?;
        video.set_property("is-live", &true)?;

        Ok(Input::Fake(Fake {
            name: name.to_string(),
            pipeline: None,
            audio,
            video,
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
        pipeline.add_many(&[&self.audio, &self.video])?;

        self.pipeline = Some(pipeline);

        gst::Element::link_many(&[&self.audio, &audio])?;
        gst::Element::link_many(&[&self.video, &video])?;

        Ok(())
    }

    fn unlink(&self) -> Result<()> {
        self.pipeline
            .as_ref()
            .unwrap()
            .remove_many(&[&self.audio, &self.video])?;
        Ok(())
    }
}
