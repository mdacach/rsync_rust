use std::collections::hash_map::DefaultHasher;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::{Read, Write};

use bytes::Bytes;
use rolling_hash_rust::RollingHash;

type RollingHashType = u64;
type StrongHashType = u64;

#[derive(Debug, PartialEq, Eq)]
struct FileSignature {
    strong_hashes: Vec<StrongHashType>,
    rolling_hashes: Vec<RollingHashType>,
}

// Use the default hash is std for now
fn calculate_strong_hash(content: &[u8]) -> u64 {
    let mut s = DefaultHasher::new();
    content.hash(&mut s);
    s.finish()
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

        let mut output_file = match File::create(&output_filename) {
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
