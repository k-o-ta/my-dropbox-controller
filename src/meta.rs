use anyhow::{Context, Result};
use exif::{DateTime, In, Reader, Tag, Value};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
enum MetaError {
    #[error("PathDateTimeError: {0}")]
    ParseDateTimeError(String),
}

pub fn get_datetime(path: &Path) -> Result<DateTime> {
    let file =
        File::open(path).with_context(|| format!("failed to open file: {:?}", path.to_str()))?;
    let exif = Reader::new().read_from_container(&mut BufReader::new(&file))?;
    let date_time = exif
        .get_field(Tag::DateTime, In::PRIMARY)
        .with_context(|| format!("date tiem doesn't exist"))?;
    match &date_time.value {
        Value::Ascii(d) => Ok(DateTime::from_ascii(&d[0])?),
        _ => Err(MetaError::ParseDateTimeError(format!(
            "{:?}",
            &date_time.value
        )))?,
    }
}
