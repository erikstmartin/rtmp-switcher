extern crate serde;
extern crate serde_derive;

use crate::mixer;
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

type Error = Box<dyn std::error::Error + 'static>;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Mixer {
    pub name: String,
}

#[derive(Clone)]
pub struct Server {
    pub mixers: Arc<Mutex<HashMap<String, mixer::Mixer>>>,
}

impl Server {
    // TODO: Configuration for server
    pub fn new() -> Self {
        Server {
            mixers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn mixer_create(&self, name: &str) -> Result<(), Error> {
        let mut mixers = self.mixers.lock().unwrap();
        let mixer = mixer::Mixer::new(&name)?;
        mixers.insert(name.to_string(), mixer);

        Ok(())
    }

    pub async fn run(&self) {
        let addr: std::net::SocketAddr = "127.0.0.1:3030".parse().unwrap();
        warp::serve(filters::routes(self.clone())).run(addr).await;
    }
}

mod handlers {
    use std::convert::Infallible;
    use warp::*;

    pub async fn mixer_list(server: super::Server) -> Result<impl warp::Reply, Infallible> {
        let mixers: Vec<super::Mixer> = server
            .mixers
            .lock()
            .unwrap()
            .iter()
            .map(|(id, m)| super::Mixer {
                name: m.name.clone(),
            })
            .collect();
        Ok(warp::reply::json(&mixers))
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

    pub fn routes(
        server: super::Server,
    ) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
        mixer_list(server.clone()).or(mixer_get(server.clone()))
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
            .map(|name| format!("Mixer {}", name))
    }
}
