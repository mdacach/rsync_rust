use std::path::PathBuf;

#[derive(Debug)]
struct NotDirectoryError;

#[derive(Debug)]
struct NoInputFilesError;

pub enum TestCaseConversionErrorType {
    NotDirectoryError,
    NoInputFilesError,
}

pub struct TestCaseConversionError {
    pub error_type: TestCaseConversionErrorType,
    pub path: PathBuf,
}

/// A test consists of trying to recreate `updated_file` from `basis_file`.
/// It will use all steps of the rsync algorithm, and assert that we were successfully able to
/// recreate the file.
/// 1 - Compute signature from `basis_file`
/// 2 - Compute delta from signature and `updated_file`
/// 3 - Recreating `updated_file` from `basis_file` and `delta`
#[derive(Debug, Clone)]
pub struct TestCase {
    pub directory_path: PathBuf,
    // Directory containing the files. Useful if we want to persist intermediate
    // files such as `signature` and `delta`.
    pub basis_file: PathBuf,
    pub updated_file: PathBuf,
}

impl TryFrom<PathBuf> for TestCase {
    type Error = TestCaseConversionError;

    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        if !path.is_dir() {
            return Err(TestCaseConversionError {
                error_type: TestCaseConversionErrorType::NotDirectoryError,
                path,
            });
        }

        let basis_file = path.join("basis_file");
        let updated_file = path.join("updated_file");
        if basis_file.exists() && updated_file.exists() {
            Ok(TestCase {
                directory_path: path,
                basis_file,
                updated_file,
            })
        } else {
            Err(TestCaseConversionError {
                error_type: TestCaseConversionErrorType::NoInputFilesError,
                path,
            })
        }
    }
}
