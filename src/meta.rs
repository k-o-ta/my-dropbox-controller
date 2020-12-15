use anyhow::{Context, Result};
use chrono::DateTime as ChronoDateTime;
use chrono::{NaiveDateTime, TimeZone, Utc};
use chrono_tz::Asia::Tokyo;
use chrono_tz::Tz;
use exif::{DateTime, In, Reader, Tag, Value};
use mp4::creation_time;
use mp4::Result as Mp4Result;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
enum MetaError {
    #[error("PathDateTimeError: {0}")]
    ParseDateTimeError(String),
}

pub fn get_datetime(path: &Path) -> Result<ChronoDateTime<Tz>> {
    let file =
        File::open(path).with_context(|| format!("failed to open file: {:?}", path.to_str()))?;
    let exif = Reader::new().read_from_container(&mut BufReader::new(&file))?;
    let date_time = exif
        .get_field(Tag::DateTime, In::PRIMARY)
        .with_context(|| format!("date tiem doesn't exist"))?;
    let date_time_value: Result<exif::DateTime, anyhow::Error> = match &date_time.value {
        Value::Ascii(d) => Ok(DateTime::from_ascii(&d[0])?),
        _ => Err(MetaError::ParseDateTimeError(format!(
            "{:?}",
            &date_time.value
        )))?,
    };
    let str = &format!("{} +09:00", date_time_value?.to_string());
    let dt = ChronoDateTime::parse_from_str(&str, "%Y-%m-%d %H:%M:%S %z")?;
    Ok(dt.with_timezone(&Tokyo))
}

pub fn get_mp4_datetime(path: &Path) -> Result<ChronoDateTime<Tz>> {
    let file =
        File::open(path).with_context(|| format!("failed to open file: {:?}", path.to_str()))?;
    let size = file.metadata()?.len();
    let reader = BufReader::new(file);
    println!("{}", size);

    let mp4 = mp4::Mp4Reader::read_header(reader, size)?;
    println!(
        "  creation time:  {:?}",
        creation_time(mp4.moov.mvhd.modification_time) // mp4.moov.mvhd
    );
    // let mp4 = mp4::Mp4Reader::read_header(reader, 4)?;
    // Jan 1, 1970 UTC - Jan 1, 1904 UTC = 2082844800
    // const UTC_EPOCH_DIFF: u64 = 2082844800;
    let dt = Utc.timestamp(creation_time(mp4.moov.mvhd.modification_time) as i64, 0);
    let dt2 = dt.with_timezone(&Tokyo);

    // let meta = file.metadata()?;
    // println!("{:?}", meta);
    Ok(dt2)
}
