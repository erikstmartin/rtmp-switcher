use crate::mixer;
use crate::output::Output as MixerOutput;
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
    pub output_type: String,
    pub location: String,
}

impl CreateRequest {
    pub fn from_json_body() -> impl Filter<Extract = (Self,), Error = warp::Rejection> + Clone {
        // When accepting a body, we want a JSON body
        // (and to reject huge payloads)...
        warp::body::content_length_limit(1024 * 16).and(warp::body::json())
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Output {
    pub name: String,
    pub output_type: String,
    pub location: String,
}

pub async fn list(
    mixer_name: String,
    mixers: Arc<Mutex<super::Mixers>>,
) -> Result<impl warp::Reply, Infallible> {
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

    let outputs: Vec<Output> = mixer
        .unwrap()
        .outputs
        .iter()
        .map(|(_, output)| Output {
            name: output.name(),
            output_type: output.output_type(),
            location: output.location(),
        })
        .collect();
    Ok(warp::reply::json(&outputs).into_response())
}

pub async fn add(
    mixer: String,
    output: CreateRequest,
    mixers: Arc<Mutex<super::Mixers>>,
) -> Result<impl warp::Reply, Infallible> {
    let output = match output.output_type.as_str() {
        "Auto" => MixerOutput::create_auto(&output.name).map_err(|e| super::Error::Mixer(e)),
        "RTMP" => MixerOutput::create_rtmp(&output.name, &output.location)
            .map_err(|e| super::Error::Mixer(e)),
        "Fake" => MixerOutput::create_fake(&output.name).map_err(|e| super::Error::Mixer(e)),
        "File" => MixerOutput::create_file(&output.name, &output.location)
            .map_err(|e| super::Error::Mixer(e)),
        _ => Err(super::Error::Unknown),
    };

    match mixers.lock().await.output_add(&mixer, output.unwrap()) {
        Ok(_) => Ok(warp::reply::with_status(
            warp::reply::json(&super::Response {
                message: "Output created".to_string(),
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

pub async fn get(
    mixer_name: String,
    output_name: String,
    mixers: Arc<Mutex<super::Mixers>>,
) -> Result<impl warp::Reply, Infallible> {
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

    let output: Option<&MixerOutput> = mixer.unwrap().outputs.get(output_name.as_str());

    if output.is_none() {
        let mut response = warp::reply::json(&super::Response {
            message: "Output not found".to_string(),
        })
        .into_response();
        *response.status_mut() = StatusCode::NOT_FOUND;

        return Ok(response);
    }

    let output = output.unwrap();
    let output = Output {
        name: output.name(),
        output_type: output.output_type(),
        location: output.location(),
    };

    let mut response = warp::reply::json(&output).into_response();
    *response.status_mut() = StatusCode::OK;

    Ok(response)
}

pub async fn remove(
    mixer_name: String,
    output_name: String,
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

    match mixer.unwrap().output_remove(&output_name) {
        Ok(_) => Ok(warp::reply::with_status(
            warp::reply::json(&super::Response {
                message: "Output removed".to_string(),
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
