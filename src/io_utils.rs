use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;

use bytes::Bytes;

pub fn attempt_to_read_file<P: AsRef<Path>>(path: P) -> Result<Bytes, String> {
    match fs::read(&path) {
        Ok(bytes) => Ok(bytes.into()),
        Err(error) => Err(format!(
            "Unable to read file: {}\n\
                     Is the path provided correct?\n\
                     Caused by: {}",
            path.as_ref().display(),
            error
        )),
    }
}

pub fn write_to_file<P: AsRef<Path>>(path: P, content: Bytes) -> color_eyre::Result<()> {
    let mut file = File::create(path)?;
    file.write_all(&content)?;

    Ok(())
}
