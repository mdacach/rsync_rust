use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;

use bytes::Bytes;

pub fn read_file<P: AsRef<Path>>(path: P) -> color_eyre::Result<Bytes> {
    let contents = fs::read(path)?;

    Ok(Bytes::from(contents))
}

pub fn write_to_file<P: AsRef<Path>>(path: P, content: Bytes) -> color_eyre::Result<()> {
    let mut file = File::create(path)?;
    file.write_all(&content)?;

    Ok(())
}
