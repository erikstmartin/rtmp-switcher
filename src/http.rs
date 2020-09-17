use crate::mixer;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use thiserror::Error;

// TODO: Ensure we sanity check that mixer exists before trying to work with inputs and outputs.
// just say no to crashing...

#[derive(Error, Debug)]
pub enum Error {
    #[error("unknown error")]
    Unknown,

    #[error("already exists")]
    Exists,

    #[error("not found")]
    NotFound,

    #[error("name is invalid")]
    InvalidName,

    #[error("An error was returned from the mixer: '{0}'")]
    Mixer(#[from] mixer::Error),
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Mixer {
    pub name: String,
    pub input_count: usize,
    pub output_count: usize,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Input {
    pub name: String,
    pub input_type: String,
    pub location: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Output {
    pub name: String,
    pub output_type: String,
    pub location: String,
}

pub struct Server {
    pub mixers: Arc<Mutex<Mixers>>,
}

impl Server {
    // TODO: Configuration for server
    pub fn new() -> Self {
        Server {
            mixers: Arc::new(Mutex::new(Mixers {
                mixers: HashMap::new(),
            })),
        }
    }

    pub async fn run(&self) {
        let addr: std::net::SocketAddr = "127.0.0.1:3030".parse().unwrap();

        warp::serve(filters::routes(self.mixers.clone()))
            .run(addr)
            .await;
    }

    pub fn mixer_create(&mut self, name: &str) -> Result<(), Error> {
        self.mixers.lock().unwrap().mixer_create(name)
    }

    pub fn input_add(&mut self, mixer: &str, input: mixer::Input) -> Result<(), Error> {
        self.mixers.lock().unwrap().input_add(mixer, input)
    }

    pub fn output_add(&mut self, mixer: &str, output: mixer::Output) -> Result<(), Error> {
        self.mixers.lock().unwrap().output_add(mixer, output)
    }
}

pub struct Mixers {
    pub mixers: HashMap<String, mixer::Mixer>,
}

impl Mixers {
    pub fn mixer_create(&mut self, name: &str) -> Result<(), Error> {
        let re = Regex::new(r"^[a-zA-Z0-9-]+$").unwrap();
        if !re.is_match(name) {
            return Err(Error::InvalidName);
        }
        let mut mixer = mixer::Mixer::new(name)?;

        if self.mixers.contains_key(name) {
            return Err(Error::Exists);
        }

        mixer.play()?;
        self.mixers.insert(name.to_string(), mixer);

        Ok(())
    }

    pub fn input_add(&mut self, mixer: &str, input: mixer::Input) -> Result<(), Error> {
        match self.mixers.get_mut(mixer) {
            Some(m) => m.input_add(input).map_err(|e| Error::Mixer(e)),
            None => Err(Error::NotFound),
        }
    }

    pub fn output_add(&mut self, mixer: &str, output: mixer::Output) -> Result<(), Error> {
        match self.mixers.get_mut(mixer) {
            Some(m) => match m.output_add(output) {
                Ok(_) => Ok(()),
                Err(e) => Err(Error::Mixer(e)),
            },
            None => Err(Error::NotFound),
        }
    }
}

mod handlers {
    use crate::mixer;
    use std::convert::Infallible;
    use std::io::Write;
    use std::process::{Command, Stdio};
    use std::sync::{Arc, Mutex};
    use warp::http::StatusCode;

    pub async fn mixer_create(
        mixer: super::Mixer,
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

        let mixer = &super::Mixer {
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

    pub async fn mixer_list(
        mixers: Arc<Mutex<super::Mixers>>,
    ) -> Result<impl warp::Reply, Infallible> {
        let mixers: Vec<super::Mixer> = mixers
            .lock()
            .unwrap()
            .mixers
            .iter()
            .map(|(_, m)| super::Mixer {
                name: m.name.clone(),
                input_count: m.input_count(),
                output_count: m.output_count(),
            })
            .collect();
        Ok(warp::reply::json(&mixers))
    }

    pub async fn input_add(
        mixer: String,
        input: super::Input,
        mixers: Arc<Mutex<super::Mixers>>,
    ) -> Result<impl warp::Reply, Infallible> {
        let input = match input.input_type.as_str() {
            "URI" => crate::mixer::input::URI::new(&input.name, &input.location)
                .map_err(|e| super::Error::Mixer(e)),
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
        let inputs: Vec<super::Input> = mixers
            .lock()
            .unwrap()
            .mixers
            .get(&mixer_name)
            .unwrap()
            .inputs
            .iter()
            .map(|(_, input)| super::Input {
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
        let input = super::Input {
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
        let mut mixers = mixers.lock().unwrap();
        let input: Option<mixer::Input> = mixers
            .mixers
            .get_mut(&mixer_name)
            .unwrap()
            .inputs
            .remove(input_name.as_str());

        match input {
            Some(_) => Ok(StatusCode::OK),
            None => Ok(StatusCode::NOT_FOUND),
        }
    }

    pub async fn output_list(
        name: String,
        mixers: Arc<Mutex<super::Mixers>>,
    ) -> Result<impl warp::Reply, Infallible> {
        let outputs: Vec<super::Output> = mixers
            .lock()
            .unwrap()
            .mixers
            .get(&name)
            .unwrap()
            .outputs
            .iter()
            .map(|(_, output)| super::Output {
                name: output.name(),
                output_type: output.output_type(),
                location: output.location(),
            })
            .collect();
        Ok(warp::reply::json(&outputs))
    }

    pub async fn output_add(
        mixer: String,
        output: super::Output,
        mixers: Arc<Mutex<super::Mixers>>,
    ) -> Result<impl warp::Reply, Infallible> {
        let output = match output.output_type.as_str() {
            "Auto" => {
                crate::mixer::output::Auto::new(&output.name).map_err(|e| super::Error::Mixer(e))
            }
            "RTMP" => crate::mixer::output::RTMP::new(&output.name, &output.location)
                .map_err(|e| super::Error::Mixer(e)),
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
        let output = super::Output {
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
        let mut mixers = mixers.lock().unwrap();
        let output: Option<mixer::Output> = mixers
            .mixers
            .get_mut(&mixer_name)
            .unwrap()
            .outputs
            .remove(output_name.as_str());

        match output {
            Some(_) => Ok(StatusCode::OK),
            None => Ok(StatusCode::NOT_FOUND),
        }
    }
}

mod filters {
    use super::handlers;
    use std::sync::{Arc, Mutex};
    use warp::*;

    fn with_mixers(
        mixers: Arc<Mutex<super::Mixers>>,
    ) -> impl Filter<Extract = (Arc<Mutex<super::Mixers>>,), Error = std::convert::Infallible> + Clone
    {
        warp::any().map(move || mixers.clone())
    }

    fn mixer_json_body() -> impl Filter<Extract = (super::Mixer,), Error = warp::Rejection> + Clone
    {
        // When accepting a body, we want a JSON body
        // (and to reject huge payloads)...
        warp::body::content_length_limit(1024 * 16).and(warp::body::json())
    }

    // TODO: Can we use generics so that we don't need to duplicate this?
    fn input_json_body() -> impl Filter<Extract = (super::Input,), Error = warp::Rejection> + Clone
    {
        // When accepting a body, we want a JSON body
        // (and to reject huge payloads)...
        warp::body::content_length_limit(1024 * 16).and(warp::body::json())
    }

    // TODO: Can we use generics so that we don't need to duplicate this?
    fn output_json_body() -> impl Filter<Extract = (super::Output,), Error = warp::Rejection> + Clone
    {
        // When accepting a body, we want a JSON body
        // (and to reject huge payloads)...
        warp::body::content_length_limit(1024 * 16).and(warp::body::json())
    }

    pub fn routes(
        mixers: Arc<Mutex<super::Mixers>>,
    ) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
        mixer_list(mixers.clone())
            .or(mixer_get(mixers.clone()))
            .or(mixer_create(mixers.clone()))
            .or(mixer_debug(mixers.clone()))
            .or(input_list(mixers.clone()))
            .or(input_get(mixers.clone()))
            .or(input_add(mixers.clone()))
            .or(input_remove(mixers.clone()))
            .or(output_list(mixers.clone()))
            .or(output_get(mixers.clone()))
            .or(output_add(mixers.clone()))
            .or(output_remove(mixers.clone()))
    }

    pub(crate) fn mixer_create(
        mixers: Arc<Mutex<super::Mixers>>,
    ) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
        warp::path!("mixers")
            .and(warp::post())
            .and(mixer_json_body())
            .and(with_mixers(mixers))
            .and_then(handlers::mixer_create)
    }

    pub(crate) fn mixer_list(
        mixers: Arc<Mutex<super::Mixers>>,
    ) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
        warp::path!("mixers")
            .and(warp::get())
            .and(with_mixers(mixers))
            .and_then(handlers::mixer_list)
    }

    pub(crate) fn mixer_get(
        mixers: Arc<Mutex<super::Mixers>>,
    ) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
        warp::path!("mixers" / String)
            .and(warp::get())
            .and(with_mixers(mixers))
            .and_then(handlers::mixer_get)
    }

    pub(crate) fn mixer_debug(
        mixers: Arc<Mutex<super::Mixers>>,
    ) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
        warp::path!("mixers" / String / "debug")
            .and(warp::get())
            .and(with_mixers(mixers))
            .and_then(handlers::mixer_debug)
    }

    pub(crate) fn input_add(
        mixers: Arc<Mutex<super::Mixers>>,
    ) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
        warp::path!("mixers" / String / "inputs")
            .and(warp::post())
            .and(input_json_body())
            .and(with_mixers(mixers))
            .and_then(handlers::input_add)
    }

    pub(crate) fn input_list(
        mixers: Arc<Mutex<super::Mixers>>,
    ) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
        warp::path!("mixers" / String / "inputs")
            .and(warp::get())
            .and(with_mixers(mixers))
            .and_then(handlers::input_list)
    }

    pub(crate) fn input_get(
        mixers: Arc<Mutex<super::Mixers>>,
    ) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
        warp::path!("mixers" / String / "inputs" / String)
            .and(warp::get())
            .and(with_mixers(mixers))
            .and_then(handlers::input_get)
    }

    pub(crate) fn input_remove(
        mixers: Arc<Mutex<super::Mixers>>,
    ) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
        warp::path!("mixers" / String / "inputs" / String)
            .and(warp::delete())
            .and(with_mixers(mixers))
            .and_then(handlers::input_remove)
    }

    pub(crate) fn output_list(
        mixers: Arc<Mutex<super::Mixers>>,
    ) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
        warp::path!("mixers" / String / "outputs")
            .and(warp::get())
            .and(with_mixers(mixers))
            .and_then(handlers::output_list)
    }

    pub(crate) fn output_add(
        mixers: Arc<Mutex<super::Mixers>>,
    ) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
        warp::path!("mixers" / String / "outputs")
            .and(warp::post())
            .and(output_json_body())
            .and(with_mixers(mixers))
            .and_then(handlers::output_add)
    }

    pub(crate) fn output_get(
        mixers: Arc<Mutex<super::Mixers>>,
    ) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
        warp::path!("mixers" / String / "outputs" / String)
            .and(warp::get())
            .and(with_mixers(mixers))
            .and_then(handlers::output_get)
    }

    pub(crate) fn output_remove(
        mixers: Arc<Mutex<super::Mixers>>,
    ) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
        warp::path!("mixers" / String / "outputs" / String)
            .and(warp::delete())
            .and(with_mixers(mixers))
            .and_then(handlers::output_remove)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use warp::http::StatusCode;
    use warp::test::request;

    fn setup_server() -> Server {
        gst::init().unwrap();
        Server::new()
    }

    #[tokio::test]
    async fn test_mixer_create() {
        let server = setup_server();
        let api = filters::mixer_create(server.mixers.clone());

        let resp = request()
            .method("POST")
            .path("/mixers")
            .json(&Mixer {
                name: "test".to_string(),
                input_count: 0,
                output_count: 0,
            })
            .reply(&api)
            .await;

        assert_eq!(resp.status(), StatusCode::CREATED);
        assert_eq!(1, server.mixers.lock().unwrap().mixers.len());
    }

    #[tokio::test]
    async fn test_mixer_list() {
        let mut server = setup_server();
        server.mixer_create("test").expect("failed to create mixer");
        let api = filters::mixer_list(server.mixers.clone());

        let resp = request().method("GET").path("/mixers").reply(&api).await;

        assert_eq!(StatusCode::OK, resp.status());
        assert!(resp.body().len() != 0);
    }

    #[tokio::test]
    async fn test_mixer_get() {
        let mut server = setup_server();
        server.mixer_create("test").expect("failed to create mixer");
        let api = filters::mixer_get(server.mixers.clone());

        let resp = request()
            .method("GET")
            .path("/mixers/test")
            .reply(&api)
            .await;

        assert_eq!(StatusCode::OK, resp.status());
        assert!(resp.body().len() != 0);
    }

    #[tokio::test]
    async fn test_mixer_debug() {
        let mut server = setup_server();
        server.mixer_create("test").expect("failed to create mixer");
        let api = filters::mixer_debug(server.mixers.clone());

        let resp = request()
            .method("GET")
            .path("/mixers/test/debug")
            .reply(&api)
            .await;

        assert_eq!(StatusCode::OK, resp.status());
        assert!(resp.body().len() != 0);
    }

    #[tokio::test]
    async fn test_input_list() {
        let mut server = setup_server();
        server.mixer_create("test").expect("failed to create mixer");
        let api = filters::input_list(server.mixers.clone());

        let resp = request()
            .method("GET")
            .path("/mixers/test/inputs")
            .reply(&api)
            .await;

        assert_eq!(StatusCode::OK, resp.status());
        assert!(resp.body().len() != 0);
    }

    #[tokio::test]
    async fn test_input_add() {
        let mut server = setup_server();
        server.mixer_create("test").expect("failed to create mixer");
        let api = filters::input_add(server.mixers.clone());

        let resp = request()
            .method("POST")
            .path("/mixers/test/inputs")
            .json(&super::Input {
                name: "test".to_string(),
                input_type: "URI".to_string(),
                location: "http://nowhere".to_string(),
            })
            .reply(&api)
            .await;

        assert_eq!(resp.status(), StatusCode::CREATED);
        assert_eq!(
            1,
            server
                .mixers
                .lock()
                .unwrap()
                .mixers
                .get("test")
                .unwrap()
                .inputs
                .len()
        );
    }

    #[tokio::test]
    async fn test_input_get() {
        let mixer_name = "test";
        let mut server = setup_server();
        server
            .mixer_create(mixer_name)
            .expect("failed to create mixer");
        server
            .input_add(
                mixer_name,
                // TODO: Replace this with a testsrc
                mixer::input::URI::new("sintel", "https://www.freedesktop.org/software/gstreamer-sdk/data/media/sintel_trailer-480p.webm").expect("Failed to build Input from uri"),
            )
            .expect("Failed to add input");

        let api = filters::input_get(server.mixers.clone());

        let resp = request()
            .method("GET")
            .path("/mixers/test/inputs/sintel")
            .reply(&api)
            .await;

        assert_eq!(StatusCode::OK, resp.status());
        assert!(resp.body().len() != 0);
    }

    #[tokio::test]
    async fn test_input_remove() {
        let mixer_name = "test";
        let mut server = setup_server();
        server
            .mixer_create(mixer_name)
            .expect("failed to create mixer");
        server
            .input_add(
                mixer_name,
                // TODO: Replace this with a testsrc
                mixer::input::URI::new("sintel", "https://www.freedesktop.org/software/gstreamer-sdk/data/media/sintel_trailer-480p.webm").expect("Failed to build Input from uri"),
            )
            .expect("Failed to add input");

        let api = filters::input_remove(server.mixers.clone());

        let resp = request()
            .method("DELETE")
            .path("/mixers/test/inputs/sintel")
            .reply(&api)
            .await;

        assert_eq!(StatusCode::OK, resp.status());
        assert_eq!(
            0,
            server
                .mixers
                .lock()
                .unwrap()
                .mixers
                .get("test")
                .unwrap()
                .inputs
                .len()
        );
    }

    #[tokio::test]
    async fn test_output_list() {
        let mut server = setup_server();
        server.mixer_create("test").expect("failed to create mixer");
        let api = filters::output_list(server.mixers.clone());

        let resp = request()
            .method("GET")
            .path("/mixers/test/outputs")
            .reply(&api)
            .await;

        assert_eq!(StatusCode::OK, resp.status());
        assert!(resp.body().len() != 0);
    }

    #[tokio::test]
    async fn test_output_add() {
        let mut server = setup_server();
        server.mixer_create("test").expect("failed to create mixer");
        let api = filters::output_add(server.mixers.clone());

        let resp = request()
            .method("POST")
            .path("/mixers/test/outputs")
            .json(&super::Output {
                name: "test".to_string(),
                output_type: "Auto".to_string(),
                location: "http://nowhere".to_string(),
            })
            .reply(&api)
            .await;

        assert_eq!(resp.status(), StatusCode::CREATED);
        assert_eq!(
            1,
            server
                .mixers
                .lock()
                .unwrap()
                .mixers
                .get("test")
                .unwrap()
                .outputs
                .len()
        );
    }

    #[tokio::test]
    async fn test_output_get() {
        let mixer_name = "test";
        let mut server = setup_server();
        server
            .mixer_create(mixer_name)
            .expect("failed to create mixer");
        server
            .output_add(
                mixer_name,
                // TODO: Replace this with a fakesink
                mixer::output::Auto::new("auto").expect("Failed to build Output"),
            )
            .expect("Failed to add output");

        let api = filters::output_get(server.mixers.clone());

        let resp = request()
            .method("GET")
            .path("/mixers/test/outputs/auto")
            .reply(&api)
            .await;

        assert_eq!(StatusCode::OK, resp.status());
        assert!(resp.body().len() != 0);
    }

    #[tokio::test]
    async fn test_output_remove() {
        let mixer_name = "test";
        let mut server = setup_server();
        server
            .mixer_create(mixer_name)
            .expect("failed to create mixer");
        server
            .output_add(
                mixer_name,
                // TODO: Replace this with a fakesink
                mixer::output::Auto::new("auto").expect("Failed to build Output"),
            )
            .expect("Failed to add output");

        let api = filters::output_remove(server.mixers.clone());

        let resp = request()
            .method("DELETE")
            .path("/mixers/test/outputs/auto")
            .reply(&api)
            .await;

        assert_eq!(StatusCode::OK, resp.status());
        assert_eq!(
            0,
            server
                .mixers
                .lock()
                .unwrap()
                .mixers
                .get("test")
                .unwrap()
                .outputs
                .len()
        );
    }
}
