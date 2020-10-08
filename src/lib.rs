pub mod http;
pub mod input;
pub mod mixer;
pub mod output;

extern crate gstreamer as gst;
extern crate gstreamer_video as gst_video;
use crate::mixer::Error;
use gst::prelude::*;

type Result<T> = std::result::Result<T, Error>;

fn gst_create_element(element_type: &str, name: &str) -> Result<gst::Element> {
    Ok(gst::ElementFactory::make(element_type, Some(name))
        .map_err(|e| Error::Gstreamer(format!("Failed to create element: {}", name)))?)
}
