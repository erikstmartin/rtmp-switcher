#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("a {0} with the name '{1}' already exists")]
    Exists(String, String),

    #[error("unknown error")]
    Unknown,

    #[error("a {0} with the name '{1}' was not found")]
    NotFound(String, String),

    #[error("An error was returned from gstreamer: '{0}'")]
    GstBool(#[from] gst::glib::BoolError),

    #[error("An error was returned from gstreamer: '{0}'")]
    GstStateChange(#[from] gst::StateChangeError),

    #[error("An error was returned from gstreamer: '{0}'")]
    Gstreamer(String),
}
