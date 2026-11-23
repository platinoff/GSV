//! GASV-G37 — minimal, dependency-light PNG input for the Q4_K vision lane.
//!
//! Uses `flate2` (already in the lock file) to decode non-interlaced 8-bit
//! RGB/RGBA PNGs, then patchifies to the projector's native `patch_size`.
//! No Python, no external tools, no new native dependency.

use std::path::Path;

use flate2::read::ZlibDecoder;
use std::io::Read;

pub struct RgbImage {
    pub width: usize,
    pub height: usize,
    pub rgb: Vec<u8>,
}

fn paeth(a: u8, b: u8, c: u8) -> u8 {
    let p = a as i32 + b as i32 - c as i32;
    let pa = (p - a as i32).abs();
    let pb = (p - b as i32).abs();
    let pc = (p - c as i32).abs();
    if pa <= pb && pa <= pc {
        a
    } else if pb <= pc {
        b
    } else {
        c
    }
}

/// Decode a non-interlaced 8-bit RGB/RGBA PNG into packed RGB bytes.
pub fn decode_png(path: &Path) -> Result<RgbImage, String> {
    let data = std::fs::read(path).map_err(|e| format!("png read: {e}"))?;
    if data.len() < 33 || data[..8] != [137, 80, 78, 71, 13, 10, 26, 10] {
        return Err("not a PNG file".to_string());
    }
    let mut pos = 8usize;
    let mut width = 0usize;
    let mut height = 0usize;
    let mut channels = 0usize;
    let mut idat = Vec::new();
    while pos + 8 <= data.len() {
        let len = u32::from_be_bytes(data[pos..pos + 4].try_into().unwrap()) as usize;
        let kind = &data[pos + 4..pos + 8];
        let payload_start = pos + 8;
        if payload_start + len + 4 > data.len() {
            return Err("truncated PNG chunk".to_string());
        }
        let payload = &data[payload_start..payload_start + len];
        match kind {
            b"IHDR" => {
                if payload.len() != 13 {
                    return Err("bad IHDR".to_string());
                }
                width = u32::from_be_bytes(payload[..4].try_into().unwrap()) as usize;
                height = u32::from_be_bytes(payload[4..8].try_into().unwrap()) as usize;
                let depth = payload[8];
                let color = payload[9];
                let interlace = payload[12];
                if depth != 8 || interlace != 0 {
                    return Err(format!(
                        "unsupported PNG depth={depth} interlace={interlace} (need 8 / 0)"
                    ));
                }
                channels = match color {
                    2 => 3,
                    6 => 4,
                    other => return Err(format!("unsupported PNG color type {other}")),
                };
            }
            b"IDAT" => idat.extend_from_slice(payload),
            b"IEND" => break,
            _ => {}
        }
        pos = payload_start + len + 4;
    }
    if width == 0 || height == 0 || channels == 0 || idat.is_empty() {
        return Err("PNG has no IHDR/IDAT".to_string());
    }
    let mut raw = Vec::new();
    ZlibDecoder::new(&idat[..])
        .read_to_end(&mut raw)
        .map_err(|e| format!("png inflate: {e}"))?;
    let stride = width * channels;
    if raw.len() < (stride + 1) * height {
        return Err("PNG pixel data too short".to_string());
    }
    let mut out = vec![0u8; stride * height];
    let mut prev = vec![0u8; stride];
    for y in 0..height {
        let row_start = y * (stride + 1);
        let filter = raw[row_start];
        let src = &raw[row_start + 1..row_start + 1 + stride];
        let dst = &mut out[y * stride..(y + 1) * stride];
        for x in 0..stride {
            let a = if x >= channels { dst[x - channels] } else { 0 };
            let b = prev[x];
            let c = if x >= channels { prev[x - channels] } else { 0 };
            dst[x] = match filter {
                0 => src[x],
                1 => src[x].wrapping_add(a),
                2 => src[x].wrapping_add(b),
                3 => src[x].wrapping_add(((a as u16 + b as u16) / 2) as u8),
                4 => src[x].wrapping_add(paeth(a, b, c)),
                other => return Err(format!("unsupported PNG filter {other}")),
            };
        }
        prev.copy_from_slice(dst);
    }
    let rgb = if channels == 3 {
        out
    } else {
        out.as_chunks::<4>()
            .0
            .iter()
            .flat_map(|p| [p[0], p[1], p[2]])
            .collect()
    };
    Ok(RgbImage { width, height, rgb })
}

/// Nearest-neighbour resize to `size x size`, then sample `count` patches.
/// Pixel values are mapped to the [-1, 1] range used by CLIP preprocessing
/// (image_mean = image_std = 0.5 in both Inkling-2 and MiniCPM projectors).
pub fn patch_tokens(
    img: &RgbImage,
    size: usize,
    patch: usize,
    count: usize,
) -> Result<Vec<Vec<f64>>, String> {
    if size == 0 || patch == 0 || !size.is_multiple_of(patch) || count == 0 {
        return Err("bad patch geometry".to_string());
    }
    let patches_per_side = size / patch;
    if count > patches_per_side * patches_per_side {
        return Err(format!(
            "only {} patches available",
            patches_per_side * patches_per_side
        ));
    }
    let mut xf = vec![0u8; size * size * 3];
    for y in 0..size {
        let sy = (y * img.height / size).min(img.height - 1);
        for x in 0..size {
            let sx = (x * img.width / size).min(img.width - 1);
            let s = (sy * img.width + sx) * 3;
            let d = (y * size + x) * 3;
            xf[d..d + 3].copy_from_slice(&img.rgb[s..s + 3]);
        }
    }
    // Centre patch first, then remaining patches in row-major order.
    let c = patches_per_side / 2;
    let mut order: Vec<(usize, usize)> = Vec::new();
    order.push((c, c));
    for py in 0..patches_per_side {
        for px in 0..patches_per_side {
            if (px, py) != (c, c) {
                order.push((px, py));
            }
        }
    }
    let mut tokens = Vec::new();
    for (px, py) in order.into_iter().take(count) {
        let mut token = Vec::with_capacity(patch * patch * 3);
        for y in 0..patch {
            for x in 0..patch {
                let sx = px * patch + x;
                let sy = py * patch + y;
                for ch in 0..3 {
                    let v = xf[(sy * size + sx) * 3 + ch] as f64;
                    token.push(v / 255.0 * 2.0 - 1.0);
                }
            }
        }
        tokens.push(token);
    }
    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::write::ZlibEncoder;
    use flate2::Compression;
    use std::io::Write;

    fn png_rgb(width: usize, height: usize, rgb: &[u8]) -> Vec<u8> {
        let mut out = vec![137, 80, 78, 71, 13, 10, 26, 10];
        let mut body = Vec::new();
        body.extend_from_slice(&(width as u32).to_be_bytes());
        body.extend_from_slice(&(height as u32).to_be_bytes());
        body.extend_from_slice(&[8, 2, 0, 0, 0]);
        out.extend_from_slice(&(13u32).to_be_bytes());
        out.extend_from_slice(b"IHDR");
        out.extend_from_slice(&body);
        out.extend_from_slice(&[0; 4]);
        let mut raw = Vec::new();
        for y in 0..height {
            raw.push(0u8);
            raw.extend_from_slice(&rgb[y * width * 3..(y + 1) * width * 3]);
        }
        let mut enc = ZlibEncoder::new(Vec::new(), Compression::fast());
        enc.write_all(&raw).unwrap();
        let compressed = enc.finish().unwrap();
        out.extend_from_slice(&(compressed.len() as u32).to_be_bytes());
        out.extend_from_slice(b"IDAT");
        out.extend_from_slice(&compressed);
        out.extend_from_slice(&[0; 4]);
        out.extend_from_slice(&0u32.to_be_bytes());
        out.extend_from_slice(b"IEND");
        out.extend_from_slice(&[0; 4]);
        out
    }

    #[test]
    fn decodes_rgb_png_and_patchifies() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("vision_test_rgb.png");
        let rgb: Vec<u8> = (0..4 * 4 * 3).map(|i| (i * 7) as u8).collect();
        std::fs::write(&path, png_rgb(4, 4, &rgb)).unwrap();
        let img = decode_png(&path).unwrap();
        assert_eq!((img.width, img.height), (4, 4));
        assert_eq!(img.rgb, rgb);
        let tokens = patch_tokens(&img, 4, 2, 4).unwrap();
        assert_eq!(tokens.len(), 4);
        assert_eq!(tokens[0].len(), 2 * 2 * 3);
        let _ = std::fs::remove_file(&path);
    }
}
