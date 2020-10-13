use crate::mixer::Config as MixerConfig;
use crate::{AudioConfig, VideoConfig};
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::Arc;
use tokio::sync::Mutex;
use warp::http::StatusCode;
use warp::reply::Reply;
use warp::Filter;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CreateRequest {
    pub name: String,
    #[serde(default)]
    pub video: VideoConfig,
    #[serde(default)]
    pub audio: AudioConfig,
}

impl CreateRequest {
    pub fn from_json_body() -> impl Filter<Extract = (Self,), Error = warp::Rejection> + Clone {
        // When accepting a body, we want a JSON body
        // (and to reject huge payloads)...
        warp::body::content_length_limit(1024 * 16).and(warp::body::json())
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Mixer {
    pub name: String,
    pub input_count: usize,
    pub output_count: usize,
}

pub async fn create(
    mixer: CreateRequest,
    mixers: Arc<Mutex<super::Mixers>>,
) -> Result<impl warp::Reply, Infallible> {
    let config = MixerConfig {
        name: mixer.name,
        video: mixer.video,
        audio: mixer.audio,
    };

    match mixers.lock().await.mixer_create(config) {
        Ok(_) => Ok(warp::reply::with_status(
            warp::reply::json(&super::Response {
                message: "Mixer created".to_string(),
            }),
            StatusCode::CREATED,
        )),
        Err(e) => match e {
            super::Error::Exists => Ok(warp::reply::with_status(
                warp::reply::json(&super::Response {
                    message: format!("{}", e),
                }),
                StatusCode::INTERNAL_SERVER_ERROR,
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

pub async fn get(
    name: String,
    mixers: Arc<Mutex<super::Mixers>>,
) -> Result<impl warp::Reply, Infallible> {
    let mixers = mixers.lock().await;
    let mixer = mixers.mixers.get(name.as_str());
    match mixer {
        Some(m) => {
            let mixer = &Mixer {
                name: m.name(),
                input_count: m.input_count(),
                output_count: m.output_count(),
            };
            Ok(warp::reply::with_status(
                warp::reply::json(&mixer),
                StatusCode::OK,
            ))
        }
        None => Ok(warp::reply::with_status(
            warp::reply::json(&super::Response {
                message: "Mixer not found".to_string(),
            }),
            StatusCode::NOT_FOUND,
        )),
    }
}

pub async fn debug(
    name: String,
    mixers: Arc<Mutex<super::Mixers>>,
) -> Result<warp::reply::Response, Infallible> {
    let mixers = mixers.lock().await;
    let mixer = mixers.mixers.get(name.as_str());

    if mixer.is_none() {
        let mut response = warp::reply::json(&super::Response {
            message: "Mixer not found".to_string(),
        })
        .into_response();
        *response.status_mut() = StatusCode::NOT_FOUND;

        return Ok(response);
    }

    let mixer = mixer.unwrap();
    let mut cmd = Command::new("dot")
        .arg("-Tsvg")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed to execute process");

    let stdin = cmd.stdin.as_mut().expect("Failed to open stdin");
    stdin
        .write_all(mixer.generate_dot().as_bytes())
        .expect("Failed to write to stdin");

    let output = cmd.wait_with_output().expect("Failed to read stdout");
    Ok(warp::reply::with_header(
        String::from_utf8(output.stdout).unwrap(),
        "Content-Type",
        "image/svg+xml",
    )
    .into_response())
}

pub async fn list(mixers: Arc<Mutex<super::Mixers>>) -> Result<impl warp::Reply, Infallible> {
    let mixers: Vec<Mixer> = mixers
        .lock()
        .await
        .mixers
        .iter()
        .map(|(_, m)| Mixer {
            name: m.name(),
            input_count: m.input_count(),
            output_count: m.output_count(),
        })
        .collect();
    Ok(warp::reply::json(&mixers))
}
