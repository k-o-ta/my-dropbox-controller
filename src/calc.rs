use crate::{digest::dpx_digest, extension::Extension, meta::datetime};
use anyhow::{Context, Result};
use async_recursion::async_recursion;
use chrono::{Date, DateTime, Local, Utc};
use chrono::{Duration, NaiveDateTime};
use futures::future::join_all;
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::ops::Add;
use std::path::Path;
use tokio::sync::mpsc::{channel, Receiver, Sender};

pub fn sort_calc(hashmap: &mut DatetimeExtnameDigests) {
    for (date, exts) in hashmap {
        exts.pic
            .sort_by(|a, b| (a.name).partial_cmp(&b.name).unwrap());
        exts.mov
            .sort_by(|a, b| (a.name).partial_cmp(&b.name).unwrap());
    }
}
pub fn sum_calc(hashmap: &DatetimeExtnameDigests) -> u32 {
    hashmap.iter().fold(0, |acc, (date, exts)| acc + exts.sum)
}

// type NameDigest = (String, String);
#[derive(Debug)]
pub struct NameDigest {
    pub digest: String,
    pub name: String,
    pub path: String,
}
type ExtNameDigests = HashMap<Extension, Vec<NameDigest>>;
#[derive(Debug, Default)]
pub struct SumNameDigests {
    pub pic: Vec<NameDigest>,
    pub mov: Vec<NameDigest>,
    pub sum: u32,
}
impl SumNameDigests {
    fn merge(&mut self, mut other: Self) {
        self.pic.append(&mut other.pic);
        self.mov.append(&mut other.mov);
        self.sum = self.sum + other.sum;
    }
}

impl Add<SumNameDigests> for SumNameDigests {
    type Output = SumNameDigests;
    fn add(self, mut other: Self) -> Self {
        let mut pic = self.pic;
        pic.append(&mut other.pic);
        let mut mov = self.mov;
        mov.append(&mut other.mov);
        SumNameDigests {
            pic,
            mov,
            sum: self.sum + other.sum,
        }
    }
}

#[derive(Debug)]
pub enum CalcMessage {
    Finish(i32),
    File(String),
}
pub async fn runner(path: &Path) -> Result<DatetimeExtnameDigests> {
    println!("calc start: {:?}", path);
    if !path.is_dir() {
        Err(anyhow::anyhow!("not directory"))?
    }
    let (mut tx, mut rx) = channel(32);
    // println!("accm1: path:{:?}", path);
    let con = tokio::spawn(async move { controller(rx).await });
    accm(path, tx.clone(), false).await;

    con.await?
    // con.
    // tokio::join!(con).0
}

async fn controller(mut rx: Receiver<CalcMessage>) -> Result<(DatetimeExtnameDigests)> {
    let max = 100;
    let mut total = None;
    let mut this_total = 0;
    let mut v = Vec::with_capacity(max);
    let mut ret = Vec::new();
    println!("controlle");
    while let Some(message) = rx.recv().await {
        // println!("receive");
        match message {
            CalcMessage::Finish(t) => {
                total = Some(t);
                // break;
            }
            CalcMessage::File(path) => {
                this_total = this_total + 1;
                v.push(path);
            }
        }
        if v.len() >= max {
            println!("controlle over");
            let v2 = v.clone();
            ret.push(tokio::spawn(async move { calc2(v2) }));
            v.clear();
        }
        match total {
            Some(t) => {
                if t == this_total {
                    let v2 = v.clone();
                    ret.push(tokio::spawn(async move { calc2(v2) }));
                    break;
                }
            }
            None => {}
        }
    }
    let results = futures::future::join_all(ret).await;
    let mut hashmap: DatetimeExtnameDigests = HashMap::new();
    for dir in results {
        let result = dir?;
        for _result in result {
            for (datetime, sum_name_digest_result) in _result {
                match hashmap.get_mut(&datetime) {
                    Some(sum_name_digest) => sum_name_digest.merge(sum_name_digest_result),
                    None => {
                        hashmap.insert(datetime, sum_name_digest_result);
                    }
                }
            }
        }
    }
    Ok(hashmap)
}

#[async_recursion]
async fn accm(path: &Path, mut tx: Sender<CalcMessage>, rec: bool) -> Result<(i32)> {
    println!("accm: path{:?}", path);
    let mut sum = 0;
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let entry_path = entry.path();
        match entry_path.is_dir() {
            true => {
                sum = sum + accm(&entry_path, tx.clone(), true).await?;
            }
            false => {
                match Extension::from_path(&entry_path) {
                    Ok(ext) => match ext {
                        Extension::Jpeg | Extension::Mp4 => ext,
                        Extension::Other => continue,
                    },
                    Err(_) => {
                        continue;
                    }
                };
                // println!("send: {:?}", entry_path);
                tx.send(CalcMessage::File(entry_path.display().to_string()))
                    .await;
                sum = sum + 1;
                // .await;
            }
        }
    }
    if !rec {
        tx.send(CalcMessage::Finish(sum)).await;
    }
    // println!("accm: path{:?}", path);
    Ok(sum)
}

pub fn calc2(paths: Vec<String>) -> Result<DatetimeExtnameDigests> {
    let start_time: DateTime<Local> = Local::now();
    println!("thread start: {}", start_time);
    let mut hashmap: DatetimeExtnameDigests = HashMap::new();
    for path in paths {
        let path = Path::new(&path);
        let ext = match Extension::from_path(&path) {
            Ok(ext) => match ext {
                Extension::Jpeg | Extension::Mp4 => ext,
                Extension::Other => continue,
            },
            Err(_) => {
                continue;
            }
        };
        let mut file = File::open(&path)
            .with_context(|| format!("failed to open file: {:?}", path.to_str()))?;
        let mut buff = BufReader::new(&file);
        let dtime = datetime(&mut buff, &ext)?
            .format("%Y-%m-%d %H:%M:%S")
            .to_string();
        let digest = dpx_digest(&mut buff)?;
        let path_string = path.display().to_string();
        let filename = path
            .file_name()
            .ok_or(anyhow::anyhow!("filename error1"))
            .and_then(|n| n.to_str().ok_or(anyhow::anyhow!("filename error2")))?
            .to_string();

        // let name_digest: NameDigest = (filename, digest);
        let name_digest: NameDigest = NameDigest {
            digest: digest,
            path: path_string,
            name: filename,
        };
        match hashmap.get_mut(&dtime) {
            Some(sum_exts) => match ext {
                Extension::Jpeg => {
                    sum_exts.pic.push(name_digest);
                    sum_exts.sum = sum_exts.sum + 1;
                }
                Extension::Mp4 => {
                    sum_exts.mov.push(name_digest);
                    sum_exts.sum = sum_exts.sum + 1;
                }
                Extension::Other => {}
            },
            None => {
                let mut map: ExtNameDigests = HashMap::new();
                let sum_name_digests = match ext {
                    Extension::Jpeg => SumNameDigests {
                        pic: vec![name_digest],
                        mov: Vec::new(),
                        sum: 1,
                    },
                    Extension::Mp4 => SumNameDigests {
                        mov: vec![name_digest],
                        pic: Vec::new(),
                        sum: 1,
                    },
                    Extension::Other => SumNameDigests::default(),
                };
                hashmap.insert(dtime, sum_name_digests);
            }
        }
    }
    let end_time: DateTime<Local> = Local::now();
    println!(
        "thread finish: {}, duration: {}",
        start_time,
        (end_time - start_time).num_seconds()
    );
    Ok(hashmap)
}

pub type DatetimeExtnameDigests = HashMap<String, SumNameDigests>;
pub async fn calc_starter(path: &Path) -> Result<DatetimeExtnameDigests> {
    println!("calc start: {:?}", path);
    if !path.is_dir() {
        Err(anyhow::anyhow!("not directory"))?
    }
    let mut hashmap: DatetimeExtnameDigests = HashMap::new();
    let mut dirs = Vec::new();
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        match entry.path().is_dir() {
            true => {
                dirs.push(tokio::spawn(async move { calc(&entry.path()) }));
            }
            false => {
                let path = &entry.path();
                let ext = Extension::from_path(&path);
                let ext = match ext {
                    Ok(ex) => match ex {
                        Extension::Jpeg => Extension::Jpeg,
                        Extension::Mp4 => Extension::Mp4,
                        Extension::Other => {
                            continue;
                        }
                    },
                    Err(_) => {
                        continue;
                    }
                };
                let mut file = File::open(&path)
                    .with_context(|| format!("failed to open file: {:?}", path.to_str()))?;
                let mut buff = BufReader::new(&file);
                let dtime = datetime(&mut buff, &ext)?.to_string();
                let digest = dpx_digest(&mut buff)?;
                let path_string = path.display().to_string();
                let filename = path
                    .file_name()
                    .ok_or(anyhow::anyhow!("filename error1"))
                    .and_then(|n| n.to_str().ok_or(anyhow::anyhow!("filename error2")))?
                    .to_string();
                // let name_digest: NameDigest = (filename, digest);
                let name_digest: NameDigest = NameDigest {
                    name: filename,
                    path: path_string,
                    digest: digest,
                };
                match hashmap.get_mut(&dtime) {
                    Some(sum_exts) => match ext {
                        Extension::Jpeg => {
                            sum_exts.pic.push(name_digest);
                            sum_exts.sum = sum_exts.sum + 1;
                        }
                        Extension::Mp4 => {
                            sum_exts.mov.push(name_digest);
                            sum_exts.sum = sum_exts.sum + 1;
                        }
                        Extension::Other => {}
                    },
                    None => {
                        let mut map: ExtNameDigests = HashMap::new();
                        let sum_name_digests = match ext {
                            Extension::Jpeg => SumNameDigests {
                                pic: vec![name_digest],
                                mov: Vec::new(),
                                sum: 1,
                            },
                            Extension::Mp4 => SumNameDigests {
                                mov: vec![name_digest],
                                pic: Vec::new(),
                                sum: 1,
                            },
                            Extension::Other => SumNameDigests::default(),
                        };
                        hashmap.insert(dtime, sum_name_digests);
                    }
                }
            }
        }
    }
    let results = futures::future::join_all(dirs).await;
    for dir in results {
        let result = dir?;
        for _result in result {
            for (datetime, sum_name_digest_result) in _result {
                match hashmap.get_mut(&datetime) {
                    Some(sum_name_digest) => sum_name_digest.merge(sum_name_digest_result),
                    None => {
                        hashmap.insert(datetime, sum_name_digest_result);
                    }
                }
            }
        }
    }
    // let result = calc(&entry.path())?;
    // for (datetime, sum_name_digest_result) in result {
    //     match hashmap.get_mut(&datetime) {
    //         Some(sum_name_digest) => sum_name_digest.merge(sum_name_digest_result),
    //         None => {}
    //     }
    // }
    println!("calc end: {:?}", path);
    Ok(hashmap)
}
// pub fn calc(path: &Path) -> Result<HashMap<String, Vec<HashMap<Extension, Vec<String>>>>> {
pub fn calc(path: &Path) -> Result<DatetimeExtnameDigests> {
    println!("thread start: {:?}", path);
    if !path.is_dir() {
        Err(anyhow::anyhow!("not directory"))?
    }
    let mut hashmap: DatetimeExtnameDigests = HashMap::new();

    // let mut hashmap: DatetimeExtnameDigests = HashMap::new();
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        match entry.path().is_dir() {
            true => {
                let result = calc(&entry.path())?;
                for (datetime, sum_name_digest_result) in result {
                    match hashmap.get_mut(&datetime) {
                        Some(sum_name_digest) => sum_name_digest.merge(sum_name_digest_result),
                        None => {}
                    }
                }
            }
            false => {
                let path = &entry.path();
                let ext = Extension::from_path(&path);
                let ext = match ext {
                    Ok(ex) => match ex {
                        Extension::Jpeg => Extension::Jpeg,
                        Extension::Mp4 => Extension::Mp4,
                        Extension::Other => {
                            continue;
                        }
                    },
                    Err(_) => {
                        continue;
                    }
                };
                let mut file = File::open(&path)
                    .with_context(|| format!("failed to open file: {:?}", path.to_str()))?;
                let mut buff = BufReader::new(&file);
                let dtime = datetime(&mut buff, &ext)?.to_string();
                let digest = dpx_digest(&mut buff)?;
                let path_string = path.display().to_string();
                let filename = path
                    .file_name()
                    .ok_or(anyhow::anyhow!("filename error1"))
                    .and_then(|n| n.to_str().ok_or(anyhow::anyhow!("filename error2")))?
                    .to_string();
                // let name_digest: NameDigest = (filename, digest);
                let name_digest: NameDigest = NameDigest {
                    name: filename,
                    path: path_string,
                    digest: digest,
                };
                match hashmap.get_mut(&dtime) {
                    Some(sum_exts) => match ext {
                        Extension::Jpeg => {
                            sum_exts.pic.push(name_digest);
                            sum_exts.sum = sum_exts.sum + 1;
                        }
                        Extension::Mp4 => {
                            sum_exts.mov.push(name_digest);
                            sum_exts.sum = sum_exts.sum + 1;
                        }
                        Extension::Other => {}
                    },
                    None => {
                        let mut map: ExtNameDigests = HashMap::new();
                        let sum_name_digests = match ext {
                            Extension::Jpeg => SumNameDigests {
                                pic: vec![name_digest],
                                mov: Vec::new(),
                                sum: 1,
                            },
                            Extension::Mp4 => SumNameDigests {
                                mov: vec![name_digest],
                                pic: Vec::new(),
                                sum: 1,
                            },
                            Extension::Other => SumNameDigests::default(),
                        };
                        hashmap.insert(dtime, sum_name_digests);
                    }
                }
            }
        }
    }
    println!("thread end: {:?}", path);
    Ok(hashmap)
}
