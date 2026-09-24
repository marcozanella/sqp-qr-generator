//! The 4096-byte byte pool used for RC6 key derivation, embedded at compile time.
//!
//! Byte-for-byte identical to `platform01/lib/sqp/bytepool.bin`.

/// The embedded 4096-byte pool.
pub const DATA: &[u8] = include_bytes!("../assets/bytepool.bin");

/// Reads `len` bytes starting at `base`, wrapping around the 4096-byte pool.
pub fn slice(base: usize, len: usize) -> Vec<u8> {
    let size = DATA.len();
    (0..len).map(|i| DATA[(base + i) % size]).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pool_is_4096_bytes() {
        assert_eq!(DATA.len(), 4096);
    }

    #[test]
    fn slice_wraps_around_end() {
        let s = slice(4095, 3);
        assert_eq!(s, vec![DATA[4095], DATA[0], DATA[1]]);
    }
}
