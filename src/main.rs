mod http;
mod input;
mod mixer;
mod output;

use http::Server;
extern crate gstreamer as gst;

// TODO: Bring in Clap for command line arguments

/*
fn main() {
    // Initialize GStreamer
    gst::init().unwrap();

    let rtmp_uri = "rtmp://learntv-transcoder.eastus.azurecontainer.io:1935/live/STREAM_KEY";
    let rtmp_uri2 = "rtmp://learntv-backup.eastus.azurecontainer.io:1935/live/STREAM_KEY";

    let uri =
        "https://www.freedesktop.org/software/gstreamer-sdk/data/media/sintel_trailer-480p.webm";

    let mut mixer = Mixer::new("test").unwrap();

    mixer
        .add_input(URI::new("sintel", uri).expect("Failed to build Input from uri"))
        .expect("Failed to add input");

    mixer
        .add_output(Auto::new("auto").expect("Failed to build Output"))
        .expect("Failed to add output");
    /*
     */
    mixer
        .add_output(RTMP::new("rtmp", rtmp_uri).expect("Failed to build Output"))
        .expect("Failed to add output");

    mixer
        .add_output(RTMP::new("backup", rtmp_uri2).expect("Failed to build Output"))
        .expect("Failed to add output");

    mixer
        .remove_output("backup")
        .expect("Failed to remove output");

    mixer.play().expect("Error setting pipeline state to play");
}
*/

#[tokio::main]
async fn main() {
    gst::init().unwrap();

    let server = Server::new();
    server
        .mixer_create("erikdotdev")
        .expect("failed to create new mixer");

    // let fut = warp::run(); tokio::select! { fut => {}, timeout => {}, signal => {} }
    server.run().await;
}
