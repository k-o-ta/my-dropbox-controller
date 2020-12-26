use crate::{extension::Extension, meta::datetime};
use anyhow::Result;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub fn sort_calc(hashmap: &mut HashMap<String, Vec<HashMap<Extension, Vec<String>>>>) {
    for (date, exts) in hashmap {
        for ext in exts {
            for (ex, files) in ext {
                files.sort()
            }
        }
    }
}

pub fn calc(path: &Path) -> Result<HashMap<String, Vec<HashMap<Extension, Vec<String>>>>> {
    if !path.is_dir() {
        Err(anyhow::anyhow!("not directory"))?
    }

    let mut hashmap: HashMap<String, Vec<HashMap<Extension, Vec<String>>>> = HashMap::new();
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        match entry.path().is_dir() {
            true => hashmap.extend(calc(&entry.path())?),
            false => {
                let ext = Extension::from_path(&entry.path());
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
                let dtime = datetime(&entry.path())?.to_string();
                let filename = entry
                    .path()
                    .file_name()
                    .ok_or(anyhow::anyhow!("filename error1"))
                    .and_then(|n| n.to_str().ok_or(anyhow::anyhow!("filename error2")))?
                    .to_string();
                match hashmap.get_mut(&dtime) {
                    Some(exts) => {
                        for e in exts {
                            match e.get_mut(&ext) {
                                Some(filenames) => {
                                    filenames.push(filename);
                                    break;
                                }
                                None => {}
                            }
                        }
                    }
                    None => {
                        let mut map: HashMap<Extension, Vec<String>> = HashMap::new();
                        match ext {
                            Extension::Jpeg => {
                                map.insert(Extension::Jpeg, vec![filename]);
                            }
                            Extension::Mp4 => {
                                map.insert(Extension::Mp4, vec![filename]);
                            }
                            Extension::Other => {}
                        }
                        hashmap.insert(dtime, vec![map]);
                    }
                }
            }
        }
    }
    Ok(hashmap)
}
