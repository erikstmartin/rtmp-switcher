use super::Config;
use crate::{gst_create_element, Result, VideoEncoder, VideoEncoderProfile, VideoEncoderSpeed};
use gst::prelude::*;
use gstreamer as gst;

pub struct File {
    pub name: String,
    pub location: String,
    pipeline: Option<gst::Pipeline>,
    video_queue: gst::Element,
    video_convert: gst::Element,
    video_scale: gst::Element,
    video_rate: gst::Element,
    video_capsfilter: gst::Element,
    video_encoder: gst::Element,
    encoder_parse: Option<gst::Element>,
    mux_queue: gst::Element,
    output_mux: gst::Element,
    queue_sink: gst::Element,
    video_sink: gst::Element,

    audio_queue: gst::Element,
    audio_convert: gst::Element,
    audio_resample: gst::Element,
    audioenc: gst::Element,
}

impl File {
    pub fn create(config: Config, location: &str) -> Result<Self> {
        let Config { name, .. } = config;
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
            .field("framerate", &gst::Fraction::new(config.video.framerate, 1))
            .field("format", &config.video.format.to_string())
            .field(
                "profile",
                &config
                    .encoder
                    .video
                    .profile
                    .unwrap_or(VideoEncoderProfile::High)
                    .to_string(),
            )
            .field(
                "speed",
                &config
                    .encoder
                    .video
                    .speed
                    .unwrap_or(VideoEncoderSpeed::None)
                    .to_string(),
            )
            .build();
        video_capsfilter.set_property("caps", &video_caps)?;

        let video_encoder = gst_create_element(
            &config.encoder.video.encoder.to_string(),
            &format!("output_{}_video_{}", name, config.encoder.video.encoder),
        )?;

        let encoder_parse = match config.encoder.video.encoder {
            VideoEncoder::H264 | VideoEncoder::NVENC => Some(gst_create_element(
                "h264parse",
                &format!("output_{}_video_parse", name),
            )?),
            _ => None,
        };

        let mux_queue =
            gst_create_element("queue", &format!("output_{}_video_output_queue", name))?;
        let output_mux = gst_create_element("matroskamux", &format!("output_{}_output_mux", name))?;
        output_mux.set_property_from_str("streamable", "true");

        let queue_sink = gst_create_element("queue", &format!("output_{}_rtmp_queuesink", name))?;
        let video_sink = gst_create_element("filesink", &format!("output_{}_file_sink", name))?;
        // TODO: Configure recording directory, also use timestamp
        video_sink.set_property("location", &location)?;

        // Audio stream
        let audio_queue = gst_create_element("queue", &format!("output_{}_audio_queue", name))?;
        let audio_convert =
            gst_create_element("audioconvert", &format!("output_{}_audio_convert", name))?;
        let audio_resample =
            gst_create_element("audioresample", &format!("output_{}_audio_resample", name))?;
        let audioenc =
            gst_create_element("fdkaacenc", &format!("output_{}_audio_fdkaacenc", name))?;

        Ok(Self {
            name,
            location: location.to_string(),
            pipeline: None,
            video_queue,
            video_convert,
            video_scale,
            video_rate,
            video_capsfilter,
            video_encoder,
            encoder_parse,
            mux_queue,
            output_mux,
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
            &self.video_encoder,
            &self.mux_queue,
            &self.output_mux,
            &self.queue_sink,
            &self.video_sink,
        ])?;

        if let Some(encoder_parse) = self.encoder_parse.as_ref() {
            pipeline.add(encoder_parse)?;
        }

        gst::Element::link_many(&[
            &video,
            &self.video_queue,
            &self.video_convert,
            &self.video_scale,
            &self.video_rate,
            &self.video_capsfilter,
            &self.video_encoder,
        ])?;

        // We only need to add the encoder_parse to the pipeline when we are using h264
        if let Some(encoder_parse) = self.encoder_parse.as_ref() {
            gst::Element::link_many(&[&self.video_encoder, encoder_parse, &self.mux_queue])?;
        } else {
            gst::Element::link_many(&[&self.video_encoder, &self.mux_queue])?;
        }

        gst::Element::link_many(&[
            &self.mux_queue,
            &self.output_mux,
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
            &self.output_mux,
        ])?;

        self.pipeline = Some(pipeline);

        Ok(())
    }

    pub fn unlink(&self) -> Result<()> {
        super::release_request_pad(&self.audio_queue)?;
        super::release_request_pad(&self.video_queue)?;

        if let Some(pipeline) = self.pipeline.as_ref() {
            pipeline.remove_many(&[
                &self.video_queue,
                &self.video_convert,
                &self.video_scale,
                &self.video_rate,
                &self.video_capsfilter,
                &self.video_encoder,
                &self.mux_queue,
                &self.output_mux,
                &self.queue_sink,
                &self.video_sink,
            ])?;

            if let Some(encoder_parse) = self.encoder_parse.as_ref() {
                pipeline.remove(encoder_parse)?;
            }

            pipeline.remove_many(&[
                &self.audio_queue,
                &self.audio_convert,
                &self.audio_resample,
                &self.audioenc,
            ])?;
        }

        Ok(())
    }

    pub fn set_state(&mut self, state: gst::State) -> Result<()> {
        self.video_queue.set_state(state)?;
        self.video_convert.set_state(state)?;
        self.video_scale.set_state(state)?;
        self.video_rate.set_state(state)?;
        self.video_capsfilter.set_state(state)?;
        self.video_encoder.set_state(state)?;
        if let Some(encoder_parse) = &self.encoder_parse {
            encoder_parse.set_state(state)?;
        }
        self.mux_queue.set_state(state)?;
        self.output_mux.set_state(state)?;
        self.queue_sink.set_state(state)?;
        self.video_sink.set_state(state)?;

        self.audio_queue.set_state(state)?;
        self.audio_convert.set_state(state)?;
        self.audio_resample.set_state(state)?;
        self.audioenc.set_state(state)?;
        Ok(())
    }
}
