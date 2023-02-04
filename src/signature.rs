use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use bytes::Bytes;
use rolling_hash_rust::RollingHash;
use serde::{Deserialize, Serialize};

type StrongHashType = u64;
type RollingHashType = u64;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct FileSignature {
    pub strong_hashes: Vec<StrongHashType>,
    pub rolling_hashes: Vec<RollingHashType>,
}

impl From<FileSignature> for Bytes {
    fn from(value: FileSignature) -> Self {
        serde_json::to_vec_pretty(&value)
            .expect("Could not serialize FileSignature into JSON")
            .into()
    }
}

// TODO: should it be TryFrom instead?
// I am using From<Bytes> based on usage I have seen of FromStr, instead of TryFrom<str>
impl From<Bytes> for FileSignature {
    fn from(bytes: Bytes) -> Self {
        serde_json::from_slice(&bytes).expect("Could not deserialize Bytes into FileSignature")
    }
}

pub fn compute_signature(content: Bytes, chunk_size: usize) -> FileSignature {
    let blocks = content.chunks(chunk_size);
    let strong_hashes = blocks.map(calculate_strong_hash).collect();

    let mut rolling_hashes = Vec::new();
    let blocks = content.chunks(chunk_size);
    blocks.for_each(|block| {
        // TODO: change rolling hash to accept bytes
        // TODO: make this code better
        let hasher =
            RollingHash::from_initial_string(&String::from_utf8(Vec::from(block)).unwrap());
        let hash = hasher.get_current_hash();
        rolling_hashes.push(hash);
    });

    FileSignature {
        strong_hashes,
        rolling_hashes,
    }
}

// Use the default hash is std for now
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
}
