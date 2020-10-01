use crate::mixer;
use crate::Result;

use gst::prelude::*;
use gstreamer as gst;

pub struct URI {
    pub name: String,
    pub location: String,
    config: mixer::Config,
    pipeline: Option<gst::Pipeline>,
    source: gst::Element,
    audioconvert: gst::Element,
    audioresample: gst::Element,
    volume: gst::Element,
    audioqueue: gst::Element,
    videoconvert: gst::Element,
    videoscale: gst::Element,
    videorate: gst::Element,
    video_capsfilter: gst::Element,
    videoqueue: gst::Element,
}

impl URI {
    pub fn new(config: mixer::Config, uri: &str) -> Result<super::Input> {
        let name = config.name.clone();

        let source =
            gst::ElementFactory::make("uridecodebin", Some(format!("{}_source", name).as_str()))?;
        source.set_property("uri", &uri)?;

        let videoconvert = gst::ElementFactory::make(
            "videoconvert",
            Some(format!("{}_videoconvert", name).as_str()),
        )?;
        let videoscale =
            gst::ElementFactory::make("videoscale", Some(format!("{}_videoscale", name).as_str()))?;
        let videorate =
            gst::ElementFactory::make("videorate", Some(format!("{}_videorate", name).as_str()))?;
        let video_caps = gst::Caps::builder("video/x-raw")
            .field(
                "framerate",
                &gst::Fraction::new(config.video.framerate.unwrap(), 1),
            )
            .field("format", &config.video.format.clone().unwrap().as_str())
            .field("width", &config.video.width.unwrap())
            .field("height", &config.video.height.unwrap())
            .build();
        let video_capsfilter =
            gst::ElementFactory::make("capsfilter", Some(format!("{}_capsfilter", name).as_str()))?;
        video_capsfilter.set_property("caps", &video_caps).unwrap();

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

        let volume =
            gst::ElementFactory::make("volume", Some(format!("{}_audio_volume", name).as_str()))?;
        volume.set_property("volume", &config.audio.volume.unwrap())?;

        let audio = audioconvert.clone();
        let video = videoconvert.clone();
        let vqueue = videoqueue.clone();
        let video_config = config.video.clone();
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

            let running_time = video.get_current_running_time();

            if new_pad_type.starts_with("audio/x-raw") {
                let sink_pad = audio
                    .get_static_pad("sink")
                    .expect("Failed to get sink pad from audio mixer");
                if sink_pad.is_linked() {
                    println!("We are already linked. Ignoring.");
                    return;
                }

                // Offset src_pad by current running time. So that videos do not fast-forward to
                // get in sync with running time of pipeline.
                src_pad
                    .set_offset(gst::format::GenericFormattedValue::Time(running_time).get_value());

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

                // Offset src_pad by current running time. So that videos do not fast-forward to
                // get in sync with running time of pipeline.
                src_pad
                    .set_offset(gst::format::GenericFormattedValue::Time(running_time).get_value());

                let queue_pad = vqueue.get_static_pad("src").unwrap();
                if queue_pad.is_linked() {
                    let compositor_pad = queue_pad.get_peer().unwrap();

                    // Look at config
                    if let Some(zorder) = video_config.zorder {
                        compositor_pad.set_property("zorder", &zorder);
                    }

                    if let Some(alpha) = video_config.alpha {
                        compositor_pad.set_property("alpha", &alpha);
                    }

                    if let Some(xpos) = video_config.xpos {
                        compositor_pad.set_property("xpos", &xpos);
                    }

                    if let Some(ypos) = video_config.ypos {
                        compositor_pad.set_property("ypos", &ypos);
                    }

                    if let Some(repeat) = video_config.repeat {
                        compositor_pad.set_property("repeat-after-eos", &repeat);
                    }
                }

                let res = src_pad.link(&sink_pad);
                if res.is_err() {
                    println!("Type is {} but link failed.", new_pad_type);
                } else {
                    println!("Link succeeded (type {}).", new_pad_type);
                }
            }
        });

        Ok(super::Input::URI(Self {
            name: name.to_string(),
            location: name.to_string(),
            config,
            pipeline: None,
            source,
            audioconvert,
            volume,
            audioresample,
            audioqueue,
            videoconvert,
            videoscale,
            videorate,
            video_capsfilter,
            videoqueue,
        }))
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
            &self.source,
            &self.audioconvert,
            &self.volume,
            &self.audioresample,
            &self.audioqueue,
            &self.videoconvert,
            &self.videoscale,
            &self.videorate,
            &self.video_capsfilter,
            &self.videoqueue,
        ])?;

        self.pipeline = Some(pipeline);

        gst::Element::link_many(&[
            &self.audioconvert,
            &self.volume,
            &self.audioresample,
            &self.audioqueue,
            &audio,
        ])?;
        gst::Element::link_many(&[
            &self.videoconvert,
            &self.videoscale,
            &self.videorate,
            &self.video_capsfilter,
            &self.videoqueue,
            &video,
        ])?;

        let prop = self
            .videoqueue
            .get_static_pad("src")
            .unwrap()
            .get_peer()
            .unwrap()
            .get_property("zorder")?;
        let zorder = prop.downcast::<u32>().map_err(|_| mixer::Error::Unknown)?;

        self.config.video.zorder = Some(zorder.get_some());

        Ok(())
    }

    pub fn unlink(&self) -> Result<()> {
        super::release_request_pad(&self.audioqueue)?;
        super::release_request_pad(&self.videoqueue)?;

        self.pipeline.as_ref().unwrap().remove_many(&[
            &self.source,
            &self.audioconvert,
            &self.volume,
            &self.audioresample,
            &self.audioqueue,
            &self.videoconvert,
            &self.videoscale,
            &self.videorate,
            &self.video_capsfilter,
            &self.videoqueue,
        ])?;

        Ok(())
    }

    pub fn set_state(&mut self, state: gst::State) -> Result<()> {
        self.source.set_state(state)?;
        self.audioconvert.set_state(state)?;
        self.audioresample.set_state(state)?;
        self.volume.set_state(state)?;
        self.audioqueue.set_state(state)?;
        self.videoconvert.set_state(state)?;
        self.videoscale.set_state(state)?;
        self.videorate.set_state(state)?;
        self.video_capsfilter.set_state(state)?;
        self.videoqueue.set_state(state)?;
        Ok(())
    }

    pub fn set_volume(&mut self, volume: f64) -> Result<()> {
        self.config.audio.volume = Some(volume);
        self.volume.set_property("volume", &volume)?;
        Ok(())
    }

    pub fn set_zorder(&mut self, zorder: u32) -> Result<()> {
        self.config.video.zorder = Some(zorder);
        super::set_peer_pad_property(
            &self.videoqueue.get_static_pad("src").unwrap(),
            "zorder",
            &zorder,
        )?;

        Ok(())
    }

    pub fn set_width(&mut self, width: i32) -> Result<()> {
        self.config.video.width = Some(width);
        super::set_peer_pad_property(
            &self.videoqueue.get_static_pad("src").unwrap(),
            "width",
            &width,
        )?;

        Ok(())
    }

    pub fn set_height(&mut self, height: i32) -> Result<()> {
        self.config.video.height = Some(height);
        super::set_peer_pad_property(
            &self.videoqueue.get_static_pad("src").unwrap(),
            "height",
            &height,
        )?;

        Ok(())
    }

    pub fn set_xpos(&mut self, xpos: i32) -> Result<()> {
        self.config.video.xpos = Some(xpos);
        super::set_peer_pad_property(
            &self.videoqueue.get_static_pad("src").unwrap(),
            "xpos",
            &xpos,
        )?;

        Ok(())
    }

    pub fn set_ypos(&mut self, ypos: i32) -> Result<()> {
        self.config.video.ypos = Some(ypos);
        super::set_peer_pad_property(
            &self.videoqueue.get_static_pad("src").unwrap(),
            "ypos",
            &ypos,
        )?;

        Ok(())
    }

    pub fn set_alpha(&mut self, alpha: f64) -> Result<()> {
        self.config.video.alpha = Some(alpha);
        super::set_peer_pad_property(
            &self.videoqueue.get_static_pad("src").unwrap(),
            "alpha",
            &alpha,
        )?;

        Ok(())
    }

    pub fn config(&self) -> mixer::Config {
        self.config.clone()
    }
}
