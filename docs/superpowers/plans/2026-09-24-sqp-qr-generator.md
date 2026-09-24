# SQP QR Generator Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a native, offline desktop app in Rust that reproduces the `platform01` SQP ink-canister code generator and renders each code as text plus a printable QR.

**Architecture:** A Cargo workspace with two crates — `sqp-core` (a byte-for-byte port of the Ruby `lib/sqp/*` algorithm, no UI) and `sqp-gui` (an egui/eframe desktop front-end). `sqp-core` is built and validated test-first against reference vectors captured from the Ruby encoder before any GUI work begins.

**Tech Stack:** Rust (2021 edition), Cargo workspace; `eframe`/`egui` for GUI; `qrcode` + `image` for QR PNGs; `rfd` for native file dialogs; `arboard` for clipboard; `getrandom`/`rand` for RNG; `chrono` for dates.

**Spec:** `docs/superpowers/specs/2026-09-24-sqp-qr-generator-design.md`

## Global Constraints

- **Byte-identical output:** `sqp-core::encode` MUST produce the exact string the Ruby `Sqp::ProductCode.encode` produces for the same fields. Verified against committed reference vectors.
- **All 32-bit arithmetic wraps:** every RC6 `u32` op uses `wrapping_add`/`wrapping_mul`/`rotate_left`/`rotate_right` to match Ruby's `& 0xFFFFFFFF`.
- **RFE32 alphabet (exact, order matters):** `ABCDEFGHJKLMNPQRSTUVWXYZ23456789` (A–Z minus I,O + 2–9; index 0 = 'A').
- **Byte pool:** the 4096-byte `bytepool.bin` from `platform01/lib/sqp/bytepool.bin`, copied into `crates/sqp-core/assets/bytepool.bin` and embedded with `include_bytes!`.
- **Constants copied verbatim from Ruby:** RC6 `P32=0xB7E15163`, `Q32=0x9E3779B9`, rounds=20; year encode `year - 2007` for preview and `(year - 2007 + 0x2CC) & 0x3FF` for plaintext; PREVIEW_INDICES `[4,9,14,19]`; key-derivation multipliers `0x111DF`, `0x165B9`, `0x185E1` and slice lengths `7,19,6`.
- **Maps:** color→id `{Black:1,Cyan:2,Magenta:3,Yellow:4,Light Cyan:5,Light Magenta:6,White:7,Clear:8,Light Black:10}`; volume→id `{5:20,1:8}`; tara-by-volume `{5:[18,19,20,21,22],1:[7,8,9]}`; ink_type_id: `(KX2,!Clear)=19`, `(SQSG3,!Clear)=21`, `(KX2,Clear)=8`, `(SQSG3,Clear)=refused`.
- **Batch format:** `^[A-Z0-9]{9}$`; `auto_batch(date) = date.strftime("UB%y%jF1")` (e.g. 2026-06-17 → `UB26168F1`).
- **Count range:** 1..=20. **Number range:** 0..=0xFFFF. **QR:** error-correction level L, zero border.
- **No `panic!` on user input** in `sqp-gui`; all fallible core calls return `Result<_, SqpError>`.

### Reference vectors (captured from the Ruby encoder, pinned number+tara)

| ink | color | vol | month/year | batch | number | tara | expected code |
|---|---|---|---|---|---|---|---|
| KX2 | Black | 5 | 5/2027 | UB26082F1 | 34291 | 20 | `2XEL-58NW-D4PC-L34B-5E3M-8E6F-LYWJ-LCML` |
| KX2 | Cyan | 5 | 5/2027 | UB26103F1 | 1 | 18 | `8QSS-N7GW-D9RD-5FDC-6MTA-TGAF-3ZWT-A8DL` |
| SQSG3 | Magenta | 1 | 12/2028 | UB17483F1 | 65535 | 7 | `AKZZ-JK7J-G3AE-U27D-29XY-SG76-RVCK-KT8L` |
| KX2 | Clear | 1 | 1/2030 | AAAAAAAAA | 0 | 9 | `PZCT-7VDJ-ASEY-99AJ-HHCE-UYDT-P9MJ-P2CM` |
| KX2 | Yellow | 5 | 5/2027 | UB26168F1 | 39920 | 22 | `P6VR-43AW-V8LR-ZQ2E-N2EJ-2W6F-A2J7-Z4ZL` |

Each expected code, when decoded, has all cross-checks true (`valid == true`).

---

## File Structure

```
SQP-QR-generator/
├── Cargo.toml                         # [workspace] members
├── Cargo.lock                         # committed (this is an app)
├── .gitignore                         # /target only
├── README.md
├── crates/
│   ├── sqp-core/
│   │   ├── Cargo.toml
│   │   ├── assets/bytepool.bin        # 4096 bytes, embedded
│   │   ├── src/
│   │   │   ├── lib.rs                 # re-exports; SqpError; public API
│   │   │   ├── rfe32.rs               # base-32 codec (20 bytes <-> 32 chars)
│   │   │   ├── rc6.rs                 # RC6 cipher + key schedule
│   │   │   ├── bytepool.rs            # include_bytes! + slice()
│   │   │   ├── codec.rs               # preview/plaintext pack+parse, encode, decode
│   │   │   ├── maps.rs                # color/volume/tara/ink_type maps
│   │   │   └── generate.rs            # generate(), auto_batch()
│   │   └── tests/vectors.rs           # reference-vector + round-trip tests
│   └── sqp-gui/
│       ├── Cargo.toml
│       └── src/main.rs                # egui app: form, results, save/export
└── docs/superpowers/{specs,plans}/...
```

---

## Task 1: Workspace scaffold + embedded byte pool

**Files:**
- Create: `Cargo.toml` (workspace), `.gitignore`, `crates/sqp-core/Cargo.toml`, `crates/sqp-core/src/lib.rs`, `crates/sqp-core/src/bytepool.rs`
- Create (copy): `crates/sqp-core/assets/bytepool.bin`
- Test: in `crates/sqp-core/src/bytepool.rs` (`#[cfg(test)]`)

**Interfaces:**
- Produces: `sqp_core::bytepool::DATA: &[u8]` (len 4096); `sqp_core::bytepool::slice(base: usize, len: usize) -> Vec<u8>` (wraps modulo 4096).

- [ ] **Step 1: Copy the byte pool into the repo**

Run (copy only — read-only w.r.t. platform01):
```bash
mkdir -p crates/sqp-core/assets
cp /Users/marcozanella/Documents/GitHub/platform01/lib/sqp/bytepool.bin crates/sqp-core/assets/bytepool.bin
wc -c crates/sqp-core/assets/bytepool.bin   # must print 4096
```

- [ ] **Step 2: Write workspace + crate manifests and .gitignore**

`Cargo.toml`:
```toml
[workspace]
members = ["crates/sqp-core", "crates/sqp-gui"]
resolver = "2"
```
`.gitignore`:
```
/target
```
`crates/sqp-core/Cargo.toml`:
```toml
[package]
name = "sqp-core"
version = "0.1.0"
edition = "2021"

[dependencies]
rand = "0.8"
chrono = { version = "0.4", default-features = false, features = ["clock"] }
```

- [ ] **Step 3: Write the failing test for the byte pool**

`crates/sqp-core/src/bytepool.rs`:
```rust
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
```
`crates/sqp-core/src/lib.rs`:
```rust
pub mod bytepool;
```

- [ ] **Step 4: Run tests, expect PASS (pool present) — this also proves the file copied correctly**

Run: `cargo test -p sqp-core --lib bytepool`
Expected: 2 passed. (If `pool_is_4096_bytes` fails, the copy in Step 1 was wrong.)

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat: workspace scaffold + embedded 4096-byte SQP byte pool

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 2: RFE32 base-32 codec

**Files:**
- Create: `crates/sqp-core/src/rfe32.rs`
- Modify: `crates/sqp-core/src/lib.rs` (add `pub mod rfe32;`)
- Test: in `rfe32.rs`

**Interfaces:**
- Produces:
  - `sqp_core::rfe32::decode(s: &str) -> Result<[u8; 20], SqpError>` (32 chars → 20 bytes, big-endian)
  - `sqp_core::rfe32::encode(bytes: &[u8; 20]) -> String` (20 bytes → 32 chars)
- Consumes: `SqpError` (define a minimal version now in `lib.rs`; Task 7 extends it).

- [ ] **Step 1: Add a minimal error enum to `lib.rs`**

```rust
pub mod bytepool;
pub mod rfe32;

#[derive(Debug, PartialEq, Eq)]
pub enum SqpError {
    BadLength(usize),
    CharNotInAlphabet(char),
}
```

- [ ] **Step 2: Write the failing round-trip + known-vector test**

`crates/sqp-core/src/rfe32.rs`:
```rust
use crate::SqpError;

const ALPHABET: &[u8; 32] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_all_bytes() {
        let bytes: [u8; 20] = [
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9,
            250, 251, 252, 253, 254, 255, 128, 64, 32, 16,
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
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cargo test -p sqp-core --lib rfe32`
Expected: FAIL — `encode`/`decode` not found.

- [ ] **Step 4: Implement `encode`/`decode`**

Add above the tests in `rfe32.rs`:
```rust
fn val_of(c: char) -> Option<u8> {
    ALPHABET.iter().position(|&b| b as char == c).map(|i| i as u8)
}

/// 32 chars * 5 bits = 160 bits = 20 bytes, big-endian.
pub fn decode(s: &str) -> Result<[u8; 20], SqpError> {
    if s.chars().count() != 32 {
        return Err(SqpError::BadLength(s.chars().count()));
    }
    let mut n: u128 = 0; // low 32 bits held separately below
    let mut hi: u128 = 0; // we need 160 bits; use two u128 halves
    // Simpler: accumulate into a 20-byte big-endian buffer via bit pushes.
    let mut bits: Vec<u8> = Vec::with_capacity(160);
    for c in s.chars() {
        let v = val_of(c).ok_or(SqpError::CharNotInAlphabet(c))?;
        for shift in (0..5).rev() {
            bits.push((v >> shift) & 1);
        }
    }
    let _ = (n, hi);
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

/// 20 bytes = 160 bits -> 32 chars.
pub fn encode(bytes: &[u8; 20]) -> String {
    let mut bits: Vec<u8> = Vec::with_capacity(160);
    for &b in bytes.iter() {
        for shift in (0..8).rev() {
            bits.push((b >> shift) & 1);
        }
    }
    bits
        .chunks(5)
        .map(|chunk| {
            let mut v = 0u8;
            for &bit in chunk {
                v = (v << 1) | bit;
            }
            ALPHABET[v as usize] as char
        })
        .collect()
}
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cargo test -p sqp-core --lib rfe32`
Expected: 3 passed.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "feat: RFE32 base-32 codec with round-trip tests

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 3: RC6 cipher

**Files:**
- Create: `crates/sqp-core/src/rc6.rs`
- Modify: `crates/sqp-core/src/lib.rs` (add `pub mod rc6;`)
- Test: in `rc6.rs`

**Interfaces:**
- Produces:
  - `sqp_core::rc6::key_schedule(key: &[u8]) -> Vec<u32>` (key len 16/24/32; 20 rounds → 44 words)
  - `sqp_core::rc6::encrypt_block(pt: &[u8; 16], s: &[u32]) -> [u8; 16]`
  - `sqp_core::rc6::decrypt_block(ct: &[u8; 16], s: &[u32]) -> [u8; 16]`
- Word packing is **little-endian** (`u32::from_le_bytes` / `to_le_bytes`), matching Ruby `unpack("V4")`.

- [ ] **Step 1: Write the failing encrypt/decrypt round-trip test**

`crates/sqp-core/src/rc6.rs`:
```rust
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p sqp-core --lib rc6`
Expected: FAIL — functions not found.

- [ ] **Step 3: Implement RC6 (all ops wrapping; LE word packing)**

Add above the tests in `rc6.rs`:
```rust
const P32: u32 = 0xB7E1_5163;
const Q32: u32 = 0x9E37_79B9;

pub fn key_schedule(key: &[u8]) -> Vec<u32> {
    assert!(matches!(key.len(), 16 | 24 | 32), "bad key len {}", key.len());
    let rounds: usize = 20;
    let c = key.len() / 4;
    let mut l: Vec<u32> = (0..c)
        .map(|i| u32::from_le_bytes([key[4*i], key[4*i+1], key[4*i+2], key[4*i+3]]))
        .collect();
    let t = 2 * rounds + 4;
    let mut s: Vec<u32> = (0..t).map(|i| P32.wrapping_add((i as u32).wrapping_mul(Q32))).collect();

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
        a = (a ^ t).rotate_left(u & 31).wrapping_add(s[2*i]);
        c = (c ^ u).rotate_left(t & 31).wrapping_add(s[2*i+1]);
        let (na, nb, nc, nd) = (b, c, d, a);
        a = na; b = nb; c = nc; d = nd;
    }
    a = a.wrapping_add(s[2*rounds+2]);
    c = c.wrapping_add(s[2*rounds+3]);

    let mut out = [0u8; 16];
    out[0..4].copy_from_slice(&a.to_le_bytes());
    out[4..8].copy_from_slice(&b.to_le_bytes());
    out[8..12].copy_from_slice(&c.to_le_bytes());
    out[12..16].copy_from_slice(&d.to_le_bytes());
    out
}

pub fn decrypt_block(ct: &[u8; 16], s: &[u32]) -> [u8; 16] {
    let rounds = (s.len() - 4) / 2;
    let mut a = u32::from_le_bytes([ct[0], ct[1], ct[2], ct[3]]);
    let mut b = u32::from_le_bytes([ct[4], ct[5], ct[6], ct[7]]);
    let mut c = u32::from_le_bytes([ct[8], ct[9], ct[10], ct[11]]);
    let mut d = u32::from_le_bytes([ct[12], ct[13], ct[14], ct[15]]);

    c = c.wrapping_sub(s[2*rounds+3]);
    a = a.wrapping_sub(s[2*rounds+2]);
    for i in (1..=rounds).rev() {
        let (na, nb, nc, nd) = (d, a, b, c);
        a = na; b = nb; c = nc; d = nd;
        let u = d.wrapping_mul(d.wrapping_mul(2).wrapping_add(1)).rotate_left(5);
        let t = b.wrapping_mul(b.wrapping_mul(2).wrapping_add(1)).rotate_left(5);
        c = c.wrapping_sub(s[2*i+1]).rotate_right(t & 31) ^ u;
        a = a.wrapping_sub(s[2*i]).rotate_right(u & 31) ^ t;
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
```
Add `pub mod rc6;` to `lib.rs`.

Note: Ruby rotates by `u` (a full u32) and relies on `n &= 31` inside `rotl32`. Rust's `rotate_left` takes the count modulo 32 automatically, but we pass `u & 31` explicitly to make the parity with Ruby obvious and correct.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p sqp-core --lib rc6`
Expected: 1 passed.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat: RC6 cipher (key schedule, encrypt, decrypt) with round-trip test

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 4: Maps (color / volume / tara / ink type)

**Files:**
- Create: `crates/sqp-core/src/maps.rs`
- Modify: `crates/sqp-core/src/lib.rs` (add `pub mod maps;`, extend `SqpError`)
- Test: in `maps.rs`

**Interfaces:**
- Produces:
  - `sqp_core::maps::InkType` enum `{ Kx2, Sqsg3 }`
  - `color_id(name: &str) -> Result<u8, SqpError>`
  - `volume_id(vol_l: u8) -> Result<u8, SqpError>`
  - `tara_options(vol_l: u8) -> Result<&'static [u8], SqpError>`
  - `derive_ink_type_id(ink: InkType, color_name: &str) -> Result<u8, SqpError>`
  - `SUPPORTED_COLOR_NAMES: [&str; 9]`
- Consumes: `SqpError` from `lib.rs`.

- [ ] **Step 1: Extend `SqpError` in `lib.rs`**

Replace the enum with:
```rust
#[derive(Debug, PartialEq, Eq)]
pub enum SqpError {
    BadLength(usize),
    CharNotInAlphabet(char),
    UnknownColor(String),
    UnknownVolume(u8),
    UnknownInkType(String),
    InvalidCombo(String),
}
```
And add `pub mod maps;`.

- [ ] **Step 2: Write the failing test**

`crates/sqp-core/src/maps.rs`:
```rust
use crate::SqpError;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_ids_match_ruby() {
        assert_eq!(color_id("Black").unwrap(), 1);
        assert_eq!(color_id("Light Black").unwrap(), 10);
        assert_eq!(color_id("Clear").unwrap(), 8);
        assert_eq!(color_id("Puce"), Err(SqpError::UnknownColor("Puce".into())));
    }

    #[test]
    fn volume_and_tara() {
        assert_eq!(volume_id(5).unwrap(), 20);
        assert_eq!(volume_id(1).unwrap(), 8);
        assert_eq!(volume_id(3), Err(SqpError::UnknownVolume(3)));
        assert_eq!(tara_options(5).unwrap(), &[18, 19, 20, 21, 22]);
        assert_eq!(tara_options(1).unwrap(), &[7, 8, 9]);
    }

    #[test]
    fn ink_type_id_rules() {
        assert_eq!(derive_ink_type_id(InkType::Kx2, "Cyan").unwrap(), 19);
        assert_eq!(derive_ink_type_id(InkType::Sqsg3, "Cyan").unwrap(), 21);
        assert_eq!(derive_ink_type_id(InkType::Kx2, "Clear").unwrap(), 8);
        assert!(matches!(
            derive_ink_type_id(InkType::Sqsg3, "Clear"),
            Err(SqpError::InvalidCombo(_))
        ));
    }
}
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cargo test -p sqp-core --lib maps`
Expected: FAIL — items not found.

- [ ] **Step 4: Implement the maps**

Add above the tests in `maps.rs`:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InkType { Kx2, Sqsg3 }

pub const SUPPORTED_COLOR_NAMES: [&str; 9] = [
    "Black", "Cyan", "Magenta", "Yellow",
    "Light Cyan", "Light Magenta", "White", "Clear", "Light Black",
];

pub fn color_id(name: &str) -> Result<u8, SqpError> {
    let id = match name {
        "Black" => 1, "Cyan" => 2, "Magenta" => 3, "Yellow" => 4,
        "Light Cyan" => 5, "Light Magenta" => 6, "White" => 7,
        "Clear" => 8, "Light Black" => 10,
        _ => return Err(SqpError::UnknownColor(name.to_string())),
    };
    Ok(id)
}

pub fn volume_id(vol_l: u8) -> Result<u8, SqpError> {
    match vol_l {
        5 => Ok(20),
        1 => Ok(8),
        other => Err(SqpError::UnknownVolume(other)),
    }
}

pub fn tara_options(vol_l: u8) -> Result<&'static [u8], SqpError> {
    match vol_l {
        5 => Ok(&[18, 19, 20, 21, 22]),
        1 => Ok(&[7, 8, 9]),
        other => Err(SqpError::UnknownVolume(other)),
    }
}

pub fn derive_ink_type_id(ink: InkType, color_name: &str) -> Result<u8, SqpError> {
    let is_clear = color_name == "Clear";
    match ink {
        InkType::Kx2 => Ok(if is_clear { 8 } else { 19 }),
        InkType::Sqsg3 => {
            if is_clear {
                Err(SqpError::InvalidCombo("SQSG3 Clear is not a known SKU".into()))
            } else {
                Ok(21)
            }
        }
    }
}
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cargo test -p sqp-core --lib maps`
Expected: 3 passed.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "feat: SQP color/volume/tara/ink-type maps

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 5: Codec — preview/plaintext packing, key derivation, encode/decode

**Files:**
- Create: `crates/sqp-core/src/codec.rs`
- Modify: `crates/sqp-core/src/lib.rs` (add `pub mod codec;`, extend `SqpError` with `DecodeError(String)`)
- Test: in `codec.rs` (unit) — the full reference-vector test lives in Task 6.

**Interfaces:**
- Produces:
  - `pub struct Fields { pub volume: u8, pub color: u8, pub number: u16, pub month: u8, pub year: i32, pub ink_type_id: u8, pub legacy_tara_raw: u8, pub batch: String }`
  - `sqp_core::codec::encode(f: &Fields) -> String` (returns dashed code)
  - `sqp_core::codec::decode(code: &str) -> Result<DecodeResult, SqpError>` where `DecodeResult { pub fields: Fields, pub valid: bool }`
- Consumes: `rfe32`, `rc6`, `bytepool`.

Bit layout is copied verbatim from `Sqp::ProductCode`. The key derivation and index maps are:
- `PREVIEW_INDICES = [4, 9, 14, 19]`, `CIPHERTEXT_INDICES = (0..=19) \ PREVIEW_INDICES` (16 values).
- preview bytes are read/written in the order `[rev[19], rev[14], rev[9], rev[4]]`.
- key = `slice(base0,7) ++ slice(base1,19) ++ slice(base2,6)` (total 32 bytes) with bases as in Global Constraints.

- [ ] **Step 1: Extend `SqpError` and add module**

In `lib.rs` add variant `DecodeError(String),` to `SqpError` and `pub mod codec;`.

- [ ] **Step 2: Write the failing unit tests (pack/parse inverse + one full round-trip)**

`crates/sqp-core/src/codec.rs`:
```rust
use crate::{bytepool, rc6, rfe32, SqpError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fields {
    pub volume: u8,
    pub color: u8,
    pub number: u16,
    pub month: u8,
    pub year: i32,
    pub ink_type_id: u8,
    pub legacy_tara_raw: u8,
    pub batch: String,
}

#[derive(Debug)]
pub struct DecodeResult {
    pub fields: Fields,
    pub valid: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Fields {
        Fields {
            volume: 20, color: 1, number: 34291, month: 5, year: 2027,
            ink_type_id: 19, legacy_tara_raw: 20, batch: "UB26082F1".into(),
        }
    }

    #[test]
    fn encode_matches_reference_vector() {
        assert_eq!(encode(&sample()), "2XEL-58NW-D4PC-L34B-5E3M-8E6F-LYWJ-LCML");
    }

    #[test]
    fn decode_roundtrips_and_is_valid() {
        let code = encode(&sample());
        let r = decode(&code).unwrap();
        assert!(r.valid);
        assert_eq!(r.fields.volume, 20);
        assert_eq!(r.fields.color, 1);
        assert_eq!(r.fields.number, 34291);
        assert_eq!(r.fields.month, 5);
        assert_eq!(r.fields.year, 2027);
        assert_eq!(r.fields.batch, "UB26082F1");
    }
}
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cargo test -p sqp-core --lib codec`
Expected: FAIL — `encode`/`decode` not found.

- [ ] **Step 4: Implement the codec**

Add above the tests in `codec.rs`:
```rust
const PREVIEW_INDICES: [usize; 4] = [4, 9, 14, 19];

fn ciphertext_indices() -> Vec<usize> {
    (0..20).filter(|i| !PREVIEW_INDICES.contains(i)).collect()
}

// --- preview (4 unencrypted bytes) ---
struct Preview { number: u16, year: u8, month: u8, color: u8, volume: u8 }

fn parse_preview(b: &[u8; 4]) -> Preview {
    let (b0, b1, b2, b3) = (b[0] as u16, b[1] as u16, b[2] as u16, b[3] as u16);
    let number = (((b0 & 7) << 3 | (b1 & 7)) << 3 | (b2 & 7)) << 3 | (b3 & 7);
    let year = (((b[0] >> 3) << 1) | (b[1] >> 7)) as u8;
    let month = ((b[1] >> 3) & 0xF) as u8;
    let color = (b[2] >> 3) as u8;
    let volume = (b[3] >> 3) as u8;
    Preview { number, year, month, color, volume }
}

fn pack_preview(number12: u16, year6: u8, month4: u8, color5: u8, volume5: u8) -> [u8; 4] {
    let number = (number12 & 0xFFF) as u32;
    let year = (year6 & 0x3F) as u32;
    let month = (month4 & 0x0F) as u32;
    let color = (color5 & 0x1F) as u32;
    let volume = (volume5 & 0x1F) as u32;
    let b0 = ((number >> 9) & 7) | (((year >> 1) & 0x1F) << 3);
    let b1 = ((number >> 6) & 7) | ((month & 0xF) << 3) | ((year & 1) << 7);
    let b2 = ((number >> 3) & 7) | ((color & 0x1F) << 3);
    let b3 = (number & 7) | ((volume & 0x1F) << 3);
    [b0 as u8, b1 as u8, b2 as u8, b3 as u8]
}

// --- key derivation ---
fn derive_key(preview_bytes: &[u8; 4]) -> Vec<u8> {
    let pv = parse_preview(preview_bytes);
    let b0 = (pv.number & 0xFF) as u32;
    let i4 = pv.year as u32;
    let i8v = pv.month as u32;
    let b12 = pv.color as u32;
    let b16 = pv.volume as u32;
    let size = bytepool::DATA.len() as u32;
    let base0 = ((((b16 & 0x1F) << 3) | (b0 & 7)).wrapping_mul(0x111DF)) % size;
    let base1 = ((((b0 >> 3) & 7) | ((b12 & 0x1F) << 3)).wrapping_mul(0x165B9)) % size;
    let base2 = ((i8v | (i4 << 4)).wrapping_mul(0x185E1)) % size;
    let mut key = bytepool::slice(base0 as usize, 7);
    key.extend(bytepool::slice(base1 as usize, 19));
    key.extend(bytepool::slice(base2 as usize, 6));
    key
}

// --- plaintext (16 decrypted bytes) ---
fn parse_plaintext(p: &[u8; 16]) -> Fields {
    let c: Vec<u32> = p.iter().map(|&x| x as u32).collect();
    let volume = (c[0] & 0x1F) as u8;
    let color = ((c[0] >> 5) | ((c[1] & 0x3) << 3)) as u8;
    let number = ((c[1] >> 2) | (c[2] << 6) | ((c[3] & 0x3) << 14)) as u32;
    let month = ((c[3] >> 2) & 0xF) as u8;
    let year_raw = (((c[3] >> 6) | (c[4] << 2)) as i32) - 0x2CC;
    let ink_type_id = p[5];
    let legacy_tara_raw = p[6];
    let batch: String = p[7..16].iter().cloned().filter(|&b| b != 0)
        .map(|b| b as char).collect();
    Fields {
        volume, color, number: (number & 0xFFFF) as u16, month,
        year: year_raw + 0x7D7, ink_type_id, legacy_tara_raw, batch,
    }
}

fn pack_plaintext(f: &Fields) -> [u8; 16] {
    let volume = (f.volume & 0x1F) as u32;
    let color = (f.color & 0x1F) as u32;
    let number = (f.number as u32) & 0xFFFF;
    let month = (f.month & 0x0F) as u32;
    let year_bits = ((f.year - 2007 + 0x2CC) & 0x3FF) as u32;
    let mut out = [0u8; 16];
    out[0] = ((volume & 0x1F) | ((color & 0x7) << 5)) as u8;
    out[1] = (((color >> 3) & 0x3) | ((number & 0x3F) << 2)) as u8;
    out[2] = ((number >> 6) & 0xFF) as u8;
    out[3] = (((number >> 14) & 0x3) | ((month & 0xF) << 2) | ((year_bits & 0x3) << 6)) as u8;
    out[4] = ((year_bits >> 2) & 0xFF) as u8;
    out[5] = f.ink_type_id;
    out[6] = f.legacy_tara_raw;
    let batch = f.batch.as_bytes();
    for (i, &b) in batch.iter().take(9).enumerate() {
        out[7 + i] = b;
    }
    out
}

// --- top level ---
pub fn encode(f: &Fields) -> String {
    let preview_bytes = pack_preview(
        (f.number & 0xFFF) as u16,
        (f.year - 2007) as u8,
        f.month, f.color, f.volume,
    );
    let key = derive_key(&preview_bytes);
    let mut key_arr = [0u8; 32];
    key_arr.copy_from_slice(&key);
    let schedule = rc6::key_schedule(&key_arr);
    let plaintext = pack_plaintext(f);
    let ciphertext = rc6::encrypt_block(&plaintext, &schedule);

    let mut rev = [0u8; 20];
    // PREVIEW_INDICES.reverse.each_with_index { |idx,i| rev[idx] = preview[i] }
    for (i, &idx) in PREVIEW_INDICES.iter().rev().enumerate() {
        rev[idx] = preview_bytes[i];
    }
    for (i, &idx) in ciphertext_indices().iter().enumerate() {
        rev[idx] = ciphertext[i];
    }
    // rfe_in = rev.reverse ; encode ; reverse back
    let mut rfe_in = rev;
    rfe_in.reverse();
    let encoded_reversed = rfe32::encode(&rfe_in);
    let encoded: String = encoded_reversed.chars().rev().collect();
    // dash into groups of 4
    encoded
        .as_bytes()
        .chunks(4)
        .map(|c| std::str::from_utf8(c).unwrap())
        .collect::<Vec<_>>()
        .join("-")
}

pub fn decode(code: &str) -> Result<DecodeResult, SqpError> {
    let raw: String = code.chars().filter(|&c| c != '-').collect();
    if raw.len() != 32 {
        return Err(SqpError::DecodeError(format!("length {} != 32", raw.len())));
    }
    let rev_str: String = raw.chars().rev().collect();
    let rfe_out = rfe32::decode(&rev_str)?;
    let mut rev = rfe_out;
    rev.reverse();

    let preview_bytes = [rev[19], rev[14], rev[9], rev[4]];
    let preview = parse_preview(&preview_bytes);

    let ct_idx = ciphertext_indices();
    let mut ct = [0u8; 16];
    for (i, &idx) in ct_idx.iter().enumerate() {
        ct[i] = rev[idx];
    }
    let key = derive_key(&preview_bytes);
    let mut key_arr = [0u8; 32];
    key_arr.copy_from_slice(&key);
    let schedule = rc6::key_schedule(&key_arr);
    let pt = rc6::decrypt_block(&ct, &schedule);
    let fields = parse_plaintext(&pt);

    let year_raw = fields.year - 0x7D7;
    let valid = preview.volume == fields.volume
        && preview.color == fields.color
        && preview.month == fields.month
        && (preview.year as i32) == year_raw
        && preview.number == (fields.number & 0xFFF);

    Ok(DecodeResult { fields, valid })
}
```
Add `pub mod codec;` to `lib.rs`.

- [ ] **Step 5: Run test to verify it passes**

Run: `cargo test -p sqp-core --lib codec`
Expected: 2 passed. (If `encode_matches_reference_vector` fails, a bit layout or endianness diverged from Ruby — do not proceed until it matches.)

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "feat: SQP codec (preview/plaintext packing, key derivation, encode/decode)

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 6: Reference-vector suite (all captured vectors + validity)

**Files:**
- Create: `crates/sqp-core/tests/vectors.rs`
- Test: this file (integration test)

**Interfaces:**
- Consumes: `sqp_core::codec::{Fields, encode, decode}`, `sqp_core::maps`.

- [ ] **Step 1: Write the failing integration test with all five vectors**

`crates/sqp-core/tests/vectors.rs`:
```rust
use sqp_core::codec::{decode, encode, Fields};
use sqp_core::maps::{color_id, derive_ink_type_id, volume_id, InkType};

fn fields(ink: InkType, color: &str, vol: u8, month: u8, year: i32,
          batch: &str, number: u16, tara: u8) -> Fields {
    Fields {
        volume: volume_id(vol).unwrap(),
        color: color_id(color).unwrap(),
        number,
        month,
        year,
        ink_type_id: derive_ink_type_id(ink, color).unwrap(),
        legacy_tara_raw: tara,
        batch: batch.to_string(),
    }
}

#[test]
fn all_reference_vectors_encode_exactly() {
    let cases = [
        (fields(InkType::Kx2, "Black", 5, 5, 2027, "UB26082F1", 34291, 20),
         "2XEL-58NW-D4PC-L34B-5E3M-8E6F-LYWJ-LCML"),
        (fields(InkType::Kx2, "Cyan", 5, 5, 2027, "UB26103F1", 1, 18),
         "8QSS-N7GW-D9RD-5FDC-6MTA-TGAF-3ZWT-A8DL"),
        (fields(InkType::Sqsg3, "Magenta", 1, 12, 2028, "UB17483F1", 65535, 7),
         "AKZZ-JK7J-G3AE-U27D-29XY-SG76-RVCK-KT8L"),
        (fields(InkType::Kx2, "Clear", 1, 1, 2030, "AAAAAAAAA", 0, 9),
         "PZCT-7VDJ-ASEY-99AJ-HHCE-UYDT-P9MJ-P2CM"),
        (fields(InkType::Kx2, "Yellow", 5, 5, 2027, "UB26168F1", 39920, 22),
         "P6VR-43AW-V8LR-ZQ2E-N2EJ-2W6F-A2J7-Z4ZL"),
    ];
    for (f, expected) in cases {
        let code = encode(&f);
        assert_eq!(code, expected, "encode mismatch for {expected}");
        let r = decode(&code).unwrap();
        assert!(r.valid, "decode not valid for {expected}");
    }
}
```

- [ ] **Step 2: Run test to verify it passes** (encode already implemented in Task 5)

Run: `cargo test -p sqp-core --test vectors`
Expected: 1 passed. (If any vector mismatches, the codec has a residual bug — fix in `codec.rs` before continuing.)

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "test: full SQP reference-vector suite (encode-exact + decode-valid)

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 7: Public `generate()` + `auto_batch()`

**Files:**
- Create: `crates/sqp-core/src/generate.rs`
- Modify: `crates/sqp-core/src/lib.rs` (add `pub mod generate;`, re-exports, extend `SqpError` with `BadBatch(String)`, `EncoderRegression(String)`, `CollisionExhausted`)
- Test: in `generate.rs`

**Interfaces:**
- Produces:
  - `pub struct GenerateRequest { pub ink_type: InkType, pub color: String, pub volume_l: u8, pub expires_year: i32, pub expires_month: u8, pub batch: String, pub count: u8 }`
  - `pub struct GeneratedCode { pub code: String, pub code_undashed: String, pub number: u16, pub legacy_tara_raw: u8 }`
  - `sqp_core::generate::generate(req: &GenerateRequest, already: &mut std::collections::HashSet<String>) -> Result<Vec<GeneratedCode>, SqpError>`
  - `sqp_core::generate::auto_batch(date: chrono::NaiveDate) -> String`
- Consumes: `codec`, `maps`.

- [ ] **Step 1: Extend `SqpError` and add re-exports in `lib.rs`**

Add variants `BadBatch(String)`, `EncoderRegression(String)`, `CollisionExhausted` to `SqpError`; add `pub mod generate;` and:
```rust
pub use codec::{decode, encode, DecodeResult, Fields};
pub use generate::{auto_batch, generate, GenerateRequest, GeneratedCode};
pub use maps::{InkType, SUPPORTED_COLOR_NAMES};
```

- [ ] **Step 2: Write the failing tests**

`crates/sqp-core/src/generate.rs`:
```rust
use crate::codec::{decode, encode, Fields};
use crate::maps::{color_id, derive_ink_type_id, tara_options, volume_id, InkType};
use crate::SqpError;
use chrono::{Datelike, NaiveDate};
use rand::seq::SliceRandom;
use rand::Rng;
use std::collections::HashSet;

const MAX_DUP_RETRIES: usize = 20;
const BATCH_LEN: usize = 9;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_batch_matches_ruby_strftime() {
        let d = NaiveDate::from_ymd_opt(2026, 6, 17).unwrap();
        assert_eq!(auto_batch(d), "UB26168F1");
    }

    #[test]
    fn generate_produces_requested_count_of_valid_codes() {
        let req = GenerateRequest {
            ink_type: InkType::Kx2, color: "Cyan".into(), volume_l: 5,
            expires_year: 2027, expires_month: 5, batch: "UB26103F1".into(), count: 3,
        };
        let mut seen = HashSet::new();
        let out = generate(&req, &mut seen).unwrap();
        assert_eq!(out.len(), 3);
        for g in &out {
            assert_eq!(g.code_undashed, g.code.replace('-', ""));
            let r = decode(&g.code).unwrap();
            assert!(r.valid);
        }
        // all unique and recorded
        assert_eq!(seen.len(), 3);
    }

    #[test]
    fn generate_rejects_bad_batch() {
        let req = GenerateRequest {
            ink_type: InkType::Kx2, color: "Cyan".into(), volume_l: 5,
            expires_year: 2027, expires_month: 5, batch: "bad".into(), count: 1,
        };
        let mut seen = HashSet::new();
        assert!(matches!(generate(&req, &mut seen), Err(SqpError::BadBatch(_))));
    }

    #[test]
    fn generate_rejects_sqsg3_clear() {
        let req = GenerateRequest {
            ink_type: InkType::Sqsg3, color: "Clear".into(), volume_l: 5,
            expires_year: 2027, expires_month: 5, batch: "UB26103F1".into(), count: 1,
        };
        let mut seen = HashSet::new();
        assert!(matches!(generate(&req, &mut seen), Err(SqpError::InvalidCombo(_))));
    }
}
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cargo test -p sqp-core --lib generate`
Expected: FAIL — items not found.

- [ ] **Step 4: Implement `generate` + `auto_batch`**

Add above the tests in `generate.rs`:
```rust
#[derive(Debug, Clone)]
pub struct GenerateRequest {
    pub ink_type: InkType,
    pub color: String,
    pub volume_l: u8,
    pub expires_year: i32,
    pub expires_month: u8,
    pub batch: String,
    pub count: u8,
}

#[derive(Debug, Clone)]
pub struct GeneratedCode {
    pub code: String,
    pub code_undashed: String,
    pub number: u16,
    pub legacy_tara_raw: u8,
}

pub fn auto_batch(date: NaiveDate) -> String {
    // Ruby: date.strftime("UB%y%jF1") -> "UB" + 2-digit year + 3-digit day-of-year + "F1"
    format!("UB{:02}{:03}F1", date.year() % 100, date.ordinal())
}

fn valid_batch(b: &str) -> bool {
    b.len() == BATCH_LEN && b.bytes().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
}

pub fn generate(
    req: &GenerateRequest,
    already: &mut HashSet<String>,
) -> Result<Vec<GeneratedCode>, SqpError> {
    if !valid_batch(&req.batch) {
        return Err(SqpError::BadBatch(req.batch.clone()));
    }
    // derive_ink_type_id enforces SQSG3+Clear rejection and unknown ink/color.
    let ink_type_id = derive_ink_type_id(req.ink_type, &req.color)?;
    let color = color_id(&req.color)?;
    let volume = volume_id(req.volume_l)?;
    let tara_pool = tara_options(req.volume_l)?;

    let mut rng = rand::thread_rng();
    let mut out = Vec::with_capacity(req.count as usize);

    for _ in 0..req.count {
        let mut made: Option<GeneratedCode> = None;
        for _ in 0..MAX_DUP_RETRIES {
            let number: u16 = rng.gen();
            let legacy_tara_raw = *tara_pool.choose(&mut rng).unwrap();
            let fields = Fields {
                volume, color, number, month: req.expires_month,
                year: req.expires_year, ink_type_id, legacy_tara_raw,
                batch: req.batch.clone(),
            };
            let code = encode(&fields);
            let check = decode(&code)?;
            if !check.valid {
                return Err(SqpError::EncoderRegression(code));
            }
            if !already.contains(&code) {
                already.insert(code.clone());
                made = Some(GeneratedCode {
                    code_undashed: code.replace('-', ""),
                    code,
                    number,
                    legacy_tara_raw,
                });
                break;
            }
        }
        match made {
            Some(g) => out.push(g),
            None => return Err(SqpError::CollisionExhausted),
        }
    }
    Ok(out)
}
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cargo test -p sqp-core --lib generate`
Expected: 4 passed.

- [ ] **Step 6: Run the full core test suite**

Run: `cargo test -p sqp-core`
Expected: all tests pass (bytepool, rfe32, rc6, maps, codec, generate, vectors).

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "feat: public generate() with retry/dedup + auto_batch()

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 8: GUI crate scaffold — form + generate button (no QR yet)

**Files:**
- Create: `crates/sqp-gui/Cargo.toml`, `crates/sqp-gui/src/main.rs`
- Test: manual run (GUI); logic is already covered by `sqp-core` tests.

**Interfaces:**
- Consumes: `sqp_core::{generate, GenerateRequest, GeneratedCode, InkType, SUPPORTED_COLOR_NAMES, auto_batch}`.

- [ ] **Step 1: Write `crates/sqp-gui/Cargo.toml`**

```toml
[package]
name = "sqp-gui"
version = "0.1.0"
edition = "2021"

[dependencies]
sqp-core = { path = "../sqp-core" }
eframe = "0.27"
egui = "0.27"
chrono = { version = "0.4", default-features = false, features = ["clock"] }
qrcode = "0.14"
image = { version = "0.25", default-features = false, features = ["png"] }
rfd = "0.14"
arboard = "3"
```

- [ ] **Step 2: Write `main.rs` with the input form and a results list (text only)**

`crates/sqp-gui/src/main.rs`:
```rust
use chrono::{Datelike, Local, Months};
use eframe::egui;
use sqp_core::{auto_batch, generate, GenerateRequest, GeneratedCode, InkType, SUPPORTED_COLOR_NAMES};
use std::collections::HashSet;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([720.0, 640.0]),
        ..Default::default()
    };
    eframe::run_native(
        "SQP QR Generator",
        options,
        Box::new(|_cc| Box::new(App::default())),
    )
}

struct App {
    ink_type: InkType,
    color: String,
    volume_l: u8,
    expires_year: i32,
    expires_month: u8,
    batch: String,
    count: u8,
    results: Vec<GeneratedCode>,
    produced: HashSet<String>,
    error: Option<String>,
}

impl Default for App {
    fn default() -> Self {
        let today = Local::now().date_naive();
        let exp = today.checked_add_months(Months::new(12)).unwrap();
        Self {
            ink_type: InkType::Kx2,
            color: "Cyan".to_string(),
            volume_l: 5,
            expires_year: exp.year(),
            expires_month: exp.month() as u8,
            batch: auto_batch(today),
            count: 1,
            results: Vec::new(),
            produced: HashSet::new(),
            error: None,
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::SidePanel::left("form").min_width(280.0).show(ctx, |ui| {
            ui.heading("New codes");
            ui.add_space(8.0);

            ui.label("Ink type");
            egui::ComboBox::from_id_source("ink")
                .selected_text(match self.ink_type { InkType::Kx2 => "KX2", InkType::Sqsg3 => "SQSG3" })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.ink_type, InkType::Kx2, "KX2");
                    ui.selectable_value(&mut self.ink_type, InkType::Sqsg3, "SQSG3");
                });

            ui.label("Color");
            egui::ComboBox::from_id_source("color")
                .selected_text(&self.color)
                .show_ui(ui, |ui| {
                    for name in SUPPORTED_COLOR_NAMES {
                        ui.selectable_value(&mut self.color, name.to_string(), name);
                    }
                });

            ui.label("Volume");
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.volume_l, 1, "1 L");
                ui.selectable_value(&mut self.volume_l, 5, "5 L");
            });

            ui.label("Expiry (month / year)");
            ui.horizontal(|ui| {
                ui.add(egui::DragValue::new(&mut self.expires_month).range(1..=12));
                ui.add(egui::DragValue::new(&mut self.expires_year).range(2007..=2070));
            });

            ui.label("Batch (9 chars, A-Z 0-9)");
            ui.text_edit_singleline(&mut self.batch);

            ui.label("How many codes (1-20)");
            ui.add(egui::DragValue::new(&mut self.count).range(1..=20));

            ui.add_space(12.0);
            if ui.button("Generate").clicked() {
                self.error = None;
                let req = GenerateRequest {
                    ink_type: self.ink_type,
                    color: self.color.clone(),
                    volume_l: self.volume_l,
                    expires_year: self.expires_year,
                    expires_month: self.expires_month,
                    batch: self.batch.clone(),
                    count: self.count,
                };
                match generate(&req, &mut self.produced) {
                    Ok(mut codes) => self.results.append(&mut codes),
                    Err(e) => self.error = Some(format!("{e:?}")),
                }
            }
            if let Some(err) = &self.error {
                ui.colored_label(egui::Color32::RED, err);
            }
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Results");
            egui::ScrollArea::vertical().show(ui, |ui| {
                for g in &self.results {
                    ui.monospace(&g.code);
                    ui.separator();
                }
            });
        });
    }
}
```

- [ ] **Step 3: Build and run manually**

Run: `cargo run -p sqp-gui`
Expected: window opens; selecting KX2/Cyan/5L and Generate appends a dashed code to Results. Selecting SQSG3 + Clear then Generate shows a red `InvalidCombo(...)` message and produces nothing.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "feat: egui GUI scaffold — input form + generate (text results)

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 9: QR rendering on screen

**Files:**
- Create: `crates/sqp-gui/src/qr.rs`
- Modify: `crates/sqp-gui/src/main.rs` (module + render each result's QR texture)
- Test: unit test for the QR→RGBA buffer helper.

**Interfaces:**
- Produces: `qr::qr_rgba(data: &str) -> (usize, Vec<u8>)` returning `(side_px, rgba)` — a square RGBA8 image, level L, zero border, black modules on white.

- [ ] **Step 1: Write the failing unit test**

`crates/sqp-gui/src/qr.rs`:
```rust
use qrcode::{EcLevel, QrCode};

/// Renders `data` to a square RGBA8 buffer at ~`scale` px per module.
/// Level L, no quiet-zone border — matches the Rails labels.
pub fn qr_rgba(data: &str, scale: usize) -> (usize, Vec<u8>) {
    let code = QrCode::with_error_correction_level(data.as_bytes(), EcLevel::L).unwrap();
    let modules = code.width();
    let side = modules * scale;
    let mut rgba = vec![255u8; side * side * 4];
    let colors = code.to_colors();
    for y in 0..modules {
        for x in 0..modules {
            let dark = colors[y * modules + x] == qrcode::Color::Dark;
            if dark {
                for dy in 0..scale {
                    for dx in 0..scale {
                        let px = ((y * scale + dy) * side + (x * scale + dx)) * 4;
                        rgba[px] = 0; rgba[px + 1] = 0; rgba[px + 2] = 0; rgba[px + 3] = 255;
                    }
                }
            }
        }
    }
    (side, rgba)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn produces_square_rgba_buffer() {
        let (side, rgba) = qr_rgba("2XELN7GWD9RD5FDC6MTATGAF3ZWTA8DL", 4);
        assert!(side > 0);
        assert_eq!(rgba.len(), side * side * 4);
        // has at least one black and one white pixel
        assert!(rgba.chunks(4).any(|p| p[0] == 0));
        assert!(rgba.chunks(4).any(|p| p[0] == 255));
    }
}
```

- [ ] **Step 2: Run test to verify it fails, then passes**

Run: `cargo test -p sqp-gui --lib qr`
First expected: FAIL if `qr` module not declared. Add `mod qr;` to `main.rs`, then re-run.
Expected: 1 passed.

- [ ] **Step 3: Render the texture per result in `main.rs`**

In `main.rs`, add `mod qr;` at the top, and in the Results loop replace the body with:
```rust
for g in &self.results {
    ui.horizontal(|ui| {
        let (side, rgba) = qr::qr_rgba(&g.code_undashed, 4);
        let image = egui::ColorImage::from_rgba_unmultiplied([side, side], &rgba);
        let tex = ui.ctx().load_texture(&g.code, image, egui::TextureOptions::NEAREST);
        ui.image((tex.id(), egui::vec2(120.0, 120.0)));
        ui.monospace(&g.code);
    });
    ui.separator();
}
```

- [ ] **Step 4: Run manually to confirm QR shows**

Run: `cargo run -p sqp-gui`
Expected: each generated code shows its QR next to the text.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat: render QR (level L, no border) beside each result

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 10: Save PNG, copy code, export-all (PNGs + CSV)

**Files:**
- Modify: `crates/sqp-gui/src/qr.rs` (add `qr_png_bytes`), `crates/sqp-gui/src/main.rs` (buttons + CSV)
- Test: unit test for `qr_png_bytes` (valid PNG header) and a `csv_line` helper.

**Interfaces:**
- Produces:
  - `qr::qr_png_bytes(data: &str, scale: usize) -> Vec<u8>` — PNG-encoded bytes.
  - `csv helper` inline in `main.rs`: header + one row per result.

- [ ] **Step 1: Write the failing test for PNG bytes**

Add to `crates/sqp-gui/src/qr.rs` tests:
```rust
    #[test]
    fn png_bytes_have_png_signature() {
        let bytes = qr_png_bytes("2XELN7GWD9RD5FDC6MTATGAF3ZWTA8DL", 6);
        assert_eq!(&bytes[0..8], &[137, 80, 78, 71, 13, 10, 26, 10]);
    }
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p sqp-gui --lib qr`
Expected: FAIL — `qr_png_bytes` not found.

- [ ] **Step 3: Implement `qr_png_bytes`**

Add to `qr.rs`:
```rust
use image::{ImageBuffer, Rgba};
use std::io::Cursor;

pub fn qr_png_bytes(data: &str, scale: usize) -> Vec<u8> {
    let (side, rgba) = qr_rgba(data, scale);
    let img: ImageBuffer<Rgba<u8>, _> =
        ImageBuffer::from_raw(side as u32, side as u32, rgba).unwrap();
    let mut out = Cursor::new(Vec::new());
    img.write_to(&mut out, image::ImageFormat::Png).unwrap();
    out.into_inner()
}
```

- [ ] **Step 4: Run to verify it passes**

Run: `cargo test -p sqp-gui --lib qr`
Expected: 2 passed.

- [ ] **Step 5: Wire up buttons in `main.rs`**

Track the request context so the CSV can include the fields. Add per-result buttons and a top-level Export-all. In the Results panel add above the loop:
```rust
if !self.results.is_empty() && ui.button("Export all (PNGs + CSV)").clicked() {
    if let Some(dir) = rfd::FileDialog::new().pick_folder() {
        let mut csv = String::from("code,ink_type,color,volume,expires,batch,number\n");
        for g in &self.results {
            let png = qr::qr_png_bytes(&g.code_undashed, 6);
            let _ = std::fs::write(dir.join(format!("{}.png", g.code_undashed)), png);
            csv.push_str(&format!(
                "{},{},{},{},{:02}/{},{},{}\n",
                g.code,
                match self.ink_type { InkType::Kx2 => "KX2", InkType::Sqsg3 => "SQSG3" },
                self.color, self.volume_l, self.expires_month, self.expires_year,
                self.batch, g.number
            ));
        }
        let _ = std::fs::write(dir.join("codes.csv"), csv);
    }
}
```
Inside the per-result `ui.horizontal`, after the monospace code add:
```rust
if ui.button("Copy").clicked() {
    ui.ctx().copy_text(g.code.clone());
}
if ui.button("Save QR").clicked() {
    if let Some(path) = rfd::FileDialog::new()
        .set_file_name(format!("{}.png", g.code_undashed))
        .save_file()
    {
        let _ = std::fs::write(path, qr::qr_png_bytes(&g.code_undashed, 6));
    }
}
```
Note: `ui.ctx().copy_text` is the egui 0.27 clipboard call; the `arboard` dependency is a fallback if a future egui version changes this — remove it if unused before final commit.

- [ ] **Step 6: Run manually**

Run: `cargo run -p sqp-gui`
Expected: "Copy" puts the dashed code on the clipboard; "Save QR" writes a PNG; "Export all" writes N PNGs + `codes.csv` into the chosen folder. Open a PNG and scan it — it must decode to the 32-char undashed code.

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "feat: copy code, save QR PNG, export-all (PNGs + CSV)

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 11: README + build/packaging docs + push to GitHub

**Files:**
- Create: `README.md`
- Modify: `.gitignore` (ensure `Cargo.lock` is committed — remove it from ignore if present)

- [ ] **Step 1: Ensure `Cargo.lock` is tracked**

Run:
```bash
grep -v '^Cargo.lock$' .gitignore > .gitignore.tmp && mv .gitignore.tmp .gitignore
cargo build   # regenerates Cargo.lock at workspace root
git add Cargo.lock
```

- [ ] **Step 2: Write `README.md`**

Cover, with exact commands:
```markdown
# SQP QR Generator

Native desktop app that generates SQP ink-canister codes and printable QR codes.
Offline; a single binary per OS.

## Prerequisites
- Rust (stable) via https://rustup.rs

## Run (development)
    cargo run -p sqp-gui

## Test the core algorithm
    cargo test -p sqp-core

## Release build (current OS)
    cargo build --release -p sqp-gui
Binary: target/release/sqp-gui (sqp-gui.exe on Windows)

## macOS .app bundle
    cargo install cargo-bundle
    cargo bundle --release -p sqp-gui
Bundle: target/release/bundle/osx/SQP QR Generator.app

## Windows
- Native: on Windows, `cargo build --release -p sqp-gui` -> target\release\sqp-gui.exe
- Cross from macOS:
      rustup target add x86_64-pc-windows-gnu
      brew install mingw-w64
      cargo build --release -p sqp-gui --target x86_64-pc-windows-gnu

## Linux
    cargo build --release -p sqp-gui
Binary: target/release/sqp-gui

## Notes
- Uniqueness is enforced only within a single run of the app (no database).
- QR error-correction level L, no border, matching the production labels.
```

- [ ] **Step 3: Commit and push**

```bash
git add -A
git commit -m "docs: README with run, test, and per-OS build/packaging steps

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
git branch -M main
git remote add origin https://github.com/marcozanella/sqp-qr-generator.git 2>/dev/null || true
git push -u origin main
```

- [ ] **Step 4: Confirm remote**

Run: `git ls-remote --heads origin main`
Expected: one line with the pushed `main` commit hash.

---

## Self-Review notes

- **Spec coverage:** core port (Tasks 1–7) ↔ spec §"sqp-core"; GUI (8–10) ↔ §"sqp-gui"/data flow; TDD vectors (5–6) ↔ §"Correctness strategy"; README/build (11) ↔ §"Build & distribution"; uniqueness caveat surfaced in Task 11 README ↔ spec §"Uniqueness caveat". SQSG3+Clear rejection covered in Tasks 4 & 7. All spec sections map to a task.
- **Endianness:** RC6 uses LE words (Ruby `V`); RFE32 uses a big-endian 160-bit view (Ruby `H*`/big-endian) — implemented via bit buffers, no `u128` overflow risk.
- **Type consistency:** `Fields`, `GenerateRequest`, `GeneratedCode`, `InkType`, `SqpError` variants are defined once and referenced with the same names/fields across tasks.
- **Reference vectors** are real output captured from the Ruby encoder (pinned number+tara), so Task 5/6 assertions are concrete, not placeholders.
