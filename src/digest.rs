use anyhow::Result;
use ring::digest::{Context, Digest, SHA256};
use std::io::Read;
use std::io::{Seek, SeekFrom};

pub fn sha_256_digest<R: Read + Seek>(reader: &mut R) -> Result<Digest> {
    let mut context = Context::new(&SHA256);
    let mut buffer = [0; 1024];

    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        context.update(&buffer[..count]);
    }
    reader.seek(SeekFrom::Start(0))?;
    Ok(context.finish())
}
