use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use bytes::Bytes;
use color_eyre::eyre::Context;
use color_eyre::Help;
use rolling_hash_rust::RollingHash;
use serde::{Deserialize, Serialize};

type StrongHashType = u64;
type RollingHashType = u64;

/// Represents the contents of a File
///
/// A file is divided into blocks of `chunk_size` bytes.
/// For each block, we represent it with two hashes.
/// The rolling hash is fast to compute, but weak.
/// The strong hash is a more computationally expensive, but stronger hash.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct FileSignature {
    // We will generally be accessing `rolling_hashes` together, so it's better if they are
    // closely packed. (As opposed to a single Vec<(strong_hash, rolling_hash)>.
    // SoA vs AoS: https://en.wikipedia.org/wiki/AoS_and_SoA
    pub strong_hashes: Vec<StrongHashType>,
    pub rolling_hashes: Vec<RollingHashType>,
}

// We are using `rmp_serde` as a efficient binary format to save the files in.
// TODO: we can experiment with a custom made binary format and optimizations (the paper has some suggestions).
impl TryFrom<FileSignature> for Bytes {
    type Error = color_eyre::Report;

    fn try_from(signature: FileSignature) -> Result<Self, Self::Error> {
        let serialized = rmp_serde::to_vec(&signature)?;
        Ok(serialized.into())
    }
}

impl TryFrom<Bytes> for FileSignature {
    type Error = color_eyre::Report;

    fn try_from(bytes: Bytes) -> Result<Self, Self::Error> {
        let file_signature = rmp_serde::from_slice(&bytes)
            .wrap_err("Could not read FileSignature from file provided.")
            .suggestion(
                "Did you provide the correct path for the Signature file?\n\
                         It must have been generated as an output from a previous `signature` command.",
            )?;
        Ok(file_signature)
    }
}

/// Computes a FileSignature for the content of a file.
///
/// The file is split into equally-sized blocks (or possibly a smaller last block)
/// and each block is represented by two hashes.
///
/// # Arguments
/// * `basis_file` - A Bytes structure which holds the content of the file.
/// * `chunk_size` - The size for each block.
///
pub fn compute_signature(basis_file: Bytes, chunk_size: usize) -> FileSignature {
    let blocks = basis_file.chunks(chunk_size);
    let strong_hashes = blocks.map(calculate_strong_hash).collect();

    let mut rolling_hashes = Vec::new();
    let blocks = basis_file.chunks(chunk_size);
    blocks.for_each(|block| {
        let hasher = RollingHash::from_initial_bytes(String::from_utf8_lossy(block).as_bytes());
        let hash = hasher.get_current_hash();
        rolling_hashes.push(hash);
    });

    FileSignature {
        strong_hashes,
        rolling_hashes,
    }
}

/// Computes a strong hash for a slice of bytes.
///
/// # Arguments
/// * `content` - Bytes to hash.
///
pub fn calculate_strong_hash(content: &[u8]) -> StrongHashType {
    let mut s = DefaultHasher::new();
    content.hash(&mut s);

    s.finish()
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;

    use super::*;

    #[test]
    fn equal_files_have_equal_signatures() {
        // Signatures are just hashes. Equal files should have equal Signatures.
        // Very rarely, *different* files will have equal signatures too! (Hash Collision)
        // But note that in order for this to happen, we need a collision in both rolling hash
        // and strong hash. That won't really happen...
        let test_chunk_size = 4;

        let file1 = Bytes::from("ABCDEFGH");
        let file2 = Bytes::from("ABCDEFGH");

        let file1_signature = compute_signature(file1, test_chunk_size);
        let file2_signature = compute_signature(file2, test_chunk_size);

        assert_eq!(file1_signature, file2_signature);
    }

    #[test]
    fn different_files_have_different_signatures() {
        // It is actually possible for different files to have equal signatures
        // due to the nature of the algorithm (hashing), but that is very rare.
        let test_chunk_size = 4;

        let file1 = Bytes::from("ABCDEFGH");
        let file2 = Bytes::from("AB");

        let file1_signature = compute_signature(file1, test_chunk_size);
        let file2_signature = compute_signature(file2, test_chunk_size);

        assert_ne!(file1_signature, file2_signature);
    }

    #[test]
    fn chunk_size_too_big_means_only_one_block() {
        let test_chunk_size = 100;

        let file = Bytes::from("ABCDEFGH");

        let file_signature = compute_signature(file, test_chunk_size);

        assert_eq!(file_signature.rolling_hashes.len(), 1);
        assert_eq!(file_signature.strong_hashes.len(), 1);
    }
}
