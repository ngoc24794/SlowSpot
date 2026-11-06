//! Minimal standalone 24-bit uncompressed BMP image writer.
//!
//! Writes valid Windows bitmap (.bmp) files without external dependencies.

use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::Path;

/// Saves an [`egui::ColorImage`] to a 24-bit uncompressed Windows bitmap (.bmp) file.
pub fn save_bmp(path: &Path, image: &eframe::egui::ColorImage) -> io::Result<()> {
    let width = image.size[0];
    let height = image.size[1];
    let row_stride = (width * 3).div_ceil(4) * 4;
    let padding = row_stride - width * 3;
    let pixel_data_size = row_stride * height;
    let file_size = 54 + pixel_data_size;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let file = File::create(path)?;
    let mut w = BufWriter::new(file);

    // Bitmap File Header (14 bytes)
    w.write_all(b"BM")?;
    w.write_all(&(file_size as u32).to_le_bytes())?;
    w.write_all(&0u16.to_le_bytes())?; // reserved 1
    w.write_all(&0u16.to_le_bytes())?; // reserved 2
    w.write_all(&54u32.to_le_bytes())?; // pixel data offset

    // DIB Header / BITMAPINFOHEADER (40 bytes)
    w.write_all(&40u32.to_le_bytes())?; // header size
    w.write_all(&(width as i32).to_le_bytes())?;
    w.write_all(&(height as i32).to_le_bytes())?; // positive height for bottom-up
    w.write_all(&1u16.to_le_bytes())?; // color planes
    w.write_all(&24u16.to_le_bytes())?; // bits per pixel (24-bit RGB)
    w.write_all(&0u32.to_le_bytes())?; // BI_RGB (uncompressed)
    w.write_all(&(pixel_data_size as u32).to_le_bytes())?;
    w.write_all(&2835i32.to_le_bytes())?; // ~72 DPI horizontal (2835 ppm)
    w.write_all(&2835i32.to_le_bytes())?; // ~72 DPI vertical
    w.write_all(&0u32.to_le_bytes())?; // colors used
    w.write_all(&0u32.to_le_bytes())?; // important colors

    // Pixel data (bottom-up: row height-1 down to row 0)
    let pad_bytes = [0u8; 3];
    for y in (0..height).rev() {
        for x in 0..width {
            let pixel = image.pixels[y * width + x];
            w.write_all(&[pixel.b(), pixel.g(), pixel.r()])?;
        }
        if padding > 0 {
            w.write_all(&pad_bytes[..padding])?;
        }
    }

    w.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::ColorImage;

    #[test]
    fn test_save_bmp_header_and_dimensions() {
        let temp_dir = std::env::temp_dir();
        let test_path = temp_dir.join("slowspot_test_save.bmp");

        let img = ColorImage::from_rgba_unmultiplied(
            [2, 3],
            &[
                255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 0, 255, 255, 0, 255, 255,
                0, 255, 255, 255,
            ],
        );

        save_bmp(&test_path, &img).expect("Failed to write bmp");
        let bytes = std::fs::read(&test_path).expect("Failed to read bmp");
        let _ = std::fs::remove_file(&test_path);

        // Check BMP magic
        assert_eq!(&bytes[0..2], b"BM");
        // Check offset
        let offset = u32::from_le_bytes([bytes[10], bytes[11], bytes[12], bytes[13]]);
        assert_eq!(offset, 54);
        // Check width and height
        let w = i32::from_le_bytes([bytes[18], bytes[19], bytes[20], bytes[21]]);
        let h = i32::from_le_bytes([bytes[22], bytes[23], bytes[24], bytes[25]]);
        assert_eq!(w, 2);
        assert_eq!(h, 3);
        // Check bit count = 24
        let bpp = u16::from_le_bytes([bytes[28], bytes[29]]);
        assert_eq!(bpp, 24);
    }
}
