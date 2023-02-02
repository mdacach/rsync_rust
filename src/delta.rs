use std::io::Read;

use bytes::Bytes;
use itertools::Itertools;
use rolling_hash_rust::RollingHash;
use serde::{Deserialize, Serialize};

use crate::signature::FileSignature;

#[derive(Debug, Eq, PartialEq)]
#[derive(Serialize, Deserialize)]
pub struct Delta {
    pub(crate) content: Vec<Content>,
}

#[derive(Debug, Eq, PartialEq)]
#[derive(Serialize, Deserialize)]
pub enum Content {
    BlockIndex(usize),
    LiteralBytes(Vec<u8>),
}

impl From<Delta> for Bytes {
    fn from(value: Delta) -> Self {
        serde_json::to_vec_pretty(&value).expect("Could not serialize Delta into JSON").into()
    }
}

impl From<Bytes> for Delta {
    fn from(value: Bytes) -> Delta {
        serde_json::from_slice(&value).expect("Could not deserialize Delta from JSON")
    }
}

pub fn handle_delta_command(signature_file_bytes: Bytes, our_file_bytes: Bytes, chunk_size: usize) -> Delta
{
    let their_signature = FileSignature::from(signature_file_bytes);
    // we need to compare with our signature

    let bytes = Bytes::from_iter(our_file_bytes.bytes().map(|x| x.unwrap()));

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
            hasher.push_back(*window.last().unwrap() as char);
            rolling_hashes.push(hasher.get_current_hash());
        });

        rolling_hashes
    };


    let mut delta_content = Vec::new();
    // TODO: optimize this
    let block_iter = bytes.windows(chunk_size);

    let combined_iter = rolling_hashes.iter().zip(block_iter);
    let _: Vec<_> = combined_iter.batching(|current_iter| {
        if let Some((our_hash, block)) = current_iter.next() {
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

    // The last block will be sent as literal
    let remainder = bytes.len() % chunk_size;
    if remainder != 0 {
        let leftover_items = remainder;
        let leftover_block = &bytes[bytes.len() - leftover_items..];
        delta_content.push(Content::LiteralBytes(leftover_block.into()));
    }

    Delta { content: delta_content }
}


#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::{Read, Write};

    use bytes::Bytes;

    use crate::signature::handle_signature_command;

    use super::*;

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

    #[test]
    fn delta_for_equal_files_is_just_block_indexes() {
        let original_bytes = Bytes::from("Hello world");
        let signature = handle_signature_command(original_bytes, 3);
        let our_bytes = Bytes::from("Hello world");
        let delta = handle_delta_command(signature.into(), our_bytes, 3);

        for c in delta.content {
            assert!(matches!(c, Content::BlockIndex(_)));
        }
    }

    #[test]
    fn delta_for_different_files_has_byte_literals() {
        let original_bytes = Bytes::from("Hello world");
        let signature = handle_signature_command(original_bytes, 3);
        let our_bytes = Bytes::from("Hello world from somewhere else");
        let delta = handle_delta_command(signature.into(), our_bytes, 3);

        let literal_bytes = delta.content.iter().filter(|x| matches!(x, Content::LiteralBytes(_)));
        assert!(literal_bytes.count() > 0);
    }
}