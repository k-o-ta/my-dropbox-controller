use anyhow::{Context, Result};
use std::fmt;
use std::str::FromStr;
use structopt::StructOpt;
use thiserror::Error;

#[derive(StructOpt)]
struct Cli {
    pattern: String,
    #[structopt(parse(from_os_str))]
    path: std::path::PathBuf,
}

enum Extension {
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

#[derive(Debug, Error)]
enum MyError {
    #[error("InvalidPathError: {0}")]
    InvalidPathError(String),
    #[error("InvalidExtensionString: {0}")]
    InvalidExtensionString(String),
}

fn main() -> Result<()> {
    let args = Cli::from_args();
    let path = args.path;
    let ext = Extension::from_str(
        path.extension()
            .ok_or(MyError::InvalidPathError("invalid path".to_string()))
            .and_then(|ext| {
                ext.to_str()
                    .ok_or(MyError::InvalidExtensionString(format!("{:?}", path)))
            })?,
    )?;
    match ext {
        Extension::Jpeg => {
            println!("pic")
        }
        Extension::Mp4 => {
            println!("mov")
        }
    }
    // let e = Extension::from_str(ext)?;
    // match Extension::from_str(ext)? {
    //     Extension::Jpeg => {
    //         println!("ok")
    //     }
    //     // Err(e) => {
    //     //     println!("ng")
    //     // }
    // }

    // let content = std::fs::read_to_string(&path).with_context(|| {
    //     format!(
    //         "failed to open file: {}",
    //         &path.to_str().unwrap_or("invalid path")
    //     )
    // })?;
    // let content = std::fs::read_to_string(&args.path).with_context(|| {
    //     format!(
    //         "failed to open file: {}",
    //         &args.path.to_str().unwrap_or("invalid path")
    //     )
    // })?;
    // for line in content.lines() {
    //     if line.contains(&args.pattern) {
    //         println!("{}", line);
    //     }
    // }
    Ok(())
}
