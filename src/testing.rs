//! The determinism the e2e harness injects into the page
//! (`three.js/test/e2e/deterministic-injection.js`), so the ported example can
//! reproduce the reference screenshot, and the harness's image comparison
//! itself ([`compare()`](crate::testing::compare)), so a gate outside this crate grades
//! the same way.

mod strip;

#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;
#[cfg(not(target_arch = "wasm32"))]
use std::process::Command;

pub use strip::{strip, Step, Strip, StripFrame};

/// `Math.random` as the grader replaces it:
///
/// ```js
/// let seed = Math.PI / 4;
/// window.Math.random = function () {
///     const x = Math.sin( seed ++ ) * 10000;
///     return x - Math.floor( x );
/// };
/// ```
///
/// The arithmetic is f64 throughout, matching JavaScript numbers.
#[derive(Debug, Clone)]
pub struct DeterministicRandom {
    seed: f64,
}

impl Default for DeterministicRandom {
    fn default() -> Self {
        Self::new()
    }
}

impl DeterministicRandom {
    pub fn new() -> Self {
        Self {
            seed: std::f64::consts::PI / 4.0,
        }
    }

    /// Consume `n` draws without using them — what any `Math.random()` call the
    /// page makes *before* the graded frame does to the shared sequence.
    pub fn skip(&mut self, n: usize) {
        for _ in 0..n {
            self.next();
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> f64 {
        let x = self.seed.sin() * 10000.0;
        self.seed += 1.0;
        x - x.floor()
    }
}

/// The harness' clock injection: pins [`crate::utils::now_ms`] (`performance
/// .now()`) and [`crate::utils::date_now_ms`] (`Date.now()`) to `ms`, or
/// restores the real clocks with `None`.
///
/// This is three.js' `test/e2e/deterministic-injection.js`:
///
/// ```js
/// Date.now = () => 0;
/// performance.now = () => 0;
/// ```
///
/// Every ported example's `main()` calls `pin_time( Some( 0.0 ) )`, and so
/// does each rung of `tests/e2e`, which is why an `animate()` that reads the
/// clock faithfully still produces exactly the frame the reference screenshot
/// holds. A viewer or a browser leaves it unset and the same `animate()`
/// animates.
///
/// The override is per thread: the e2e harness runs its rungs on Rust's test
/// threads and each pins its own.
pub fn pin_time(ms: Option<f64>) {
    crate::utils::time::pin(ms);
}

/// Writes RGBA8 pixels as a PNG — the same container `page.screenshot()`
/// produces, so the comparator's decode path is identical.
///
/// Not gated to native, unlike [`compare`], although a browser has nowhere to
/// write to: every ported example's `main()` calls it, and the browser shell
/// compiles those examples as modules (`#[path = "../../examples/<name>.rs"]`)
/// exactly as the viewer and the e2e harness do. `std::fs` compiles for
/// wasm32-unknown-unknown and fails at run time, which is the right shape here
/// — nothing in a page ever reaches an example's `main()`.
pub fn write_png(path: &str, width: u32, height: u32, pixels: &[u8]) {
    if let Some(parent) = std::path::Path::new(path).parent() {
        std::fs::create_dir_all(parent).ok();
    }

    let file = std::fs::File::create(path).expect("three-rs: cannot create PNG");
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().expect("three-rs: PNG header");
    writer.write_image_data(pixels).expect("three-rs: PNG data");
}

/// What `page.screenshot()` sees where the canvas is not opaque: Chrome
/// composites the canvas — `alphaMode: 'premultiplied'`, which three.js asks
/// for whenever the renderer's `alpha` is on, the default — over the page
/// behind it, `source-over` on 8-bit sRGB values.
///
/// Every rung so far either covers the whole canvas or sits on
/// `example.css`'s black body, where the composite changes nothing. A page
/// that leaves the canvas transparent over a coloured `<body>` (its own
/// `<style>`, not the reference image, gives the colour) needs this before
/// its frame is graded: `webgpu_textures_anisotropy` clears to
/// `( 0, 0, 0, 0 )` and draws the floor only below the horizon.
pub fn composite_over_page(pixels: &mut [u8], page_background: u32) {
    let page = [
        (page_background >> 16) & 0xff,
        (page_background >> 8) & 0xff,
        page_background & 0xff,
    ];
    for pixel in pixels.as_chunks_mut::<4>().0 {
        let transparency = 255 - u32::from(pixel[3]);
        for (channel, &behind) in pixel.iter_mut().zip(&page) {
            let over = u32::from(*channel) + (behind * transparency + 127) / 255;
            *channel = over.min(255) as u8;
        }
        pixel[3] = 255;
    }
}

/// A vendored upstream checkout the examples and tests read from: `env_var` if
/// set, else `$HOME/src/vendor/<name>`. Nothing under `src/` reads these; only
/// the examples (their textures and models come from Three's own `examples/`)
/// and the tests (Three's e2e reference screenshots) do.
#[cfg(not(target_arch = "wasm32"))]
pub fn vendor_dir(env_var: &str, name: &str) -> std::path::PathBuf {
    match std::env::var(env_var) {
        Ok(dir) => std::path::PathBuf::from(dir),
        Err(_) => std::path::PathBuf::from(std::env::var("HOME").expect("HOME"))
            .join("src/vendor")
            .join(name),
    }
}

/// A fixed virtual root in a browser, where there is no checkout, no `$HOME`
/// and no environment to read one out of.
///
/// The paths the examples build under it are never opened: they are the keys
/// [`crate::io::preload`] is called with, so all that matters is that the host
/// and the example agree, and they agree by both going through this function.
/// The browser shell derives each key as
/// `three_js_dir().join(<path relative to the three.js checkout>)`, exactly the
/// relative paths a native run recorded into the example's manifest (#128).
#[cfg(target_arch = "wasm32")]
pub fn vendor_dir(_env_var: &str, name: &str) -> std::path::PathBuf {
    std::path::PathBuf::from("/vendor").join(name)
}

/// The three.js checkout (`THREE_JS_DIR`), at the commit CI pins
/// (`THREE_JS_REV` in `.github/workflows/ci.yml`). See `docs/grader.md`.
pub fn three_js_dir() -> std::path::PathBuf {
    vendor_dir("THREE_JS_DIR", "three.js")
}

/// What `test/e2e/puppeteer.js`'s `checkFile()` computes for one frame.
#[derive(Debug, Clone, PartialEq)]
pub struct Comparison {
    pub width: u32,
    pub height: u32,
    pub num_different_pixels: u64,
    /// `num_different_pixels` as a percentage of the frame.
    pub different_pixels: f64,
    /// The grader's ceiling on `different_pixels`, 0.1.
    pub max_different_pixels: f64,
    pub pass: bool,
}

/// Runs three.js' own comparator over a rendered frame, unchanged: `actual`
/// is a PNG at the harness's `viewScale` of 2 (what [`write_png`] writes from
/// an 800 × 500 canvas), `expected` a reference JPEG at 1×, and `out` is where
/// `actual.jpg`, `expected.jpg` and `diff.jpg` land for a look. `image.js`'s
/// `scale()` and `compare()` come from [`three_js_dir`] by shelling out to
/// `node`, so no second implementation of the comparator exists on this side.
///
/// Native only: it shells out to `node` against a three.js checkout, and a
/// browser build has neither. Grading a browser frame is issue #128's fourth
/// item, and it runs the comparator *outside* the page.
#[cfg(not(target_arch = "wasm32"))]
pub fn compare(actual: &Path, expected: &Path, out: &Path) -> Comparison {
    assert!(
        expected.exists(),
        "reference screenshot missing: {}",
        expected.display()
    );
    std::fs::create_dir_all(out).expect("three-rs: cannot create the comparison directory");

    let script = out.join("compare.mjs");
    std::fs::write(&script, COMPARE_MJS).expect("three-rs: cannot write compare.mjs");

    let output = Command::new("node")
        .arg(&script)
        .arg(three_js_dir())
        .arg(actual)
        .arg(expected)
        .arg(out)
        .output()
        .expect("three-rs: failed to run node");

    assert!(
        output.status.success(),
        "comparator failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("three-rs: the comparator's JSON");
    let get = |key: &str| {
        json.get(key)
            .and_then(serde_json::Value::as_f64)
            .unwrap_or_else(|| panic!("three-rs: the comparator's JSON has no numeric `{key}`"))
    };

    Comparison {
        width: get("width") as u32,
        height: get("height") as u32,
        num_different_pixels: get("numDifferentPixels") as u64,
        different_pixels: get("differentPixels"),
        max_different_pixels: get("maxDifferentPixels"),
        pass: json.get("pass").and_then(serde_json::Value::as_bool) == Some(true),
    }
}

/// The script [`compare`] runs; written next to the images it produces.
#[cfg(not(target_arch = "wasm32"))]
const COMPARE_MJS: &str = include_str!("testing/compare.mjs");
