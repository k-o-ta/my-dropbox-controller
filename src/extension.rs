use anyhow::{Context, Result};
use std::fmt;
use std::path::Path;
use std::str::FromStr;
#[derive(PartialEq, Hash, Eq, Debug)]
pub enum Extension {
    Jpeg,
    Mp4,
    Other,
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

// impl FromStr for Extension {
//     type Err = anyhow::Error;
//     fn from_str(s: &str) -> Result<Self, Self::Err> {
//         match s {
//             "jpeg" | "JPEG" | "jpg" | "JPG" => Ok(Extension::Jpeg),
//             "mp4" | "MP4" => Ok(Extension::Mp4),
//             extension => Err(ExtensionError::UnknownExtensionError(extension.to_string()))?,
//         }
//     }
// }

impl FromStr for Extension {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "jpeg" | "JPEG" | "jpg" | "JPG" => Ok(Extension::Jpeg),
            "mp4" | "MP4" => Ok(Extension::Mp4),
            _ => Ok(Extension::Other),
        }
    }
}

impl Extension {
    pub fn from_path(path: &Path) -> Result<Self> {
        let ex = path
            .extension()
            .ok_or(anyhow::anyhow!(format!("no extension: {:?}", path)))
            .and_then(|ext| ext.to_str().ok_or(anyhow::anyhow!("")))?;
        Extension::from_str(ex)
    }
}
