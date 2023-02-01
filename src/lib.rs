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

pub fn read_file<P: AsRef<Path>>(path: P) -> color_eyre::Result<Bytes> {
    let contents = fs::read(path)?;

    Ok(Bytes::from(contents))
}

pub fn write_to_file<P: AsRef<Path>>(path: P, content: Bytes) -> color_eyre::Result<()> {
    let mut file = File::open(path)?;
    file.write_all(&content)?;

    Ok(())
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


pub fn handle_signature_command(file_bytes: Bytes, output_filename: &str) {
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

pub fn handle_delta_command(signature_filename: &str, desired_filename: &str, delta_filename: &str)
{
    let signature_file = File::open(signature_filename).expect("Could not open file");
    let their_signature = FileSignature::from(signature_file);
    // we need to compare with our signature
    let desired_file = File::open(desired_filename).expect("Could not open file");

    // we need to know the chunk size too
    let chunk_size = 10;

    let mut rolling_hashes = Vec::new();
    let bytes = Bytes::from_iter(desired_file.bytes().map(|x| x.unwrap()));
    let mut windows_iter = bytes.windows(chunk_size);
    let first_string = String::from_utf8_lossy(windows_iter.next().unwrap());
    let mut hasher = RollingHash::from_initial_string(&first_string);
    rolling_hashes.push(hasher.get_current_hash());

    // we do not need windows here, just iterate one-by-one after the initial one
    windows_iter.for_each(|window| {
        hasher.pop_front();
        hasher.push_back(dbg!(*window.last().unwrap() as char));
        rolling_hashes.push(hasher.get_current_hash());
    });

    dbg!(rolling_hashes);

    // but our signature is actually a multi-step process
    // we need to compute a rolling hash for each byte
    // and only compute a strong hash if needed
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
