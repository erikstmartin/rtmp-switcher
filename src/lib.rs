pub mod http;
pub mod input;
pub mod mixer;
pub mod output;

extern crate gstreamer as gst;
extern crate gstreamer_video as gst_video;

type Result<T> = std::result::Result<T, crate::mixer::Error>;
