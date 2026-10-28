#[derive(Debug)]
pub enum CodecError {
    /// `orig_len` > 0 但频率表全为 0，文件头损坏
    EmptyFreqTable,
    /// payload 不足以解码出 `orig_len` 个字节
    TruncatedPayload,
    /// stored 模式长度与 `orig_len` 不一致
    StoredLengthMismatch { expected: usize, got: usize },
    /// 解码路径意外走到叶子（理论不可达，防御性保留）
    InvalidTraversal,
    /// 预分配 `orig_len` 字节失败（恶意头部）
    AllocationFailed { requested: usize },
}

impl std::fmt::Display for CodecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyFreqTable => write!(
                f,
                "corrupt header: original length is non-zero but frequency table is empty"
            ),
            Self::TruncatedPayload => write!(
                f,
                "corrupt payload: bitstream ended before all bytes were decoded"
            ),
            Self::StoredLengthMismatch { expected, got } => write!(
                f,
                "corrupt stored payload: expected {expected} bytes, got {got}"
            ),
            Self::InvalidTraversal => write!(f, "corrupt payload: invalid huffman traversal"),
            Self::AllocationFailed { requested } => write!(
                f,
                "cannot allocate {requested} bytes for decompressed output"
            ),
        }
    }
}

impl std::error::Error for CodecError {}

use crate::bitio::{BitReader, BitWriter};
use std::cmp::Ordering;
use std::collections::BinaryHeap;

#[derive(Debug)]
enum Node {
    Leaf {
        byte: u8,
        freq: u64,
    },
    Internal {
        freq: u64,
        left: Box<Node>,
        right: Box<Node>,
    },
}

impl Node {
    fn freq(&self) -> u64 {
        match self {
            Node::Leaf { freq, .. } | Node::Internal { freq, .. } => *freq,
        }
    }
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.freq() == other.freq()
    }
}
impl Eq for Node {}

impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Node {
    fn cmp(&self, other: &Self) -> Ordering {
        // 反向：让 BinaryHeap（最大堆）变成最小堆
        other.freq().cmp(&self.freq())
    }
}

#[derive(Debug)]
pub struct Encoded {
    pub payload: Vec<u8>,
    pub freq: [u64; 256],
    pub orig_len: usize,
    pub stored: Option<Vec<u8>>,
}

/// Encode `data` into an `Encoded` representation.
///
/// # Panics
///
/// Panics if `data` is non-empty but `build_tree` returns `None`.
/// This cannot happen: non-empty input has at least one byte with
/// non-zero frequency, so the heap is non-empty.
#[must_use]
pub fn encode(data: &[u8]) -> Encoded {
    let freq = count_freq(data);
    if data.is_empty() {
        return Encoded {
            payload: Vec::new(),
            freq,
            orig_len: 0,
            stored: None,
        };
    }
    let tree = build_tree(&freq).expect("non-empty data must produce a tree");
    let codes = build_codes(&tree);
    let mut w = BitWriter::new();
    for &b in data {
        w.write_bits(&codes[b as usize]);
    }
    let compressed = w.finish();
    let stored = if compressed.len() >= data.len() {
        Some(data.to_vec())
    } else {
        None
    };
    Encoded {
        payload: compressed,
        freq,
        orig_len: data.len(),
        stored,
    }
}

/// Decode `encoded` back into the original bytes.
///
/// # Errors
///
/// Returns [`CodecError`] if the frequency table is empty, the
/// payload is truncated, stored length mismatches `orig_len`, or
/// allocation for the output buffer fails.
pub fn decode(encoded: &Encoded) -> Result<Vec<u8>, CodecError> {
    if encoded.orig_len == 0 {
        return Ok(Vec::new());
    }

    if let Some(stored) = &encoded.stored {
        if stored.len() != encoded.orig_len {
            return Err(CodecError::StoredLengthMismatch {
                expected: encoded.orig_len,
                got: stored.len(),
            });
        }
        return Ok(stored.clone());
    }

    let tree = build_tree(&encoded.freq).ok_or(CodecError::EmptyFreqTable)?;

    if let Node::Leaf { byte, .. } = tree.as_ref() {
        let mut out = Vec::new();
        out.try_reserve(encoded.orig_len)
            .map_err(|_| CodecError::AllocationFailed {
                requested: encoded.orig_len,
            })?;
        out.resize(encoded.orig_len, *byte);
        return Ok(out);
    }

    let mut reader = BitReader::new(&encoded.payload);
    let mut out = Vec::new();
    out.try_reserve(encoded.orig_len)
        .map_err(|_| CodecError::AllocationFailed {
            requested: encoded.orig_len,
        })?;

    let mut node = &tree;
    while out.len() < encoded.orig_len {
        let bit = reader.read_bit().ok_or(CodecError::TruncatedPayload)?;
        node = match node.as_ref() {
            Node::Internal { left, right, .. } => {
                if bit {
                    right
                } else {
                    left
                }
            }
            Node::Leaf { .. } => return Err(CodecError::InvalidTraversal),
        };
        if let Node::Leaf { byte, .. } = node.as_ref() {
            out.push(*byte);
            node = &tree;
        }
    }
    Ok(out)
}

fn count_freq(data: &[u8]) -> [u64; 256] {
    let mut freq = [0u64; 256];
    for &b in data {
        freq[b as usize] += 1;
    }
    freq
}

fn build_tree(freq: &[u64; 256]) -> Option<Box<Node>> {
    let mut heap: BinaryHeap<Box<Node>> = BinaryHeap::new();
    for byte in 0..=255u8 {
        let f = freq[byte as usize];
        if f > 0 {
            heap.push(Box::new(Node::Leaf { byte, freq: f }));
        }
    }
    // 下面的 while 不变
    if heap.is_empty() {
        return None;
    }
    while heap.len() > 1 {
        let a = heap.pop().unwrap();
        let b = heap.pop().unwrap();
        let f = a.freq() + b.freq();
        heap.push(Box::new(Node::Internal {
            freq: f,
            left: a,
            right: b,
        }));
    }
    heap.pop()
}

fn build_codes(root: &Node) -> [Vec<bool>; 256] {
    let mut codes: [Vec<bool>; 256] = std::array::from_fn(|_| Vec::new());
    let mut path = Vec::new();
    walk(root, &mut path, &mut codes);
    codes
}

fn walk(node: &Node, path: &mut Vec<bool>, codes: &mut [Vec<bool>; 256]) {
    match node {
        Node::Leaf { byte, .. } => {
            codes[*byte as usize].clone_from(path);
        }
        Node::Internal { left, right, .. } => {
            path.push(false);
            walk(left, path, codes);
            path.pop();
            path.push(true);
            walk(right, path, codes);
            path.pop();
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::assert_is_empty,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
mod tests {
    use super::*;

    fn roundtrip(data: &[u8]) {
        let encoded = encode(data);
        let decoded = decode(&encoded).unwrap();
        assert_eq!(decoded, data, "roundtrip failed for len={}", data.len());
    }

    #[test]
    fn empty() {
        let e = encode(&[]);
        assert_eq!(e.orig_len, 0);
        assert!(e.payload.is_empty());
        assert!(decode(&e).unwrap().is_empty());
    }

    #[test]
    fn single_byte() {
        roundtrip(&[42]);
        let e = encode(&[42]);
        assert!(e.payload.is_empty(), "single symbol should use 0 bits");
    }

    #[test]
    fn same_byte_repeated() {
        roundtrip(&[7u8; 1000]);
    }

    #[test]
    fn all_256_bytes() {
        let data: Vec<u8> = (0..=255u8).collect();
        roundtrip(&data);
    }

    #[test]
    fn typical_text() {
        roundtrip(b"hello huffman hello huffman hello huffman");
    }

    #[test]
    fn pseudo_random_binary() {
        let data: Vec<u8> = (0..10000).map(|i| (i * 37 + 13) as u8).collect();
        roundtrip(&data);
    }

    #[test]
    fn compresses_skewed_text() {
        // a 多，b/c/d 少，Huffman 应该明显压缩
        let data = b"aaaaaaaabbbbccdd";
        let e = encode(data);
        assert!(
            e.payload.len() < data.len(),
            "skewed text should compress: {} vs {}",
            e.payload.len(),
            data.len()
        );
        roundtrip(data);
    }

    #[test]
    fn decode_rejects_empty_freq_with_nonzero_len() {
        let e = Encoded {
            payload: Vec::new(),
            freq: [0u64; 256],
            orig_len: 10,
            stored: None,
        };
        let err = decode(&e).unwrap_err();
        assert!(matches!(err, CodecError::EmptyFreqTable));
    }

    #[test]
    fn decode_rejects_truncated_payload() {
        let data = b"hello huffman hello huffman";
        let mut e = encode(data);
        e.payload.clear(); // 清空 payload，解码必然截断
        let err = decode(&e).unwrap_err();
        assert!(matches!(err, CodecError::TruncatedPayload));
    }

    #[test]
    fn decode_rejects_stored_length_mismatch() {
        let e = Encoded {
            payload: Vec::new(),
            freq: [0u64; 256],
            orig_len: 10,
            stored: Some(vec![1, 2, 3]),
        };
        let err = decode(&e).unwrap_err();
        assert!(matches!(
            err,
            CodecError::StoredLengthMismatch {
                expected: 10,
                got: 3
            }
        ));
    }

    #[test]
    fn decode_rejects_absurd_orig_len() {
        // 单符号 + 超大 orig_len，try_reserve 应失败而不是 abort
        let mut freq = [0u64; 256];
        freq[b'x' as usize] = 1;
        let e = Encoded {
            payload: Vec::new(),
            freq,
            orig_len: usize::MAX / 2,
            stored: None,
        };
        let err = decode(&e).unwrap_err();
        assert!(matches!(err, CodecError::AllocationFailed { .. }));
    }
}
