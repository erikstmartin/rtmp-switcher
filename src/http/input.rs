use crate::input;
use crate::mixer;

use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::sync::Arc;
use tokio::sync::Mutex;
use warp::http::StatusCode;
use warp::reply::Reply;
use warp::Filter;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CreateRequest {
    pub name: String,
    pub input_type: String,
    pub location: String,
    pub audio: Option<mixer::AudioConfig>,
    pub video: Option<mixer::VideoConfig>,
}

impl CreateRequest {
    pub fn from_json_body() -> impl Filter<Extract = (Self,), Error = warp::Rejection> + Clone {
        // When accepting a body, we want a JSON body
        // (and to reject huge payloads)...
        warp::body::content_length_limit(1024 * 16).and(warp::body::json())
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UpdateRequest {
    pub audio: Option<mixer::AudioConfig>,
    pub video: Option<mixer::VideoConfig>,
}

impl UpdateRequest {
    pub fn from_json_body() -> impl Filter<Extract = (Self,), Error = warp::Rejection> + Clone {
        // When accepting a body, we want a JSON body
        // (and to reject huge payloads)...
        warp::body::content_length_limit(1024 * 16).and(warp::body::json())
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Input {
    pub name: String,
    pub input_type: String,
    pub location: String,
}

pub async fn add(
    mixer_name: String,
    input: CreateRequest,
    mixers: Arc<Mutex<super::Mixers>>,
) -> Result<impl warp::Reply, Infallible> {
    let mut mixers = mixers.lock().await;
    let mixer_config = mixers.mixer_config(&mixer_name);

    if mixer_config.is_err() {
        return Ok(warp::reply::with_status(
            warp::reply::json(&super::Response {
                message: "Mixer not found".to_string(),
            }),
            StatusCode::NOT_FOUND,
        ));
    }
    let mixer_config = mixer_config.unwrap();

    let config = mixer::Config {
        name: input.name.clone(),
        video: input.video.unwrap_or(mixer_config.video),
        audio: input.audio.unwrap_or(mixer_config.audio),
    };

    let input = match input.input_type.as_str() {
        "URI" => crate::mixer::input::URI::new(config, &input.location)
            .map_err(|e| super::Error::Mixer(e)),
        "Fake" => crate::mixer::input::Fake::new(config).map_err(|e| super::Error::Mixer(e)),
        "Test" => crate::mixer::input::Test::new(config).map_err(|e| super::Error::Mixer(e)),
        _ => Err(super::Error::Unknown),
    };

    if let Err(err) = input {
        return Ok(warp::reply::with_status(
            warp::reply::json(&super::Response {
                message: format!("{}", err),
            }),
            StatusCode::INTERNAL_SERVER_ERROR,
        ));
    }

    match mixers.input_add(&mixer_name, input.unwrap()) {
        Ok(_) => Ok(warp::reply::with_status(
            warp::reply::json(&super::Response {
                message: "Input created".to_string(),
            }),
            StatusCode::CREATED,
        )),
        Err(e) => match e {
            super::Error::NotFound => Ok(warp::reply::with_status(
                warp::reply::json(&super::Response {
                    message: "Mixer not found".to_string(),
                }),
                StatusCode::NOT_FOUND,
            )),
            _ => Ok(warp::reply::with_status(
                warp::reply::json(&super::Response {
                    message: format!("{}", e),
                }),
                StatusCode::INTERNAL_SERVER_ERROR,
            )),
        },
    }
}

pub async fn list(
    mixer_name: String,
    mixers: Arc<Mutex<super::Mixers>>,
) -> Result<warp::reply::Response, Infallible> {
    let mixers = mixers.lock().await;
    let mixer = mixers.mixers.get(&mixer_name);
    if mixer.is_none() {
        let mut response = warp::reply::json(&super::Response {
            message: "Mixer not found".to_string(),
        })
        .into_response();
        *response.status_mut() = StatusCode::NOT_FOUND;

        return Ok(response);
    }

    let inputs: Vec<Input> = mixer
        .unwrap()
        .inputs
        .iter()
        .map(|(_, input)| Input {
            name: input.name(),
            input_type: input.input_type(),
            location: input.location(),
        })
        .collect();
    Ok(warp::reply::json(&inputs).into_response())
}

pub async fn get(
    mixer_name: String,
    input_name: String,
    mixers: Arc<Mutex<super::Mixers>>,
) -> Result<warp::reply::Response, Infallible> {
    let mixers = mixers.lock().await;
    let mixer = mixers.mixers.get(&mixer_name);
    if mixer.is_none() {
        let mut response = warp::reply::json(&super::Response {
            message: "Mixer not found".to_string(),
        })
        .into_response();
        *response.status_mut() = StatusCode::NOT_FOUND;

        return Ok(response);
    }

    let input: Option<&input::Input> = mixer.unwrap().inputs.get(input_name.as_str());

    if input.is_none() {
        let mut response = warp::reply::json(&super::Response {
            message: "Input not found".to_string(),
        })
        .into_response();
        *response.status_mut() = StatusCode::NOT_FOUND;

        return Ok(response);
    }

    let input = input.unwrap();
    let input = Input {
        name: input.name(),
        input_type: input.input_type(),
        location: input.location(),
    };

    let mut response = warp::reply::json(&input).into_response();
    *response.status_mut() = StatusCode::OK;

    Ok(response)
}

pub async fn update(
    mixer_name: String,
    input_name: String,
    request: UpdateRequest,
    mixers: Arc<Mutex<super::Mixers>>,
) -> Result<impl warp::Reply, Infallible> {
    let mut mixers = mixers.lock().await;
    let mixer = mixers.mixers.get_mut(&mixer_name);
    if mixer.is_none() {
        return Ok(warp::reply::with_status(
            warp::reply::json(&super::Response {
                message: "Mixer not found".to_string(),
            }),
            StatusCode::NOT_FOUND,
        ));
    }

    let input: Option<&mut input::Input> = mixer.unwrap().inputs.get_mut(input_name.as_str());
    if input.is_none() {
        return Ok(warp::reply::with_status(
            warp::reply::json(&super::Response {
                message: "Input not found".to_string(),
            }),
            StatusCode::OK,
        ));
    }

    let input = input.unwrap();
    if let Some(volume) = request.audio.unwrap().volume {
        if input.set_volume(volume).is_err() {
            return Ok(warp::reply::with_status(
                warp::reply::json(&super::Response {
                    message: "set_volume failed".to_string(),
                }),
                StatusCode::OK,
            ));
        }
    }

    let video_config = request.video.unwrap();
    if let Some(zorder) = video_config.zorder {
        if input.set_zorder(zorder).is_err() {
            return Ok(warp::reply::with_status(
                warp::reply::json(&super::Response {
                    message: "set_zorder failed".to_string(),
                }),
                StatusCode::OK,
            ));
        }
    }

    if let Some(width) = video_config.width {
        if input.set_width(width).is_err() {
            return Ok(warp::reply::with_status(
                warp::reply::json(&super::Response {
                    message: "set_zorder failed".to_string(),
                }),
                StatusCode::OK,
            ));
        }
    }

    if let Some(height) = video_config.height {
        if input.set_height(height).is_err() {
            return Ok(warp::reply::with_status(
                warp::reply::json(&super::Response {
                    message: "set_height failed".to_string(),
                }),
                StatusCode::OK,
            ));
        }
    }

    if let Some(xpos) = video_config.xpos {
        if input.set_xpos(xpos).is_err() {
            return Ok(warp::reply::with_status(
                warp::reply::json(&super::Response {
                    message: "set_xpos failed".to_string(),
                }),
                StatusCode::OK,
            ));
        }
    }

    if let Some(ypos) = video_config.ypos {
        if input.set_ypos(ypos).is_err() {
            return Ok(warp::reply::with_status(
                warp::reply::json(&super::Response {
                    message: "set_ypos failed".to_string(),
                }),
                StatusCode::OK,
            ));
        }
    }

    if let Some(alpha) = video_config.alpha {
        if input.set_alpha(alpha).is_err() {
            return Ok(warp::reply::with_status(
                warp::reply::json(&super::Response {
                    message: "set_alpha failed".to_string(),
                }),
                StatusCode::OK,
            ));
        }
    }

    Ok(warp::reply::with_status(
        warp::reply::json(&super::Response {
            message: "Input updated".to_string(),
        }),
        StatusCode::OK,
    ))
}

pub async fn remove(
    mixer_name: String,
    input_name: String,
    mixers: Arc<Mutex<super::Mixers>>,
) -> Result<impl warp::Reply, Infallible> {
    let mut mixers = mixers.lock().await;
    let mixer = mixers.mixers.get_mut(&mixer_name);
    if mixer.is_none() {
        return Ok(warp::reply::with_status(
            warp::reply::json(&super::Response {
                message: "Mixer not found".to_string(),
            }),
            StatusCode::NOT_FOUND,
        ));
    }

    match mixer.unwrap().input_remove(&input_name) {
        Ok(_) => Ok(warp::reply::with_status(
            warp::reply::json(&super::Response {
                message: "Input removed".to_string(),
            }),
            StatusCode::OK,
        )),
        Err(e) => match e {
            mixer::Error::NotFound(_, _) => Ok(warp::reply::with_status(
                warp::reply::json(&super::Response {
                    message: format!("{}", e),
                }),
                StatusCode::NOT_FOUND,
            )),
            e => Ok(warp::reply::with_status(
                warp::reply::json(&super::Response {
                    message: format!("{}", e),
                }),
                StatusCode::INTERNAL_SERVER_ERROR,
            )),
        },
    }
}

pub async fn set_active(
    mixer_name: String,
    input_name: String,
    mixers: Arc<Mutex<super::Mixers>>,
) -> Result<impl warp::Reply, Infallible> {
    let mut mixers = mixers.lock().await;
    let mixer = mixers.mixers.get_mut(&mixer_name);
    if mixer.is_none() {
        return Ok(warp::reply::with_status(
            warp::reply::json(&super::Response {
                message: "Mixer not found".to_string(),
            }),
            StatusCode::NOT_FOUND,
        ));
    }

    match mixer.unwrap().input_set_active(&input_name) {
        Ok(_) => Ok(warp::reply::with_status(
            warp::reply::json(&super::Response {
                message: "Input set to active".to_string(),
            }),
            StatusCode::OK,
        )),
        Err(e) => match e {
            mixer::Error::NotFound(_, _) => Ok(warp::reply::with_status(
                warp::reply::json(&super::Response {
                    message: format!("{}", e),
                }),
                StatusCode::NOT_FOUND,
            )),
            e => Ok(warp::reply::with_status(
                warp::reply::json(&super::Response {
                    message: format!("{}", e),
                }),
                StatusCode::INTERNAL_SERVER_ERROR,
            )),
        },
    }
}
