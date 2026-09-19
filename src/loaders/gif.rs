//! What the browser hands `createImageBitmap` for a GIF: the **first** frame,
//! composited onto the logical screen, as 8-bit RGBA, top-down.
//!
//! `TextureLoader` goes through `ImageBitmapLoader` / `<img>` in the browser,
//! and neither animates a texture — a `Texture` made from a GIF is its first
//! frame and nothing else. So this reads the header, the global colour table
//! and the first image descriptor, LZW-expands that one frame, and stops. The
//! rest of the file (further frames, application extensions, the loop count)
//! is not read.
//!
//! GIF is palette + LZW and is exact: unlike the JPEG path there is no inverse
//! DCT to round, so this decode and the browser's agree byte for byte.
//! `webgpu_postprocessing_difference` is the rung that needs it —
//! `textures/crate.gif` is a 256×256 GIF89a with a 256-entry global table.
//!
//! Spec: GIF89a, <https://www.w3.org/Graphics/GIF/spec-gif89a.txt>.

use std::path::Path;

use crate::error::Error;
use crate::textures::Image;

/// A cursor that refuses to run off the end, so a truncated file is an
/// `Error::image` rather than a panic.
struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
    path: &'a Path,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8], path: &'a Path) -> Self {
        Self { bytes, at: 0, path }
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8], Error> {
        let end = self
            .at
            .checked_add(n)
            .filter(|end| *end <= self.bytes.len());
        match end {
            Some(end) => {
                let out = &self.bytes[self.at..end];
                self.at = end;
                Ok(out)
            }
            None => Err(Error::image(self.path, "the GIF ends mid-block")),
        }
    }

    fn u8(&mut self) -> Result<u8, Error> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, Error> {
        let bytes = self.take(2)?;
        Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    /// A chain of length-prefixed sub-blocks, terminated by a zero length,
    /// concatenated — the form both extensions and image data take.
    fn sub_blocks(&mut self) -> Result<Vec<u8>, Error> {
        let mut out = Vec::new();
        loop {
            let len = self.u8()? as usize;
            if len == 0 {
                return Ok(out);
            }
            out.extend_from_slice(self.take(len)?);
        }
    }
}

/// `createImageBitmap( gifBlob )`.
pub fn decode(path: &Path, bytes: &[u8]) -> Result<Image, Error> {
    let mut r = Reader::new(bytes, path);

    let signature = r.take(6)?;
    if &signature[..3] != b"GIF" {
        return Err(Error::image(path, "not a GIF"));
    }

    // Logical screen descriptor. The frame below is composited into this
    // rectangle, which is the size the browser reports for the image.
    let (screen_width, screen_height) = (r.u16()? as usize, r.u16()? as usize);
    let packed = r.u8()?;
    let _background_index = r.u8()?;
    let _pixel_aspect_ratio = r.u8()?;

    let global_palette = if packed & 0b1000_0000 != 0 {
        Some(palette(&mut r, 2usize.pow((packed & 0b111) as u32 + 1))?)
    } else {
        None
    };

    // Areas the first frame does not cover are transparent, as they are in the
    // browser: a GIF's background colour is not painted by `<img>`.
    let mut rgba = vec![0u8; screen_width * screen_height * 4];
    let mut transparent_index: Option<u8> = None;

    loop {
        match r.u8()? {
            // Extension introducer. Only the graphic control extension says
            // anything about the pixels, and only its transparent index.
            0x21 => {
                let label = r.u8()?;
                let block = r.sub_blocks()?;
                if label == 0xF9 && block.len() >= 4 && block[0] & 1 != 0 {
                    transparent_index = Some(block[3]);
                }
            }

            // Image descriptor: the first one is the frame the browser shows.
            0x2C => {
                let (left, top) = (r.u16()? as usize, r.u16()? as usize);
                let (width, height) = (r.u16()? as usize, r.u16()? as usize);
                let packed = r.u8()?;
                let interlaced = packed & 0b0100_0000 != 0;

                let local_palette = if packed & 0b1000_0000 != 0 {
                    Some(palette(&mut r, 2usize.pow((packed & 0b111) as u32 + 1))?)
                } else {
                    None
                };
                let palette = local_palette
                    .as_ref()
                    .or(global_palette.as_ref())
                    .ok_or_else(|| Error::image(path, "the GIF frame has no colour table"))?;

                let min_code_size = r.u8()?;
                let data = r.sub_blocks()?;
                let indices = lzw(path, &data, min_code_size, width * height)?;

                for (n, &index) in indices.iter().enumerate() {
                    let (x, y) = (n % width, deinterlace(n / width, height, interlaced));
                    let (x, y) = (left + x, top + y);
                    if x >= screen_width || y >= screen_height {
                        continue;
                    }
                    if Some(index) == transparent_index {
                        continue;
                    }
                    let entry = 3 * index as usize;
                    let out = 4 * (y * screen_width + x);
                    rgba[out..out + 3].copy_from_slice(
                        palette
                            .get(entry..entry + 3)
                            .ok_or_else(|| Error::image(path, "a GIF index is off its palette"))?,
                    );
                    rgba[out + 3] = 255;
                }

                return Ok(Image {
                    width: screen_width as u32,
                    height: screen_height as u32,
                    data: rgba,
                });
            }

            0x3B => return Err(Error::image(path, "the GIF has no image")),

            other => {
                return Err(Error::image(
                    path,
                    format!("unexpected GIF block 0x{other:02x}"),
                ))
            }
        }
    }
}

fn palette(r: &mut Reader<'_>, entries: usize) -> Result<Vec<u8>, Error> {
    Ok(r.take(entries * 3)?.to_vec())
}

/// GIF's four interlace passes: rows 0, 8, 16 …, then 4, 12 …, then 2, 6 …,
/// then the odd rows. `row` is the index within the decoded stream.
fn deinterlace(row: usize, height: usize, interlaced: bool) -> usize {
    if !interlaced {
        return row;
    }
    let pass = |start: usize, step: usize| height.saturating_sub(start).div_ceil(step);
    let (p0, p1, p2) = (pass(0, 8), pass(4, 8), pass(2, 4));
    if row < p0 {
        row * 8
    } else if row < p0 + p1 {
        4 + (row - p0) * 8
    } else if row < p0 + p1 + p2 {
        2 + (row - p0 - p1) * 4
    } else {
        1 + (row - p0 - p1 - p2) * 2
    }
}

/// The variable-width LZW of the GIF spec: codes start at `min_code_size + 1`
/// bits and grow by one each time the dictionary fills, a clear code resets
/// the dictionary and the width, and the end code stops the stream.
///
/// The dictionary is kept as `(prefix, suffix)` pairs rather than as byte
/// strings, so expanding a code walks the chain backwards and the whole table
/// is two flat arrays.
fn lzw(path: &Path, data: &[u8], min_code_size: u8, pixels: usize) -> Result<Vec<u8>, Error> {
    if !(2..=11).contains(&min_code_size) {
        return Err(Error::image(path, "the GIF LZW code size is out of range"));
    }

    let clear = 1u16 << min_code_size;
    let end = clear + 1;

    let mut prefix = vec![0u16; 4096];
    let mut suffix = vec![0u8; 4096];
    for i in 0..clear {
        suffix[i as usize] = i as u8;
    }

    let reset = |next: &mut u16, width: &mut u32| {
        *next = end + 1;
        *width = min_code_size as u32 + 1;
    };
    let (mut next, mut width) = (0u16, 0u32);
    reset(&mut next, &mut width);

    let mut out = Vec::with_capacity(pixels);
    let mut previous: Option<u16> = None;
    let mut chain = Vec::with_capacity(4096);

    let (mut bits, mut held) = (0u32, 0u32);
    let mut at = 0usize;

    loop {
        while held < width {
            let Some(&byte) = data.get(at) else {
                // The encoder is allowed to stop without an end code once the
                // frame is complete; anything short of it is truncation.
                if out.len() == pixels {
                    return Ok(out);
                }
                return Err(Error::image(path, "the GIF LZW stream ends early"));
            };
            at += 1;
            bits |= (byte as u32) << held;
            held += 8;
        }

        let code = (bits & ((1 << width) - 1)) as u16;
        bits >>= width;
        held -= width;

        if code == clear {
            reset(&mut next, &mut width);
            previous = None;
            continue;
        }
        if code == end {
            return Ok(out);
        }

        // The one self-referential case: a code that is being defined by this
        // very step expands to `previous` plus `previous`'s first byte.
        let first = if code < next {
            code
        } else if let Some(previous) = previous {
            previous
        } else {
            return Err(Error::image(path, "the GIF LZW stream starts with a gap"));
        };

        chain.clear();
        let mut walk = first;
        loop {
            chain.push(suffix[walk as usize]);
            if walk < clear {
                break;
            }
            walk = prefix[walk as usize];
            if chain.len() > 4096 {
                return Err(Error::image(path, "the GIF LZW dictionary is circular"));
            }
        }
        chain.reverse();
        if code >= next {
            let head = chain[0];
            chain.push(head);
        }
        out.extend_from_slice(&chain);

        if let Some(previous) = previous {
            if next < 4096 {
                prefix[next as usize] = previous;
                suffix[next as usize] = chain[0];
                next += 1;
                if next == 1 << width && width < 12 {
                    width += 1;
                }
            }
        }
        previous = Some(code);

        if out.len() >= pixels {
            out.truncate(pixels);
            return Ok(out);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A four-pixel GIF87a written by hand, LZW-encoded with nothing but
    /// literal codes and a clear: red, green, blue, transparent. It pins the
    /// header walk, the palette, the bit packing and the transparent index
    /// without depending on any file in the tree.
    fn tiny_gif() -> Vec<u8> {
        let mut gif = Vec::new();
        gif.extend_from_slice(b"GIF89a");
        gif.extend_from_slice(&2u16.to_le_bytes()); // screen width
        gif.extend_from_slice(&2u16.to_le_bytes()); // screen height
                                                    // A 16-entry global table, so `minCodeSize` 4 leaves the code width
                                                    // at 5 bits for the whole stream — the width growth has its own
                                                    // exercise in `crate.gif`, which is 67 KB of it.
        gif.push(0b1000_0011);
        gif.push(0); // background index
        gif.push(0); // aspect ratio
        gif.extend_from_slice(&[
            255, 0, 0, // 0 red
            0, 255, 0, // 1 green
            0, 0, 255, // 2 blue
            9, 9, 9, // 3 the transparent entry, whose colour is never used
        ]);
        gif.extend_from_slice(&[0; 12 * 3]); // 4..15, unused

        // Graphic control extension: index 3 is transparent.
        gif.extend_from_slice(&[0x21, 0xF9, 0x04, 0b0000_0001, 0, 0, 3, 0x00]);

        // Image descriptor: the whole screen, not interlaced, no local table.
        gif.push(0x2C);
        gif.extend_from_slice(&0u16.to_le_bytes());
        gif.extend_from_slice(&0u16.to_le_bytes());
        gif.extend_from_slice(&2u16.to_le_bytes());
        gif.extend_from_slice(&2u16.to_le_bytes());
        gif.push(0);

        // minCodeSize 4, so codes are 5 bits: clear( 16 ), 0, 1, 2, 3,
        // end( 17 ).
        gif.push(4);
        let codes = [16u16, 0, 1, 2, 3, 17];
        let (mut bits, mut held, mut packed) = (0u32, 0u32, Vec::new());
        for code in codes {
            bits |= (code as u32) << held;
            held += 5;
            while held >= 8 {
                packed.push(bits as u8);
                bits >>= 8;
                held -= 8;
            }
        }
        if held > 0 {
            packed.push(bits as u8);
        }
        gif.push(packed.len() as u8);
        gif.extend_from_slice(&packed);
        gif.push(0x00); // block terminator
        gif.push(0x3B); // trailer
        gif
    }

    #[test]
    fn the_first_frame_is_palette_lookups_and_a_transparent_index() {
        let image = decode(Path::new("<tiny>"), &tiny_gif()).unwrap();
        assert_eq!((image.width, image.height), (2, 2));
        assert_eq!(
            image.data,
            vec![
                255, 0, 0, 255, // red
                0, 255, 0, 255, // green
                0, 0, 255, 255, // blue
                0, 0, 0, 0, // transparent: not the palette's 9,9,9
            ]
        );
    }

    #[test]
    fn a_truncated_gif_is_an_error_not_a_panic() {
        let full = tiny_gif();
        for cut in [3, 10, 20, full.len() - 4] {
            assert!(decode(Path::new("<tiny>"), &full[..cut]).is_err());
        }
    }
}
