//! JPEG and PNG parsing for embedding images (port of the FPDF image parsers).
//!
//! JPEG data is embedded as is (`DCTDecode`). PNG data is embedded without recompression
//! (`FlateDecode` with the PNG predictors); images with an alpha channel are split into a
//! colour stream and a soft mask.

use std::io::{Read, Write};

use flate2::{Compression, read::ZlibDecoder, write::ZlibEncoder};

use crate::error::FpdfError;

/// A parsed image ready to be written as an XObject.
#[derive(Debug, Clone, Default)]
pub(crate) struct ImageInfo {
    /// PDF object number, assigned when the image is written.
    pub n: usize,
    /// Resource index (`/I<i>`), starting at 1.
    pub i: usize,
    /// Cache key (file name) or empty for stream images.
    pub name: String,
    pub data: Vec<u8>,
    pub w: u32,
    pub h: u32,
    pub cs: String,
    pub bpc: u32,
    pub f: String,
    pub dp: String,
    pub pal: Vec<u8>,
    pub trns: Vec<u8>,
    pub smask: Vec<u8>,
}

/// Compresses with zlib at the best compression level.
pub(crate) fn zcompress(data: &[u8]) -> Vec<u8> {
    let mut enc = ZlibEncoder::new(Vec::new(), Compression::best());
    // Writing to a Vec cannot fail.
    let _ = enc.write_all(data);
    enc.finish().unwrap_or_default()
}

fn zdecompress(data: &[u8]) -> Result<Vec<u8>, FpdfError> {
    let mut out = Vec::new();
    ZlibDecoder::new(data)
        .read_to_end(&mut out)
        .map_err(|e| FpdfError::msg(format!("PNG - invalid image data: {e}")))?;
    Ok(out)
}

/// Parses an image given its (case-insensitive) extension.
pub(crate) fn parse_image(data: &[u8], ext: &str) -> Result<ImageInfo, FpdfError> {
    let ext = ext.trim().to_ascii_uppercase();
    match ext.as_str() {
        "" => Err(FpdfError::msg("Image Stream without an extension!")),
        "PNG" => parse_png(data),
        "JPG" | "JPEG" => parse_jpg(data),
        other => Err(FpdfError::msg(format!(
            "Unsupported image formatting: {other}"
        ))),
    }
}

fn parse_jpg(data: &[u8]) -> Result<ImageInfo, FpdfError> {
    if data.len() < 4 || data[0] != 0xFF || data[1] != 0xD8 {
        return Err(FpdfError::msg("JPG - Invalid"));
    }
    let mut pos = 2;
    loop {
        // Scan for the next marker: 0xFF followed by a non-0xFF, non-zero byte.
        while pos < data.len() && data[pos] != 0xFF {
            pos += 1;
        }
        while pos < data.len() && data[pos] == 0xFF {
            pos += 1;
        }
        let Some(&marker) = data.get(pos) else {
            return Err(FpdfError::msg("JPG - SOF Invalid"));
        };
        pos += 1;
        if marker == 0xD9 || marker == 0xDA {
            return Err(FpdfError::msg("JPG - SOF Invalid"));
        }
        // Standalone markers carry no length.
        if marker == 0x00 || marker == 0x01 || (0xD0..=0xD8).contains(&marker) {
            continue;
        }
        let Some(len_bytes) = data.get(pos..pos + 2) else {
            return Err(FpdfError::msg("JPG - SOF Invalid"));
        };
        let len = usize::from(u16::from_be_bytes([len_bytes[0], len_bytes[1]]));
        let seg = data.get(pos + 2..pos + len).unwrap_or(&[]);
        // SOF0 (baseline), SOF1 (extended sequential), SOF2 (progressive).
        if matches!(marker, 0xC0..=0xC2) && seg.len() > 5 {
            let bpc = u32::from(seg[0]);
            let h = u32::from(u16::from_be_bytes([seg[1], seg[2]]));
            let w = u32::from(u16::from_be_bytes([seg[3], seg[4]]));
            let cs = match seg[5] {
                3 => "DeviceRGB",
                4 => "DeviceCMYK",
                _ => "DeviceGray",
            };
            if w == 0 {
                return Err(FpdfError::msg("JPG - SOF Invalid"));
            }
            return Ok(ImageInfo {
                w,
                h,
                bpc,
                cs: cs.to_owned(),
                f: "DCTDecode".to_owned(),
                data: data.to_vec(),
                ..ImageInfo::default()
            });
        }
        pos += len;
    }
}

struct PngChunks<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> PngChunks<'a> {
    fn next_chunk(&mut self) -> Option<(&'a [u8], &'a [u8])> {
        let head = self.data.get(self.pos..self.pos + 8)?;
        let len = u32::from_be_bytes([head[0], head[1], head[2], head[3]]) as usize;
        let kind = &head[4..8];
        let body = self.data.get(self.pos + 8..self.pos + 8 + len)?;
        // Skip the CRC.
        self.pos += 8 + len + 4;
        Some((kind, body))
    }
}

fn parse_png(data: &[u8]) -> Result<ImageInfo, FpdfError> {
    const SIGNATURE: [u8; 8] = [137, b'P', b'N', b'G', 13, 10, 26, 10];
    if data.get(..8) != Some(&SIGNATURE) {
        return Err(FpdfError::msg("PNG - Invalid"));
    }
    let mut chunks = PngChunks { data, pos: 8 };
    let Some((kind, ihdr)) = chunks.next_chunk() else {
        return Err(FpdfError::msg("PNG - Incorrect IHDR"));
    };
    if kind != b"IHDR" || ihdr.len() != 13 {
        return Err(FpdfError::msg("PNG - Incorrect IHDR"));
    }
    let width = u32::from_be_bytes([ihdr[0], ihdr[1], ihdr[2], ihdr[3]]);
    let height = u32::from_be_bytes([ihdr[4], ihdr[5], ihdr[6], ihdr[7]]);
    let (bit_depth, color_type, compression, filter, interlace) =
        (ihdr[8], ihdr[9], ihdr[10], ihdr[11], ihdr[12]);
    if bit_depth > 8 {
        return Err(FpdfError::msg("PNG - 16-bit depth not supported"));
    }
    let cs = match color_type {
        0 | 4 => "DeviceGray",
        2 | 6 => "DeviceRGB",
        3 => "Indexed",
        _ => return Err(FpdfError::msg("PNG - Unknown color type")),
    };
    if compression != 0 {
        return Err(FpdfError::msg("PNG - Unknown compression method"));
    }
    if filter != 0 {
        return Err(FpdfError::msg("PNG - Unknown filter method"));
    }
    if interlace != 0 {
        return Err(FpdfError::msg("PNG - Interlacing not supported"));
    }

    let mut info = ImageInfo {
        cs: cs.to_owned(),
        ..ImageInfo::default()
    };
    while let Some((kind, body)) = chunks.next_chunk() {
        match kind {
            b"PLTE" => info.pal = body.to_vec(),
            b"tRNS" => match color_type {
                0 if body.len() >= 2 => info.trns = vec![body[1]],
                2 if body.len() >= 6 => info.trns = vec![body[1], body[3], body[5]],
                3 => {
                    if let Some(p) = body.iter().position(|b| *b == 0) {
                        info.trns = vec![p as u8];
                    }
                }
                _ => {}
            },
            b"IDAT" => info.data.extend_from_slice(body),
            b"IEND" => break,
            _ => {}
        }
    }
    if cs == "Indexed" && info.pal.is_empty() {
        return Err(FpdfError::msg("PNG - Missing palette"));
    }
    info.w = width;
    info.h = height;
    info.bpc = u32::from(bit_depth);
    info.f = "FlateDecode".to_owned();
    info.dp = format!(
        "/Predictor 15 /Colors {} /BitsPerComponent {} /Columns {}",
        if cs == "DeviceRGB" { 3 } else { 1 },
        bit_depth,
        width
    );

    if color_type >= 4 {
        split_alpha(&mut info, color_type)?;
    }
    Ok(info)
}

/// Splits the filtered scanlines of a grey+alpha or RGBA image into a colour stream and an
/// alpha stream. Every PNG filter only looks at bytes of the same channel, so the
/// de-interleaved streams stay valid for `/Predictor 15`.
fn split_alpha(info: &mut ImageInfo, color_type: u8) -> Result<(), FpdfError> {
    let raw = zdecompress(&info.data)?;
    let color_len = if color_type == 4 { 1 } else { 3 };
    let row_len = (color_len + 1) * info.w as usize;
    let mut color = Vec::with_capacity((1 + info.w as usize * color_len) * info.h as usize);
    let mut alpha = Vec::with_capacity((1 + info.w as usize) * info.h as usize);
    for row in 0..info.h as usize {
        let start = (1 + row_len) * row;
        let Some(line) = raw.get(start..start + 1 + row_len) else {
            return Err(FpdfError::msg("PNG - truncated image data"));
        };
        color.push(line[0]);
        alpha.push(line[0]);
        for px in line[1..].chunks_exact(color_len + 1) {
            color.extend_from_slice(&px[..color_len]);
            alpha.push(px[color_len]);
        }
    }
    info.data = zcompress(&color);
    info.smask = zcompress(&alpha);
    Ok(())
}

impl ImageInfo {
    /// `true` when the image carries a soft mask (needs PDF 1.4).
    pub(crate) fn has_alpha(&self) -> bool {
        !self.smask.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png(width: u32, height: u32, color_type: u8, scanlines: &[u8]) -> Vec<u8> {
        let mut out = vec![137, b'P', b'N', b'G', 13, 10, 26, 10];
        let mut chunk = |kind: &[u8; 4], body: &[u8]| {
            out.extend_from_slice(&(body.len() as u32).to_be_bytes());
            out.extend_from_slice(kind);
            out.extend_from_slice(body);
            out.extend_from_slice(&[0, 0, 0, 0]);
        };
        let mut ihdr = Vec::new();
        ihdr.extend_from_slice(&width.to_be_bytes());
        ihdr.extend_from_slice(&height.to_be_bytes());
        ihdr.extend_from_slice(&[8, color_type, 0, 0, 0]);
        chunk(b"IHDR", &ihdr);
        chunk(b"IDAT", &zcompress(scanlines));
        chunk(b"IEND", &[]);
        out
    }

    #[test]
    fn parses_rgb_png() {
        let data = png(2, 1, 2, &[0, 1, 2, 3, 4, 5, 6]);
        let info = parse_image(&data, "png").expect("png");
        assert_eq!((info.w, info.h, info.bpc), (2, 1, 8));
        assert_eq!(info.cs, "DeviceRGB");
        assert_eq!(
            info.dp,
            "/Predictor 15 /Colors 3 /BitsPerComponent 8 /Columns 2"
        );
        assert!(!info.has_alpha());
    }

    #[test]
    fn splits_rgba_png_into_color_and_mask() {
        let data = png(2, 1, 6, &[0, 10, 20, 30, 255, 40, 50, 60, 128]);
        let info = parse_image(&data, "PNG").expect("png");
        assert!(info.has_alpha());
        assert_eq!(
            zdecompress(&info.data).expect("color"),
            vec![0, 10, 20, 30, 40, 50, 60]
        );
        assert_eq!(zdecompress(&info.smask).expect("alpha"), vec![0, 255, 128]);
    }

    #[test]
    fn rejects_unsupported_and_corrupt_images() {
        assert!(parse_image(b"not an image", "png").is_err());
        assert!(parse_image(b"x", "gif").is_err());
        assert!(parse_image(b"x", "").is_err());
        assert!(parse_image(&[0xFF, 0xD8, 0xFF, 0xD9], "jpg").is_err());
    }

    #[test]
    fn parses_jpeg_header() {
        // SOI, APP0 stub, SOF0 (8 bit, 3x2, 3 components), EOI.
        let mut jpg = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x04, 0x00, 0x00];
        jpg.extend_from_slice(&[0xFF, 0xC0, 0x00, 0x11, 8, 0, 2, 0, 3, 3]);
        jpg.extend_from_slice(&[1, 0x22, 0, 2, 0x11, 1, 3, 0x11, 1]);
        jpg.extend_from_slice(&[0xFF, 0xD9]);
        let info = parse_image(&jpg, "jpeg").expect("jpeg");
        assert_eq!((info.w, info.h, info.bpc), (3, 2, 8));
        assert_eq!(info.cs, "DeviceRGB");
        assert_eq!(info.f, "DCTDecode");
        assert_eq!(info.data, jpg);
    }
}
