//! `sqp-core` — pure, UI-free port of the `platform01` Ruby SQP code generator.

pub mod bytepool;
pub mod maps;
pub mod rc6;
pub mod rfe32;

/// All fallible operations in `sqp-core` return this error type.
#[derive(Debug, PartialEq, Eq)]
pub enum SqpError {
    /// An RFE32 string was not exactly 32 characters.
    BadLength(usize),
    /// An RFE32 string contained a character outside the alphabet.
    CharNotInAlphabet(char),
    /// A color name that is not one of the supported SKUs.
    UnknownColor(String),
    /// A volume (in litres) that has no known mapping.
    UnknownVolume(u8),
    /// A combination of fields that does not correspond to a real SKU
    /// (e.g. SQSG3 + Clear).
    InvalidCombo(String),
}
