//! Color / volume / tara / ink-type maps, copied verbatim from the Ruby
//! `Sqp::ColorMap` and product-code rules.

use crate::SqpError;

/// The two ink product families.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InkType {
    Kx2,
    Sqsg3,
}

/// The nine supported color names, in canonical order.
pub const SUPPORTED_COLOR_NAMES: [&str; 9] = [
    "Black",
    "Cyan",
    "Magenta",
    "Yellow",
    "Light Cyan",
    "Light Magenta",
    "White",
    "Clear",
    "Light Black",
];

/// Map a color name to its numeric id.
pub fn color_id(name: &str) -> Result<u8, SqpError> {
    let id = match name {
        "Black" => 1,
        "Cyan" => 2,
        "Magenta" => 3,
        "Yellow" => 4,
        "Light Cyan" => 5,
        "Light Magenta" => 6,
        "White" => 7,
        "Clear" => 8,
        "Light Black" => 10,
        _ => return Err(SqpError::UnknownColor(name.to_string())),
    };
    Ok(id)
}

/// Map a volume (litres) to its numeric id.
pub fn volume_id(vol_l: u8) -> Result<u8, SqpError> {
    match vol_l {
        5 => Ok(20),
        1 => Ok(8),
        other => Err(SqpError::UnknownVolume(other)),
    }
}

/// The tara (empty-weight) options that are valid for a given volume.
pub fn tara_options(vol_l: u8) -> Result<&'static [u8], SqpError> {
    match vol_l {
        5 => Ok(&[18, 19, 20, 21, 22]),
        1 => Ok(&[7, 8, 9]),
        other => Err(SqpError::UnknownVolume(other)),
    }
}

/// Derive the ink-type id from the ink family and color.
///
/// - KX2, non-Clear => 19
/// - SQSG3, non-Clear => 21
/// - KX2, Clear => 8
/// - SQSG3, Clear => not a real SKU (rejected)
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
