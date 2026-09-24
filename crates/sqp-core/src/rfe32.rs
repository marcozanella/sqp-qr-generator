//! RFE32 base-32 codec: 20 bytes (160 bits) <-> 32 characters.
//!
//! The 160-bit view is big-endian — bytes are streamed MSB-first into a bit
//! buffer, then re-grouped into 5-bit symbols (also MSB-first). This matches the
//! Ruby encoder's `H*`/big-endian handling.

use crate::SqpError;

/// RFE32 alphabet (order matters): A–Z minus I and O, plus 2–9. Index 0 = 'A'.
const ALPHABET: &[u8; 32] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";

fn val_of(c: char) -> Option<u8> {
    ALPHABET.iter().position(|&b| b as char == c).map(|i| i as u8)
}

/// Decode 32 characters into 20 bytes (big-endian, 5 bits per symbol).
pub fn decode(s: &str) -> Result<[u8; 20], SqpError> {
    if s.chars().count() != 32 {
        return Err(SqpError::BadLength(s.chars().count()));
    }
    let mut bits: Vec<u8> = Vec::with_capacity(160);
    for c in s.chars() {
        let v = val_of(c).ok_or(SqpError::CharNotInAlphabet(c))?;
        for shift in (0..5).rev() {
            bits.push((v >> shift) & 1);
        }
    }
    let mut out = [0u8; 20];
    for (i, chunk) in bits.chunks(8).enumerate() {
        let mut byte = 0u8;
        for &bit in chunk {
            byte = (byte << 1) | bit;
        }
        out[i] = byte;
    }
    Ok(out)
}

/// Encode 20 bytes (160 bits) into 32 characters.
pub fn encode(bytes: &[u8; 20]) -> String {
    let mut bits: Vec<u8> = Vec::with_capacity(160);
    for &b in bytes.iter() {
        for shift in (0..8).rev() {
            bits.push((b >> shift) & 1);
        }
    }
    bits.chunks(5)
        .map(|chunk| {
            let mut v = 0u8;
            for &bit in chunk {
                v = (v << 1) | bit;
            }
            ALPHABET[v as usize] as char
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_all_bytes() {
        let bytes: [u8; 20] = [
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 250, 251, 252, 253, 254, 255, 128, 64, 32, 16,
        ];
        let s = encode(&bytes);
        assert_eq!(s.len(), 32);
        assert_eq!(decode(&s).unwrap(), bytes);
    }

    #[test]
    fn decode_rejects_wrong_length() {
        assert_eq!(decode("ABC"), Err(SqpError::BadLength(3)));
    }

    #[test]
    fn decode_rejects_bad_char() {
        let bad = "I".to_string() + &"A".repeat(31); // 'I' not in alphabet
        assert_eq!(decode(&bad), Err(SqpError::CharNotInAlphabet('I')));
    }
}
