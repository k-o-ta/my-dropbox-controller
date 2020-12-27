use crate::{digest::dpx_digest, extension::Extension, meta::datetime};
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

pub fn sort_calc(hashmap: &mut DatetimeExtnameDigests) {
    for (date, exts) in hashmap {
        for mut ext in &mut exts.ext_name_digests {
            for (ex, name_digests) in ext {
                name_digests.sort_by(|a, b| (a.0).partial_cmp(&b.0).unwrap())
            }
        }
    }
}

type NameDigest = (String, String);
type ExtNameDigests = HashMap<Extension, Vec<NameDigest>>;
#[derive(Debug)]
pub struct SumExtNameDigests {
    ext_name_digests: Vec<ExtNameDigests>,
    sum: u32,
}
impl SumExtNameDigests {
    fn new() -> Self {
        SumExtNameDigests {
            ext_name_digests: Vec::new(),
            sum: 0,
        }
    }
}
type DatetimeExtnameDigests = HashMap<String, SumExtNameDigests>;

// pub fn calc(path: &Path) -> Result<HashMap<String, Vec<HashMap<Extension, Vec<String>>>>> {
pub fn calc(path: &Path, hashmap: &mut DatetimeExtnameDigests) -> Result<()> {
    if !path.is_dir() {
        Err(anyhow::anyhow!("not directory"))?
    }

    // let mut hashmap: DatetimeExtnameDigests = HashMap::new();
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        match entry.path().is_dir() {
            true => {
                // hashmap.extend(calc(&entry.path())?);
                calc(&entry.path(), hashmap);
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
                    Some(sum_exts) => {
                        for e in &mut sum_exts.ext_name_digests {
                            match e.get_mut(&ext) {
                                Some(name_digests) => {
                                    name_digests.push(name_digest);
                                    sum_exts.sum = sum_exts.sum + 1;
                                    break;
                                }
                                None => {}
                            }
                        }
                    }
                    None => {
                        let mut map: ExtNameDigests = HashMap::new();
                        match ext {
                            Extension::Jpeg => {
                                map.insert(Extension::Jpeg, vec![name_digest]);
                            }
                            Extension::Mp4 => {
                                map.insert(Extension::Mp4, vec![name_digest]);
                            }
                            Extension::Other => {}
                        }
                        hashmap.insert(
                            dtime,
                            SumExtNameDigests {
                                ext_name_digests: vec![map],
                                sum: 1,
                            },
                        );
                    }
                }
            }
        }
    }
    Ok(())
}
