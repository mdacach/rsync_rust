use bytes::Bytes;

use crate::delta::{Delta, Token};

pub fn apply_delta(basis_file: Bytes, delta: Delta, chunk_size: usize) -> Bytes {
    let blocks: Vec<_> = basis_file.chunks(chunk_size).collect();
    let mut reconstructed = Vec::new();

    delta.content.iter().for_each(|c| match c {
        Token::BlockIndex(index) => {
            reconstructed.extend(blocks.get(*index).unwrap().to_vec());
        }
        Token::ByteLiteral(byte) => reconstructed.push(*byte),
    });

    Bytes::from(reconstructed)
}

#[cfg(test)]
mod tests {
    use crate::delta::{Delta, Token};

    use super::*;

    fn create_byte_literals(bytes: &[u8]) -> Vec<Token> {
        bytes.iter().copied().map(Token::ByteLiteral).collect()
    }

    #[test]
    fn can_construct_file_from_literal_bytes() {
        let test_chunk_size = 3;

        let delta = {
            let mut content = Vec::new();
            content.extend(create_byte_literals(b"abc"));
            content.extend(create_byte_literals(b"def"));
            Delta { content }
        };

        let empty_file = Bytes::new();
        let reconstructed = apply_delta(empty_file, delta, test_chunk_size);

        assert_eq!(reconstructed, Bytes::from("abcdef"));
    }

    #[test]
    fn can_construct_file_from_block_indexes() {
        let test_chunk_size = 7;

        let basis_file = Bytes::from("block1 block2 block3 ");
        let delta = Delta {
            content: vec![
                Token::BlockIndex(1),
                Token::BlockIndex(2),
                Token::BlockIndex(1),
                Token::BlockIndex(0),
            ],
        };

        let reconstructed = apply_delta(basis_file, delta, test_chunk_size);

        assert_eq!(reconstructed, Bytes::from("block2 block3 block2 block1 "));
    }

    #[test]
    fn can_construct_file_from_both_block_and_literals() {
        let test_chunk_size = 7;

        let basis_file = Bytes::from("block1 ");

        let delta = {
            let mut content = Vec::new();
            content.extend(create_byte_literals(b"abc"));
            content.push(Token::BlockIndex(0));
            content.extend(create_byte_literals(b"abc"));
            Delta { content }
        };

        let reconstructed = apply_delta(basis_file, delta, test_chunk_size);

        assert_eq!(reconstructed, Bytes::from("abcblock1 abc"));
    }
}
