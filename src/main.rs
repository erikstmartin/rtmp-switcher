mod input;
mod mixer;
mod output;

use gstreamer as gst;
use input::*;
use mixer::*;
use output::*;

fn main() {
    // Initialize GStreamer
    gst::init().unwrap();

    let rtmp_uri = "rtmp://learntv-transcoder.eastus.azurecontainer.io:1935/live/STREAM_KEY";

    let uri =
        "https://www.freedesktop.org/software/gstreamer-sdk/data/media/sintel_trailer-480p.webm";

    let mut mixer = Mixer::new("test").unwrap();

    mixer
        .add_input(Input::from_uri("sintel", uri).expect("Failed to build Input from uri"))
        .expect("Failed to add input");

    /*
    mixer
        .add_output(Output::autosink("auto2").expect("Failed to build Output from uri"))
        .expect("Failed to add output");

    */
    mixer
        .add_output(Output::autosink("auto").expect("Failed to build Output"))
        .expect("Failed to add output");

    //mixer.remove_output("rtmp");

    mixer.play().expect("Error setting pipeline state to play");
}
