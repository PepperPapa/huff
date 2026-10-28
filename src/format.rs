use crate::codec::Encoded;

pub const MAGIC: &[u8; 4] = b"HUFF";
pub const VERSION: u8 = 1;
pub const FLAG_STORED: u8 = 0b0000_0001;
pub const KNOWN_FLAGS: u8 = FLAG_STORED;
pub const HEADER_SIZE: usize = 4 + 1 + 1 + 8 + 256 * 8;

#[derive(Debug)]
pub enum FormatError {
    TooShort { expected: usize, got: usize },
    InvalidMagic,
    UnsupportedVersion(u8),
    UnknownFlags(u8),
    PayloadMismatch { expected: usize, got: usize },
}

impl std::fmt::Display for FormatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FormatError::TooShort { expected, got } =>
                write!(f, "file too short: need at least {} bytes, got {}", expected, got),
            FormatError::InvalidMagic =>
                write!(f, "not a HUFF file: invalid magic bytes"),
            FormatError::UnsupportedVersion(v) =>
                write!(f, "unsupported format version: {} (this build supports {})", v, VERSION),
            FormatError::UnknownFlags(b) =>
                write!(f, "unknown flags in header: 0x{:02x}", b),
            FormatError::PayloadMismatch { expected, got } =>
                write!(f, "stored payload size mismatch: expected {}, got {}", expected, got),
        }
    }
}

impl std::error::Error for FormatError {}

pub fn serialize(encoded: &Encoded) -> Vec<u8> {
    let is_stored = encoded.stored.is_some();
    let payload: &[u8] = match &encoded.stored {
        Some(v) => v.as_slice(),
        None => &encoded.payload,
    };
    let mut out = Vec::with_capacity(HEADER_SIZE + payload.len());
    out.extend_from_slice(MAGIC);
    out.push(VERSION);
    out.push(if is_stored { FLAG_STORED } else { 0 });
    out.extend_from_slice(&(encoded.orig_len as u64).to_le_bytes());
    for &f in &encoded.freq {
        out.extend_from_slice(&f.to_le_bytes());
    }
    out.extend_from_slice(payload);
    out
}

pub fn deserialize(bytes: &[u8]) -> Result<Encoded, FormatError> {
    if bytes.len() < HEADER_SIZE {
        return Err(FormatError::TooShort { expected: HEADER_SIZE, got: bytes.len() });
    }
    if !bytes.starts_with(MAGIC) {
        return Err(FormatError::InvalidMagic);
    }
    let version = bytes[4];
    if version != VERSION {
        return Err(FormatError::UnsupportedVersion(version));
    }
    let flags = bytes[5];
    if flags & !KNOWN_FLAGS != 0 {
        return Err(FormatError::UnknownFlags(flags));
    }
    let is_stored = flags & FLAG_STORED != 0;

    let mut len_bytes = [0u8; 8];
    len_bytes.copy_from_slice(&bytes[6..14]);
    let orig_len = u64::from_le_bytes(len_bytes) as usize;

    let mut freq = [0u64; 256];
    for i in 0..256 {
        let start = 14 + i * 8;
        let mut fb = [0u8; 8];
        fb.copy_from_slice(&bytes[start..start + 8]);
        freq[i] = u64::from_le_bytes(fb);
    }

    let payload_bytes = &bytes[HEADER_SIZE..];

    if is_stored {
        if payload_bytes.len() != orig_len {
            return Err(FormatError::PayloadMismatch {
                expected: orig_len,
                got: payload_bytes.len(),
            });
        }
        Ok(Encoded {
            payload: Vec::new(),
            freq,
            orig_len,
            stored: Some(payload_bytes.to_vec()),
        })
    } else {
        Ok(Encoded {
            payload: payload_bytes.to_vec(),
            freq,
            orig_len,
            stored: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::{decode, encode};

    #[test]
    fn roundtrip_with_header() {
        let data = b"the quick brown fox jumps over the lazy dog";
        let encoded = encode(data);
        let bytes = serialize(&encoded);
        let parsed = deserialize(&bytes).expect("valid header");
        assert_eq!(decode(&parsed), data);
    }

    #[test]
    fn empty_file_roundtrip() {
        let encoded = encode(&[]);
        let bytes = serialize(&encoded);
        let parsed = deserialize(&bytes).expect("valid header");
        assert!(decode(&parsed).is_empty());
    }

    #[test]
    fn rejects_short_input() {
        let err = deserialize(b"HUFF").unwrap_err();
        assert!(matches!(err, FormatError::TooShort { .. }));
    }

    #[test]
    fn rejects_bad_magic() {
        let mut bytes = serialize(&encode(b"hello"));
        bytes[0] = b'X';
        let err = deserialize(&bytes).unwrap_err();
        assert!(matches!(err, FormatError::InvalidMagic));
    }

    #[test]
    fn rejects_bad_version() {
        let mut bytes = serialize(&encode(b"hello"));
        bytes[4] = 99;
        let err = deserialize(&bytes).unwrap_err();
        assert!(matches!(err, FormatError::UnsupportedVersion(99)));
    }

    #[test]
    fn rejects_unknown_flags() {
        let mut bytes = serialize(&encode(b"hello"));
        bytes[5] = 0b1000_0000;
        let err = deserialize(&bytes).unwrap_err();
        assert!(matches!(err, FormatError::UnknownFlags(_)));
    }

    #[test]
    fn uniform_256_bytes_uses_stored() {
        let data: Vec<u8> = (0..=255u8).collect();
        let encoded = encode(&data);
        assert!(encoded.stored.is_some(), "uniform 256 bytes should fall back to stored");
        let parsed = deserialize(&serialize(&encoded)).expect("valid header");
        assert_eq!(decode(&parsed), data);
    }

    #[test]
    fn stored_truncated_is_rejected() {
        let data: Vec<u8> = (0..=255u8).collect();
        let mut bytes = serialize(&encode(&data));
        bytes.pop();
        let err = deserialize(&bytes).unwrap_err();
        assert!(matches!(err, FormatError::PayloadMismatch { .. }));
    }
}