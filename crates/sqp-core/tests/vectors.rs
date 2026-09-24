//! Reference-vector suite: every captured vector must encode byte-identically to
//! the Ruby encoder and decode back as valid.

use sqp_core::codec::{decode, encode, Fields};
use sqp_core::maps::{color_id, derive_ink_type_id, volume_id, InkType};

#[allow(clippy::too_many_arguments)]
fn fields(
    ink: InkType,
    color: &str,
    vol: u8,
    month: u8,
    year: i32,
    batch: &str,
    number: u16,
    tara: u8,
) -> Fields {
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
        (
            fields(InkType::Kx2, "Black", 5, 5, 2027, "UB26082F1", 34291, 20),
            "2XEL-58NW-D4PC-L34B-5E3M-8E6F-LYWJ-LCML",
        ),
        (
            fields(InkType::Kx2, "Cyan", 5, 5, 2027, "UB26103F1", 1, 18),
            "8QSS-N7GW-D9RD-5FDC-6MTA-TGAF-3ZWT-A8DL",
        ),
        (
            fields(InkType::Sqsg3, "Magenta", 1, 12, 2028, "UB17483F1", 65535, 7),
            "AKZZ-JK7J-G3AE-U27D-29XY-SG76-RVCK-KT8L",
        ),
        (
            fields(InkType::Kx2, "Clear", 1, 1, 2030, "AAAAAAAAA", 0, 9),
            "PZCT-7VDJ-ASEY-99AJ-HHCE-UYDT-P9MJ-P2CM",
        ),
        (
            fields(InkType::Kx2, "Yellow", 5, 5, 2027, "UB26168F1", 39920, 22),
            "P6VR-43AW-V8LR-ZQ2E-N2EJ-2W6F-A2J7-Z4ZL",
        ),
    ];
    for (f, expected) in cases {
        let code = encode(&f);
        assert_eq!(code, expected, "encode mismatch for {expected}");
        let r = decode(&code).unwrap();
        assert!(r.valid, "decode not valid for {expected}");
    }
}
