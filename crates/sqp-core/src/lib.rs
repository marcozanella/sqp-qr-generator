//! `sqp-core` — pure, UI-free port of the `platform01` Ruby SQP code generator.
//!
//! `encode` produces byte-identical output to the Ruby `Sqp::ProductCode.encode`
//! for the same fields (enforced by `tests/vectors.rs`).

pub mod bytepool;
pub mod codec;
pub mod generate;
pub mod maps;
pub mod rc6;
pub mod rfe32;

pub use codec::{decode, encode, DecodeResult, Fields};
pub use generate::{auto_batch, generate, GenerateRequest, GeneratedCode};
pub use maps::{InkType, SUPPORTED_COLOR_NAMES};

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
    /// A code could not be decoded (bad length or structure).
    DecodeError(String),
    /// A batch string that fails the `^[A-Z0-9]{9}$` format.
    BadBatch(String),
    /// A freshly encoded code failed to decode as valid — indicates an
    /// encoder/decoder mismatch, never expected in practice.
    EncoderRegression(String),
    /// Too many collisions in a row while generating unique codes.
    CollisionExhausted,
}
