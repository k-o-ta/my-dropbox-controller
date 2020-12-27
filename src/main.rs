use anyhow::{Context, Result};
use data_encoding::HEXUPPER;
use my_dropbox_controller::calc::{calc, sort_calc};
use my_dropbox_controller::digest::{dpx_digest, sha_256_digest};
use my_dropbox_controller::dropbox::{get_file_metadata, list_directory, upload_file};
use my_dropbox_controller::extension::Extension;
use my_dropbox_controller::meta::{get_datetime, get_mp4_datetime};
use my_dropbox_controller::sqlite::reset_db as sqlite_reset_db;
use std::fmt;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::Path;
use std::str::FromStr;
use structopt::StructOpt;
use thiserror::Error;

#[derive(StructOpt)]
struct Cli {
    #[structopt(subcommand)]
    sub: Sub,
}

#[derive(StructOpt)]
enum Sub {
    #[structopt(name = "reset-db", about = "reset db")]
    ResetDb,
    #[structopt(name = "upload", about = "upload pictures")]
    Upload {
        #[structopt(parse(from_os_str))]
        path: std::path::PathBuf,
    },
    #[structopt(name = "meta", about = "get metadata of file")]
    Meta {
        #[structopt(parse(from_os_str))]
        path: std::path::PathBuf,
    },
    #[structopt(name = "test", about = "test")]
    Test {
        #[structopt(parse(from_os_str))]
        path: std::path::PathBuf,
    },
}

#[derive(Debug, Error)]
enum MyError {
    #[error("InvalidPathError: {0}")]
    InvalidPathError(String),
    #[error("InvalidExtensionString: {0}")]
    InvalidExtensionString(String),
}

async fn reset_db() -> Result<()> {
    println!("resetDB");
    println!("{:?}", sqlite_reset_db("my-dropbox.db3").await);
    let mut source_file = File::open("my-dropbox.db3")?;
    upload_file(source_file, "/my-dropbox2.db3".to_string())?;
    Ok(())
}

fn upload(path: &Path) {
    println!("upload");
}
fn get_metadata(path: &Path) -> Result<()> {
    println!("meta");
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
            println!("{:?}", get_datetime(&mut buff));
            // get_file_metadata(&format!(
            //     "/ファミリー ルーム/写真/{}",
            //     path.to_str().unwrap()
            // ));
        }
        Extension::Mp4 => {
            println!("mov");
            println!("{:?}", get_mp4_datetime(&mut buff));
            buff.seek(SeekFrom::Start(0))?;
            // get_file_metadata(&format!(
            //     "/ファミリー ルーム/動画/{}",
            //     path.to_str().unwrap()
            // ));
        }
        _ => {}
    }
    println!("digest: {:?}", HEXUPPER.encode(digest.unwrap().as_ref()));
    println!("dpx_digest: {:?}", dpx_digest(&mut buff));
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::from_args();
    match args.sub {
        Sub::ResetDb => {
            reset_db();
        }
        Sub::Upload { path } => {
            upload(&path);
        }
        Sub::Meta { path } => {
            get_metadata(&path);
        }
        Sub::Test { path } => {
            println!("test");
            let mut cal = calc(Path::new("./"))?;
            sort_calc(&mut cal);
            println!("{:?}", cal);
        }
    };
    // list_directory("/");
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
