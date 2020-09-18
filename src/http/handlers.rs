use crate::mixer;
use std::convert::Infallible;
use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use warp::http::StatusCode;

pub async fn mixer_create(
    mixer: super::MixerCreateRequest,
    mixers: Arc<Mutex<super::Mixers>>,
) -> Result<impl warp::Reply, Infallible> {
    match mixers.lock().unwrap().mixer_create(&mixer.name) {
        Ok(_) => Ok(StatusCode::CREATED),
        Err(_) => Ok(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn mixer_get(
    name: String,
    mixers: Arc<Mutex<super::Mixers>>,
) -> Result<impl warp::Reply, Infallible> {
    let mixers = mixers.lock().unwrap();
    let mixer = mixers.mixers.get(name.as_str()).unwrap();

    let mixer = &super::MixerResponse {
        name: mixer.name.clone(),
        input_count: mixer.input_count(),
        output_count: mixer.output_count(),
    };

    Ok(warp::reply::json(mixer))
}

pub async fn mixer_debug(
    name: String,
    mixers: Arc<Mutex<super::Mixers>>,
) -> Result<impl warp::Reply, Infallible> {
    let mixers = mixers.lock().unwrap();
    let mixer = mixers.mixers.get(name.as_str()).unwrap();

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
    ))
}

pub async fn mixer_list(mixers: Arc<Mutex<super::Mixers>>) -> Result<impl warp::Reply, Infallible> {
    let mixers: Vec<super::MixerResponse> = mixers
        .lock()
        .unwrap()
        .mixers
        .iter()
        .map(|(_, m)| super::MixerResponse {
            name: m.name.clone(),
            input_count: m.input_count(),
            output_count: m.output_count(),
        })
        .collect();
    Ok(warp::reply::json(&mixers))
}

pub async fn input_add(
    mixer: String,
    input: super::InputCreateRequest,
    mixers: Arc<Mutex<super::Mixers>>,
) -> Result<impl warp::Reply, Infallible> {
    let input = match input.input_type.as_str() {
        "URI" => crate::mixer::input::URI::new(&input.name, &input.location)
            .map_err(|e| super::Error::Mixer(e)),
        "Fake" => crate::mixer::input::Fake::new(&input.name).map_err(|e| super::Error::Mixer(e)),
        "Test" => crate::mixer::input::Test::new(&input.name).map_err(|e| super::Error::Mixer(e)),
        _ => Err(super::Error::Unknown),
    };

    match mixers.lock().unwrap().input_add(&mixer, input.unwrap()) {
        Ok(_) => Ok(StatusCode::CREATED),
        Err(_) => Ok(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn input_list(
    mixer_name: String,
    mixers: Arc<Mutex<super::Mixers>>,
) -> Result<impl warp::Reply, Infallible> {
    let inputs: Vec<super::InputResponse> = mixers
        .lock()
        .unwrap()
        .mixers
        .get(&mixer_name)
        .unwrap()
        .inputs
        .iter()
        .map(|(_, input)| super::InputResponse {
            name: input.name(),
            input_type: input.input_type(),
            location: input.location(),
        })
        .collect();
    Ok(warp::reply::json(&inputs))
}

pub async fn input_get(
    mixer_name: String,
    input_name: String,
    mixers: Arc<Mutex<super::Mixers>>,
) -> Result<impl warp::Reply, Infallible> {
    let mixers = mixers.lock().unwrap();
    let input: Option<&mixer::Input> = mixers
        .mixers
        .get(&mixer_name)
        .unwrap()
        .inputs
        .get(input_name.as_str());

    if input.is_none() {
        return Ok(warp::reply::with_status(
            warp::reply::json(&""),
            StatusCode::NOT_FOUND,
        ));
    }

    let input = input.unwrap();
    let input = super::InputResponse {
        name: input.name(),
        input_type: input.input_type(),
        location: input.location(),
    };

    Ok(warp::reply::with_status(
        warp::reply::json(&input),
        StatusCode::OK,
    ))
}

pub async fn input_remove(
    mixer_name: String,
    input_name: String,
    mixers: Arc<Mutex<super::Mixers>>,
) -> Result<impl warp::Reply, Infallible> {
    match mixers
        .lock()
        .unwrap()
        .input_remove(&mixer_name, &input_name)
    {
        Ok(_) => Ok(StatusCode::OK),
        Err(_) => Ok(StatusCode::NOT_FOUND),
    }
}

pub async fn output_list(
    name: String,
    mixers: Arc<Mutex<super::Mixers>>,
) -> Result<impl warp::Reply, Infallible> {
    let outputs: Vec<super::OutputResponse> = mixers
        .lock()
        .unwrap()
        .mixers
        .get(&name)
        .unwrap()
        .outputs
        .iter()
        .map(|(_, output)| super::OutputResponse {
            name: output.name(),
            output_type: output.output_type(),
            location: output.location(),
        })
        .collect();
    Ok(warp::reply::json(&outputs))
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
        Ok(_) => Ok(StatusCode::CREATED),
        Err(_) => Ok(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn output_get(
    mixer_name: String,
    output_name: String,
    mixers: Arc<Mutex<super::Mixers>>,
) -> Result<impl warp::Reply, Infallible> {
    let mixers = mixers.lock().unwrap();
    let output: Option<&mixer::Output> = mixers
        .mixers
        .get(&mixer_name)
        .unwrap()
        .outputs
        .get(output_name.as_str());

    if output.is_none() {
        return Ok(warp::reply::with_status(
            warp::reply::json(&""),
            StatusCode::NOT_FOUND,
        ));
    }

    let output = output.unwrap();
    let output = super::OutputResponse {
        name: output.name(),
        output_type: output.output_type(),
        location: output.location(),
    };

    Ok(warp::reply::with_status(
        warp::reply::json(&output),
        StatusCode::OK,
    ))
}

pub async fn output_remove(
    mixer_name: String,
    output_name: String,
    mixers: Arc<Mutex<super::Mixers>>,
) -> Result<impl warp::Reply, Infallible> {
    match mixers
        .lock()
        .unwrap()
        .output_remove(&mixer_name, &output_name)
    {
        Ok(_) => Ok(StatusCode::OK),
        Err(_) => Ok(StatusCode::NOT_FOUND),
    }
}
