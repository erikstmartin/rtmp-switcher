use clap::{App, Arg};
use gstreamer as gst;
use std::net::SocketAddr;
use switcher::http::Server;
use thiserror::Error;

#[derive(Debug, Error)]
enum RTMPSwitcherError {
    #[error("failed setting up gstreamer {0}")]
    FailedInitGstreamer(#[from] gst::glib::Error),

    #[error("invalid listen address `{0}`")]
    InvalidSocketAddr(String),

    #[error("missing listen address")]
    MissingListenAddr,
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let matches = App::new("rtmpswitcher")
        .version("0.1.0")
        .about("It switches things")
        .arg(
            Arg::with_name("addr")
                .short("a")
                .long("addr")
                .value_name("ADDRESS")
                .help("sets the server listen address")
                .takes_value(true),
        )
        .get_matches();
    let addr: SocketAddr = parse_addr(
        matches
            .value_of("addr")
            .ok_or(RTMPSwitcherError::MissingListenAddr)?,
    )?;

    gst::init().map_err(RTMPSwitcherError::FailedInitGstreamer)?;

    let server = Server::new_with_config(addr);

    // let fut = warp::run(); tokio::select! { fut => {}, timeout => {}, signal => {} }
    server.run().await;

    Ok(())
}

fn parse_addr(raw_addr: &str) -> Result<SocketAddr, RTMPSwitcherError> {
    raw_addr
        .parse::<SocketAddr>()
        .map_err(|_| RTMPSwitcherError::InvalidSocketAddr(raw_addr.to_string()))
}
