mod filters;
mod handlers;

use crate::mixer;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
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

pub struct Server {
    pub mixers: Arc<Mutex<Mixers>>,
    socket_addr: SocketAddr,
}

impl Server {
    pub fn new_with_config(socket_addr: SocketAddr) -> Self {
        Server {
            socket_addr,
            mixers: Arc::new(Mutex::new(Mixers {
                mixers: HashMap::new(),
            })),
        }
    }

    // TODO: Configuration for server
    pub fn new() -> Self {
        Server {
            socket_addr: SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 3030)),
            mixers: Arc::new(Mutex::new(Mixers {
                mixers: HashMap::new(),
            })),
        }
    }

    pub async fn run(&self) {
        warp::serve(filters::routes(self.mixers.clone()))
            .run(self.socket_addr)
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

    pub fn input_remove(&mut self, mixer: &str, input: &str) -> Result<(), Error> {
        let mixer = self.mixers.get_mut(mixer).ok_or(Error::NotFound)?;

        mixer.input_remove(input)?;
        Ok(())
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

    pub fn output_remove(&mut self, mixer: &str, output: &str) -> Result<(), Error> {
        let mixer = self.mixers.get_mut(mixer).ok_or(Error::NotFound)?;

        mixer.output_remove(output)?;
        Ok(())
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
                mixer::input::Fake::new("fakesrc").expect("failed to create fakesrc"),
            )
            .expect("Failed to add input");

        let api = filters::input_get(server.mixers.clone());

        let resp = request()
            .method("GET")
            .path("/mixers/test/inputs/fakesrc")
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
                mixer::input::Fake::new("fakesrc").expect("failed to create fakesrc"),
            )
            .expect("Failed to add input");

        let api = filters::input_remove(server.mixers.clone());

        let resp = request()
            .method("DELETE")
            .path("/mixers/test/inputs/fakesrc")
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
                output_type: "Fake".to_string(),
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
                mixer::output::Fake::new("fake").expect("failed to create fake output"),
            )
            .expect("Failed to add output");

        let api = filters::output_get(server.mixers.clone());

        let resp = request()
            .method("GET")
            .path("/mixers/test/outputs/fake")
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
                mixer::output::Fake::new("fake").expect("failed to create fake output"),
            )
            .expect("Failed to add output");

        let api = filters::output_remove(server.mixers.clone());

        let resp = request()
            .method("DELETE")
            .path("/mixers/test/outputs/fake")
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
