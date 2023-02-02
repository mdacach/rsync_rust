use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use bytes::Bytes;
use rolling_hash_rust::RollingHash;
use serde::{Deserialize, Serialize};

type StrongHashType = u64;
type RollingHashType = u64;

#[derive(Serialize, Deserialize)]
#[derive(Debug, PartialEq, Eq)]
pub struct FileSignature {
    strong_hashes: Vec<StrongHashType>,
    pub(crate) rolling_hashes: Vec<RollingHashType>,
}

impl From<FileSignature> for Bytes {
    fn from(value: FileSignature) -> Self {
        serde_json::to_vec_pretty(&value).expect("Could not serialize FileSignature into JSON").into()
    }
}

// TODO: should it be TryFrom instead?
// I am using From<Bytes> based on usage I have seen of FromStr, instead of TryFrom<str>
impl From<Bytes> for FileSignature {
    fn from(bytes: Bytes) -> Self {
        serde_json::from_slice(&bytes).expect("Could not deserialize Bytes into FileSignature")
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


pub fn handle_signature_command(file_bytes: Bytes, chunk_size: usize) -> FileSignature {
    compute_signature(file_bytes, chunk_size)
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;

    use super::*;

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
}