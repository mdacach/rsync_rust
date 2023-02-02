use std::collections::hash_map::DefaultHasher;
use std::convert::TryInto;
use std::fs;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::{Read, Write};
use std::path::Path;

use bytes::{BufMut, Bytes, BytesMut};
use itertools::Itertools;
use rolling_hash_rust::RollingHash;
use serde::{Deserialize, Serialize};

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
    let mut file = File::create(path)?;
    file.write_all(&content)?;

    Ok(())
}

// TODO: should it be TryFrom instead?
// I am using From<Bytes> based on usage I have seen of FromStr, instead of TryFrom<str>
impl From<Bytes> for FileSignature {
    fn from(bytes: Bytes) -> Self {
        let mut strong_hashes = Vec::new();
        let mut rolling_hashes = Vec::new();
        let each_line = bytes.split(|&byte| byte == b'\n');

        each_line.tuples().for_each(|(strong_hash, rolling_hash)| {
            let strong_hash_as_array = strong_hash.try_into().unwrap();
            let strong_hash_as_hash_type = StrongHashType::from_be_bytes(strong_hash_as_array);

            let rolling_hash_as_array = rolling_hash.try_into().unwrap();
            let rolling_hash_as_hash_type = RollingHashType::from_be_bytes(rolling_hash_as_array);

            strong_hashes.push(strong_hash_as_hash_type);
            rolling_hashes.push(rolling_hash_as_hash_type);
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


pub fn handle_signature_command(file_bytes: Bytes, chunk_size: usize) -> Bytes {
    let signature = compute_signature(file_bytes, chunk_size);

    let strong_hashes = signature.strong_hashes;
    let rolling_hashes = signature.rolling_hashes;
    let content = BytesMut::new();
    let mut writer = content.writer();
    strong_hashes.iter().zip(rolling_hashes.iter()).for_each(|(s, r)| {
        let s = s.to_be_bytes();
        let r = r.to_be_bytes();
        writer.write_all(&s).unwrap();
        writer.write_all(b"\n").unwrap();
        writer.write_all(&r).unwrap();
        writer.write_all(b"\n").unwrap();
        // writer.write_all(&s).unwrap();
        // let formatted_string = format!("{s}\n{r}\n");

        // As we are writing into bytes::BufMut, this will not Err
        // writer.write_all(formatted_string.as_bytes()).unwrap();
    });

    // This way we convert from BytesMut into Bytes
    Bytes::from(writer.into_inner())
}

#[derive(Debug, Eq, PartialEq)]
#[derive(Serialize, Deserialize)]
pub struct Delta {
    content: Vec<Content>,
}

#[derive(Debug, Eq, PartialEq)]
#[derive(Serialize, Deserialize)]
enum Content {
    BlockIndex(usize),
    LiteralBytes(Vec<u8>),
}

// These tests are unnecessary, as serde itself is already well-tested
// I just wanted to see if it indeed just worked! Amazing.
#[test]
fn serde_json() {
    let content = Delta { content: vec!(Content::BlockIndex(0), Content::LiteralBytes(Vec::from("hello")), Content::BlockIndex(2)) };
    let json = serde_json::to_string(&content).expect("Something wrong with serde");

    let mut file = File::create("temp").unwrap();
    file.write_all(json.as_bytes()).unwrap();

    let mut file_to_decode = File::open("temp").unwrap();
    let mut contents = String::new();
    file_to_decode.read_to_string(&mut contents).unwrap();
    let decoded_content: Delta = serde_json::from_str(&contents).unwrap();

    assert_eq!(decoded_content, content);
}

#[test]
fn serde_rmp() {
    let content = Delta { content: vec!(Content::BlockIndex(0), Content::LiteralBytes(Vec::from("hello")), Content::BlockIndex(2)) };
    let encoded = rmp_serde::encode::to_vec(&content).unwrap();

    let mut file = File::create("temp2").unwrap();
    file.write_all(&encoded).unwrap();

    let mut file_to_decode = File::open("temp2").unwrap();
    let mut contents = vec![];
    file_to_decode.read_to_end(&mut contents).unwrap();
    let decoded_content: Delta = rmp_serde::decode::from_slice(&contents).unwrap();

    assert_eq!(decoded_content, content);
}

pub fn handle_delta_command(signature_file_bytes: Bytes, our_file_bytes: Bytes, chunk_size: usize) -> Delta
{
    let their_signature = FileSignature::from(signature_file_bytes);
    // we need to compare with our signature

    let bytes = Bytes::from_iter(our_file_bytes.clone().bytes().map(|x| x.unwrap()));

    let rolling_hashes = {
        let bytes = bytes.clone();
        let mut rolling_hashes = Vec::new();

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

        rolling_hashes
    };

    dbg!(&their_signature.rolling_hashes);

    let mut delta_content = Vec::new();
    // TODO: optimize this
    let block_iter = bytes.windows(chunk_size);

    let combined_iter = rolling_hashes.iter().zip(block_iter);
    let _: Vec<_> = combined_iter.batching(|current_iter| {
        if let Some((our_hash, block)) = dbg!(current_iter.next()) {
            let found_this_block_at = their_signature.rolling_hashes.iter().position(|x| x == our_hash);
            match found_this_block_at {
                Some(index) => {
                    delta_content.push(Content::BlockIndex(index));
                    // Skip the next window iterators, this block is already matched
                    // TODO: probably a better way
                    //       `advance_by` is experimental
                    for _ in 0..chunk_size - 2 {
                        current_iter.next();
                    }
                    current_iter.next()
                }
                None => {
                    delta_content.push(Content::LiteralBytes(block.into()));
                    current_iter.next()
                }
            }
        } else { None }
    }).collect();

    dbg!(&delta_content);
    Delta { content: delta_content }
}

#[test]
fn delta_for_equal_files_is_just_block_indexes() {
    let original_bytes = Bytes::from("Hello world");
    let signature = handle_signature_command(original_bytes, 3);
    let our_bytes = Bytes::from("Hello world");
    let delta = handle_delta_command(signature, our_bytes, 3);

    for c in delta.content {
        assert!(matches!(c, Content::BlockIndex(_)));
    }
}

#[test]
fn delta_for_different_files_has_byte_literals() {
    let original_bytes = Bytes::from("Hello world");
    let signature = handle_signature_command(original_bytes, 3);
    let our_bytes = Bytes::from("Hello world from somewhere else");
    let delta = handle_delta_command(signature, our_bytes, 3);

    let literal_bytes = delta.content.iter().filter(|x| matches!(x, Content::LiteralBytes(_)));
    assert!(literal_bytes.count() > 0);
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
