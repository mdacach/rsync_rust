//! Tests for correctness
//!
//! A test here means to be able to reconstruct an updated file using the algorithm.
//! A TestCase consists of a directory with:
//! 1 - `basis_file` - meaning the file we would like to update
//! 2 - `updated_file` - meaning the file we would like in the end
//! (`basis_file` would be the one User A has, and `updated_file` the one User B has)
//!
//! A test follows the steps a user would do manually:
//! 1 - Compute a signature for `basis_file`
//! 2 - Compute a delta for `updated_file`
//! 3 - Recreate the updated file
//!
//! A test is successful if the recreated file is exactly equal to `updated_file`
//!
//! TestCase directories are tests/integration_tests/test_files
//! Each TestCase has a unique identifier id for reference
use std::fs;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use nanoid::nanoid;
use rand::distr::{Alphanumeric, Distribution};

use rsync_rust::io_utils;
use rsync_rust::test_utils::*;

// This test runs all the saved TestCases and asserts the algorithm works for each of them.
// Running this test alone should be enough to verify the correctness of the algorithm
// (given that we have TestCases in the repository, which we do).
#[test]
#[ignore]
// I could not get this test running in GitHub actions
// I guess the problem is that it tries to use the already-built binary
// of the project, which GitHub does not have access?
// For now we are ignoring this in the CI and will keep testing manually
fn run_all_test_files() {
    create_release_mode_target();

    let test_cases =
        gather_test_cases_in_directory(&PathBuf::from("tests/integration_tests/test_files"));
    let total_tests = test_cases.len();
    for (counter, test_case) in test_cases.iter().enumerate() {
        println!("Current test case: {}", test_case.directory_path.display());

        assert_reconstruction_is_correct_for_test_case(test_case);

        println!("{}/{total_tests}", counter + 1);
    }
}

#[test]
#[ignore]
// Helper code to create more test_cases, not meant to be used for testing
fn create_5_test_cases() {
    let files_directory = Path::new("tests/integration_tests/test_files");
    for _ in 0..5 {
        let identifier = nanoid!(5);

        generate_test_case(&files_directory.join(Path::new(&identifier)), 1_000_000);
    }
}

fn generate_random_bytes(length: usize) -> Vec<u8> {
    let mut rng = rand::rng();

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

fn assert_files_have_equal_content(desired_file: &PathBuf, recreated_file: &PathBuf) {
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

fn assert_reconstruction_is_correct_for_test_case(test_case: &TestCase) {
    let TestCase {
        directory_path,
        basis_file,
        updated_file,
    } = test_case;

    let signature = directory_path.join("signature");
    let delta = directory_path.join("delta");

    let recreated_file = directory_path.join("recreated_file");

    run_signature_command(basis_file, &signature, 10);
    run_delta_command(&signature, updated_file, &delta, 10);
    run_patch_command(basis_file, &delta, &recreated_file, 10);

    assert_files_have_equal_content(updated_file, &recreated_file);
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
        .map(TestCase::try_from)
        .filter_map(|x| x.ok())
        .collect()
}
