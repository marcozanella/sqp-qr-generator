//! QR helpers: render an SQP code to an RGBA buffer (for on-screen textures).
//! Level L error correction, no quiet-zone border — matching the Rails labels.

use image::{ImageBuffer, Rgba};
use qrcode::{EcLevel, QrCode};
use std::io::Cursor;

/// Renders `data` to a square RGBA8 buffer at `scale` px per module.
/// Level L, no quiet-zone border; black modules on white.
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
                        rgba[px] = 0;
                        rgba[px + 1] = 0;
                        rgba[px + 2] = 0;
                        rgba[px + 3] = 255;
                    }
                }
            }
        }
    }
    (side, rgba)
}

/// Encodes `data` as a PNG (level L, no border) and returns the raw PNG bytes.
pub fn qr_png_bytes(data: &str, scale: usize) -> Vec<u8> {
    let (side, rgba) = qr_rgba(data, scale);
    let img: ImageBuffer<Rgba<u8>, _> =
        ImageBuffer::from_raw(side as u32, side as u32, rgba).unwrap();
    let mut out = Cursor::new(Vec::new());
    img.write_to(&mut out, image::ImageFormat::Png).unwrap();
    out.into_inner()
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

    #[test]
    fn png_bytes_have_png_signature() {
        let bytes = qr_png_bytes("2XELN7GWD9RD5FDC6MTATGAF3ZWTA8DL", 6);
        assert_eq!(&bytes[0..8], &[137, 80, 78, 71, 13, 10, 26, 10]);
    }
}
