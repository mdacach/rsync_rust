use bytes::Bytes;

use crate::delta::{Content, Delta};

pub fn apply_delta(basis_file: Bytes, delta: Delta, chunk_size: usize) -> Bytes {
    let blocks: Vec<_> = basis_file.chunks(chunk_size).collect();
    let mut reconstructed = Vec::new();

    delta.content.iter().for_each(|c| match c {
        Content::BlockIndex(index) => {
            reconstructed.extend(blocks.get(*index).unwrap().to_vec());
        }
        Content::LiteralBytes(bytes) => reconstructed.extend(bytes),
    });

    Bytes::from(reconstructed)
}

