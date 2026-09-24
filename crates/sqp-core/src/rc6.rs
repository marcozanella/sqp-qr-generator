//! RC6 block cipher (RC6-32/20/b) — key schedule, encrypt, decrypt.
//!
//! All 32-bit arithmetic wraps (`wrapping_*` / `rotate_*`) to match Ruby's
//! `& 0xFFFFFFFF`. Words are packed little-endian, matching Ruby `unpack("V4")`.

const P32: u32 = 0xB7E1_5163;
const Q32: u32 = 0x9E37_79B9;

/// Expand a 16/24/32-byte key into 2*rounds+4 = 44 round-key words (20 rounds).
pub fn key_schedule(key: &[u8]) -> Vec<u32> {
    assert!(
        matches!(key.len(), 16 | 24 | 32),
        "bad key len {}",
        key.len()
    );
    let rounds: usize = 20;
    let c = key.len() / 4;
    let mut l: Vec<u32> = (0..c)
        .map(|i| u32::from_le_bytes([key[4 * i], key[4 * i + 1], key[4 * i + 2], key[4 * i + 3]]))
        .collect();
    let t = 2 * rounds + 4;
    let mut s: Vec<u32> = (0..t)
        .map(|i| P32.wrapping_add((i as u32).wrapping_mul(Q32)))
        .collect();

    let (mut a, mut b) = (0u32, 0u32);
    let (mut i, mut j) = (0usize, 0usize);
    for _ in 0..(3 * t.max(c)) {
        a = s[i].wrapping_add(a).wrapping_add(b).rotate_left(3);
        s[i] = a;
        let ab = a.wrapping_add(b) & 31;
        b = l[j].wrapping_add(a).wrapping_add(b).rotate_left(ab);
        l[j] = b;
        i = (i + 1) % t;
        j = (j + 1) % c;
    }
    s
}

/// Encrypt a single 16-byte block with the given key schedule.
pub fn encrypt_block(pt: &[u8; 16], s: &[u32]) -> [u8; 16] {
    let rounds = (s.len() - 4) / 2;
    let mut a = u32::from_le_bytes([pt[0], pt[1], pt[2], pt[3]]);
    let mut b = u32::from_le_bytes([pt[4], pt[5], pt[6], pt[7]]);
    let mut c = u32::from_le_bytes([pt[8], pt[9], pt[10], pt[11]]);
    let mut d = u32::from_le_bytes([pt[12], pt[13], pt[14], pt[15]]);

    b = b.wrapping_add(s[0]);
    d = d.wrapping_add(s[1]);
    for i in 1..=rounds {
        let t = b.wrapping_mul(b.wrapping_mul(2).wrapping_add(1)).rotate_left(5);
        let u = d.wrapping_mul(d.wrapping_mul(2).wrapping_add(1)).rotate_left(5);
        a = (a ^ t).rotate_left(u & 31).wrapping_add(s[2 * i]);
        c = (c ^ u).rotate_left(t & 31).wrapping_add(s[2 * i + 1]);
        let (na, nb, nc, nd) = (b, c, d, a);
        a = na;
        b = nb;
        c = nc;
        d = nd;
    }
    a = a.wrapping_add(s[2 * rounds + 2]);
    c = c.wrapping_add(s[2 * rounds + 3]);

    let mut out = [0u8; 16];
    out[0..4].copy_from_slice(&a.to_le_bytes());
    out[4..8].copy_from_slice(&b.to_le_bytes());
    out[8..12].copy_from_slice(&c.to_le_bytes());
    out[12..16].copy_from_slice(&d.to_le_bytes());
    out
}

/// Decrypt a single 16-byte block with the given key schedule.
pub fn decrypt_block(ct: &[u8; 16], s: &[u32]) -> [u8; 16] {
    let rounds = (s.len() - 4) / 2;
    let mut a = u32::from_le_bytes([ct[0], ct[1], ct[2], ct[3]]);
    let mut b = u32::from_le_bytes([ct[4], ct[5], ct[6], ct[7]]);
    let mut c = u32::from_le_bytes([ct[8], ct[9], ct[10], ct[11]]);
    let mut d = u32::from_le_bytes([ct[12], ct[13], ct[14], ct[15]]);

    c = c.wrapping_sub(s[2 * rounds + 3]);
    a = a.wrapping_sub(s[2 * rounds + 2]);
    for i in (1..=rounds).rev() {
        let (na, nb, nc, nd) = (d, a, b, c);
        a = na;
        b = nb;
        c = nc;
        d = nd;
        let u = d.wrapping_mul(d.wrapping_mul(2).wrapping_add(1)).rotate_left(5);
        let t = b.wrapping_mul(b.wrapping_mul(2).wrapping_add(1)).rotate_left(5);
        c = (c.wrapping_sub(s[2 * i + 1])).rotate_right(t & 31) ^ u;
        a = (a.wrapping_sub(s[2 * i])).rotate_right(u & 31) ^ t;
    }
    d = d.wrapping_sub(s[1]);
    b = b.wrapping_sub(s[0]);

    let mut out = [0u8; 16];
    out[0..4].copy_from_slice(&a.to_le_bytes());
    out[4..8].copy_from_slice(&b.to_le_bytes());
    out[8..12].copy_from_slice(&c.to_le_bytes());
    out[12..16].copy_from_slice(&d.to_le_bytes());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypt_then_decrypt_is_identity() {
        let key: [u8; 16] = *b"0123456789abcdef";
        let pt: [u8; 16] = *b"FEDCBA9876543210";
        let s = key_schedule(&key);
        assert_eq!(s.len(), 44); // 2*20 + 4
        let ct = encrypt_block(&pt, &s);
        assert_ne!(ct, pt);
        let back = decrypt_block(&ct, &s);
        assert_eq!(back, pt);
    }
}
