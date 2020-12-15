use anyhow::{Context, Result};
use my_dropbox_controller::extension::Extension;
use my_dropbox_controller::meta::{get_datetime, get_mp4_datetime};
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
            println!("pic");
            println!("{:?}", get_datetime(path.as_path()));
        }
        Extension::Mp4 => {
            println!("mov");
            println!("{:?}", get_mp4_datetime(path.as_path()));
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
