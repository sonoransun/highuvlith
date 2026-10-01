//! PNG export: 8-bit RGBA through the `png` crate (adaptive per-row filters
//! plus DEFLATE, `Compression::Balanced`).
//!
//! Saves exactly what the app renders — a heatmap texture or a window
//! screenshot read back from the GPU. Flat UI areas and smooth colormaps
//! compress well; the earlier stored-block writer produced ~4 bytes per
//! pixel (~20 MB for a 2× full-window capture).

use std::path::Path;

/// Encode a `width × height` RGBA8 image (row-major, top row first).
pub fn encode_rgba(width: u32, height: u32, rgba: &[u8]) -> Result<Vec<u8>, String> {
    let (w, h) = (width as usize, height as usize);
    if w == 0 || h == 0 {
        return Err(format!("image must be non-empty (got {width}×{height})"));
    }
    if rgba.len() != w * h * 4 {
        return Err(format!(
            "expected {} RGBA bytes for {width}×{height}, got {}",
            w * h * 4,
            rgba.len()
        ));
    }
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_compression(png::Compression::Balanced);
    let mut writer = encoder
        .write_header()
        .map_err(|e| format!("PNG header: {e}"))?;
    writer
        .write_image_data(rgba)
        .map_err(|e| format!("PNG data: {e}"))?;
    writer.finish().map_err(|e| format!("PNG finish: {e}"))?;
    Ok(out)
}

/// Encode and write an RGBA8 image to `path`.
pub fn write_rgba(path: &Path, width: u32, height: u32, rgba: &[u8]) -> Result<(), String> {
    let bytes = encode_rgba(width, height, rgba)?;
    std::fs::write(path, bytes).map_err(|e| format!("cannot write {}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Decode with the `png` crate's reader (independent of the encoder path).
    fn decode(bytes: &[u8]) -> (u32, u32, Vec<u8>) {
        let decoder = png::Decoder::new(std::io::Cursor::new(bytes));
        let mut reader = decoder.read_info().expect("valid PNG header");
        let mut buf = vec![0; reader.output_buffer_size().expect("buffer size")];
        let info = reader.next_frame(&mut buf).expect("valid PNG data");
        assert_eq!(info.color_type, png::ColorType::Rgba);
        assert_eq!(info.bit_depth, png::BitDepth::Eight);
        buf.truncate(info.buffer_size());
        (info.width, info.height, buf)
    }

    #[test]
    fn round_trip_small_image() {
        let rgba: Vec<u8> = (0..(3 * 2 * 4)).map(|v| v as u8).collect();
        let png = encode_rgba(3, 2, &rgba).unwrap();
        assert_eq!(&png[..8], &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);
        let (w, h, raw) = decode(&png);
        assert_eq!((w, h), (3, 2));
        assert_eq!(raw, rgba);
    }

    #[test]
    fn large_image_round_trips_and_compresses() {
        // A smooth gradient with a flat panel, like a heatmap next to UI chrome.
        let (w, h) = (600u32, 400u32);
        let mut rgba = Vec::with_capacity((w * h * 4) as usize);
        for y in 0..h {
            for x in 0..w {
                let (r, g, b) = if x < 200 {
                    (40, 40, 48)
                } else {
                    ((x / 3) as u8, (y * 255 / h) as u8, 128)
                };
                rgba.extend_from_slice(&[r, g, b, 255]);
            }
        }
        let png = encode_rgba(w, h, &rgba).unwrap();
        let (dw, dh, raw) = decode(&png);
        assert_eq!((dw, dh), (w, h));
        assert_eq!(raw, rgba);
        // Stored blocks would be ≥ 4·w·h = 960 000 bytes; require ≥ 20× smaller.
        assert!(png.len() * 20 < rgba.len(), "PNG is {} bytes", png.len());
    }

    #[test]
    fn rejects_bad_dimensions() {
        assert!(encode_rgba(0, 4, &[]).is_err());
        assert!(encode_rgba(2, 2, &[0; 15]).is_err());
    }
}
