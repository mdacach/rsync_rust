use std::collections::HashMap;

use bytes::Bytes;
use rolling_hash_rust::RollingHash;
use serde::{Deserialize, Serialize};

use crate::signature::{calculate_strong_hash, FileSignature};

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Delta {
    pub(crate) content: Vec<Content>,
}

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Content {
    BlockIndex(usize),
    ByteLiteral(u8),
}

impl From<Delta> for Bytes {
    fn from(value: Delta) -> Self {
        rmp_serde::to_vec(&value)
            .expect("Could not serialize Delta into Bytes")
            .into()
    }
}

impl From<Bytes> for Delta {
    fn from(value: Bytes) -> Delta {
        rmp_serde::from_slice(&value).expect("Could not deserialize Delta from Bytes")
    }
}

pub fn compute_delta_to_our_file(
    signature: FileSignature,
    our_file_bytes: Bytes,
    chunk_size: usize,
) -> Delta {
    let rolling_hashes = {
        let bytes = our_file_bytes.clone();

        if chunk_size <= our_file_bytes.len() {
            // We will have a rolling hash for each sliding block
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
        } else {
            // We do not have enough bytes to construct a block
            Vec::new()
        }
    };

    let mut delta_content = Vec::new();

    // Rolling Hash -> Block Index in their file
    let their_rolling_hashes = {
        let mut map = HashMap::new();
        signature
            .rolling_hashes
            .iter()
            .enumerate()
            .for_each(|(index, hash)| {
                map.insert(hash, index);
            });
        map
    };

    // We have one rolling hash for each potential block
    let mut index = 0;
    let our_file_size = our_file_bytes.len();
    while index < our_file_size {
        let block_starting_byte = our_file_bytes[index];

        let end_of_this_block = index + chunk_size - 1; // inclusive
        if end_of_this_block >= our_file_size {
            // This is part of a trailing chunk, which shall be sent directly
            // as ByteLiteral
            delta_content.push(Content::ByteLiteral(block_starting_byte));
            index += 1;
            continue;
        }

        // Otherwise, we may try to match this block
        let block_rolling_hash = rolling_hashes[index];

        match their_rolling_hashes.get(&block_rolling_hash) {
            Some(&block_index) => {
                // This is a potential match. The rolling hashes have matched, but it may be just a
                // hash collision.

                // Now we must (compute and) check if the strong hashes match too.
                let block_strong_hash = {
                    let block_bytes = &our_file_bytes[index..=end_of_this_block];
                    calculate_strong_hash(block_bytes)
                };
                let their_strong_hash = signature.strong_hashes[block_index];

                if block_strong_hash == their_strong_hash {
                    // We are confident it is a match.
                    delta_content.push(Content::BlockIndex(block_index));
                    // All this block is already accounted for
                    index += chunk_size;
                } else {
                    // It was just a hash collision on the rolling hashes. Dodged a bullet here!
                    delta_content.push(Content::ByteLiteral(block_starting_byte));
                    index += 1;
                }
            }
            None => {
                // No blocks match the rolling hash. The best we can do is to send the byte directly.
                delta_content.push(Content::ByteLiteral(block_starting_byte));
                index += 1;
            }
        }
    }

    Delta {
        content: delta_content,
    }
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;

    use crate::signature::compute_signature;

    use super::*;

    // These tests establish that the general idea of the algorithm is working:
    // 1 - We are referencing blocks on matching chunks
    // 2 - We are sending byte literals otherwise
    // The actual specifics of correctness will be tested by integration tests.

    // TODO: test function names are becoming too specific. Think about refactoring with some
    //       crate or table-driven tests.
    #[test]
    fn delta_for_equal_content_is_just_block_indexes_when_chunks_divide_evenly() {
        let test_chunk_size = 3;
        // Hello World! has 12 bytes. We will have 4 chunks of size 3 and no leftover.
        // This means our delta can be 4 references to Blocks.
        let file1 = Bytes::from("Hello World!");
        let file2 = Bytes::from("Hello World!");

        let file1_signature = compute_signature(file1, test_chunk_size);
        // We need to calculate the delta from our file `file2` to `file1` based on
        // `file1`'s signature.
        let delta = compute_delta_to_our_file(file1_signature, file2, test_chunk_size);

        // Delta is all BlockIndexes.
        for c in delta.content {
            assert!(matches!(c, Content::BlockIndex(_)));
        }
    }

    #[test]
    fn delta_for_equal_content_is_block_indexes_plus_literals_when_there_is_leftover() {
        let test_chunk_size = 5;
        // Hello World! has 12 bytes. We will have 2 chunks of size 5
        // and a leftover chunk of size 2. This last chunk will be sent as two ByteLiterals.
        let file1 = Bytes::from("Hello World!");
        let file2 = Bytes::from("Hello World!");

        let file1_signature = compute_signature(file1, test_chunk_size);
        // We need to calculate the delta from our file `file2` to `file1` based on
        // `file1`'s signature.
        let delta = compute_delta_to_our_file(file1_signature, file2, test_chunk_size);

        // 2 BlockIndex (for the first two chunks).
        let block_indexes = &delta.content[0..2];
        for b in block_indexes {
            assert!(matches!(b, Content::BlockIndex(_)));
        }

        // 2 ByteLiterals (for the leftover chunk).
        let byte_literals = &delta.content[2..];
        for b in byte_literals {
            assert!(matches!(b, Content::ByteLiteral(_)));
        }
    }

    #[test]
    fn delta_for_completely_different_files_has_only_literal_bytes() {
        let test_chunk_size = 3;

        // Files are completely different, no block will match.
        let file1 = Bytes::from("ABCDEF");
        let file2 = Bytes::from("GHIJKL");

        let file1_signature = compute_signature(file1, test_chunk_size);
        let delta = compute_delta_to_our_file(file1_signature, file2, test_chunk_size);

        for b in delta.content {
            assert!(matches!(b, Content::ByteLiteral(_)));
        }
    }

    #[test]
    fn delta_for_similar_files_has_block_indexes_and_literal_bytes() {
        let test_chunk_size = 3;

        // We should have two matching chunks: "ABC" and "EF ".
        let file1 = Bytes::from("ZY ABCDEF ");
        let file2 = Bytes::from("ABCDxEF Z");

        let file1_signature = compute_signature(file1, test_chunk_size);
        let delta = compute_delta_to_our_file(file1_signature, file2, test_chunk_size);

        let byte_literals = delta
            .content
            .iter()
            .filter(|x| matches!(x, Content::ByteLiteral(_)));
        let block_indexes = delta
            .content
            .iter()
            .filter(|x| matches!(x, Content::BlockIndex(_)));

        assert!(byte_literals.count() > 0);
        assert!(block_indexes.count() > 0);
    }

    #[test]
    fn chunk_size_bigger_means_only_literals() {
        let test_chunk_size = 100;

        // We should have two matching chunks: "ABC" and "EF ".
        let file1 = Bytes::from("ZY ABCDEF ");
        let file2 = Bytes::from("ABCDxEF Z");

        let file1_signature = compute_signature(file1, test_chunk_size);
        let delta = compute_delta_to_our_file(file1_signature, file2, test_chunk_size);

        let block_indexes = delta
            .content
            .iter()
            .filter(|x| matches!(x, Content::BlockIndex(_)));

        assert_eq!(block_indexes.count(), 0);
    }
}
