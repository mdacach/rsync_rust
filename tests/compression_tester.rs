use std::fs;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use rsync_rust::io_utils;
use rsync_rust::test_utils::*;

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

    io_utils::write_to_file(
        "tests/linux_kernel_source_code/as_single_files/basis_file",
        all_old.into(),
    )
    .expect("Could not write linux to single file");
    io_utils::write_to_file(
        "tests/linux_kernel_source_code/as_single_files/updated_file",
        all_new.into(),
    )
    .expect("Could not write linux to single file");

    // inspect_size_of_generated_files("file1.tmp", "file2.tmp", 100);
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

fn test_compressed_size(test_case: TestCase, chunk_size: usize) {
    let basis_file = test_case.basis_file;
    let updated_file = test_case.updated_file;

    inspect_size_of_generated_files(&basis_file, &updated_file, chunk_size);
}

fn inspect_size_of_generated_files(file1: &PathBuf, file2: &PathBuf, chunk_size: usize) {
    let (signature_size, delta_size) = compute_size_of_generated_files(file1, file2, chunk_size);
    let size_using_algorithm = signature_size + delta_size;

    let original_file_size = {
        let original_file_metadata = fs::metadata(file2).expect("Could not read metadata of file2");
        original_file_metadata.len()
    };

    println!("File1 -> File2");
    println!("**************************************");
    println!("[file2 size]: {original_file_size}");
    println!("[signature size]: {signature_size}");
    println!("[delta size]: {delta_size}");
    println!("**************************************");
    println!("Sending the file directly [file2 size]: {original_file_size} bytes");
    println!("Using the algorithm [signature + delta size]: {size_using_algorithm} bytes");
    println!("**************************************");
}

fn compute_size_of_generated_files(
    basis_file: &PathBuf,
    updated_file: &PathBuf,
    chunk_size: usize,
) -> (u64, u64) {
    let signature = basis_file.join("signature");
    let delta = updated_file.join("delta");

    run_signature_command(basis_file, &signature, chunk_size);
    run_delta_command(&signature, updated_file, &delta, chunk_size);

    // Now we have created the files: `signature` and `delta`
    // In order for the algorithm to be efficient, we need that the combined size of those
    // two files to be smaller than the size of `updated_file`
    // (otherwise we would be better off sending the file directly)

    let signature_metadata = fs::metadata(&signature)
        .unwrap_or_else(|_| panic!("Could not read metadata of {}", signature.display()));
    let delta_metadata = fs::metadata(&delta)
        .unwrap_or_else(|_| panic!("Could not read metadata of {}", delta.display()));

    (signature_metadata.len(), delta_metadata.len())
}
