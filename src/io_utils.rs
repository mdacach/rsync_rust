use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;

use bytes::Bytes;
use color_eyre::eyre::Context;
use color_eyre::Help;

pub fn attempt_to_read_file<P: AsRef<Path>>(
    path: P,
) -> color_eyre::Result<Bytes, color_eyre::Report> {
    match fs::read(&path) {
        Ok(bytes) => Ok(bytes.into()),
        Err(error) => Err(color_eyre::Report::new(error))
            .context(format!(r#"Path provided: "{}""#, path.as_ref().display()))
            .suggestion("Are you sure the path provided is correct? Note that it should be a relative path."),
    }
}

pub fn write_to_file<P: AsRef<Path>>(path: P, content: Bytes) -> color_eyre::Result<()> {
    let mut file = File::create(path)?;
    file.write_all(&content)?;

    Ok(())
}
