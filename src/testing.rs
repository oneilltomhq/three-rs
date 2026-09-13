//! The determinism the e2e harness injects into the page
//! (`three.js/test/e2e/deterministic-injection.js`), so the ported example can
//! reproduce the reference screenshot.

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

/// Writes RGBA8 pixels as a PNG — the same container `page.screenshot()`
/// produces, so the comparator's decode path is identical.
pub fn write_png(path: &str, width: u32, height: u32, pixels: &[u8]) {
    if let Some(parent) = std::path::Path::new(path).parent() {
        std::fs::create_dir_all(parent).ok();
    }

    let file = std::fs::File::create(path).expect("three-rs: cannot create PNG");
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().expect("three-rs: PNG header");
    writer
        .write_image_data(pixels)
        .expect("three-rs: PNG data");
}

/// A vendored upstream checkout the examples and tests read from: `env_var` if
/// set, else `$HOME/src/vendor/<name>`. Nothing under `src/` reads these; only
/// the examples (their textures and models come from Three's own `examples/`)
/// and the tests (Three's e2e reference screenshots) do.
pub fn vendor_dir(env_var: &str, name: &str) -> std::path::PathBuf {
    match std::env::var(env_var) {
        Ok(dir) => std::path::PathBuf::from(dir),
        Err(_) => std::path::PathBuf::from(std::env::var("HOME").expect("HOME"))
            .join("src/vendor")
            .join(name),
    }
}

/// The three.js checkout (`THREE_JS_DIR`), expected at tag r186 with
/// `handoff/rung0/grader-flags.patch` applied for the e2e tests.
pub fn three_js_dir() -> std::path::PathBuf {
    vendor_dir("THREE_JS_DIR", "three.js")
}
