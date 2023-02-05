use std::collections::HashMap;

use bytes::Bytes;
use rolling_hash_rust::RollingHash;
use serde::{Deserialize, Serialize};

use crate::signature::{calculate_strong_hash, FileSignature};

/// Represents how to transform the basis file into the updated file, in order.
///
/// The updated file can be reconstructed by reusing some of the basis file blocks
/// (through a BlockIndex), or by writing (new) byte literals.
#[derive(Debug, Eq, PartialEq, Serialize, Deserialize, Clone)]
pub struct Delta {
    pub(crate) content: Vec<Token>,
}

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize, Clone)]
pub enum Token {
    BlockIndex(usize),
    // A reference to a block within the basis file.
    ByteLiteral(u8), // A byte literal to be reconstructed directly.
}

// We are using `rmp_serde` as a efficient binary format to save the files in.
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

/// Computes a Delta from a FileSignature.
///
/// Given a Signature and our file, creates the Delta that specifies how to reconstruct
/// the basis file (the one the Signature represents) into our updated file.
/// Note that the `chunk_size` argument must be the same as what was used when creating
/// the FileSignature).
///
/// # Arguments
/// * `signature` - The FileSignature representing the basis file.
/// * `updated_file` - Our updated file, in bytes.
/// * `chunk_size` - The size for each block used in the Signature.
///
pub fn compute_delta_to_our_file(
    signature: FileSignature,
    updated_file: Bytes,
    chunk_size: usize,
) -> Delta {
    // Each of our "sliding" blocks can match to a block in the basis file.
    // So we need to test all of the "sliding block", which means we will compare
    // rolling_hashes and (potentially) strong_hashes.

    let our_sliding_blocks_rolling_hashes = {
        let bytes = updated_file.clone();

        if chunk_size <= updated_file.len() {
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

    // Map with key: RollingHash and value: index of the block with given hash.
    // This map is used to quickly match blocks from our file and theirs with
    // equal rolling_hash.
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

    let delta_tokens = {
        let mut tokens = Vec::new();

        let our_file_size = updated_file.len();
        // We need to construct the delta considering ALL of our bytes:
        // We have one rolling hash for each potential block
        let mut index = 0;
        while index < our_file_size {
            let our_block_starting_byte = updated_file[index];

            let end_of_our_block = index + chunk_size - 1; // inclusive
            if end_of_our_block >= our_file_size {
                // This is part of a trailing block, which shall be sent directly
                // as ByteLiteral
                tokens.push(Token::ByteLiteral(our_block_starting_byte));
                index += 1;
                continue;
            }

            // For each block, we will try to match it to an existing one in the basis file
            // using the rolling_hashes.
            let our_block_rolling_hash = our_sliding_blocks_rolling_hashes[index];
            match their_rolling_hashes.get(&our_block_rolling_hash) {
                Some(&matched_block_index) => {
                    // We have matched our current block with block at `matched_block_index` in the basis file.
                    // Note this is only a *potential* match, as it may be a collision in the rolling_hashes.

                    // We only consider the block to be a true match if we match the strong_hashes as well.
                    // As the strong_hash is computationally expensive, we only compute it when needed
                    // (if the rolling_hashes have matched).
                    let our_block_strong_hash = {
                        let block_bytes = &updated_file[index..=end_of_our_block];
                        calculate_strong_hash(block_bytes)
                    };
                    let their_strong_hash = signature.strong_hashes[matched_block_index];

                    if our_block_strong_hash == their_strong_hash {
                        // These blocks have matched both rolling_hashes and strong_hashes.
                        // We are confident they are the same.
                        tokens.push(Token::BlockIndex(matched_block_index));
                        // All this block is already accounted for, jump to the next unaccounted byte.
                        index += chunk_size;
                    } else {
                        // The rolling_hashes matched but not the strong_hashes. It was a false positive.
                        tokens.push(Token::ByteLiteral(our_block_starting_byte));
                        index += 1;
                        // Note that if we, mistakenly, thought that the rolling_hashes were sufficient,
                        // we would have pushed a reference to a different block, thus reconstructing
                        // a wrong file in the end! Dodged a bullet here!
                    }
                }
                None => {
                    // No blocks match the rolling hash. The best we can do is to send the byte directly.
                    tokens.push(Token::ByteLiteral(our_block_starting_byte));
                    index += 1;
                    // Note that we can be confident that no matching block exists at all, because equal
                    // blocks would have equal hashes.
                }
            }
        }

        tokens
    };

    Delta {
        content: delta_tokens,
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
            assert!(matches!(c, Token::BlockIndex(_)));
        }
    }

    #[test]
    fn delta_for_equal_content_is_block_indexes_plus_literals_when_there_is_leftover() {
        let test_chunk_size = 5;
        // Hello World! has 12 bytes. We will have 2 chunks of size 5
        // and a leftover chunk of size 2. This last chunk will be sent as two ByteLiterals.
        let basis_file = Bytes::from("Hello World!");
        let updated_file = Bytes::from("Hello World!");

        let signature = compute_signature(basis_file, test_chunk_size);
        // We need to calculate the delta from our `updated_file` to `basis_file` based on signature.
        let delta = compute_delta_to_our_file(signature, updated_file, test_chunk_size);

        // 2 BlockIndex (for the first two chunks).
        let block_indexes = &delta.content[0..2];
        for b in block_indexes {
            assert!(matches!(b, Token::BlockIndex(_)));
        }

        // 2 ByteLiterals (for the leftover chunk).
        let byte_literals = &delta.content[2..];
        for b in byte_literals {
            assert!(matches!(b, Token::ByteLiteral(_)));
        }
    }

    #[test]
    fn delta_for_completely_different_files_has_only_literal_bytes() {
        let test_chunk_size = 3;

        // Files are completely different, no block will match.
        let basis_file = Bytes::from("ABCDEF");
        let updated_file = Bytes::from("GHIJKL");

        let signature = compute_signature(basis_file, test_chunk_size);
        let delta = compute_delta_to_our_file(signature, updated_file, test_chunk_size);

        for b in delta.content {
            assert!(matches!(b, Token::ByteLiteral(_)));
        }
    }

    #[test]
    fn delta_for_similar_files_has_block_indexes_and_literal_bytes() {
        let test_chunk_size = 3;

        // We should have two matching chunks: "ABC" and "EF ".
        let basis_file = Bytes::from("ZY ABCDEF ");
        let updated_file = Bytes::from("ABCDxEF Z");

        let signature = compute_signature(basis_file, test_chunk_size);
        let delta = compute_delta_to_our_file(signature, updated_file, test_chunk_size);

        let byte_literals = delta
            .content
            .iter()
            .filter(|x| matches!(x, Token::ByteLiteral(_)));
        let block_indexes = delta
            .content
            .iter()
            .filter(|x| matches!(x, Token::BlockIndex(_)));

        assert!(byte_literals.count() > 0);
        assert!(block_indexes.count() > 0);
    }

    #[test]
    fn chunk_size_bigger_means_only_literals() {
        let test_chunk_size = 100;

        // We should have two matching chunks: "ABC" and "EF ".
        let basis_file = Bytes::from("ZY ABCDEF ");
        let updated_file = Bytes::from("ABCDxEF Z");

        let signature = compute_signature(basis_file, test_chunk_size);
        let delta = compute_delta_to_our_file(signature, updated_file, test_chunk_size);

        let block_indexes = delta
            .content
            .iter()
            .filter(|x| matches!(x, Token::BlockIndex(_)));

        assert_eq!(block_indexes.count(), 0);
    }
}
