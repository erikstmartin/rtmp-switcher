use crate::mixer;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use thiserror::Error;

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

#[derive(Clone)]
pub struct Server {
    pub mixers: HashMap<String, Arc<Mutex<mixer::Mixer>>>,
}

impl Server {
    // TODO: Configuration for server
    pub fn new() -> Self {
        Server {
            mixers: HashMap::new(),
        }
    }

    pub fn mixer_create(&mut self, name: &str) -> Result<(), Error> {
        let re = Regex::new(r"^[a-zA-Z0-9-]+$").unwrap();
        if !re.is_match(name) {
            return Err(Error::InvalidName);
        }
        let m = mixer::Mixer::new(name)?;
        let mixer = Arc::new(Mutex::new(m));

        if self.mixers.contains_key(name) {
            return Err(Error::Exists);
        }

        mixer.lock().unwrap().play();
        self.mixers.insert(name.to_string(), mixer);

        Ok(())
    }

    pub fn input_add(&mut self, mixer: &str, input: mixer::Input) -> Result<(), Error> {
        match self.mixers.get_mut(mixer) {
            Some(m) => m
                .lock()
                .unwrap()
                .input_add(input)
                .map_err(|e| Error::Mixer(e)),
            None => Err(Error::NotFound),
        }
    }

    pub fn output_add(&mut self, mixer: &str, output: mixer::Output) -> Result<(), Error> {
        match self.mixers.get_mut(mixer) {
            Some(m) => match m.lock().unwrap().output_add(output) {
                Ok(_) => Ok(()),
                Err(e) => Err(Error::Mixer(e)),
            },
            None => Err(Error::NotFound),
        }
    }

    pub async fn run(&self) {
        let addr: std::net::SocketAddr = "127.0.0.1:3030".parse().unwrap();

        warp::serve(filters::routes(self.clone())).run(addr).await;
    }
}

mod handlers {
    use std::convert::Infallible;
    use std::io::Write;
    use std::process::{Command, Stdio};
    use warp::http::StatusCode;
    use warp::reply::Response;

    pub async fn mixer_create(
        mixer: super::Mixer,
        mut server: super::Server,
    ) -> Result<impl warp::Reply, Infallible> {
        match server.mixer_create(&mixer.name) {
            Ok(_) => Ok(StatusCode::CREATED),
            Err(_) => Ok(StatusCode::INTERNAL_SERVER_ERROR),
        }
    }

    pub async fn mixer_get(
        name: String,
        server: super::Server,
    ) -> Result<impl warp::Reply, Infallible> {
        let mixer = server.mixers.get(name.as_str()).unwrap().lock().unwrap();

        let mixer = &super::Mixer {
            name: mixer.name.clone(),
            input_count: mixer.input_count(),
            output_count: mixer.output_count(),
        };

        Ok(warp::reply::json(mixer))
    }

    pub async fn mixer_debug(
        name: String,
        server: super::Server,
    ) -> Result<impl warp::Reply, Infallible> {
        let mixer = server.mixers.get(name.as_str()).unwrap().lock().unwrap();

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

    pub async fn mixer_list(server: super::Server) -> Result<impl warp::Reply, Infallible> {
        let mixers: Vec<super::Mixer> = server
            .mixers
            .iter()
            .map(|(_, m)| {
                let m = m.lock().unwrap();

                super::Mixer {
                    name: m.name.clone(),
                    input_count: m.input_count(),
                    output_count: m.output_count(),
                }
            })
            .collect();
        Ok(warp::reply::json(&mixers))
    }

    pub async fn input_list(
        name: String,
        server: super::Server,
    ) -> Result<impl warp::Reply, Infallible> {
        let inputs: Vec<super::Input> = server
            .mixers
            .get(&name)
            .unwrap()
            .lock()
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

    pub async fn output_list(
        name: String,
        server: super::Server,
    ) -> Result<impl warp::Reply, Infallible> {
        let outputs: Vec<super::Output> = server
            .mixers
            .get(&name)
            .unwrap()
            .lock()
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
}

mod filters {
    use super::handlers;
    use warp::*;

    fn with_server(
        server: super::Server,
    ) -> impl Filter<Extract = (super::Server,), Error = std::convert::Infallible> + Clone {
        warp::any().map(move || server.clone())
    }

    fn json_body() -> impl Filter<Extract = (super::Mixer,), Error = warp::Rejection> + Clone {
        // When accepting a body, we want a JSON body
        // (and to reject huge payloads)...
        warp::body::content_length_limit(1024 * 16).and(warp::body::json())
    }

    pub fn routes(
        server: super::Server,
    ) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
        mixer_list(server.clone())
            .or(mixer_get(server.clone()))
            .or(mixer_create(server.clone()))
            .or(mixer_debug(server.clone()))
            .or(input_list(server.clone()))
            .or(output_list(server.clone()))
    }

    fn mixer_create(
        server: super::Server,
    ) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
        warp::path!("mixers")
            .and(warp::post())
            .and(json_body())
            .and(with_server(server))
            .and_then(handlers::mixer_create)
    }

    fn mixer_list(
        server: super::Server,
    ) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
        warp::path!("mixers")
            .and(warp::get())
            .and(with_server(server))
            .and_then(handlers::mixer_list)
    }

    fn mixer_get(
        server: super::Server,
    ) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
        warp::path!("mixers" / String)
            .and(warp::get())
            .and(with_server(server))
            .and_then(handlers::mixer_get)
    }

    fn mixer_debug(
        server: super::Server,
    ) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
        warp::path!("mixers" / String / "debug")
            .and(warp::get())
            .and(with_server(server))
            .and_then(handlers::mixer_debug)
    }

    fn input_list(
        server: super::Server,
    ) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
        warp::path!("mixers" / String / "inputs")
            .and(warp::get())
            .and(with_server(server))
            .and_then(handlers::input_list)
    }

    fn output_list(
        server: super::Server,
    ) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
        warp::path!("mixers" / String / "outputs")
            .and(warp::get())
            .and(with_server(server))
            .and_then(handlers::output_list)
    }
}
