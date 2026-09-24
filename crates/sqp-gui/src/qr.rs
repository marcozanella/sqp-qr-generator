//! QR helpers: render an SQP code to an RGBA buffer (for on-screen textures).
//! Level L error correction, no quiet-zone border — matching the Rails labels.

use qrcode::{EcLevel, QrCode};

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
