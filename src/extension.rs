use std::fmt;
use std::str::FromStr;

pub enum Extension {
    Jpeg,
    Mp4,
}

#[derive(Debug)]
enum ExtensionError {
    UnknownExtensionError(String),
}

impl fmt::Display for ExtensionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::ExtensionError::*;
        match self {
            UnknownExtensionError(s) => write!(f, "UnknownExtensionError: {}", s),
        }
    }
}

impl std::error::Error for ExtensionError {}

impl FromStr for Extension {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "jpeg" | "JPEG" | "jpg" | "JPG" => Ok(Extension::Jpeg),
            "mp4" | "MP4" => Ok(Extension::Mp4),
            extension => Err(ExtensionError::UnknownExtensionError(extension.to_string()))?,
        }
    }
}
