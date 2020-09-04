extern crate gstreamer as gst;
use gst::prelude::*;

mod mixer;
use mixer::*;

fn main() {
    // Initialize GStreamer
    gst::init().unwrap();

    let rtmp_uri = "rtmp://learntv-transcoder.eastus.azurecontainer.io:1935/live/STREAM_KEY";

    let uri =
        "https://www.freedesktop.org/software/gstreamer-sdk/data/media/sintel_trailer-480p.webm";
    let uri2 = "https://davelage.com/art/0001-0350.mp4";
    let uri3 = "https://rockerboo.net/yeah-its-super-creative.mp4";

    let mut mixer = Mixer::new("test").unwrap();

    /* mixer
         .add_input("sintel", uri, 0)
         .expect("Failed to add input");
    */
    mixer
        .add_input("rockerBOO", uri3, 0)
        .expect("Failed to add input");
    mixer
        .add_output("main", rtmp_uri)
        .expect("Failed to add output");

    mixer.play().expect("Error setting pipeline state to play");

    /*
    uridecodebin uri=https://www.freedesktop.org/software/gstreamer-sdk/data/media/sintel_trailer-480p.webm !\
    mixer.sink_1 \

    gst-launch-1.0 \
    uridecodebin uri=https://www.freedesktop.org/software/gstreamer-sdk/data/media/sintel_trailer-480p.webm !\
    mixer.sink_0 \
    videomixer name=mixer ! \
    videoconvert ! \
    autovideosink

    x264enc ! flvmux ! rtmpsink location=rtmp://learntv-transcoder.eastus.azurecontainer.io:1935/live/STREAM_KEY
      */
}
