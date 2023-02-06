use std::fmt::Formatter;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::{fmt, fs};

use rsync_rust::io_utils;
use rsync_rust::test_utils::*;

struct CompressionData {
    test_case: TestCase,
    chunk_size_used: usize,

    basis_file_size: u64,
    updated_file_size: u64,
    signature_file_size: u64,
    delta_file_size: u64,
}

impl fmt::Display for CompressionData {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let size_using_rsync = self.signature_file_size + self.delta_file_size;
        let compression_rate = size_using_rsync as f64 / self.updated_file_size as f64;
        write!(
            f,
            "Compression Data for TestCase {}\
            and chunk size {} bytes\n\
            basis_file size: {}\n\
            updated_file size: {}\n\
            signature_file size: {}\n\
            delta_file size: {}\n\
            total size using rsync: {}\n\
            efficiency: {:.3}",
            self.test_case.directory_path.display(),
            self.chunk_size_used,
            self.basis_file_size,
            self.updated_file_size,
            self.signature_file_size,
            self.delta_file_size,
            size_using_rsync,
            compression_rate
        )
    }
}

fn compute_compression_data(test_case: &TestCase, chunk_size: usize) -> CompressionData {
    let (signature_file_size, delta_file_size) =
        compute_size_of_generated_files(test_case, chunk_size);

    let basis_file_size = test_case.basis_file.metadata().unwrap().len();
    let updated_file_size = test_case.updated_file.metadata().unwrap().len();

    CompressionData {
        test_case: test_case.clone(),
        chunk_size_used: chunk_size,
        basis_file_size,
        updated_file_size,
        signature_file_size,
        delta_file_size,
    }
}

// This creates three new files in the same directory as `test_case`
// 1 - signature - the signature from `basis_file`
// 2 - delta - the delta from `signature` and `updated_file`
// and returns the file paths for both, in order
fn generate_intermediate_files(test_case: &TestCase, chunk_size: usize) -> (PathBuf, PathBuf) {
    let basis_file = &test_case.basis_file;
    let updated_file = &test_case.updated_file;
    let current_directory = &test_case.directory_path;

    let signature = current_directory.join("signature");
    let delta = current_directory.join("delta");

    run_signature_command(basis_file, &signature, chunk_size);
    run_delta_command(&signature, updated_file, &delta, chunk_size);

    (signature, delta)
}

// Linux chunk size efficiency
// 100 -> 0.40365
// 500 -> 0.10546
// 2500 -> 0.05376
// 10000 -> 0.07616

#[test]
#[ignore]
fn test_linux_compression() {
    let test_case = TestCase::try_from(PathBuf::from(
        "tests/linux_kernel_source_code/as_single_files",
    ))
    .unwrap();
    inspect_size_of_generated_files(&test_case, 500);
}

#[test]
#[ignore]
// Hacky workaround for testing the whole directory but only currently supporting one file
// Let's just mesh all the files together and reconstruct that instead
fn merge_linux_directories_in_single_file() {
    // 84080 files
    let linux_files_old =
        gather_files_in_directory(Path::new("tests/linux_kernel_source_code/linux-6.1.8"));
    let all_old = concatenate_2d_bytes(&linux_files_old);

    // 84078 files
    let linux_files_new =
        gather_files_in_directory(Path::new("tests/linux_kernel_source_code/linux-6.1.9"));
    let all_new = concatenate_2d_bytes(&linux_files_new);

    let basis_file = Path::new("tests/linux_kernel_source_code/as_single_files/basis_file");
    let updated_file = Path::new("tests/linux_kernel_source_code/as_single_files/updated_file");
    io_utils::write_to_file(basis_file, all_old.into())
        .expect("Could not write linux to single file");
    io_utils::write_to_file(updated_file, all_new.into())
        .expect("Could not write linux to single file");
}

fn concatenate_2d_bytes(vecs: &[Vec<u8>]) -> Vec<u8> {
    let total_len = vecs.iter().map(|v| v.len()).sum();
    let mut result = Vec::with_capacity(total_len);
    for v in vecs {
        result.extend_from_slice(v);
    }
    result
}

fn gather_files_in_directory(path: &Path) -> Vec<Vec<u8>> {
    let mut files = Vec::new();

    if path.is_dir() {
        for entry in fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            let entry_path = entry.path();
            if entry_path.is_dir() {
                files.extend(gather_files_in_directory(&entry_path));
            } else {
                let mut file_contents = Vec::new();
                let mut file = File::open(entry_path).unwrap();
                file.read_to_end(&mut file_contents).unwrap();
                files.push(file_contents);
            }
        }
    }

    files
}

fn inspect_size_of_generated_files(test_case: &TestCase, chunk_size: usize) {
    let compression_data = compute_compression_data(test_case, chunk_size);

    println!("{compression_data}");
}

fn compute_size_of_generated_files(test_case: &TestCase, chunk_size: usize) -> (u64, u64) {
    let (signature_path, delta_path) = generate_intermediate_files(test_case, chunk_size);

    // Now we have created the files: `signature` and `delta`
    // In order for the algorithm to be efficient, we need that the combined size of those
    // two files to be smaller than the size of `updated_file`
    // (otherwise we would be better off sending the file directly)

    let signature_metadata = fs::metadata(&signature_path)
        .unwrap_or_else(|_| panic!("Could not read metadata of {}", signature_path.display()));
    let delta_metadata = fs::metadata(&delta_path)
        .unwrap_or_else(|_| panic!("Could not read metadata of {}", delta_path.display()));

    (signature_metadata.len(), delta_metadata.len())
}
