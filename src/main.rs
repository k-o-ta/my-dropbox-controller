use anyhow::{Context, Result};
use data_encoding::HEXUPPER;
use my_dropbox_controller::digest::sha_256_digest;
use my_dropbox_controller::extension::Extension;
use my_dropbox_controller::meta::{get_datetime, get_mp4_datetime};
use std::fmt;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
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
    let mut file =
        File::open(&path).with_context(|| format!("failed to open file: {:?}", path.to_str()))?;
    let mut buff = BufReader::new(&file);
    let digest = sha_256_digest(&mut buff);
    // sha_256_digest2(&file);
    match ext {
        Extension::Jpeg => {
            println!("pic");
            println!("{:?}", get_datetime(buff));
        }
        Extension::Mp4 => {
            println!("mov");
            println!("{:?}", get_mp4_datetime(&mut file));
            file.seek(SeekFrom::Start(0))?;
        }
    }
    println!("digest: {:?}", HEXUPPER.encode(digest.unwrap().as_ref()));
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
