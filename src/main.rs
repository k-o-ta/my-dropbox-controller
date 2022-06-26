use anyhow::{Context, Result};
use data_encoding::HEXUPPER;
use my_dropbox_controller::calc::{calc, calc_starter, runner, sort_calc, sum_calc};
use my_dropbox_controller::digest::{dpx_digest, sha_256_digest};
use my_dropbox_controller::dropbox::{
    get_file_metadata, list_directory, upload_file, upload_files,
};
use my_dropbox_controller::extension::Extension;
use my_dropbox_controller::meta::{get_datetime, get_mp4_datetime};
use my_dropbox_controller::sqlite::reset_db as sqlite_reset_db;
use std::collections::HashMap;
use std::fmt;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::Path;
use std::str::FromStr;
use structopt::StructOpt;
use thiserror::Error;
use tokio::sync::mpsc::{channel, Receiver, Sender};

#[derive(StructOpt)]
struct Cli {
    #[structopt(subcommand)]
    sub: Sub,
}

#[derive(StructOpt)]
enum Sub {
    #[structopt(name = "reset-db", about = "reset db")]
    ResetDb { path: String },
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

async fn reset_db(path: String) -> Result<()> {
    println!("resetDB");
    println!("{:?}", sqlite_reset_db("my-dropbox3.db3", &path).await);
    let mut source_file = File::open("my-dropbox3.db3")?;
    upload_file(source_file, "/my-dropbox2.db3".to_string())?;
    Ok(())
}

async fn upload(path: &Path) -> Result<()> {
    println!("upload");
    // let mut init = calc_starter(&path).await?;
    let mut init = runner(&path).await?;
    sort_calc(&mut init);
    // println!("{:?}", init);
    println!("sum: {}", sum_calc(&init));
    // println!("{:?}", upload_files(init).await?);
    upload_files(init).await?;
    println!("ok?");

    Ok(())
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
fn sp(tx: Sender<i32>) {
    let tx2 = tx.clone();
    tokio::spawn(async move {
        tx.send(1).await;
        println!("spawn 1");
        tokio::spawn(async move {
            println!("spawn 2");
            tx2.send(2).await;
        });
    });
}
async fn sp2() {
    println!("sp2-0");
    let ans = tokio::spawn(async { println!("sp2-1") });
    println!("sp2-2");
    tokio::join!(ans);
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::from_args();
    match args.sub {
        Sub::ResetDb { path } => {
            reset_db(path).await;
        }
        Sub::Upload { path } => {
            upload(&path).await;
        }
        Sub::Meta { path } => {
            get_metadata(&path);
        }
        Sub::Test { path } => {
            sp2().await;
            println!("test");
            let (mut tx, mut rx): (Sender<i32>, Receiver<i32>) = channel(32);
            sp(tx);
            while let Some(message) = rx.recv().await {
                println!("received: {}", message);
                if message == 2 {
                    break;
                }
            }
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
