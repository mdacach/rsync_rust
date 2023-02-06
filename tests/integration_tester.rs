use std::fs;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

use nanoid::nanoid;
use rand::distributions::Alphanumeric;
use rand::prelude::*;

use rsync_rust::io_utils;
use rsync_rust::test_utils::TestCase;

fn generate_random_bytes(length: usize) -> Vec<u8> {
    let mut rng = thread_rng();

    Alphanumeric.sample_iter(&mut rng).take(length).collect()
}

// Adding linebreaks helps reading the generated files
fn generate_random_bytes_with_linebreaks(length: usize) -> Vec<u8> {
    let chunk_size = 64;
    let number_of_chunks = length / chunk_size;
    let last_chunk_size = length % chunk_size;

    let mut result = Vec::new();
    for _ in 0..number_of_chunks {
        result.extend(generate_random_bytes(chunk_size));
        result.push(b'\n');
    }
    result.extend(generate_random_bytes(last_chunk_size));

    result
}

fn assert_files_have_equal_content(desired_file: &str, recreated_file: &str) {
    let mut file1_contents = Vec::new();
    let _ = File::open(desired_file)
        .unwrap()
        .read_to_end(&mut file1_contents);

    let mut file2_contents = Vec::new();
    let _ = File::open(recreated_file)
        .unwrap()
        .read_to_end(&mut file2_contents);

    assert_eq!(file1_contents, file2_contents);
}

fn run_signature_command(filename: &str, output_filename: &str, chunk_size: usize) {
    Command::new("target/release/rsync_rust")
        .arg("signature")
        .arg(filename)
        .arg(output_filename)
        .args(["-c", &chunk_size.to_string()])
        .spawn()
        .expect("failed to spawn child process")
        .wait()
        .expect("failed to wait on child");
}

fn run_delta_command(
    signature_filename: &str,
    our_filename: &str,
    delta_filename: &str,
    chunk_size: usize,
) {
    Command::new("target/release/rsync_rust")
        .arg("delta")
        .arg(signature_filename)
        .arg(our_filename)
        .arg(delta_filename)
        .args(["-c", &chunk_size.to_string()])
        .spawn()
        .expect("failed to spawn child process")
        .wait()
        .expect("failed to wait on child");
}

fn run_patch_command(
    basis_filename: &str,
    delta_filename: &str,
    recreated_filename: &str,
    chunk_size: usize,
) {
    Command::new("target/release/rsync_rust")
        .arg("patch")
        .arg(basis_filename)
        .arg(delta_filename)
        .arg(recreated_filename)
        .args(["-c", &chunk_size.to_string()])
        .spawn()
        .expect("failed to spawn child process")
        .wait()
        .expect("failed to wait on child");
}

fn assert_reconstruction_is_correct_for_test_case(test_case: &TestCase) {
    let TestCase {
        directory_path,
        basis_file,
        updated_file,
    } = test_case;

    let signature = format!("{}/signature", directory_path.display());
    let delta = format!("{}/delta", directory_path.display());

    let recreated_file = format!("{}/recreated_file", directory_path.display());

    // TODO: Make stuff accept Path instead of &str
    run_signature_command(basis_file.to_str().unwrap(), &signature, 10);
    run_delta_command(&signature, updated_file.to_str().unwrap(), &delta, 10);
    run_patch_command(basis_file.to_str().unwrap(), &delta, &recreated_file, 10);

    assert_files_have_equal_content(updated_file.to_str().unwrap(), &recreated_file);
}

#[test]
#[ignore]
fn create_5_test_cases() {
    let files_directory = Path::new("tests/integration_tests/test_files");
    for _ in 0..5 {
        let identifier = nanoid!(5);

        generate_test_case(&files_directory.join(Path::new(&identifier)), 1_000_000);
    }
}

fn generate_test_case(directory: &PathBuf, length: usize) -> TestCase {
    let basis_file = generate_random_bytes_with_linebreaks(length);
    let updated_file = generate_random_bytes_with_linebreaks(length);

    fs::create_dir(directory).expect("Could not create directory");

    let basis_file_path = directory.join("basis_file");
    let updated_file_path = directory.join("updated_file");

    io_utils::write_to_file(basis_file_path.clone(), basis_file.into())
        .expect("Could not write to file");
    io_utils::write_to_file(updated_file_path.clone(), updated_file.into())
        .expect("Could not write to file");

    TestCase {
        directory_path: directory.into(),
        basis_file: basis_file_path,
        updated_file: updated_file_path,
    }
}

fn gather_test_cases_in_directory(directory_path: &PathBuf) -> Vec<TestCase> {
    fs::read_dir(directory_path)
        .expect("Could not read directory path")
        .filter_map(|x| x.ok())
        .map(|x| x.path())
        .map(TestCase::try_from) // TODO: try here
        .filter_map(|x| x.ok())
        .collect()
}

#[test]
#[ignore]
// I could not get this test running in GitHub actions
// I guess the problem is that it tries to use the already-built binary
// of the project, which GitHub does not have access?
// For now we are ignoring this in the CI and will keep testing manually
fn run_all_test_files() {
    let test_cases =
        gather_test_cases_in_directory(&PathBuf::from("tests/integration_tests/test_files"));
    let total_tests = test_cases.len();
    for (counter, test_case) in test_cases.iter().enumerate() {
        println!("Current test case: {}", test_case.directory_path.display());

        assert_reconstruction_is_correct_for_test_case(test_case);

        println!("{}/{total_tests}", counter + 1);
    }
}

// TODO: this will be moved
//       this file is supposed to test for correctness
//       compression will be tested somewhere else
mod temp_test_compression {
    use super::*;

    #[test]
    #[ignore]
    fn test_linux_kernel_source_code_similarity() {
        // 84080 files
        let linux_files_old = gather_files(Path::new("tests/linux_kernel_source_code/linux-6.1.8"));

        let all_old = concat_vecs(&linux_files_old);
        println!("old total size: {}", all_old.len());

        // 84078 files
        let linux_files_new = gather_files(Path::new("tests/linux_kernel_source_code/linux-6.1.9"));
        let all_new = concat_vecs(&linux_files_new);
        println!("new total size: {}", all_new.len());

        io_utils::write_to_file("file1.tmp", all_old.into())
            .expect("Could not write linux to single file");
        io_utils::write_to_file("file2.tmp", all_new.into())
            .expect("Could not write linux to single file");

        inspect_size_of_generated_files("file1.tmp", "file2.tmp", 100);
    }

    fn concat_vecs(vecs: &[Vec<u8>]) -> Vec<u8> {
        let total_len = vecs.iter().map(|v| v.len()).sum();
        let mut result = Vec::with_capacity(total_len);
        for v in vecs {
            result.extend_from_slice(v);
        }
        result
    }

    fn gather_files(path: &Path) -> Vec<Vec<u8>> {
        let mut files = Vec::new();

        if path.is_dir() {
            for entry in fs::read_dir(path).unwrap() {
                let entry = entry.unwrap();
                let entry_path = entry.path();
                if entry_path.is_dir() {
                    files.extend(gather_files(&entry_path));
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

    #[test]
    fn test_compressed_size() {
        let file1 = "tests/integration_tests/test_files/equal_files/big/file1";
        let file2 = "tests/integration_tests/test_files/equal_files/big/file2";

        inspect_size_of_generated_files(file1, file2, 10000);
    }

    fn inspect_size_of_generated_files(file1: &str, file2: &str, chunk_size: usize) {
        let (signature_size, delta_size) =
            compute_size_of_generated_files(file1, file2, chunk_size);
        let size_using_algorithm = signature_size + delta_size;

        let original_file_size = {
            let original_file_metadata =
                fs::metadata(file2).expect("Could not read metadata of file2");
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
        fn compute_size_of_generated_files(
            file1: &str,
            file2: &str,
            chunk_size: usize,
        ) -> (u64, u64) {
            let file1_signature = format!("{file1}.signature");
            let file2_delta = format!("{file2}.delta");

            run_signature_command(file1, &file1_signature, chunk_size);
            run_delta_command(&file1_signature, file2, &file2_delta, chunk_size);

            // Now we have created the files: `file1.signature` and `file2.delta`
            // In order for the algorithm to be efficient, we need that the combined size of those
            // two files to be smaller than the size of `file2`
            // (otherwise we would be better off sending the file directly)

            let signature_file = format!("{file1}.signature");
            let delta_file = format!("{file2}.delta");
            let signature_metadata = fs::metadata(&signature_file)
                .unwrap_or_else(|_| panic!("Could not read metadata of {}", &signature_file));
            let delta_metadata = fs::metadata(&delta_file)
                .unwrap_or_else(|_| panic!("Could not read metadata of {}", &delta_file));

            (signature_metadata.len(), delta_metadata.len())
        }
    }
}
