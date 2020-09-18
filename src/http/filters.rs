use super::handlers;
use std::sync::{Arc, Mutex};
use warp::*;

fn with_mixers(
    mixers: Arc<Mutex<super::Mixers>>,
) -> impl Filter<Extract = (Arc<Mutex<super::Mixers>>,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || mixers.clone())
}

fn mixer_json_body() -> impl Filter<Extract = (super::Mixer,), Error = warp::Rejection> + Clone {
    // When accepting a body, we want a JSON body
    // (and to reject huge payloads)...
    warp::body::content_length_limit(1024 * 16).and(warp::body::json())
}

// TODO: Can we use generics so that we don't need to duplicate this?
fn input_json_body() -> impl Filter<Extract = (super::Input,), Error = warp::Rejection> + Clone {
    // When accepting a body, we want a JSON body
    // (and to reject huge payloads)...
    warp::body::content_length_limit(1024 * 16).and(warp::body::json())
}

// TODO: Can we use generics so that we don't need to duplicate this?
fn output_json_body() -> impl Filter<Extract = (super::Output,), Error = warp::Rejection> + Clone {
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
