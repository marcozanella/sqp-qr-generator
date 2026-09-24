//! SQP codec: preview/plaintext bit-packing, RC6 key derivation, and the
//! top-level `encode`/`decode` that produce byte-identical output to the Ruby
//! `Sqp::ProductCode`.

use crate::{bytepool, rc6, rfe32, SqpError};

const PREVIEW_INDICES: [usize; 4] = [4, 9, 14, 19];

fn ciphertext_indices() -> Vec<usize> {
    (0..20).filter(|i| !PREVIEW_INDICES.contains(i)).collect()
}

/// The decoded, human-meaningful fields of a canister code.
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

/// Result of decoding a code: the recovered fields plus whether every internal
/// cross-check (preview vs. plaintext) agreed.
#[derive(Debug)]
pub struct DecodeResult {
    pub fields: Fields,
    pub valid: bool,
}

// --- preview (4 unencrypted bytes) ---
struct Preview {
    number: u16,
    year: u8,
    month: u8,
    color: u8,
    volume: u8,
}

fn parse_preview(b: &[u8; 4]) -> Preview {
    let (b0, b1, b2, b3) = (b[0] as u16, b[1] as u16, b[2] as u16, b[3] as u16);
    let number = (((b0 & 7) << 3 | (b1 & 7)) << 3 | (b2 & 7)) << 3 | (b3 & 7);
    let year = (((b[0] >> 3) << 1) | (b[1] >> 7)) as u8;
    let month = ((b[1] >> 3) & 0xF) as u8;
    let color = (b[2] >> 3) as u8;
    let volume = (b[3] >> 3) as u8;
    Preview {
        number,
        year,
        month,
        color,
        volume,
    }
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
    let number = (c[1] >> 2) | (c[2] << 6) | ((c[3] & 0x3) << 14);
    let month = ((c[3] >> 2) & 0xF) as u8;
    let year_raw = (((c[3] >> 6) | (c[4] << 2)) as i32) - 0x2CC;
    let ink_type_id = p[5];
    let legacy_tara_raw = p[6];
    let batch: String = p[7..16]
        .iter()
        .cloned()
        .filter(|&b| b != 0)
        .map(|b| b as char)
        .collect();
    Fields {
        volume,
        color,
        number: (number & 0xFFFF) as u16,
        month,
        year: year_raw + 0x7D7,
        ink_type_id,
        legacy_tara_raw,
        batch,
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

/// Encode fields into the dashed 40-character canister code.
pub fn encode(f: &Fields) -> String {
    let preview_bytes = pack_preview(
        (f.number & 0xFFF) as u16,
        (f.year - 2007) as u8,
        f.month,
        f.color,
        f.volume,
    );
    let key = derive_key(&preview_bytes);
    let mut key_arr = [0u8; 32];
    key_arr.copy_from_slice(&key);
    let schedule = rc6::key_schedule(&key_arr);
    let plaintext = pack_plaintext(f);
    let ciphertext = rc6::encrypt_block(&plaintext, &schedule);

    let mut rev = [0u8; 20];
    for (i, &idx) in PREVIEW_INDICES.iter().rev().enumerate() {
        rev[idx] = preview_bytes[i];
    }
    for (i, &idx) in ciphertext_indices().iter().enumerate() {
        rev[idx] = ciphertext[i];
    }
    let mut rfe_in = rev;
    rfe_in.reverse();
    let encoded_reversed = rfe32::encode(&rfe_in);
    let encoded: String = encoded_reversed.chars().rev().collect();
    encoded
        .as_bytes()
        .chunks(4)
        .map(|c| std::str::from_utf8(c).unwrap())
        .collect::<Vec<_>>()
        .join("-")
}

/// Decode a dashed (or undashed) canister code back into fields, with a validity
/// flag from the preview vs. plaintext cross-check.
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

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Fields {
        Fields {
            volume: 20,
            color: 1,
            number: 34291,
            month: 5,
            year: 2027,
            ink_type_id: 19,
            legacy_tara_raw: 20,
            batch: "UB26082F1".into(),
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
