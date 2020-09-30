mod filters;
mod handlers;

use crate::mixer;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::Mutex;

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
pub struct MixerCreateRequest {
    pub name: String,
    pub video: Option<mixer::VideoConfig>,
    pub audio: Option<mixer::AudioConfig>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct MixerResponse {
    pub name: String,
    pub input_count: usize,
    pub output_count: usize,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct InputCreateRequest {
    pub name: String,
    pub input_type: String,
    pub location: String,
    pub audio: Option<mixer::AudioConfig>,
    pub video: Option<mixer::VideoConfig>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct InputUpdateRequest {
    pub volume: Option<f64>,
    pub zorder: Option<u32>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct InputResponse {
    pub name: String,
    pub input_type: String,
    pub location: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct OutputCreateRequest {
    pub name: String,
    pub output_type: String,
    pub location: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct OutputResponse {
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

    pub fn new() -> Self {
        Server {
            socket_addr: SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 3030)),
            mixers: Arc::new(Mutex::new(Mixers {
                mixers: HashMap::new(),
            })),
        }
    }

    pub async fn run(&self) {
        warp::serve(filters::routes(Arc::clone(&self.mixers)))
            .run(self.socket_addr)
            .await;
    }

    pub async fn mixer_create(&mut self, config: mixer::Config) -> Result<(), Error> {
        self.mixers.lock().await.mixer_create(config)
    }

    pub async fn input_add(&mut self, mixer: &str, input: mixer::Input) -> Result<(), Error> {
        self.mixers.lock().await.input_add(mixer, input)
    }

    pub async fn output_add(&mut self, mixer: &str, output: mixer::Output) -> Result<(), Error> {
        self.mixers.lock().await.output_add(mixer, output)
    }
}

pub struct Mixers {
    pub mixers: HashMap<String, mixer::Mixer>,
}

impl Mixers {
    pub fn mixer_config(&self, name: &str) -> Result<mixer::Config, Error> {
        match self.mixers.get(name) {
            Some(m) => Ok(m.config()),
            None => Err(Error::NotFound),
        }
    }

    pub fn mixer_create(&mut self, config: mixer::Config) -> Result<(), Error> {
        let re = Regex::new(r"^[a-zA-Z0-9-_]+$").unwrap();
        if !re.is_match(config.name.as_str()) {
            return Err(Error::InvalidName);
        }

        let name = config.name.clone();
        let mut mixer = mixer::Mixer::new(config)?;

        if self.mixers.contains_key(name.as_str()) {
            return Err(Error::Exists);
        }

        mixer.play()?;
        self.mixers.insert(name, mixer);

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
    use crate::mixer;
    use warp::http::StatusCode;
    use warp::test::request;

    fn setup_server() -> Server {
        gst::init().unwrap();
        Server::new()
    }

    #[tokio::test]
    async fn test_mixer_create() {
        let server = setup_server();
        let api = filters::mixer_create(Arc::clone(&server.mixers));

        let resp = request()
            .method("POST")
            .path("/mixers")
            .json(&MixerCreateRequest {
                name: "test_mixer_create".to_string(),
                video: Some(mixer::default_video_config()),
                audio: Some(mixer::default_audio_config()),
            })
            .reply(&api)
            .await;

        assert_eq!(resp.status(), StatusCode::CREATED);
        assert_eq!(1, server.mixers.lock().await.mixers.len());
    }

    #[tokio::test]
    async fn test_mixer_list() {
        let mut server = setup_server();
        let config = mixer::Config {
            name: "test_mixer_list".to_string(),
            ..mixer::default_config()
        };
        server
            .mixer_create(config)
            .await
            .expect("failed to create mixer");
        let api = filters::mixer_list(Arc::clone(&server.mixers));

        let resp = request().method("GET").path("/mixers").reply(&api).await;

        assert_eq!(StatusCode::OK, resp.status());
        assert!(resp.body().len() != 0);
    }

    #[tokio::test]
    async fn test_mixer_get() {
        let mut server = setup_server();
        let config = mixer::Config {
            name: "test_mixer_get".to_string(),
            ..mixer::default_config()
        };
        server
            .mixer_create(config)
            .await
            .expect("failed to create mixer");
        let api = filters::mixer_get(Arc::clone(&server.mixers));

        let resp = request()
            .method("GET")
            .path("/mixers/test_mixer_get")
            .reply(&api)
            .await;

        assert_eq!(StatusCode::OK, resp.status());
        assert!(resp.body().len() != 0);
    }

    #[tokio::test]
    async fn test_mixer_debug() {
        let mut server = setup_server();
        let config = mixer::Config {
            name: "test_mixer_debug".to_string(),
            ..mixer::default_config()
        };
        server
            .mixer_create(config)
            .await
            .expect("failed to create mixer");
        let api = filters::mixer_debug(Arc::clone(&server.mixers));

        let resp = request()
            .method("GET")
            .path("/mixers/test_mixer_debug/debug")
            .reply(&api)
            .await;

        assert_eq!(StatusCode::OK, resp.status());
        assert!(resp.body().len() != 0);
    }

    #[tokio::test]
    async fn test_input_list() {
        let mut server = setup_server();
        let config = mixer::Config {
            name: "test_input_list".to_string(),
            ..mixer::default_config()
        };
        server
            .mixer_create(config)
            .await
            .expect("failed to create mixer");
        let api = filters::input_list(Arc::clone(&server.mixers));

        let resp = request()
            .method("GET")
            .path("/mixers/test_input_list/inputs")
            .reply(&api)
            .await;

        assert_eq!(StatusCode::OK, resp.status());
        assert!(resp.body().len() != 0);
    }

    #[tokio::test]
    async fn test_input_add() {
        let mut server = setup_server();
        let config = mixer::Config {
            name: "test_input_add".to_string(),
            ..mixer::default_config()
        };
        server
            .mixer_create(config)
            .await
            .expect("failed to create mixer");
        let api = filters::input_add(Arc::clone(&server.mixers).clone());

        let resp = request()
            .method("POST")
            .path("/mixers/test_input_add/inputs")
            .json(&super::InputCreateRequest {
                name: "test".to_string(),
                input_type: "URI".to_string(),
                location: "http://nowhere".to_string(),
                video: Some(mixer::default_video_config()),
                audio: Some(mixer::default_audio_config()),
            })
            .reply(&api)
            .await;

        assert_eq!(resp.status(), StatusCode::CREATED);
        assert_eq!(
            1,
            server
                .mixers
                .lock()
                .await
                .mixers
                .get("test_input_add")
                .unwrap()
                .inputs
                .len()
        );
    }

    #[tokio::test]
    async fn test_input_get() {
        let mixer_name = "test_input_get";
        let mut server = setup_server();
        let config = mixer::Config {
            name: mixer_name.to_string(),
            ..mixer::default_config()
        };
        server
            .mixer_create(config)
            .await
            .expect("failed to create mixer");
        server
            .input_add(
                mixer_name,
                mixer::input::Fake::new("fakesrc").expect("failed to create fakesrc"),
            )
            .await
            .expect("Failed to add input");

        let api = filters::input_get(Arc::clone(&server.mixers));

        let resp = request()
            .method("GET")
            .path("/mixers/test_input_get/inputs/fakesrc")
            .reply(&api)
            .await;

        assert_eq!(StatusCode::OK, resp.status());
        assert!(resp.body().len() != 0);
    }

    #[tokio::test]
    async fn test_input_remove() {
        let mixer_name = "test_input_remove";
        let mut server = setup_server();
        let config = mixer::Config {
            name: mixer_name.to_string(),
            ..mixer::default_config()
        };
        server
            .mixer_create(config)
            .await
            .expect("failed to create mixer");
        server
            .input_add(
                mixer_name,
                mixer::input::Fake::new("fakesrc").expect("failed to create fakesrc"),
            )
            .await
            .expect("Failed to add input");

        let api = filters::input_remove(Arc::clone(&server.mixers));

        let resp = request()
            .method("DELETE")
            .path("/mixers/test_input_remove/inputs/fakesrc")
            .reply(&api)
            .await;

        assert_eq!(StatusCode::OK, resp.status());
        assert_eq!(
            0,
            server
                .mixers
                .lock()
                .await
                .mixers
                .get(mixer_name)
                .unwrap()
                .inputs
                .len()
        );
    }

    #[tokio::test]
    async fn test_output_list() {
        let mut server = setup_server();
        let config = mixer::Config {
            name: "test_output_list".to_string(),
            ..mixer::default_config()
        };
        server
            .mixer_create(config)
            .await
            .expect("failed to create mixer");
        let api = filters::output_list(Arc::clone(&server.mixers));

        let resp = request()
            .method("GET")
            .path("/mixers/test_output_list/outputs")
            .reply(&api)
            .await;

        assert_eq!(StatusCode::OK, resp.status());
        assert!(resp.body().len() != 0);
    }

    #[tokio::test]
    async fn test_output_add() {
        let mut server = setup_server();
        let config = mixer::Config {
            name: "test_output_add".to_string(),
            ..mixer::default_config()
        };
        server
            .mixer_create(config)
            .await
            .expect("failed to create mixer");
        let api = filters::output_add(Arc::clone(&server.mixers));

        let resp = request()
            .method("POST")
            .path("/mixers/test_output_add/outputs")
            .json(&super::OutputCreateRequest {
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
                .await
                .mixers
                .get("test_output_add")
                .unwrap()
                .outputs
                .len()
        );
    }

    #[tokio::test]
    async fn test_output_get() {
        let mixer_name = "test_output_get";
        let mut server = setup_server();
        let config = mixer::Config {
            name: mixer_name.to_string(),
            ..mixer::default_config()
        };
        server
            .mixer_create(config)
            .await
            .expect("failed to create mixer");
        server
            .output_add(
                mixer_name,
                mixer::output::Fake::new("fake").expect("failed to create fake output"),
            )
            .await
            .expect("Failed to add output");

        let api = filters::output_get(Arc::clone(&server.mixers));

        let resp = request()
            .method("GET")
            .path("/mixers/test_output_get/outputs/fake")
            .reply(&api)
            .await;

        assert_eq!(StatusCode::OK, resp.status());
        assert!(resp.body().len() != 0);
    }

    #[tokio::test]
    async fn test_output_remove() {
        let mixer_name = "test_output_remove";
        let mut server = setup_server();
        let config = mixer::Config {
            name: mixer_name.to_string(),
            ..mixer::default_config()
        };
        server
            .mixer_create(config)
            .await
            .expect("failed to create mixer");
        server
            .output_add(
                mixer_name,
                mixer::output::Fake::new("fake").expect("failed to create fake output"),
            )
            .await
            .expect("Failed to add output");

        let api = filters::output_remove(Arc::clone(&server.mixers));

        let resp = request()
            .method("DELETE")
            .path("/mixers/test_output_remove/outputs/fake")
            .reply(&api)
            .await;

        assert_eq!(StatusCode::OK, resp.status());
        assert_eq!(
            0,
            server
                .mixers
                .lock()
                .await
                .mixers
                .get(mixer_name)
                .unwrap()
                .outputs
                .len()
        );
    }
}
