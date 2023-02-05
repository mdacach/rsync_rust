use std::path::PathBuf;

use bytes::Bytes;

use crate::io_utils;

/// A test consists of trying to recreate `updated_file` from `basis_file`.
/// It will use all steps of the rsync algorithm, and assert that we were successfully able to
/// recreate the file.
/// 1 - Compute signature from `basis_file`
/// 2 - Compute delta from signature and `updated_file`
/// 3 - Recreating `updated_file` from `basis_file` and `delta`
#[derive(Clone)]
pub struct TestCase {
    pub directory_path: PathBuf,
    // Directory containing the files. Useful if we want to persist intermediate
    // files such as `signature` and `delta`.
    pub basis_file: PathBuf,
    pub updated_file: PathBuf,
}

impl From<PathBuf> for TestCase {
    fn from(path: PathBuf) -> Self {
        // TODO: better error handling
        if !path.is_dir() {
            panic!(
                "{} is not a directory",
                path.to_str().expect("Could not convert path to str.")
            );
        }

        let basis_file = path.join("basis_file");
        let updated_file = path.join("updated_file");
        if basis_file.exists() && updated_file.exists() {
            TestCase {
                directory_path: path,
                basis_file,
                updated_file,
            }
        } else {
            panic!(
                "Test directory does not contain required files. \n\
                    It must contain two files:\n\
                    - basis_file: file which will be updated\n\
                    - updated_file: file which will be reconstructed"
            );
        }
    }
}
