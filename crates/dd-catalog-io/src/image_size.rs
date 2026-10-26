//! Image dimensions from the first bytes of a texture, without decoding it.

/// Enough bytes to read the dimensions of every supported format.
pub const HEADER_BYTES: u64 = 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PixelSize {
    pub width: u32,
    pub height: u32,
}

/// Dimensions of a WebP, PNG or Godot 3 `.stex` texture, from its header.
#[must_use]
pub fn image_size(header: &[u8]) -> Option<PixelSize> {
    if header.starts_with(b"RIFF") && header.get(8..12) == Some(b"WEBP") {
        return webp_size(header);
    }
    if header.starts_with(b"\x89PNG\r\n\x1a\n") {
        return png_size(header);
    }
    if header.starts_with(b"GDST") {
        return stream_texture_size(header);
    }
    None
}

/// The first chunk tells the bitstream; its data starts at byte 20.
fn webp_size(header: &[u8]) -> Option<PixelSize> {
    const FOURTEEN_BITS: u32 = 0x3fff;
    match header.get(12..16)? {
        b"VP8X" => Some(PixelSize {
            width: 1 + u24_le(header, 24)?,
            height: 1 + u24_le(header, 27)?,
        }),
        b"VP8 " => Some(PixelSize {
            width: u32::from(u16_le(header, 26)?) & FOURTEEN_BITS,
            height: u32::from(u16_le(header, 28)?) & FOURTEEN_BITS,
        }),
        b"VP8L" => {
            let bits = u32_le(header, 21)?;
            Some(PixelSize {
                width: 1 + (bits & FOURTEEN_BITS),
                height: 1 + ((bits >> 14) & FOURTEEN_BITS),
            })
        }
        _ => None,
    }
}

/// Width and height open the `IHDR` chunk, right after the signature.
fn png_size(header: &[u8]) -> Option<PixelSize> {
    Some(PixelSize {
        width: u32::from_be_bytes(header.get(16..20)?.try_into().ok()?),
        height: u32::from_be_bytes(header.get(20..24)?.try_into().ok()?),
    })
}

/// Godot 3 `StreamTexture`: stored width and height, each followed by the
/// original one when the texture was resized on import.
fn stream_texture_size(header: &[u8]) -> Option<PixelSize> {
    let original_or_stored =
        |stored: u16, original: u16| u32::from(if original == 0 { stored } else { original });
    Some(PixelSize {
        width: original_or_stored(u16_le(header, 4)?, u16_le(header, 6)?),
        height: original_or_stored(u16_le(header, 8)?, u16_le(header, 10)?),
    })
}

fn u16_le(bytes: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(bytes.get(at..at + 2)?.try_into().ok()?))
}

fn u24_le(bytes: &[u8], at: usize) -> Option<u32> {
    let [low, middle, high] = bytes.get(at..at + 3)?.try_into().ok()?;
    Some(u32::from_le_bytes([low, middle, high, 0]))
}

fn u32_le(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}

#[cfg(test)]
mod tests {
    use super::{PixelSize, image_size};

    fn size(width: u32, height: u32) -> PixelSize {
        PixelSize { width, height }
    }

    fn webp(chunk: [u8; 4], data: &[u8]) -> Vec<u8> {
        let mut header = b"RIFF\0\0\0\0WEBP".to_vec();
        header.extend(chunk);
        header.extend([0, 0, 0, 0]);
        header.extend(data);
        header.resize(30, 0);
        header
    }

    #[test]
    fn reads_extended_webp() {
        let data = [0, 0, 0, 0, 0xc3, 0x01, 0x00, 0x85, 0x01, 0x00];
        assert_eq!(image_size(&webp(*b"VP8X", &data)), Some(size(452, 390)));
    }

    #[test]
    fn reads_lossy_webp() {
        let data = [0, 0, 0, 0x9d, 0x01, 0x2a, 0x00, 0x02, 0x00, 0x01];
        assert_eq!(image_size(&webp(*b"VP8 ", &data)), Some(size(512, 256)));
    }

    #[test]
    fn reads_lossless_webp() {
        let bits: u32 = (512 - 1) | ((256 - 1) << 14);
        let mut data = vec![0x2f];
        data.extend(bits.to_le_bytes());
        assert_eq!(image_size(&webp(*b"VP8L", &data)), Some(size(512, 256)));
    }

    #[test]
    fn reads_png() {
        let mut header = b"\x89PNG\r\n\x1a\n\0\0\0\x0dIHDR".to_vec();
        header.extend(300_u32.to_be_bytes());
        header.extend(150_u32.to_be_bytes());
        assert_eq!(image_size(&header), Some(size(300, 150)));
    }

    #[test]
    fn reads_godot_stream_textures_preferring_the_original_size() {
        let mut header = b"GDST".to_vec();
        for field in [87_u16, 0, 88, 0] {
            header.extend(field.to_le_bytes());
        }
        assert_eq!(image_size(&header), Some(size(87, 88)));
        let mut resized = b"GDST".to_vec();
        for field in [128_u16, 100, 64, 60] {
            resized.extend(field.to_le_bytes());
        }
        assert_eq!(image_size(&resized), Some(size(100, 60)));
    }

    #[test]
    fn unknown_or_short_headers_have_no_size() {
        assert_eq!(image_size(b"GIF89a"), None);
        assert_eq!(image_size(b"RIFF\0\0\0\0WEBPVP8X"), None);
        assert_eq!(image_size(b""), None);
    }
}
