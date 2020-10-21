mod filters;
pub mod input;
pub mod mixer;
pub mod output;

use crate::{
    input::Input,
    mixer::{Config as MixerConfig, Error as MixerError, Mixer},
    output::Output,
};
use regex::Regex;
use serde::Serialize;
use std::{
    collections::HashMap,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    sync::Arc,
};
use thiserror::Error;
use tokio::sync::Mutex;
use warp::{
    http::StatusCode,
    reject::{self, Reject},
    reply, Rejection, Reply,
};

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

    #[error("MixerError: '{0}'")]
    Mixer(#[from] MixerError),
}
impl Reject for Error {}

type JsonResult = Result<reply::WithStatus<reply::Json>, Rejection>;

pub fn error(error: Error) -> JsonResult {
    Err(reject::custom(error))
}

pub fn okay<T>(item: T) -> JsonResult
where
    T: Serialize,
{
    Ok(reply::with_status(reply::json(&item), StatusCode::OK))
}

pub fn message_response(message: &str, status: StatusCode) -> JsonResult {
    Ok(reply::with_status(
        reply::json(&Response {
            message: message.to_string(),
        }),
        status,
    ))
}

pub async fn recover(err: Rejection) -> Result<impl Reply, Rejection> {
    if let Some(error) = err.find::<Error>() {
        return Ok(reply::with_status(
            reply::json(&Response {
                message: format!("{}", error),
            }),
            match error {
                Error::Exists => StatusCode::BAD_REQUEST,
                Error::InvalidName => StatusCode::BAD_REQUEST,
                Error::Unknown => StatusCode::INTERNAL_SERVER_ERROR,
                Error::NotFound => StatusCode::NOT_FOUND,
                Error::Mixer(e) => match e {
                    MixerError::Exists(_, _) => StatusCode::BAD_REQUEST,
                    MixerError::Unknown => StatusCode::INTERNAL_SERVER_ERROR,
                    MixerError::NotFound(_, _) => StatusCode::NOT_FOUND,
                    MixerError::System(_) => StatusCode::INTERNAL_SERVER_ERROR,
                    MixerError::GstBool(_)
                    | MixerError::GstStateChange(_)
                    | MixerError::Gstreamer(_) => StatusCode::INTERNAL_SERVER_ERROR,
                },
            },
        ));
    }

    Err(err)
}

#[derive(Debug, Serialize)]
pub struct Response {
    pub message: String,
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

    pub async fn mixer_create(&mut self, config: MixerConfig) -> Result<(), Error> {
        self.mixers.lock().await.mixer_create(config)
    }

    pub async fn input_add(&mut self, mixer: &str, input: Input) -> Result<(), Error> {
        self.mixers.lock().await.input_add(mixer, input)
    }

    pub async fn output_add(&mut self, mixer: &str, output: Output) -> Result<(), Error> {
        self.mixers.lock().await.output_add(mixer, output)
    }
}

impl Default for Server {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Mixers {
    pub mixers: HashMap<String, Mixer>,
}

impl Mixers {
    pub fn mixer_config(&self, name: &str) -> Result<MixerConfig, Error> {
        match self.mixers.get(name) {
            Some(m) => Ok(m.config()),
            None => Err(Error::NotFound),
        }
    }

    pub fn mixer_create(&mut self, config: MixerConfig) -> Result<(), Error> {
        let re = Regex::new(r"^[a-zA-Z0-9-_]+$").map_err(|_| Error::Unknown)?;
        if !re.is_match(config.name.as_str()) {
            return Err(Error::InvalidName);
        }

        let name = config.name.clone();
        let mut mixer = Mixer::new(config)?;

        if self.mixers.contains_key(name.as_str()) {
            return Err(Error::Exists);
        }

        mixer.play()?;
        self.mixers.insert(name, mixer);

        Ok(())
    }

    pub fn input_add(&mut self, mixer: &str, input: Input) -> Result<(), Error> {
        match self.mixers.get_mut(mixer) {
            Some(m) => m.input_add(input).map_err(Error::Mixer),
            None => Err(Error::NotFound),
        }
    }

    pub fn input_remove(&mut self, mixer: &str, input: &str) -> Result<(), Error> {
        let mixer = self.mixers.get_mut(mixer).ok_or(Error::NotFound)?;

        mixer.input_remove(input)?;
        Ok(())
    }

    pub fn output_add(&mut self, mixer: &str, output: Output) -> Result<(), Error> {
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
    use crate::{
        http::{
            input::CreateRequest as InputCreateRequest, mixer::CreateRequest as MixerCreateRequest,
            output::CreateRequest as OutputCreateRequest,
        },
        input::Input,
        mixer,
        output::{Config as OutputConfig, EncoderConfig, Output},
        AudioConfig, VideoConfig,
    };
    use warp::{http::StatusCode, test::request};

    fn setup_server() -> Server {
        match gst::init() {
            Ok(_) => Server::new(),
            Err(_) => {
                tracing::error!("Failed to initialize gstreamer");
                panic!("Failed to initialize gstreamer");
            }
        }
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
                video: VideoConfig::default(),
                audio: AudioConfig::default(),
            })
            .reply(&api)
            .await;

        assert_eq!(resp.status(), StatusCode::CREATED);
        assert_eq!(1, server.mixers.lock().await.mixers.len());
    }

    #[tokio::test]
    async fn test_mixer_list() {
        let mut server = setup_server();
        let config = MixerConfig {
            name: "test_mixer_list".to_string(),
            audio: AudioConfig::default(),
            video: VideoConfig::default(),
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
        let config = MixerConfig {
            name: "test_mixer_get".to_string(),
            audio: AudioConfig::default(),
            video: VideoConfig::default(),
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
        let config = MixerConfig {
            name: "test_mixer_debug".to_string(),
            audio: AudioConfig::default(),
            video: VideoConfig::default(),
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
        let config = MixerConfig {
            name: "test_input_list".to_string(),
            audio: AudioConfig::default(),
            video: VideoConfig::default(),
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
        let config = MixerConfig {
            name: "test_input_add".to_string(),
            audio: AudioConfig::default(),
            video: VideoConfig::default(),
        };
        server
            .mixer_create(config)
            .await
            .expect("failed to create mixer");
        let api = filters::input_add(Arc::clone(&server.mixers).clone());

        let resp = request()
            .method("POST")
            .path("/mixers/test_input_add/inputs")
            .json(&InputCreateRequest {
                name: "test".to_string(),
                input_type: "URI".to_string(),
                location: "http://nowhere".to_string(),
                audio: AudioConfig::default(),
                video: VideoConfig::default(),
                record: false,
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
        let config = MixerConfig {
            name: mixer_name.to_string(),
            audio: AudioConfig::default(),
            video: VideoConfig::default(),
        };
        server
            .mixer_create(config)
            .await
            .expect("failed to create mixer");

        let input_config = crate::input::Config {
            name: "fakesrc".to_string(),
            audio: AudioConfig::default(),
            video: VideoConfig::default(),
            record: false,
        };

        server
            .input_add(
                mixer_name,
                Input::create_fake(input_config).expect("failed to create fakesrc"),
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
        let config = MixerConfig {
            name: mixer_name.to_string(),
            audio: AudioConfig::default(),
            video: VideoConfig::default(),
        };
        server
            .mixer_create(config)
            .await
            .expect("failed to create mixer");

        let input_config = crate::input::Config {
            name: "fakesrc".to_string(),
            audio: AudioConfig::default(),
            video: VideoConfig::default(),
            record: false,
        };

        server
            .input_add(
                mixer_name,
                Input::create_fake(input_config).expect("failed to create fakesrc"),
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
        let config = MixerConfig {
            name: "test_output_list".to_string(),
            audio: AudioConfig::default(),
            video: VideoConfig::default(),
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
        let config = MixerConfig {
            name: "test_output_add".to_string(),
            audio: AudioConfig::default(),
            video: VideoConfig::default(),
        };
        server
            .mixer_create(config)
            .await
            .expect("failed to create mixer");
        let api = filters::output_add(Arc::clone(&server.mixers));

        let resp = request()
            .method("POST")
            .path("/mixers/test_output_add/outputs")
            .json(&OutputCreateRequest {
                name: "test".to_string(),
                output_type: "Fake".to_string(),
                location: "http://nowhere".to_string(),
                audio: AudioConfig::default(),
                video: VideoConfig::default(),
                encoder: EncoderConfig::default(),
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
        let config = MixerConfig {
            name: mixer_name.to_string(),
            audio: AudioConfig::default(),
            video: VideoConfig::default(),
        };
        server
            .mixer_create(config)
            .await
            .expect("failed to create mixer");

        let output_config = OutputConfig {
            name: "fake".to_string(),
            audio: AudioConfig::default(),
            video: VideoConfig::default(),
            encoder: EncoderConfig::default(),
            mux: None,
        };

        server
            .output_add(
                mixer_name,
                Output::create_fake(output_config).expect("failed to create fake output"),
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
        let config = MixerConfig {
            name: mixer_name.to_string(),
            audio: AudioConfig::default(),
            video: VideoConfig::default(),
        };
        server
            .mixer_create(config)
            .await
            .expect("failed to create mixer");

        let output_config = OutputConfig {
            name: "fake".to_string(),
            audio: AudioConfig::default(),
            video: VideoConfig::default(),
            encoder: EncoderConfig::default(),
            mux: None,
        };

        server
            .output_add(
                mixer_name,
                Output::create_fake(output_config).expect("failed to create fake output"),
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
