use std::fs;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::process::Command;

use nanoid::nanoid;
use rand::distributions::Alphanumeric;
use rand::prelude::*;

use rsync_rust::io_utils;

#[test]
#[ignore]
fn inspect_size_of_generated_files() {
    let file1 = "tests/test_files/equal_files/file1";
    let file2 = "tests/test_files/equal_files/file2";

    let (signature_size, delta_size) = compute_size_of_generated_files(file1, file2);
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

fn compute_size_of_generated_files(file1: &str, file2: &str) -> (u64, u64) {
    let file1_signature = format!("{file1}.signature");
    let file2_delta = format!("{file2}.delta");

    run_signature_command(file1, &file1_signature);
    run_delta_command(&file1_signature, file2, &file2_delta);

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

/// Asserts that supplying `file1` and `file2` to the algorithm behaves correctly.
///
/// The objective here is to transform file1 into file2 using signatures and deltas.
/// In the end, the `recreated_file` must be exactly equal `file2`, otherwise we have
/// lost information in the process.
fn assert_reconstruction_is_correct_for_given_files(file1: &str, file2: &str) {
    let file1_signature = format!("{file1}.signature");
    let file2_delta = format!("{file2}.delta");

    let directory = Path::new(file1).parent();
    let recreated_file = match directory {
        Some(dir) => format!("{}/recreated_file", dir.to_str().expect("Not UTF-8 path")),
        None => "recreated_file".to_string(),
    };

    run_signature_command(file1, &file1_signature, 10);
    run_delta_command(&file1_signature, file2, &file2_delta, 10);
    run_patch_command(file1, &file2_delta, &recreated_file, 10);

    assert_files_have_equal_content(file2, &recreated_file);
}

fn generate_pair_of_random_files_for_testing(directory: &str, length: usize) -> String {
    let file1 = generate_random_bytes_with_linebreaks(length);
    let file2 = generate_random_bytes_with_linebreaks(length);

    let identifier = nanoid!(5);

    let directory_path = format!("{directory}/{identifier}");
    fs::create_dir(&directory_path).expect("Could not create directory for random generated files");

    let file1_path = format!("{directory_path}/file1");
    let file2_path = format!("{directory_path}/file2");

    io_utils::write_to_file(file1_path, file1.into()).expect("Could not write to file");
    io_utils::write_to_file(file2_path, file2.into()).expect("Could not write to file");

    directory_path
}

#[test]
#[ignore]
/// Generates a pair of small random files as input to rsync and validates the algorithm.
fn test_pair_of_random_files() {
    let test_directory = "tests/test_files/random/small";
    let identifier_directory = generate_pair_of_random_files_for_testing(test_directory, 100);

    assert_reconstruction_is_correct_for_given_files(
        &format!("{identifier_directory}/file1"),
        &format!("{identifier_directory}/file2"),
    );
}

#[test]
#[ignore]
/// Generates multiple pairs of small random files as input to rsync and validates the algorithm
/// for each pair.
fn test_multiple_pairs_of_random_files() {
    let test_directory = "tests/test_files/random/small";
    for _test_id in 0..15 {
        let identifier_directory = generate_pair_of_random_files_for_testing(test_directory, 100);

        assert_reconstruction_is_correct_for_given_files(
            &format!("{identifier_directory}/file1"),
            &format!("{identifier_directory}/file2"),
        );
    }
}

#[test]
#[ignore]
/// Generates a pair of big random files as input to rsync and validates the algorithm.
fn test_pair_of_big_random_files() {
    let test_directory = "tests/test_files/random/big";
    let identifier_directory = generate_pair_of_random_files_for_testing(test_directory, 100_000);

    assert_reconstruction_is_correct_for_given_files(
        &format!("{identifier_directory}/file1"),
        &format!("{identifier_directory}/file2"),
    );
}

#[test]
#[ignore]
// If all is well, this will take a `really` long time to find a failure.
// I have ran it for more than 15000 iterations with no failure
fn test_until_failure() {
    let mut counter = 0;
    println!("Successful test counter:");
    loop {
        let test_directory = "tests/test_files/random/big";
        let identifier_directory =
            generate_pair_of_random_files_for_testing(test_directory, 100_000);

        assert_reconstruction_is_correct_for_given_files(
            &format!("{identifier_directory}/file1"),
            &format!("{identifier_directory}/file2"),
        );

        // Delete the files we have just created, otherwise we will use a lot of memory.
        fs::remove_dir_all(identifier_directory).unwrap();

        counter += 1;
        if counter % 50 == 0 {
            println!("- {counter}")
        }
    }
}

// Test directories can contain
// 1 - One set of files for testing (specifically file1 and file2)
// 2 - Nested directories
// But not both. This way we can differentiate a directory which should be tested (because it has files)
// or that just needs to be traversed.
fn test_files_inside_directory(directory_path: &str) {
    let entries: Vec<_> = fs::read_dir(directory_path)
        .expect("Could not read directory path")
        .collect();
    let nested_directories = entries
        .iter()
        .filter(|p| p.as_ref().unwrap().path().is_dir());

    let has_only_nested_directories = nested_directories.count() > 0;
    if has_only_nested_directories {
        // We need to recursively test the files within nested directories
        for dir in entries {
            test_files_inside_directory(dir.unwrap().path().to_str().unwrap());
        }
    } else {
        // This is already just a test directory
        assert_reconstruction_is_correct_for_given_files(
            &format!("{directory_path}/file1"),
            &format!("{directory_path}/file2"),
        );
    }
}

#[test]
#[ignore]
// I could not get this test running in GitHub actions
// I guess the problem is that it tries to use the already-build binary
// of the project, which GitHub does not have access?
// For now we are ignoring this in the CI and will keep testing manually
fn run_all_test_files() {
    // TODO: improve this code
    test_files_inside_directory("tests/test_files");
}
