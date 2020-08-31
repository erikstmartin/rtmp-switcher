extern crate gstreamer as gst;
use gst::prelude::*;

mod mixer;
use mixer::*;

fn main() {
    // Initialize GStreamer
    gst::init().unwrap();

    let rtmp_uri = "rtmp://learntv-transcoder.eastus.azurecontainer.io:1935/live/STREAM_KEY";
    let rtmp_uri2 = "rtmp://learntv-transcoder.eastus.azurecontainer.io:1935/live/STREAM_KEY2";

    let uri =
        "https://www.freedesktop.org/software/gstreamer-sdk/data/media/sintel_trailer-480p.webm";

    let mut mixer = Mixer::new("test").unwrap();

    mixer.add_input(uri);
    mixer.add_output(rtmp_uri);
    dbg!(mixer.add_output(rtmp_uri2));
    //mixer.remove_output(rtmp_uri);
    mixer.play();
}
