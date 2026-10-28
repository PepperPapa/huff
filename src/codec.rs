use crate::bitio::{BitReader, BitWriter};
use std::cmp::Ordering;
use std::collections::BinaryHeap;

#[derive(Debug)]
enum Node {
    Leaf { byte: u8, freq: u64 },
    Internal { freq: u64, left: Box<Node>, right: Box<Node> },
}

impl Node {
    fn freq(&self) -> u64 {
        match self {
            Node::Leaf { freq, .. } => *freq,
            Node::Internal { freq, .. } => *freq,
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

pub fn encode(data: &[u8]) -> Encoded {
    let freq = count_freq(data);
    if data.is_empty() {
        return Encoded { payload: Vec::new(), freq, orig_len: 0, stored: None };
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

pub fn decode(encoded: &Encoded) -> Vec<u8> {
    if encoded.orig_len == 0 {
        return Vec::new();
    }
    if let Some(stored) = &encoded.stored {
        return stored.clone();
    }
    let tree = build_tree(&encoded.freq).expect("valid encoded data must produce a tree");
    if let Node::Leaf { byte, .. } = tree.as_ref() {
        return vec![*byte; encoded.orig_len];
    }
    let mut reader = BitReader::new(&encoded.payload);
    let mut out = Vec::with_capacity(encoded.orig_len);
    let mut node = &tree;
    while out.len() < encoded.orig_len {
        let bit = reader.read_bit().expect("payload truncated");
        node = match node.as_ref() {
            Node::Internal { left, right, .. } => if bit { right } else { left },
            Node::Leaf { .. } => unreachable!("leaf reached mid-traversal"),
        };
        if let Node::Leaf { byte, .. } = node.as_ref() {
            out.push(*byte);
            node = &tree;
        }
    }
    out
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
    for (byte, &f) in freq.iter().enumerate() {
        if f > 0 {
            heap.push(Box::new(Node::Leaf { byte: byte as u8, freq: f }));
        }
    }
    if heap.is_empty() {
        return None;
    }
    while heap.len() > 1 {
        let a = heap.pop().unwrap();
        let b = heap.pop().unwrap();
        let f = a.freq() + b.freq();
        heap.push(Box::new(Node::Internal { freq: f, left: a, right: b }));
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
            codes[*byte as usize] = path.clone();
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
mod tests {
    use super::*;

    fn roundtrip(data: &[u8]) {
        let encoded = encode(data);
        let decoded = decode(&encoded);
        assert_eq!(decoded, data, "roundtrip failed for len={}", data.len());
    }

    #[test]
    fn empty() {
        let e = encode(&[]);
        assert_eq!(e.orig_len, 0);
        assert!(e.payload.is_empty());
        assert!(decode(&e).is_empty());
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
}