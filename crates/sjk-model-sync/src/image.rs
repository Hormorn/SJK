//! A texture's width and height from its header, without decoding it: enough to
//! refuse what is not a real image or is too large to load.

/// `(width, height)` of a `tga`, `jpg` or `png` file, or why it is not one.
pub(crate) fn size(kind: &str, bytes: &[u8]) -> Result<(u32, u32), &'static str> {
    let (width, height) = match kind {
        "png" => png(bytes),
        "jpg" => jpeg(bytes),
        "tga" => tga(bytes),
        _ => None,
    }
    .ok_or("not a valid image of its type")?;
    if width == 0 || height == 0 {
        return Err("the image has no pixels");
    }
    Ok((width, height))
}

fn be16(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from(u16::from_be_bytes(
        bytes.get(at..at + 2)?.try_into().ok()?,
    )))
}

fn png(bytes: &[u8]) -> Option<(u32, u32)> {
    const SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";
    if bytes.get(..8)? != SIGNATURE || bytes.get(12..16)? != b"IHDR" {
        return None;
    }
    let read = |at: usize| Some(u32::from_be_bytes(bytes.get(at..at + 4)?.try_into().ok()?));
    Some((read(16)?, read(20)?))
}

/// Walks the markers to the first frame header (SOFn), which holds the size.
fn jpeg(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.get(..2)? != [0xFF, 0xD8] {
        return None;
    }
    let mut at = 2;
    loop {
        if *bytes.get(at)? != 0xFF {
            return None;
        }
        let marker = *bytes.get(at + 1)?;
        match marker {
            // Fill bytes before a marker.
            0xFF => at += 1,
            // Markers with no length.
            0x01 | 0xD0..=0xD7 => at += 2,
            // Image data or the end before any frame header.
            0xD9 | 0xDA => return None,
            _ => {
                let length = be16(bytes, at + 2)? as usize;
                if length < 2 {
                    return None;
                }
                let frame = matches!(marker, 0xC0..=0xCF) && !matches!(marker, 0xC4 | 0xC8 | 0xCC);
                if frame {
                    return Some((be16(bytes, at + 7)?, be16(bytes, at + 5)?));
                }
                at += 2 + length;
            }
        }
    }
}

/// TGA's 18-byte header; an uncompressed one must hold all its pixels.
fn tga(bytes: &[u8]) -> Option<(u32, u32)> {
    let header = bytes.get(..18)?;
    let id_length = usize::from(header[0]);
    let image_type = header[2];
    let bits = header[16];
    if header[1] > 1 || !matches!(image_type, 1 | 2 | 3 | 9 | 10 | 11) {
        return None;
    }
    if !matches!(bits, 8 | 15 | 16 | 24 | 32) {
        return None;
    }
    let width = u32::from(u16::from_le_bytes([header[12], header[13]]));
    let height = u32::from(u16::from_le_bytes([header[14], header[15]]));
    if matches!(image_type, 2 | 3) {
        let pixels = u64::from(width) * u64::from(height) * u64::from(bits).div_ceil(8);
        if (bytes.len() as u64) < 18 + id_length as u64 + pixels {
            return None;
        }
    }
    Some((width, height))
}
