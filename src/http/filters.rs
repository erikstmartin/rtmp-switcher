use super::{input, mixer, output, recover};
use std::sync::Arc;
use tokio::sync::Mutex;
use warp::*;

/// Helper method used for passing our Mixer vect to the HTTP handler
fn with_mixers(
    mixers: Arc<Mutex<super::Mixers>>,
) -> impl Filter<Extract = (Arc<Mutex<super::Mixers>>,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || mixers.clone())
}

/// Generates HTTP routes.
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

/// Setup route for `POST /mixers`
pub(crate) fn mixer_create(
    mixers: Arc<Mutex<super::Mixers>>,
) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
    warp::path!("mixers")
        .and(warp::post())
        .and(mixer::CreateRequest::from_json_body())
        .and(with_mixers(mixers))
        .and_then(mixer::create)
        .recover(recover)
}

/// Setup route for `GET /mixers`
pub(crate) fn mixer_list(
    mixers: Arc<Mutex<super::Mixers>>,
) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
    warp::path!("mixers")
        .and(warp::get())
        .and(with_mixers(mixers))
        .and_then(mixer::list)
        .recover(recover)
}

/// Setup route for `GET /mixer/name`
pub(crate) fn mixer_get(
    mixers: Arc<Mutex<super::Mixers>>,
) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
    warp::path!("mixers" / String)
        .and(warp::get())
        .and(with_mixers(mixers))
        .and_then(mixer::get)
        .recover(recover)
}

/// Setup route for `GET /mixer/name/debug`
pub(crate) fn mixer_debug(
    mixers: Arc<Mutex<super::Mixers>>,
) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
    warp::path!("mixers" / String / "debug")
        .and(warp::get())
        .and(with_mixers(mixers))
        .and_then(mixer::debug)
        .recover(recover)
}

/// Setup route for `POST /mixers/name/inputs`
pub(crate) fn input_add(
    mixers: Arc<Mutex<super::Mixers>>,
) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
    warp::path!("mixers" / String / "inputs")
        .and(warp::post())
        .and(input::CreateRequest::from_json_body())
        .and(with_mixers(mixers))
        .and_then(input::add)
        .recover(recover)
}

/// Setup route for `GET /mixers/name/inputs`
pub(crate) fn input_list(
    mixers: Arc<Mutex<super::Mixers>>,
) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
    warp::path!("mixers" / String / "inputs")
        .and(warp::get())
        .and(with_mixers(mixers))
        .and_then(input::list)
        .recover(recover)
}

/// Setup route for `GET /mixers/name/inputs/name`
pub(crate) fn input_get(
    mixers: Arc<Mutex<super::Mixers>>,
) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
    warp::path!("mixers" / String / "inputs" / String)
        .and(warp::get())
        .and(with_mixers(mixers))
        .and_then(input::get)
        .recover(recover)
}

/// Setup route for `PUT /mixers/name/inputs/name`
pub(crate) fn input_update(
    mixers: Arc<Mutex<super::Mixers>>,
) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
    warp::path!("mixers" / String / "inputs" / String)
        .and(warp::put())
        .and(input::UpdateRequest::from_json_body())
        .and(with_mixers(mixers))
        .and_then(input::update)
        .recover(recover)
}

/// Setup route for `DELETE /mixers/name/inputs/name`
pub(crate) fn input_remove(
    mixers: Arc<Mutex<super::Mixers>>,
) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
    warp::path!("mixers" / String / "inputs" / String)
        .and(warp::delete())
        .and(with_mixers(mixers))
        .and_then(input::remove)
        .recover(recover)
}

/// Setup route for `POST /mixers/name/set_active_input`
pub(crate) fn input_set_active(
    mixers: Arc<Mutex<super::Mixers>>,
) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
    warp::path!("mixers" / String / "set_active_input" / String)
        .and(warp::post())
        .and(with_mixers(mixers))
        .and_then(input::set_active)
        .recover(recover)
}

/// Setup route for `GET /mixers/name/outputs`
pub(crate) fn output_list(
    mixers: Arc<Mutex<super::Mixers>>,
) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
    warp::path!("mixers" / String / "outputs")
        .and(warp::get())
        .and(with_mixers(mixers))
        .and_then(output::list)
        .recover(recover)
}

/// Setup route for `POST /mixers/name/outputs`
pub(crate) fn output_add(
    mixers: Arc<Mutex<super::Mixers>>,
) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
    warp::path!("mixers" / String / "outputs")
        .and(warp::post())
        .and(output::CreateRequest::from_json_body())
        .and(with_mixers(mixers))
        .and_then(output::add)
        .recover(recover)
}

/// Setup route for `GET /mixers/name/outputs/name`
pub(crate) fn output_get(
    mixers: Arc<Mutex<super::Mixers>>,
) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
    warp::path!("mixers" / String / "outputs" / String)
        .and(warp::get())
        .and(with_mixers(mixers))
        .and_then(output::get)
        .recover(recover)
}

/// Setup route for `DELETE /mixers/name/outputs/name`
pub(crate) fn output_remove(
    mixers: Arc<Mutex<super::Mixers>>,
) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
    warp::path!("mixers" / String / "outputs" / String)
        .and(warp::delete())
        .and(with_mixers(mixers))
        .and_then(output::remove)
        .recover(recover)
}
