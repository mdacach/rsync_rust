use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::{Read, Write};
use std::path::Path;

use bytes::Bytes;
use itertools::Itertools;
use rolling_hash_rust::RollingHash;

type StrongHashType = u64;
type RollingHashType = u64;

#[derive(Debug, PartialEq, Eq)]
struct FileSignature {
    strong_hashes: Vec<StrongHashType>,
    rolling_hashes: Vec<RollingHashType>,
}

// TODO: should it be TryFrom instead?
// I am using From<File> based on usage I have seen of FromStr, instead of TryFrom<str>
impl From<File> for FileSignature {
    // TODO: better error handling
    fn from(mut file: File) -> Self {
        let mut file_contents = String::new();
        file.read_to_string(&mut file_contents).expect("Could not open file");

        let mut strong_hashes = Vec::new();
        let mut rolling_hashes = Vec::new();
        file_contents.lines().tuples().for_each(|(strong_hash, rolling_hash)| {
            strong_hashes.push(strong_hash.parse::<StrongHashType>().unwrap());
            rolling_hashes.push(rolling_hash.parse::<RollingHashType>().unwrap());
        }
        );

        assert_eq!(strong_hashes.len(), rolling_hashes.len());
        FileSignature { strong_hashes, rolling_hashes }
    }
}

// TODO: rethink all of these tests
#[test]
fn we_can_decode_signature_from_file() {
    let data = "To lack feeling is to be dead, but to act on every feeling is to be a child. - Dalinar Kholin";
    fs::write(".input_temporary", data).expect("Could not create file");
    let input_signature = compute_signature(Bytes::from(data), 10);
    handle_signature_command(".input_temporary", ".output_temporary");
    let output_file = File::open(".output_temporary").expect("Could not open file");
    // yeah we probably want a Try here
    let output_signature = FileSignature::from(output_file);

    assert_eq!(input_signature, output_signature);
}


fn compute_signature(content: Bytes, chunk_size: usize) -> FileSignature {
    let blocks = content.chunks(chunk_size);
    let strong_hashes = blocks.map(calculate_strong_hash).collect();

    let mut rolling_hashes = Vec::new();
    let blocks = content.chunks(chunk_size);
    blocks.for_each(|block| {
        // TODO: change rolling hash to accept bytes
        // TODO: make this code better
        let hasher = RollingHash::from_initial_string(&String::from_utf8(Vec::from(block)).unwrap());
        let hash = hasher.get_current_hash();
        rolling_hashes.push(hash);
    }
    );

    FileSignature { strong_hashes, rolling_hashes }
}

// Use the default hash is std for now
fn calculate_strong_hash(content: &[u8]) -> u64 {
    let mut s = DefaultHasher::new();
    content.hash(&mut s);

    s.finish()
}


pub fn handle_signature_command(filename: &str, output_filename: &str) {
    let mut file = match File::open(filename) {
        Ok(file) => file,
        Err(error) => {
            println!("Failed to open file: {error}");
            return;
        }
    };

    let mut file_contents = Vec::new();
    if file.read_to_end(&mut file_contents).is_ok() {
        let file_bytes = Bytes::from(file_contents);
        let signature = compute_signature(file_bytes, 10);

        let mut output_file = match File::create(output_filename) {
            Ok(file) => file,
            Err(error) => {
                println!("Failed to create file: {output_filename},  {error}");
                return;
            }
        };

        let strong_hashes = signature.strong_hashes;
        let rolling_hashes = signature.rolling_hashes;
        strong_hashes.iter().zip(rolling_hashes.iter()).for_each(|(s, r)| {
            let s = s.clone().to_string();
            let r = r.clone().to_string();
            output_file.write_all(s.as_bytes()).unwrap_or_else(|_| panic!("Could not write to file: {output_filename}"));
            output_file.write_all(b"\n").unwrap_or_else(|_| panic!("Could not write to file: {output_filename}"));
            output_file.write_all(r.as_bytes()).unwrap_or_else(|_| panic!("Could not write to file: {output_filename}"));
            output_file.write_all(b"\n").unwrap_or_else(|_| panic!("Could not write to file: {output_filename}"));
        })
    }
}


#[test]
fn equal_contents_have_equal_signatures() {
    let left = Bytes::from("ABCDEFGH");
    let right = Bytes::from("ABCDEFGH");
    let chunk_size = 4;
    let left_signature = compute_signature(left, chunk_size);
    let right_signature = compute_signature(right, chunk_size);
    assert_eq!(left_signature, right_signature);
}

#[test]
fn different_contents_have_different_signatures() {
    let left = Bytes::from("ABCDEFGH");
    let right = Bytes::from("AB");
    let chunk_size = 4;
    let left_signature = compute_signature(left, chunk_size);
    let right_signature = compute_signature(right, chunk_size);
    assert_ne!(left_signature, right_signature);
}
