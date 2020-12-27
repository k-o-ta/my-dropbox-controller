use crate::extension::Extension;
use anyhow::{Context, Result};
use chrono::DateTime as ChronoDateTime;
use chrono::{NaiveDateTime, TimeZone, Utc};
use chrono_tz::Asia::Tokyo;
use chrono_tz::Tz;
use exif::{DateTime, In, Reader, Tag, Value};
use mp4::creation_time;
use mp4::Result as Mp4Result;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
enum MetaError {
    #[error("PathDateTimeError: {0}")]
    ParseDateTimeError(String),
}

pub fn datetime(mut buff: &mut BufReader<&File>, ext: &Extension) -> Result<ChronoDateTime<Tz>> {
    match ext {
        Extension::Jpeg => get_datetime(&mut buff),
        Extension::Mp4 => get_mp4_datetime(&mut buff),
        Other => Err(anyhow::anyhow!("no datetime for non image file")),
    }
    // reader: &mut BufReader<&File>
}
pub fn get_datetime(reader: &mut BufReader<&File>) -> Result<ChronoDateTime<Tz>> {
    let exif = Reader::new().read_from_container(reader)?;
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
    reader.seek(SeekFrom::Start(0))?;
    Ok(dt.with_timezone(&Tokyo))
}

pub fn get_mp4_datetime(reader: &mut BufReader<&File>) -> Result<ChronoDateTime<Tz>> {
    let file = reader.get_ref();
    let size = file.metadata()?.len();

    let mp4 = mp4::Mp4Reader::read_header(reader, size)?;
    // let mp4 = mp4::Mp4Reader::read_header(reader, 4)?;
    // Jan 1, 1970 UTC - Jan 1, 1904 UTC = 2082844800
    // const UTC_EPOCH_DIFF: u64 = 2082844800;
    // 仕様だと1904からの経過秒数だが、実際にはunix epoch timeが入っている
    let dt = Utc.timestamp(creation_time(mp4.moov.mvhd.modification_time) as i64, 0);
    let dt2 = dt.with_timezone(&Tokyo);

    // reader.seek(SeekFrom::Start(0))?;
    Ok(dt2)
}
