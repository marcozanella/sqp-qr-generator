//! Public generation API: `generate()` (random number + tara, retry on
//! collision, self-verify) and `auto_batch()`.

use crate::codec::{decode, encode, Fields};
use crate::maps::{color_id, derive_ink_type_id, tara_options, volume_id, InkType};
use crate::SqpError;
use chrono::{Datelike, NaiveDate};
use rand::seq::SliceRandom;
use rand::Rng;
use std::collections::HashSet;

const MAX_DUP_RETRIES: usize = 20;
const BATCH_LEN: usize = 9;

/// A request to generate one or more canister codes for a single SKU.
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

/// One generated code and the random fields that produced it.
#[derive(Debug, Clone)]
pub struct GeneratedCode {
    pub code: String,
    pub code_undashed: String,
    pub number: u16,
    pub legacy_tara_raw: u8,
}

/// Ruby: `date.strftime("UB%y%jF1")` — "UB" + 2-digit year + 3-digit day-of-year + "F1".
pub fn auto_batch(date: NaiveDate) -> String {
    format!("UB{:02}{:03}F1", date.year() % 100, date.ordinal())
}

fn valid_batch(b: &str) -> bool {
    b.len() == BATCH_LEN && b.bytes().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
}

/// Generate `req.count` unique, self-verified codes, recording each into `already`.
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
                volume,
                color,
                number,
                month: req.expires_month,
                year: req.expires_year,
                ink_type_id,
                legacy_tara_raw,
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
            ink_type: InkType::Kx2,
            color: "Cyan".into(),
            volume_l: 5,
            expires_year: 2027,
            expires_month: 5,
            batch: "UB26103F1".into(),
            count: 3,
        };
        let mut seen = HashSet::new();
        let out = generate(&req, &mut seen).unwrap();
        assert_eq!(out.len(), 3);
        for g in &out {
            assert_eq!(g.code_undashed, g.code.replace('-', ""));
            let r = decode(&g.code).unwrap();
            assert!(r.valid);
        }
        assert_eq!(seen.len(), 3);
    }

    #[test]
    fn generate_rejects_bad_batch() {
        let req = GenerateRequest {
            ink_type: InkType::Kx2,
            color: "Cyan".into(),
            volume_l: 5,
            expires_year: 2027,
            expires_month: 5,
            batch: "bad".into(),
            count: 1,
        };
        let mut seen = HashSet::new();
        assert!(matches!(generate(&req, &mut seen), Err(SqpError::BadBatch(_))));
    }

    #[test]
    fn generate_rejects_sqsg3_clear() {
        let req = GenerateRequest {
            ink_type: InkType::Sqsg3,
            color: "Clear".into(),
            volume_l: 5,
            expires_year: 2027,
            expires_month: 5,
            batch: "UB26103F1".into(),
            count: 1,
        };
        let mut seen = HashSet::new();
        assert!(matches!(
            generate(&req, &mut seen),
            Err(SqpError::InvalidCombo(_))
        ));
    }
}
