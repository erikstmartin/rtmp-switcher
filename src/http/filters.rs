use super::input;
use super::mixer;
use super::output;
use std::sync::Arc;
use tokio::sync::Mutex;
use warp::*;

fn with_mixers(
    mixers: Arc<Mutex<super::Mixers>>,
) -> impl Filter<Extract = (Arc<Mutex<super::Mixers>>,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || mixers.clone())
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
        .or(input_update(mixers.clone()))
        .or(input_remove(mixers.clone()))
        .or(input_set_active(mixers.clone()))
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
        .and(mixer::CreateRequest::from_json_body())
        .and(with_mixers(mixers))
        .and_then(mixer::create)
}

pub(crate) fn mixer_list(
    mixers: Arc<Mutex<super::Mixers>>,
) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
    warp::path!("mixers")
        .and(warp::get())
        .and(with_mixers(mixers))
        .and_then(mixer::list)
}

pub(crate) fn mixer_get(
    mixers: Arc<Mutex<super::Mixers>>,
) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
    warp::path!("mixers" / String)
        .and(warp::get())
        .and(with_mixers(mixers))
        .and_then(mixer::get)
}

pub(crate) fn mixer_debug(
    mixers: Arc<Mutex<super::Mixers>>,
) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
    warp::path!("mixers" / String / "debug")
        .and(warp::get())
        .and(with_mixers(mixers))
        .and_then(mixer::debug)
}

pub(crate) fn input_add(
    mixers: Arc<Mutex<super::Mixers>>,
) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
    warp::path!("mixers" / String / "inputs")
        .and(warp::post())
        .and(input::CreateRequest::from_json_body())
        .and(with_mixers(mixers))
        .and_then(input::add)
}

pub(crate) fn input_list(
    mixers: Arc<Mutex<super::Mixers>>,
) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
    warp::path!("mixers" / String / "inputs")
        .and(warp::get())
        .and(with_mixers(mixers))
        .and_then(input::list)
}

pub(crate) fn input_get(
    mixers: Arc<Mutex<super::Mixers>>,
) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
    warp::path!("mixers" / String / "inputs" / String)
        .and(warp::get())
        .and(with_mixers(mixers))
        .and_then(input::get)
}

pub(crate) fn input_update(
    mixers: Arc<Mutex<super::Mixers>>,
) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
    warp::path!("mixers" / String / "inputs" / String)
        .and(warp::put())
        .and(input::UpdateRequest::from_json_body())
        .and(with_mixers(mixers))
        .and_then(input::update)
}

pub(crate) fn input_remove(
    mixers: Arc<Mutex<super::Mixers>>,
) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
    warp::path!("mixers" / String / "inputs" / String)
        .and(warp::delete())
        .and(with_mixers(mixers))
        .and_then(input::remove)
}

pub(crate) fn input_set_active(
    mixers: Arc<Mutex<super::Mixers>>,
) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
    warp::path!("mixers" / String / "set_active_input" / String)
        .and(warp::post())
        .and(with_mixers(mixers))
        .and_then(input::set_active)
}

pub(crate) fn output_list(
    mixers: Arc<Mutex<super::Mixers>>,
) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
    warp::path!("mixers" / String / "outputs")
        .and(warp::get())
        .and(with_mixers(mixers))
        .and_then(output::list)
}

pub(crate) fn output_add(
    mixers: Arc<Mutex<super::Mixers>>,
) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
    warp::path!("mixers" / String / "outputs")
        .and(warp::post())
        .and(output::CreateRequest::from_json_body())
        .and(with_mixers(mixers))
        .and_then(output::add)
}

pub(crate) fn output_get(
    mixers: Arc<Mutex<super::Mixers>>,
) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
    warp::path!("mixers" / String / "outputs" / String)
        .and(warp::get())
        .and(with_mixers(mixers))
        .and_then(output::get)
}

pub(crate) fn output_remove(
    mixers: Arc<Mutex<super::Mixers>>,
) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
    warp::path!("mixers" / String / "outputs" / String)
        .and(warp::delete())
        .and(with_mixers(mixers))
        .and_then(output::remove)
}
