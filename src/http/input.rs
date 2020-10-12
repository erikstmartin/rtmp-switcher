use super::{error, message_response, okay, Error, JsonResult};
use crate::input::{Config as InputConfig, Input as MixerInput};
use crate::mixer;

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use warp::{http::StatusCode, Filter};

/// HTTP Request for creating a new [`input::Input`](../input/struct.Input.html)
/// to be used by the [`mixer`](../mixer/struct.Mixer.html).
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CreateRequest {
    pub name: String,
    pub input_type: String,
    pub location: String,
    #[serde(default)]
    pub audio: mixer::AudioConfig,
    #[serde(default)]
    pub video: mixer::VideoConfig,
    #[serde(default)]
    pub record: bool,
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

/// HTTP Request for update a [`input::Input`](../input/struct.Input.html)
/// to be used by the [`mixer`](../mixer/struct.Mixer.html).
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UpdateRequest {
    pub audio: mixer::AudioConfig,
    pub video: mixer::VideoConfig,
}

impl UpdateRequest {
    /// Constructs a new `UpdateRequest` from a json body.
    /// This function consumes the http request body through warp::body::json().
    pub fn from_json_body() -> impl Filter<Extract = (Self,), Error = warp::Rejection> + Clone {
        // When accepting a body, we want a JSON body
        // (and to reject huge payloads)...
        warp::body::content_length_limit(1024 * 16).and(warp::body::json())
    }
}

/// HTTP Response for a [`input::Input`](../input/struct.Input.html)
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Input {
    pub name: String,
    pub input_type: String,
    pub location: String,
}

/// HTTP Handler for creating an [`input::Input`](../input/struct.Input.html)
/// It will add the resulting input to the [`mixer`](../mixer/struct.Mixer.html) which will
/// link the new input to the Gstreamer pipeline.
#[tracing::instrument(skip(mixers))]
pub async fn add(
    mixer_name: String,
    input: CreateRequest,
    mixers: Arc<Mutex<super::Mixers>>,
) -> JsonResult {
    let mut mixers = mixers.lock().await;
    let config = InputConfig {
        name: input.name.clone(),
        video: input.video,
        audio: input.audio,
        record: input.record,
    };

    let input = match input.input_type.as_str() {
        "URI" => MixerInput::create_uri(config, &input.location).map_err(super::Error::Mixer),
        "Fake" => MixerInput::create_fake(config).map_err(super::Error::Mixer),
        "Test" => MixerInput::create_test(config).map_err(super::Error::Mixer),
        _ => Err(super::Error::Unknown),
    };

    let input = match input {
        Err(e) => return error(e),
        Ok(i) => i,
    };

    match mixers.input_add(&mixer_name, input) {
        Ok(_) => message_response("Input created.", StatusCode::CREATED),
        Err(e) => error(e),
    }
}

/// HTTP Handler for listing [`input::Input`](../input/struct.Input.html)'s associated with
/// a given mixer.
#[tracing::instrument(skip(mixers))]
pub async fn list(mixer_name: String, mixers: Arc<Mutex<super::Mixers>>) -> JsonResult {
    let mixers = mixers.lock().await;
    let mixer = match mixers.mixers.get(&mixer_name) {
        None => return error(Error::NotFound),
        Some(mixer) => mixer,
    };

    let inputs: Vec<Input> = mixer
        .inputs
        .iter()
        .map(|(_, input)| Input {
            name: input.name(),
            input_type: input.input_type(),
            location: input.location(),
        })
        .collect();
    okay(&inputs)
}

/// HTTP Handler for retrieving an [`input::Input`](../input/struct.Input.html) associated with
/// a given mixer.
#[tracing::instrument(skip(mixers))]
pub async fn get(
    mixer_name: String,
    input_name: String,
    mixers: Arc<Mutex<super::Mixers>>,
) -> JsonResult {
    let mixers = mixers.lock().await;
    let mixer = match mixers.mixers.get(&mixer_name) {
        None => return error(Error::NotFound),
        Some(mixer) => mixer,
    };

    let input = match mixer.inputs.get(input_name.as_str()) {
        None => return error(Error::NotFound),
        Some(input) => input,
    };

    let input = Input {
        name: input.name(),
        input_type: input.input_type(),
        location: input.location(),
    };

    okay(&input)
}

/// HTTP Handler for updating an [`input::Input`](../input/struct.Input.html) associated with
/// a given mixer.
#[tracing::instrument(skip(mixers))]
pub async fn update(
    mixer_name: String,
    input_name: String,
    request: UpdateRequest,
    mixers: Arc<Mutex<super::Mixers>>,
) -> JsonResult {
    let mut mixers = mixers.lock().await;
    let mixer = match mixers.mixers.get_mut(&mixer_name) {
        Some(mixer) => mixer,
        None => return error(Error::NotFound),
    };

    let input = match mixer.inputs.get_mut(input_name.as_str()) {
        Some(input) => input,
        None => return error(Error::NotFound),
    };

    if input.set_volume(request.audio.volume).is_err() {
        return message_response("set_volume failed", StatusCode::INTERNAL_SERVER_ERROR);
    }

    if let Some(zorder) = request.video.zorder {
        if input.set_zorder(zorder).is_err() {
            return message_response("set_zorder failed", StatusCode::INTERNAL_SERVER_ERROR);
        }
    }

    if input.set_width(request.video.width).is_err() {
        return message_response("set_width failed", StatusCode::INTERNAL_SERVER_ERROR);
    }

    if input.set_height(request.video.height).is_err() {
        return message_response("set_height failed", StatusCode::INTERNAL_SERVER_ERROR);
    }

    if input.set_xpos(request.video.xpos).is_err() {
        return message_response("set_xpos failed", StatusCode::INTERNAL_SERVER_ERROR);
    }

    if input.set_ypos(request.video.ypos).is_err() {
        return message_response("set_ypos failed", StatusCode::INTERNAL_SERVER_ERROR);
    }

    if input.set_alpha(request.video.alpha).is_err() {
        return message_response("set_alpha failed", StatusCode::INTERNAL_SERVER_ERROR);
    }

    message_response("Input updated", StatusCode::OK)
}

/// HTTP Handler for removing an [`input::Input`](../input/struct.Input.html) from the associated
/// mixer.
#[tracing::instrument(skip(mixers))]
pub async fn remove(
    mixer_name: String,
    input_name: String,
    mixers: Arc<Mutex<super::Mixers>>,
) -> JsonResult {
    let mut mixers = mixers.lock().await;
    let mixer = match mixers.mixers.get_mut(&mixer_name) {
        None => return error(Error::NotFound),
        Some(mixer) => mixer,
    };

    match mixer.input_remove(&input_name) {
        Ok(_) => message_response("Input removed", StatusCode::OK),
        Err(e) => error(Error::Mixer(e)),
    }
}

/// HTTP Handler for setting an [`input::Input`](../input/struct.Input.html) to be the active
/// input.
///
/// This will change the zorder of all other inputs to be lower than this input, it will then
/// adjust the volume of all other inputs to 0.
///
/// Setting an input to active will reset all its configuration to its prior configuration (if it
/// had been updated prior, due to another input being set active)
#[tracing::instrument(skip(mixers))]
pub async fn set_active(
    mixer_name: String,
    input_name: String,
    mixers: Arc<Mutex<super::Mixers>>,
) -> JsonResult {
    let mut mixers = mixers.lock().await;
    let mixer = match mixers.mixers.get_mut(&mixer_name) {
        None => return error(Error::NotFound),
        Some(mixer) => mixer,
    };

    match mixer.input_set_active(&input_name) {
        Ok(_) => message_response(
            &format!("Input '{}' set to active", input_name),
            StatusCode::OK,
        ),
        Err(e) => error(Error::Mixer(e)),
    }
}
