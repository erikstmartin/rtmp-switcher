use gstreamer as gst;
use switcher::http::Server;

// TODO: Bring in Clap for command line arguments

#[tokio::main]
async fn main() {
    gst::init().unwrap();

    let server = Server::new();

    // let fut = warp::run(); tokio::select! { fut => {}, timeout => {}, signal => {} }
    server.run().await;
}
