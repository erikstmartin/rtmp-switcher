pub mod http;
pub mod input;
pub mod mixer;
pub mod output;

extern crate gstreamer as gst;
extern crate gstreamer_video as gst_video;
use crate::mixer::Error;
use serde::{Deserialize, Serialize};

type Result<T> = std::result::Result<T, Error>;

fn gst_create_element(element_type: &str, name: &str) -> Result<gst::Element> {
    Ok(gst::ElementFactory::make(element_type, Some(name))
        .map_err(|_| Error::Gstreamer(format!("Failed to create element: {}", name)))?)
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(default)]
pub struct VideoConfig {
    #[serde(default = "VideoConfig::framerate_default")]
    pub framerate: i32,
    #[serde(default = "VideoConfig::format_default")]
    pub format: Format,
    #[serde(default = "VideoConfig::width_default")]
    pub width: i32,
    #[serde(default = "VideoConfig::height_default")]
    pub height: i32,
    pub xpos: i32,
    pub ypos: i32,
    #[serde(default)]
    pub zorder: Option<u32>,
    #[serde(default = "VideoConfig::alpha_default")]
    pub alpha: f64,
    pub repeat: bool,
}

impl VideoConfig {
    fn framerate_default() -> i32 {
        30
    }

    fn format_default() -> Format {
        Format::I420
    }

    fn height_default() -> i32 {
        1080
    }

    fn width_default() -> i32 {
        1920
    }

    fn alpha_default() -> f64 {
        1.0
    }
}

impl Default for VideoConfig {
    fn default() -> Self {
        Self {
            framerate: Self::framerate_default(),
            width: Self::width_default(),
            height: Self::height_default(),
            zorder: None,
            xpos: 0,
            ypos: 0,
            alpha: Self::alpha_default(),
            repeat: false,
            format: Self::format_default(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct VideoEncoderConfig {
    #[serde(default = "VideoEncoderConfig::encoder_default")]
    pub encoder: VideoEncoder,
    pub profile: Option<VideoEncoderProfile>,
    pub speed: Option<VideoEncoderSpeed>,
    pub preset: Option<VideoEncoderPreset>,
}

impl VideoEncoderConfig {
    fn encoder_default() -> VideoEncoder {
        VideoEncoder::H264
    }
}

impl Default for VideoEncoderConfig {
    fn default() -> Self {
        Self {
            encoder: VideoEncoderConfig::encoder_default(),
            profile: Some(VideoEncoderProfile::High),
            preset: None,
            speed: Some(VideoEncoderSpeed::Medium),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AudioEncoderConfig {
    #[serde(default = "AudioEncoderConfig::encoder_default")]
    pub encoder: AudioEncoder,
}

impl AudioEncoderConfig {
    fn encoder_default() -> AudioEncoder {
        AudioEncoder::AAC
    }
}

impl Default for AudioEncoderConfig {
    fn default() -> Self {
        Self {
            encoder: AudioEncoderConfig::encoder_default(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(default)]
pub struct AudioConfig {
    #[serde(default = "AudioConfig::volume_default")]
    pub volume: f64,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            volume: Self::volume_default(),
        }
    }
}

impl AudioConfig {
    fn volume_default() -> f64 {
        1.0
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Clone)]
#[allow(non_camel_case_types)]
pub enum VideoEncoder {
    H264,
    NVENC,
    VP9,
}

impl std::fmt::Display for VideoEncoder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use VideoEncoder::*;

        let s = match self {
            H264 => "x264enc",
            NVENC => "nvh264enc",
            VP9 => "vp9enc",
        };

        f.write_str(s)
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Clone)]
#[allow(non_camel_case_types)]
pub enum VideoEncoderPreset {
    Default,
    HighPerformance,
    HighQuality,
    LowLatency,
    LowLatencyHighQuality,
}

impl std::fmt::Display for VideoEncoderPreset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use VideoEncoderPreset::*;

        let s = match self {
            Default => "default",
            HighPerformance => "hp",
            HighQuality => "hq",
            LowLatency => "low-latency",
            LowLatencyHighQuality => "low-latency-hq",
        };

        f.write_str(s)
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Clone)]
#[allow(non_camel_case_types)]
pub enum VideoEncoderProfile {
    High,
    Main,
    Baseline,
}

impl std::fmt::Display for VideoEncoderProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use VideoEncoderProfile::*;

        let s = match self {
            High => "high",
            Main => "main",
            Baseline => "baseline",
        };

        f.write_str(s)
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Clone)]
#[allow(non_camel_case_types)]
pub enum VideoEncoderSpeed {
    None,
    UltraFast,
    SuperFast,
    VeryFast,
    Faster,
    Fast,
    Medium,
    Slow,
    Slower,
    VerySlow,
    Placebo,
}

impl std::fmt::Display for VideoEncoderSpeed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use VideoEncoderSpeed::*;

        let s = match self {
            None => "none",
            UltraFast => "ultrafast",
            SuperFast => "superfast",
            VeryFast => "veryfast",
            Faster => "faster",
            Fast => "fast",
            Medium => "medium",
            Slow => "slow",
            Slower => "slower",
            VerySlow => "veryslow",
            Placebo => "placebo",
        };

        f.write_str(s)
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Clone)]
#[allow(non_camel_case_types)]
pub enum AudioEncoder {
    AAC,
    MP3,
    Vorbis,
}

impl std::fmt::Display for AudioEncoder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use AudioEncoder::*;

        let s = match self {
            AAC => "fdkaacenc",
            MP3 => "lamemp3enc",
            Vorbis => "vorbisenc",
        };

        f.write_str(s)
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Clone)]
#[allow(non_camel_case_types)]
pub enum Mux {
    FLV,
    MP4,
    MKV,
}

impl std::fmt::Display for Mux {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Mux::*;

        let s = match self {
            FLV => "flvmux",
            MP4 => "mp4mux",
            MKV => "matroskamux",
        };

        f.write_str(s)
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Clone)]
#[allow(non_camel_case_types)]
pub enum Format {
    AYUV64,
    ARGB64,
    GBRA_12LE,
    GBRA_12BE,
    Y412_LE,
    Y412_BE,
    A444_10LE,
    GBRA_10LE,
    A444_10BE,
    GBRA_10BE,
    A422_10LE,
    A422_10BE,
    A420_10LE,
    A420_10BE,
    RGB10A2_LE,
    BGR10A2_LE,
    Y410,
    GBRA,
    ABGR,
    VUYA,
    BGRA,
    AYUV,
    ARGB,
    RGBA,
    A420,
    Y444_16LE,
    Y444_16BE,
    v216,
    P016_LE,
    P016_BE,
    Y444_12LE,
    GBR_12LE,
    Y444_12BE,
    GBR_12BE,
    I422_12LE,
    I422_12BE,
    Y212_LE,
    Y212_BE,
    I420_12LE,
    I420_12BE,
    P012_LE,
    P012_BE,
    Y444_10LE,
    GBR_10LE,
    Y444_10BE,
    GBR_10BE,
    r210,
    I422_10LE,
    I422_10BE,
    NV16_10LE32,
    Y210,
    v210,
    UYVP,
    I420_10LE,
    I420_10BE,
    P010_10LE,
    NV12_10LE32,
    NV12_10LE40,
    P010_10BE,
    Y444,
    GBR,
    NV24,
    xBGR,
    BGRx,
    xRGB,
    RGBx,
    BGR,
    IYU2,
    v308,
    RGB,
    Y42B,
    NV61,
    NV16,
    VYUY,
    UYVY,
    YVYU,
    YUY2,
    I420,
    YV12,
    NV21,
    NV12,
    NV12_64Z32,
    NV12_4L4,
    NV12_32L32,
    Y41B,
    IYU1,
    YVU9,
    YUV9,
    RGB16,
    BGR16,
    RGB15,
    BGR15,
    RGB8P,
    GRAY16_LE,
    GRAY16_BE,
    GRAY10_LE32,
    GRAY8,
}

impl std::fmt::Display for Format {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Format::*;

        let s = match self {
            AYUV64 => "AYUV64",
            ARGB64 => "ARGB64",
            GBRA_12LE => "GBRA_12LE",
            GBRA_12BE => "GBRA_12BE",
            Y412_LE => "Y412_LE",
            Y412_BE => "Y412_BE",
            A444_10LE => "A444_10LE",
            GBRA_10LE => "GBRA_10LE",
            A444_10BE => "A444_10BE",
            GBRA_10BE => "GBRA_10BE",
            A422_10LE => "A422_10LE",
            A422_10BE => "A422_10BE",
            A420_10LE => "A420_10LE",
            A420_10BE => "A420_10BE",
            RGB10A2_LE => "RGB10A2_LE",
            BGR10A2_LE => "BGR10A2_LE",
            Y410 => "Y410",
            GBRA => "GBRA",
            ABGR => "ABGR",
            VUYA => "VUYA",
            BGRA => "BGRA",
            AYUV => "AYUV",
            ARGB => "ARGB",
            RGBA => "RGBA",
            A420 => "A420",
            Y444_16LE => "Y444_16LE",
            Y444_16BE => "Y444_16BE",
            v216 => "v216",
            P016_LE => "P016_LE",
            P016_BE => "P016_BE",
            Y444_12LE => "Y444_12LE",
            GBR_12LE => "GBR_12LE",
            Y444_12BE => "Y444_12BE",
            GBR_12BE => "GBR_12BE",
            I422_12LE => "I422_12LE",
            I422_12BE => "I422_12BE",
            Y212_LE => "Y212_LE",
            Y212_BE => "Y212_BE",
            I420_12LE => "I420_12LE",
            I420_12BE => "I420_12BE",
            P012_LE => "P012_LE",
            P012_BE => "P012_BE",
            Y444_10LE => "Y444_10LE",
            GBR_10LE => "GBR_10LE",
            Y444_10BE => "Y444_10BE",
            GBR_10BE => "GBR_10BE",
            r210 => "r210",
            I422_10LE => "I422_10LE",
            I422_10BE => "I422_10BE",
            NV16_10LE32 => "NV16_10LE32",
            Y210 => "Y210",
            v210 => "v210",
            UYVP => "UYVP",
            I420_10LE => "I420_10LE",
            I420_10BE => "I420_10BE",
            P010_10LE => "P010_10LE",
            NV12_10LE32 => "NV12_10LE32",
            NV12_10LE40 => "NV12_10LE40",
            P010_10BE => "P010_10BE",
            Y444 => "Y444",
            GBR => "GBR",
            NV24 => "NV24",
            xBGR => "xBGR",
            BGRx => "BGRx",
            xRGB => "xRGB",
            RGBx => "RGBx",
            BGR => "BGR",
            IYU2 => "IYU2",
            v308 => "v308",
            RGB => "RGB",
            Y42B => "Y42B",
            NV61 => "NV61",
            NV16 => "NV16",
            VYUY => "VYUY",
            UYVY => "UYVY",
            YVYU => "YVYU",
            YUY2 => "YUY2",
            I420 => "I420",
            YV12 => "YV12",
            NV21 => "NV21",
            NV12 => "NV12",
            NV12_64Z32 => "NV12_64Z32",
            NV12_4L4 => "NV12_4L4",
            NV12_32L32 => "NV12_32L32",
            Y41B => "Y41B",
            IYU1 => "IYU1",
            YVU9 => "YVU9",
            YUV9 => "YUV9",
            RGB16 => "RGB16",
            BGR16 => "BGR16",
            RGB15 => "RGB15",
            BGR15 => "BGR15",
            RGB8P => "RGB8P",
            GRAY16_LE => "GRAY16_LE",
            GRAY16_BE => "GRAY16_BE",
            GRAY10_LE32 => "GRAY10_LE32",
            GRAY8 => "GRAY8",
        };

        f.write_str(s)
    }
}
