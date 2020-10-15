use super::{error, message_response, okay, JsonResult};
use crate::{mixer::Config as MixerConfig, AudioConfig, VideoConfig};
use serde::{Deserialize, Serialize};
use std::{
    convert::Infallible,
    io::Write,
    process::{Command, Stdio},
    sync::Arc,
};
use tokio::sync::Mutex;
use warp::{http::StatusCode, reply, Filter, Reply};

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

pub async fn create(mixer: CreateRequest, mixers: Arc<Mutex<super::Mixers>>) -> JsonResult {
    let config = MixerConfig {
        name: mixer.name,
        video: mixer.video,
        audio: mixer.audio,
    };

    match mixers.lock().await.mixer_create(config) {
        Ok(_) => message_response("Mixer created.", StatusCode::CREATED),
        Err(e) => error(e),
    }
}

pub async fn get(name: String, mixers: Arc<Mutex<super::Mixers>>) -> JsonResult {
    let mixers = mixers.lock().await;
    match mixers.mixers.get(name.as_str()) {
        Some(m) => okay(&Mixer {
            name: m.name(),
            input_count: m.input_count(),
            output_count: m.output_count(),
        }),
        None => message_response("Mixer not found", StatusCode::NOT_FOUND),
    }
}

pub async fn debug(
    name: String,
    mixers: Arc<Mutex<super::Mixers>>,
) -> Result<warp::reply::Response, Infallible> {
    let mixers = mixers.lock().await;
    let mixer = match mixers.mixers.get(name.as_str()) {
        Some(m) => m,
        None => {
            return Ok(
                reply::with_status(reply::json(&"Mixer not found"), StatusCode::NOT_FOUND)
                    .into_response(),
            )
        }
    };

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
    let output = String::from_utf8_lossy(&output.stdout).into_owned();
    Ok(warp::reply::with_header(output, "Content-Type", "image/svg+xml").into_response())
}

pub async fn list(mixers: Arc<Mutex<super::Mixers>>) -> JsonResult {
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
    okay(&mixers)
}
