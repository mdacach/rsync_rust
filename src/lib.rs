use std::collections::hash_map::DefaultHasher;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::{Read, Write};

use bytes::Bytes;

// Use the default hash is std for now
fn calculate_hash(content: &[u8]) -> u64 {
    let mut s = DefaultHasher::new();
    content.hash(&mut s);
    s.finish()
}

fn compute_signature(content: Bytes, chunk_size: usize) -> Vec<u64> {
    let blocks = content.chunks(chunk_size);
    blocks.map(calculate_hash).collect()
}

pub fn handle_signature_command(filename: String, output_filename: String) {
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

        for s in signature {
            let s = s.to_string();
            output_file.write_all(s.as_bytes()).unwrap_or_else(|_| panic!("Could not write to file: {output_filename}"));
            output_file.write_all(b"\n").unwrap_or_else(|_| panic!("Could not write to file: {output_filename}"));
        }
    };
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
