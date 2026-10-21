//! Image header inspection used to place logos.

/// Image formats the engine can embed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImgType {
    /// Not a supported image.
    Unknown,
    /// PNG.
    Png,
    /// JPEG.
    Jpg,
}

/// Width and height in pixels of a PNG or JPEG image (`None` when the data is neither).
///
/// For PNG the 16 low bits of the 32-bit IHDR dimensions are returned, as the original does.
pub fn image_size(data: &[u8]) -> Option<(u16, u16)> {
    png_size(data).or_else(|| jpg_size(data))
}

/// The format of an image, detected from its content.
pub fn image_type(data: &[u8]) -> ImgType {
    if png_size(data).is_some() {
        ImgType::Png
    } else if jpg_size(data).is_some() {
        ImgType::Jpg
    } else {
        ImgType::Unknown
    }
}

fn png_size(data: &[u8]) -> Option<(u16, u16)> {
    const SIGNATURE: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];
    if data.get(..8)? != SIGNATURE {
        return None;
    }
    let width = u16::from_be_bytes([*data.get(18)?, *data.get(19)?]);
    let height = u16::from_be_bytes([*data.get(22)?, *data.get(23)?]);
    Some((width, height))
}

fn jpg_size(data: &[u8]) -> Option<(u16, u16)> {
    if data.get(..2)? != [0xFF, 0xD8] {
        return None;
    }
    let mut pos = 2;
    loop {
        // Next marker: 0xFF followed by a marker byte.
        while *data.get(pos)? != 0xFF {
            pos += 1;
        }
        while *data.get(pos)? == 0xFF {
            pos += 1;
        }
        let marker = *data.get(pos)?;
        pos += 1;
        match marker {
            0x00 | 0x01 | 0xD0..=0xD9 => continue,
            0xC0..=0xC2 => {
                let height = u16::from_be_bytes([*data.get(pos + 3)?, *data.get(pos + 4)?]);
                let width = u16::from_be_bytes([*data.get(pos + 5)?, *data.get(pos + 6)?]);
                return Some((width, height));
            }
            _ => {
                let len = usize::from(u16::from_be_bytes([*data.get(pos)?, *data.get(pos + 1)?]));
                pos += len;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png(w: u32, h: u32) -> Vec<u8> {
        let mut v = vec![
            137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, b'I', b'H', b'D', b'R',
        ];
        v.extend_from_slice(&w.to_be_bytes());
        v.extend_from_slice(&h.to_be_bytes());
        v
    }

    #[test]
    fn reads_png_dimensions() {
        let data = png(640, 480);
        assert_eq!(image_size(&data), Some((640, 480)));
        assert_eq!(image_type(&data), ImgType::Png);
    }

    #[test]
    fn reads_jpeg_dimensions_skipping_segments() {
        let mut jpg = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x04, 0x00, 0x00];
        jpg.extend_from_slice(&[0xFF, 0xC0, 0x00, 0x11, 8, 0x01, 0xE0, 0x02, 0x80, 3]);
        assert_eq!(image_size(&jpg), Some((640, 480)));
        assert_eq!(image_type(&jpg), ImgType::Jpg);
    }

    #[test]
    fn rejects_other_data() {
        assert_eq!(image_size(b"GIF89a....."), None);
        assert_eq!(image_type(b""), ImgType::Unknown);
        assert_eq!(image_size(&[0xFF, 0xD8, 0xFF]), None);
    }
}
