use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;

use bytes::Bytes;

pub fn attempt_to_read_file<P: AsRef<Path>>(path: P) -> Bytes {
    match fs::read(&path) {
        Ok(bytes) => bytes.into(),
        Err(error) => {
            panic!(
                "Unable to read file: {}\n\
                          Are you sure the path provided is correct?\n\
                          Error: {}",
                path.as_ref().display(),
                error
            );
        }
    }
}

pub fn write_to_file<P: AsRef<Path>>(path: P, content: Bytes) -> color_eyre::Result<()> {
    let mut file = File::create(path)?;
    file.write_all(&content)?;

    Ok(())
}
