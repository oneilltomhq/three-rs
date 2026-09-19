//! A film strip: render a scene through a list of steps, keep one frame and
//! one [`Info`] per step, tile them into a PNG with the counts in the gutter
//! (issue #68).
//!
//! The pixel grader proves one frame; issue #57 bounds the time of frames two
//! and three; issue #67 counts what each frame built. This ties the three
//! together into one record, so that redundant work shows up beside the
//! picture instead of only in a consumer's frame rate. The tiling, the gutter,
//! the byte compare and the steady assertion are the same for every consumer
//! that toggles anything; only the steps are the application's.
//!
//! ```no_run
//! # use three_rs::{Renderer, Scene, PerspectiveCamera};
//! # struct App { renderer: Renderer, scene: Scene, camera: PerspectiveCamera }
//! # fn animate(app: &mut App) {}
//! # let mut app = App {
//! #     renderer: Renderer::new(Default::default()).unwrap(),
//! #     scene: Scene::new(),
//! #     camera: PerspectiveCamera::new(60.0, 1.0, 0.1, 100.0),
//! # };
//! let strip = three_rs::testing::strip(
//!     &mut app,
//!     |app| &mut app.renderer,
//!     &mut animate,
//!     &mut [("", &mut |_: &mut App| {}), ("", &mut |_: &mut App| {})],
//! )
//! .unwrap();
//!
//! strip.assert_steady(1..);
//! strip.write_png("target/e2e/strip.png");
//! ```

use std::ops::{Bound, RangeBounds};
use std::time::{Duration, Instant};

use crate::error::Error;
use crate::renderer::{Info, Renderer};

/// One render: its label, what it drew, and what it cost.
pub struct StripFrame {
    /// The step that produced it; the first frame, rendered before any step
    /// ran, has none and carries `""`.
    pub label: String,
    pub width: u32,
    pub height: u32,
    /// `Renderer::read_canvas_pixels()`, RGBA8.
    pub pixels: Vec<u8>,
    /// `Renderer::info()` for this render alone — the strip resets it per
    /// frame, so a frame that is several `render()` calls counts as one.
    pub info: Info,
    /// The frame from the first line of `render` to the GPU finishing it.
    /// Deliberately not drawn into the PNG: it is not deterministic, so it is
    /// not part of a golden. See [`Strip::times`].
    pub time: Duration,
}

/// The frames [`strip`] rendered, in order.
pub struct Strip {
    pub frames: Vec<StripFrame>,
}

/// One step: a label, and what it does to the scene before the next render.
pub type Step<'a, A> = (&'a str, &'a mut dyn FnMut(&mut A));

/// Renders `app` once, then once after each of `steps`.
///
/// The issue's signature is `strip( renderer, scene, camera, steps )`. The
/// rungs do not have that shape: `webgpu_rtt` and `webgpu_depth_texture` render
/// into a target and then draw a full-screen quad, `webgpu_postprocessing_masking`
/// has three scenes, three `PassNode`s and a `RenderPipeline`, and the viewer's
/// scenes carry their own clock. What every one of them does have is an `App`
/// and a function that renders one frame of it, so that is what this takes:
/// the app, how to reach its renderer, how to render a frame, and the steps.
/// A caller whose app really is a scene and a camera passes a two-line
/// closure.
///
/// `info.auto_reset` is turned off for the duration and reset once per frame,
/// so each [`StripFrame::info`] covers the whole frame however many `render()`
/// calls it took; the renderer's own setting is put back at the end.
pub fn strip<A>(
    app: &mut A,
    renderer: fn(&mut A) -> &mut Renderer,
    render: &mut dyn FnMut(&mut A),
    steps: &mut [Step<'_, A>],
) -> Result<Strip, Error> {
    let auto_reset = renderer(app).info().auto_reset;
    renderer(app).info_mut().auto_reset = false;

    let mut frames = Vec::with_capacity(steps.len() + 1);
    let mut result = capture(app, renderer, render, "").map(|frame| frames.push(frame));

    for (label, step) in steps.iter_mut() {
        if result.is_err() {
            break;
        }
        step(app);
        result = capture(app, renderer, render, label).map(|frame| frames.push(frame));
    }

    renderer(app).info_mut().auto_reset = auto_reset;
    result?;
    Ok(Strip { frames })
}

/// One frame: reset the counts, render, wait for the GPU, read it back.
fn capture<A>(
    app: &mut A,
    renderer: fn(&mut A) -> &mut Renderer,
    render: &mut dyn FnMut(&mut A),
    label: &str,
) -> Result<StripFrame, Error> {
    renderer(app).info_mut().reset();

    let started = Instant::now();
    render(app);
    renderer(app)
        .device()
        .poll(wgpu::PollType::wait_indefinitely())
        .map_err(|e| Error::Readback {
            reason: e.to_string(),
        })?;
    let time = started.elapsed();

    let info = renderer(app).info().clone();
    let (width, height, pixels) = renderer(app).read_canvas_pixels()?;

    Ok(StripFrame {
        label: label.to_string(),
        width,
        height,
        pixels,
        info,
        time,
    })
}

impl Strip {
    /// What each frame took, for a caller that wants to bound it. Times are
    /// not part of the PNG.
    pub fn times(&self) -> Vec<Duration> {
        self.frames.iter().map(|frame| frame.time).collect()
    }

    /// Every frame in `range` compiled, built and uploaded nothing — the
    /// steady-frame assertion of issues #56, #57 and #67, as an equality.
    ///
    /// # Panics
    ///
    /// If any frame in the range built anything, or the range runs past the
    /// end of the strip.
    pub fn assert_steady(&self, range: impl RangeBounds<usize>) {
        self.assert_steady_uploading(range, 0);
    }

    /// [`Self::assert_steady`] for a scene whose per-frame data *is* an
    /// upload: a `BatchedMesh` rewrites its indirect texture on every
    /// `onBeforeRender()` and its matrices texture whenever an instance moves,
    /// exactly as three.js does, so `textures` of them per frame are steady.
    /// Everything else must still build nothing.
    ///
    /// # Panics
    ///
    /// If any frame in the range built anything else, uploaded a different
    /// number of textures, or the range runs past the end of the strip.
    pub fn assert_steady_uploading(&self, range: impl RangeBounds<usize>, textures: u64) {
        let start = match range.start_bound() {
            Bound::Included(index) => *index,
            Bound::Excluded(index) => index + 1,
            Bound::Unbounded => 0,
        };
        let end = match range.end_bound() {
            Bound::Included(index) => index + 1,
            Bound::Excluded(index) => *index,
            Bound::Unbounded => self.frames.len(),
        };
        assert!(
            end <= self.frames.len(),
            "three-rs: the strip has {} frames, not {end}",
            self.frames.len()
        );

        for (index, frame) in self.frames.iter().enumerate().take(end).skip(start) {
            let build = &frame.info.build;
            assert_eq!(
                (
                    build.total() - build.textures_uploaded,
                    build.textures_uploaded
                ),
                (0, textures),
                "frame {index} ({}) built {build:?}; a steady frame must build \
                 nothing beyond {textures} texture upload(s)",
                frame.label,
            );
        }
    }

    /// The frames tiled left to right at 1:1, each with a gutter under it
    /// carrying its label and its counts.
    ///
    /// The result is a pure function of the pixels, the labels and the counts,
    /// so it is a golden a consumer can byte-compare; the frame times are not
    /// drawn, which is why.
    ///
    /// # Panics
    ///
    /// If the strip is empty, or the PNG cannot be written.
    pub fn write_png(&self, path: &str) {
        assert!(!self.frames.is_empty(), "three-rs: the strip has no frames");

        let width: u32 = self.frames.iter().map(|frame| frame.width).sum();
        let tallest = self
            .frames
            .iter()
            .map(|frame| frame.height)
            .max()
            .expect("three-rs: the strip has frames");
        let height = tallest + GUTTER;

        let mut canvas = vec![0u8; (width * height * 4) as usize];
        for pixel in canvas.as_chunks_mut::<4>().0.iter_mut() {
            *pixel = BACKGROUND;
        }

        let mut x = 0;
        for (index, frame) in self.frames.iter().enumerate() {
            blit(&mut canvas, width, x, 0, frame);
            gutter(&mut canvas, width, x, tallest, frame.width, index, frame);
            x += frame.width;
        }

        super::write_png(path, width, height, &canvas);
    }
}

/// One frame's pixels at `( x, y )` of the canvas.
fn blit(canvas: &mut [u8], canvas_width: u32, x: u32, y: u32, frame: &StripFrame) {
    for row in 0..frame.height {
        let from = (row * frame.width * 4) as usize;
        let to = ((y + row) * canvas_width + x) as usize * 4;
        let bytes = (frame.width * 4) as usize;
        canvas[to..to + bytes].copy_from_slice(&frame.pixels[from..from + bytes]);
    }
}

/// The three lines under one frame: what it is, what it drew, what it built.
fn gutter(
    canvas: &mut [u8],
    canvas_width: u32,
    x: u32,
    y: u32,
    cell_width: u32,
    index: usize,
    frame: &StripFrame,
) {
    let label = if frame.label.is_empty() {
        format!("#{index}")
    } else {
        format!("#{index} {}", frame.label)
    };
    let render = &frame.info.render;
    let build = &frame.info.build;
    let lines = [
        label,
        format!(
            "CALLS {} TRI {} LINES {}",
            render.calls, render.triangles, render.lines
        ),
        format!(
            "PROG {} PIPE {} GEO {} BUF {} TEX {}",
            build.programs_compiled,
            build.pipelines_built,
            build.geometries_uploaded,
            build.buffers_written,
            build.textures_uploaded,
        ),
    ];

    // A built line that is not zero is what the reader is looking for, so it
    // is drawn in the warning colour.
    let colours = [LABEL, TEXT, if build.total() == 0 { TEXT } else { BUILT }];

    let room = (cell_width.saturating_sub(PAD * 2) / ADVANCE) as usize;
    for (line, (text, colour)) in lines.iter().zip(colours).enumerate() {
        let text: String = text.chars().take(room).collect();
        draw_text(
            canvas,
            canvas_width,
            x + PAD,
            y + PAD + line as u32 * LINE,
            &text,
            colour,
        );
    }
}

/// The gutter under every frame: three lines of [`SCALE`]d 3x5 text.
const GUTTER: u32 = PAD * 2 + LINE * 3;
const PAD: u32 = 4;
const LINE: u32 = GLYPH_HEIGHT * SCALE + 4;
/// Text is drawn at 1:1 beside frames that are hundreds of pixels wide, so the
/// font is scaled rather than resampled — every pixel of a glyph is a
/// `SCALE`x`SCALE` block, which keeps the PNG exact.
const SCALE: u32 = 2;
const GLYPH_WIDTH: u32 = 3;
const GLYPH_HEIGHT: u32 = 5;
/// One character cell: the glyph plus a one-pixel gap, scaled.
const ADVANCE: u32 = (GLYPH_WIDTH + 1) * SCALE;

const BACKGROUND: [u8; 4] = [24, 24, 24, 255];
const LABEL: [u8; 4] = [255, 255, 255, 255];
const TEXT: [u8; 4] = [170, 170, 170, 255];
/// A frame that built something: the number the strip exists to show.
const BUILT: [u8; 4] = [255, 176, 64, 255];

/// `text` at `( x, y )`, one [`glyph`] per character.
fn draw_text(canvas: &mut [u8], canvas_width: u32, x: u32, y: u32, text: &str, colour: [u8; 4]) {
    for (index, character) in text.chars().enumerate() {
        let rows = glyph(character);
        let origin = x + index as u32 * ADVANCE;
        for (row, bits) in rows.iter().enumerate() {
            for column in 0..GLYPH_WIDTH {
                // The leftmost pixel is the high bit of the three.
                if bits & (1 << (GLYPH_WIDTH - 1 - column)) == 0 {
                    continue;
                }
                for dy in 0..SCALE {
                    for dx in 0..SCALE {
                        let px = origin + column * SCALE + dx;
                        let py = y + row as u32 * SCALE + dy;
                        let at = ((py * canvas_width + px) * 4) as usize;
                        if px < canvas_width && at + 4 <= canvas.len() {
                            canvas[at..at + 4].copy_from_slice(&colour);
                        }
                    }
                }
            }
        }
    }
}

/// A 3x5 bitmap font, one `u8` per row with the glyph in the low three bits.
///
/// A built-in font rather than a dependency or a system lookup: the gutter has
/// to render the same bytes on every machine for the PNG to be a golden, and
/// the whole alphabet it needs is upper case, digits and a handful of marks.
/// Text is upper-cased on the way in and anything else becomes `?`.
fn glyph(character: char) -> [u8; GLYPH_HEIGHT as usize] {
    match character.to_ascii_uppercase() {
        ' ' => [0b000, 0b000, 0b000, 0b000, 0b000],
        'A' => [0b111, 0b101, 0b111, 0b101, 0b101],
        'B' => [0b110, 0b101, 0b110, 0b101, 0b110],
        'C' => [0b111, 0b100, 0b100, 0b100, 0b111],
        'D' => [0b110, 0b101, 0b101, 0b101, 0b110],
        'E' => [0b111, 0b100, 0b110, 0b100, 0b111],
        'F' => [0b111, 0b100, 0b110, 0b100, 0b100],
        'G' => [0b111, 0b100, 0b101, 0b101, 0b111],
        'H' => [0b101, 0b101, 0b111, 0b101, 0b101],
        'I' => [0b111, 0b010, 0b010, 0b010, 0b111],
        'J' => [0b001, 0b001, 0b001, 0b101, 0b111],
        'K' => [0b101, 0b101, 0b110, 0b101, 0b101],
        'L' => [0b100, 0b100, 0b100, 0b100, 0b111],
        'M' => [0b111, 0b111, 0b101, 0b101, 0b101],
        'N' => [0b110, 0b101, 0b101, 0b101, 0b101],
        'O' => [0b111, 0b101, 0b101, 0b101, 0b111],
        'P' => [0b111, 0b101, 0b111, 0b100, 0b100],
        'Q' => [0b111, 0b101, 0b101, 0b111, 0b011],
        'R' => [0b111, 0b101, 0b111, 0b110, 0b101],
        'S' => [0b111, 0b100, 0b111, 0b001, 0b111],
        'T' => [0b111, 0b010, 0b010, 0b010, 0b010],
        'U' => [0b101, 0b101, 0b101, 0b101, 0b111],
        'V' => [0b101, 0b101, 0b101, 0b101, 0b010],
        'W' => [0b101, 0b101, 0b101, 0b111, 0b111],
        'X' => [0b101, 0b101, 0b010, 0b101, 0b101],
        'Y' => [0b101, 0b101, 0b010, 0b010, 0b010],
        'Z' => [0b111, 0b001, 0b010, 0b100, 0b111],
        '0' => [0b111, 0b101, 0b101, 0b101, 0b111],
        '1' => [0b010, 0b110, 0b010, 0b010, 0b111],
        '2' => [0b111, 0b001, 0b111, 0b100, 0b111],
        '3' => [0b111, 0b001, 0b111, 0b001, 0b111],
        '4' => [0b101, 0b101, 0b111, 0b001, 0b001],
        '5' => [0b111, 0b100, 0b111, 0b001, 0b111],
        '6' => [0b111, 0b100, 0b111, 0b101, 0b111],
        '7' => [0b111, 0b001, 0b001, 0b001, 0b001],
        '8' => [0b111, 0b101, 0b111, 0b101, 0b111],
        '9' => [0b111, 0b101, 0b111, 0b001, 0b111],
        '#' => [0b101, 0b111, 0b101, 0b111, 0b101],
        '.' => [0b000, 0b000, 0b000, 0b000, 0b010],
        ',' => [0b000, 0b000, 0b000, 0b010, 0b100],
        ':' => [0b000, 0b010, 0b000, 0b010, 0b000],
        '-' => [0b000, 0b000, 0b111, 0b000, 0b000],
        '+' => [0b000, 0b010, 0b111, 0b010, 0b000],
        '=' => [0b000, 0b111, 0b000, 0b111, 0b000],
        '/' => [0b001, 0b001, 0b010, 0b100, 0b100],
        '(' => [0b001, 0b010, 0b010, 0b010, 0b001],
        ')' => [0b100, 0b010, 0b010, 0b010, 0b100],
        _ => [0b111, 0b001, 0b011, 0b000, 0b010],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The gutter is drawn, not measured: a glyph must fit the cell the
    /// advance reserves for it, or two characters overlap.
    #[test]
    fn a_glyph_fits_its_cell() {
        const { assert!(GLYPH_WIDTH * SCALE < ADVANCE) };
        for row in glyph('W') {
            assert_eq!(row & !0b111, 0, "a glyph row is three bits wide");
        }
    }

    #[test]
    fn the_gutter_holds_its_three_lines() {
        assert_eq!(GUTTER, PAD * 2 + LINE * 3);
        const { assert!(GUTTER > GLYPH_HEIGHT * SCALE * 3) };
    }

    /// Anything the font does not have is a question mark, never a panic and
    /// never a blank the reader would take for a space.
    #[test]
    fn an_unknown_character_is_a_question_mark() {
        assert_eq!(glyph('~'), glyph('\u{1f600}'));
        assert_ne!(glyph('~'), glyph(' '));
        assert_eq!(glyph('a'), glyph('A'));
    }
}
