use super::{error, message_response, okay, Error, JsonResult};
use crate::mixer;
use crate::output::{Config as OutputConfig, EncoderConfig, Output as MixerOutput};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use warp::{http::StatusCode, Filter};

/// HTTP Request for creating a new [`output::Output`](../input/struct.Output.html)
/// to be used by the [`mixer`](../mixer/struct.Mixer.html).
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CreateRequest {
    pub name: String,
    pub output_type: String,
    pub location: String,
    pub audio: mixer::AudioConfig,
    pub video: mixer::VideoConfig,
    pub encoder: EncoderConfig,
}

impl CreateRequest {
    /// Constructs a new `CreateRequest` from a json body.
    /// This function consumes the http request body through warp::body::json().
    pub fn from_json_body() -> impl Filter<Extract = (Self,), Error = warp::Rejection> + Clone {
        // When accepting a body, we want a JSON body
        // (and to reject huge payloads)...
        warp::body::content_length_limit(1024 * 16).and(warp::body::json())
    }
}

/// HTTP Response for a [`output::Output`](../input/struct.Output.html)
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Output {
    pub name: String,
    pub output_type: String,
    pub location: String,
}

/// HTTP Handler for listing [`output::Output`](../output/struct.Output.html)'s associated with
/// a given mixer.
#[tracing::instrument(skip(mixers))]
pub async fn list(mixer_name: String, mixers: Arc<Mutex<super::Mixers>>) -> JsonResult {
    let mixers = mixers.lock().await;
    let mixer = match mixers.mixers.get(&mixer_name) {
        None => return error(Error::NotFound),
        Some(mixer) => mixer,
    };

    let outputs: Vec<Output> = mixer
        .outputs
        .iter()
        .map(|(_, output)| Output {
            name: output.name(),
            output_type: output.output_type(),
            location: output.location(),
        })
        .collect();
    okay(&outputs)
}
/// HTTP Handler for creating an [`output::Output`](../output/struct.Output.html)
/// It will add the resulting output to the [`mixer`](../mixer/struct.Mixer.html) which will
/// link the new output to the Gstreamer pipeline.
#[tracing::instrument(skip(mixers))]
pub async fn add(
    mixer_name: String,
    output: CreateRequest,
    mixers: Arc<Mutex<super::Mixers>>,
) -> JsonResult {
    let mut mixers = mixers.lock().await;

    let config = OutputConfig {
        name: output.name.clone(),
        video: output.video,
        audio: output.audio,
    };

    let output = match output.output_type.as_str() {
        "RTMP" => MixerOutput::create_rtmp(config, &output.location).map_err(super::Error::Mixer),
        "Fake" => MixerOutput::create_fake(config).map_err(super::Error::Mixer),
        "Auto" => MixerOutput::create_auto(config).map_err(super::Error::Mixer),
        _ => Err(super::Error::Unknown),
    };

    let output = match output {
        Err(e) => return error(e),
        Ok(i) => i,
    };

    match mixers.output_add(&mixer_name, output) {
        Ok(_) => message_response("Output created.", StatusCode::CREATED),
        Err(e) => error(e),
    }
}

/// HTTP Handler for retrieving an [`output::Output`](../output/struct.Output.html) associated with
/// a given mixer.
#[tracing::instrument(skip(mixers))]
pub async fn get(
    mixer_name: String,
    output_name: String,
    mixers: Arc<Mutex<super::Mixers>>,
) -> JsonResult {
    let mixers = mixers.lock().await;
    let mixer = match mixers.mixers.get(&mixer_name) {
        None => return error(Error::NotFound),
        Some(mixer) => mixer,
    };

    let output = match mixer.outputs.get(output_name.as_str()) {
        None => return error(Error::NotFound),
        Some(output) => output,
    };

    let output = Output {
        name: output.name(),
        output_type: output.output_type(),
        location: output.location(),
    };

    okay(&output)
}

/// HTTP Handler for removing an [`output::Output`](../output/struct.Output.html) from the associated
/// mixer.
#[tracing::instrument(skip(mixers))]
pub async fn remove(
    mixer_name: String,
    output_name: String,
    mixers: Arc<Mutex<super::Mixers>>,
) -> JsonResult {
    let mut mixers = mixers.lock().await;
    let mixer = match mixers.mixers.get_mut(&mixer_name) {
        None => return error(Error::NotFound),
        Some(mixer) => mixer,
    };

    match mixer.output_remove(&output_name) {
        Ok(_) => message_response("Output removed", StatusCode::OK),
        Err(e) => error(Error::Mixer(e)),
    }
}
