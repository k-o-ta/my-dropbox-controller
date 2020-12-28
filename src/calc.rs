use crate::{digest::dpx_digest, extension::Extension, meta::datetime};
use anyhow::{Context, Result};
use futures::future::join_all;
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::ops::Add;
use std::path::Path;

pub fn sort_calc(hashmap: &mut DatetimeExtnameDigests) {
    for (date, exts) in hashmap {
        exts.pic.sort_by(|a, b| (a.0).partial_cmp(&b.0).unwrap());
        exts.mov.sort_by(|a, b| (a.0).partial_cmp(&b.0).unwrap());
    }
}

type NameDigest = (String, String);
type ExtNameDigests = HashMap<Extension, Vec<NameDigest>>;
#[derive(Debug, Default)]
pub struct SumNameDigests {
    pic: Vec<NameDigest>,
    mov: Vec<NameDigest>,
    sum: u32,
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
type DatetimeExtnameDigests = HashMap<String, SumNameDigests>;
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
                let filename = path
                    .file_name()
                    .ok_or(anyhow::anyhow!("filename error1"))
                    .and_then(|n| n.to_str().ok_or(anyhow::anyhow!("filename error2")))?
                    .to_string();
                let name_digest: NameDigest = (filename, digest);
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
                let filename = path
                    .file_name()
                    .ok_or(anyhow::anyhow!("filename error1"))
                    .and_then(|n| n.to_str().ok_or(anyhow::anyhow!("filename error2")))?
                    .to_string();
                let name_digest: NameDigest = (filename, digest);
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
