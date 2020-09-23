use crate::http;
use crate::mixer;
use serde::Serialize;
use std::convert::Infallible;
use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use warp::http::StatusCode;
use warp::reply::Reply;

#[derive(Debug, Serialize)]
pub struct Response {
    pub message: String,
}

pub async fn mixer_create(
    mixer: super::MixerCreateRequest,
    mixers: Arc<Mutex<super::Mixers>>,
) -> Result<impl warp::Reply, Infallible> {
    let config = mixer::Config {
        name: mixer.name,
        video: mixer.video.unwrap_or(mixer::default_video_config()),
        audio: mixer.audio.unwrap_or(mixer::default_audio_config()),
    };

    match mixers.lock().unwrap().mixer_create(config) {
        Ok(_) => Ok(warp::reply::with_status(
            warp::reply::json(&Response {
                message: "Mixer created".to_string(),
            }),
            StatusCode::CREATED,
        )),
        Err(e) => match e {
            http::Error::Exists => Ok(warp::reply::with_status(
                warp::reply::json(&Response {
                    message: format!("{}", e),
                }),
                StatusCode::INTERNAL_SERVER_ERROR,
            )),
            e => Ok(warp::reply::with_status(
                warp::reply::json(&Response {
                    message: format!("{}", e),
                }),
                StatusCode::INTERNAL_SERVER_ERROR,
            )),
        },
    }
}

pub async fn mixer_get(
    name: String,
    mixers: Arc<Mutex<super::Mixers>>,
) -> Result<impl warp::Reply, Infallible> {
    let mixers = mixers.lock().unwrap();
    let mixer = mixers.mixers.get(name.as_str());
    match mixer {
        Some(m) => {
            let mixer = &super::MixerResponse {
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
            warp::reply::json(&Response {
                message: "Mixer not found".to_string(),
            }),
            StatusCode::NOT_FOUND,
        )),
    }
}

pub async fn mixer_debug(
    name: String,
    mixers: Arc<Mutex<super::Mixers>>,
) -> Result<warp::reply::Response, Infallible> {
    let mixers = mixers.lock().unwrap();
    let mixer = mixers.mixers.get(name.as_str());

    if mixer.is_none() {
        let mut response = warp::reply::json(&Response {
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

pub async fn mixer_list(mixers: Arc<Mutex<super::Mixers>>) -> Result<impl warp::Reply, Infallible> {
    let mixers: Vec<super::MixerResponse> = mixers
        .lock()
        .unwrap()
        .mixers
        .iter()
        .map(|(_, m)| super::MixerResponse {
            name: m.name(),
            input_count: m.input_count(),
            output_count: m.output_count(),
        })
        .collect();
    Ok(warp::reply::json(&mixers))
}

pub async fn input_add(
    mixer_name: String,
    input: super::InputCreateRequest,
    mixers: Arc<Mutex<super::Mixers>>,
) -> Result<impl warp::Reply, Infallible> {
    let mut mixers = mixers.lock().unwrap();
    let mixer_config = mixers.mixer_config(&mixer_name);

    if mixer_config.is_err() {
        return Ok(warp::reply::with_status(
            warp::reply::json(&Response {
                message: "Mixer not found".to_string(),
            }),
            StatusCode::NOT_FOUND,
        ));
    }
    let mixer_config = mixer_config.unwrap();

    let config = mixer::Config {
        name: input.name.clone(),
        // TODO: Don't default to crate default, grab current mixer settings
        video: input.video.unwrap_or(mixer_config.video),
        audio: input.audio.unwrap_or(mixer_config.audio),
    };

    let input = match input.input_type.as_str() {
        "URI" => crate::mixer::input::URI::new(config, &input.location)
            .map_err(|e| super::Error::Mixer(e)),
        "Fake" => crate::mixer::input::Fake::new(&input.name).map_err(|e| super::Error::Mixer(e)),
        "Test" => crate::mixer::input::Test::new(&input.name).map_err(|e| super::Error::Mixer(e)),
        _ => Err(super::Error::Unknown),
    };

    match mixers.input_add(&mixer_name, input.unwrap()) {
        Ok(_) => Ok(warp::reply::with_status(
            warp::reply::json(&Response {
                message: "Input created".to_string(),
            }),
            StatusCode::CREATED,
        )),
        Err(e) => match e {
            http::Error::NotFound => Ok(warp::reply::with_status(
                warp::reply::json(&Response {
                    message: "Mixer not found".to_string(),
                }),
                StatusCode::NOT_FOUND,
            )),
            _ => Ok(warp::reply::with_status(
                warp::reply::json(&Response {
                    message: format!("{}", e),
                }),
                StatusCode::INTERNAL_SERVER_ERROR,
            )),
        },
    }
}

pub async fn input_list(
    mixer_name: String,
    mixers: Arc<Mutex<super::Mixers>>,
) -> Result<warp::reply::Response, Infallible> {
    let mixers = mixers.lock().unwrap();
    let mixer = mixers.mixers.get(&mixer_name);
    if mixer.is_none() {
        let mut response = warp::reply::json(&Response {
            message: "Mixer not found".to_string(),
        })
        .into_response();
        *response.status_mut() = StatusCode::NOT_FOUND;

        return Ok(response);
    }

    let inputs: Vec<super::InputResponse> = mixer
        .unwrap()
        .inputs
        .iter()
        .map(|(_, input)| super::InputResponse {
            name: input.name(),
            input_type: input.input_type(),
            location: input.location(),
        })
        .collect();
    Ok(warp::reply::json(&inputs).into_response())
}

pub async fn input_get(
    mixer_name: String,
    input_name: String,
    mixers: Arc<Mutex<super::Mixers>>,
) -> Result<warp::reply::Response, Infallible> {
    let mixers = mixers.lock().unwrap();
    let mixer = mixers.mixers.get(&mixer_name);
    if mixer.is_none() {
        let mut response = warp::reply::json(&Response {
            message: "Mixer not found".to_string(),
        })
        .into_response();
        *response.status_mut() = StatusCode::NOT_FOUND;

        return Ok(response);
    }

    let input: Option<&mixer::Input> = mixer.unwrap().inputs.get(input_name.as_str());

    if input.is_none() {
        let mut response = warp::reply::json(&Response {
            message: "Input not found".to_string(),
        })
        .into_response();
        *response.status_mut() = StatusCode::NOT_FOUND;

        return Ok(response);
    }

    let input = input.unwrap();
    let input = super::InputResponse {
        name: input.name(),
        input_type: input.input_type(),
        location: input.location(),
    };

    let mut response = warp::reply::json(&input).into_response();
    *response.status_mut() = StatusCode::NOT_FOUND;

    Ok(response)
}

pub async fn input_remove(
    mixer_name: String,
    input_name: String,
    mixers: Arc<Mutex<super::Mixers>>,
) -> Result<impl warp::Reply, Infallible> {
    let mut mixers = mixers.lock().unwrap();
    let mixer = mixers.mixers.get_mut(&mixer_name);
    if mixer.is_none() {
        return Ok(warp::reply::with_status(
            warp::reply::json(&Response {
                message: "Mixer not found".to_string(),
            }),
            StatusCode::NOT_FOUND,
        ));
    }

    match mixer.unwrap().input_remove(&input_name) {
        Ok(_) => Ok(warp::reply::with_status(
            warp::reply::json(&Response {
                message: "Mixer created".to_string(),
            }),
            StatusCode::CREATED,
        )),
        Err(e) => match e {
            mixer::Error::NotFound(_, _) => Ok(warp::reply::with_status(
                warp::reply::json(&Response {
                    message: format!("{}", e),
                }),
                StatusCode::NOT_FOUND,
            )),
            e => Ok(warp::reply::with_status(
                warp::reply::json(&Response {
                    message: format!("{}", e),
                }),
                StatusCode::INTERNAL_SERVER_ERROR,
            )),
        },
    }
}

pub async fn output_list(
    mixer_name: String,
    mixers: Arc<Mutex<super::Mixers>>,
) -> Result<impl warp::Reply, Infallible> {
    let mixers = mixers.lock().unwrap();
    let mixer = mixers.mixers.get(&mixer_name);
    if mixer.is_none() {
        let mut response = warp::reply::json(&Response {
            message: "Mixer not found".to_string(),
        })
        .into_response();
        *response.status_mut() = StatusCode::NOT_FOUND;

        return Ok(response);
    }

    let outputs: Vec<super::OutputResponse> = mixer
        .unwrap()
        .outputs
        .iter()
        .map(|(_, output)| super::OutputResponse {
            name: output.name(),
            output_type: output.output_type(),
            location: output.location(),
        })
        .collect();
    Ok(warp::reply::json(&outputs).into_response())
}

pub async fn output_add(
    mixer: String,
    output: super::OutputCreateRequest,
    mixers: Arc<Mutex<super::Mixers>>,
) -> Result<impl warp::Reply, Infallible> {
    let output = match output.output_type.as_str() {
        "Auto" => crate::mixer::output::Auto::new(&output.name).map_err(|e| super::Error::Mixer(e)),
        "RTMP" => crate::mixer::output::RTMP::new(&output.name, &output.location)
            .map_err(|e| super::Error::Mixer(e)),
        "Fake" => crate::mixer::output::Fake::new(&output.name).map_err(|e| super::Error::Mixer(e)),
        _ => Err(super::Error::Unknown),
    };

    match mixers.lock().unwrap().output_add(&mixer, output.unwrap()) {
        Ok(_) => Ok(warp::reply::with_status(
            warp::reply::json(&Response {
                message: "Output created".to_string(),
            }),
            StatusCode::CREATED,
        )),
        Err(e) => match e {
            http::Error::NotFound => Ok(warp::reply::with_status(
                warp::reply::json(&Response {
                    message: "Mixer not found".to_string(),
                }),
                StatusCode::NOT_FOUND,
            )),
            _ => Ok(warp::reply::with_status(
                warp::reply::json(&Response {
                    message: format!("{}", e),
                }),
                StatusCode::INTERNAL_SERVER_ERROR,
            )),
        },
    }
}

pub async fn output_get(
    mixer_name: String,
    output_name: String,
    mixers: Arc<Mutex<super::Mixers>>,
) -> Result<impl warp::Reply, Infallible> {
    let mixers = mixers.lock().unwrap();
    let mixer = mixers.mixers.get(&mixer_name);
    if mixer.is_none() {
        let mut response = warp::reply::json(&Response {
            message: "Mixer not found".to_string(),
        })
        .into_response();
        *response.status_mut() = StatusCode::NOT_FOUND;

        return Ok(response);
    }

    let output: Option<&mixer::Output> = mixer.unwrap().outputs.get(output_name.as_str());

    if output.is_none() {
        let mut response = warp::reply::json(&Response {
            message: "Output not found".to_string(),
        })
        .into_response();
        *response.status_mut() = StatusCode::NOT_FOUND;

        return Ok(response);
    }

    let output = output.unwrap();
    let output = super::OutputResponse {
        name: output.name(),
        output_type: output.output_type(),
        location: output.location(),
    };

    let mut response = warp::reply::json(&output).into_response();
    *response.status_mut() = StatusCode::NOT_FOUND;

    Ok(response)
}

pub async fn output_remove(
    mixer_name: String,
    output_name: String,
    mixers: Arc<Mutex<super::Mixers>>,
) -> Result<impl warp::Reply, Infallible> {
    let mut mixers = mixers.lock().unwrap();
    let mixer = mixers.mixers.get_mut(&mixer_name);
    if mixer.is_none() {
        return Ok(warp::reply::with_status(
            warp::reply::json(&Response {
                message: "Mixer not found".to_string(),
            }),
            StatusCode::NOT_FOUND,
        ));
    }

    match mixer.unwrap().output_remove(&output_name) {
        Ok(_) => Ok(warp::reply::with_status(
            warp::reply::json(&Response {
                message: "Mixer created".to_string(),
            }),
            StatusCode::CREATED,
        )),
        Err(e) => match e {
            mixer::Error::NotFound(_, _) => Ok(warp::reply::with_status(
                warp::reply::json(&Response {
                    message: format!("{}", e),
                }),
                StatusCode::NOT_FOUND,
            )),
            e => Ok(warp::reply::with_status(
                warp::reply::json(&Response {
                    message: format!("{}", e),
                }),
                StatusCode::INTERNAL_SERVER_ERROR,
            )),
        },
    }
}
