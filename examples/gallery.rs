//! The examples gallery generator.
//!
//! One tool, three outputs, from one source of truth:
//!
//! * `docs/gallery/<name>.jpg` — a thumbnail of **our own** rendered frame
//!   (`target/e2e/<name>/actual.png`, written by the e2e ladder), committed.
//! * `target/gallery/index.html` — a local browsable index in the spirit of
//!   three.js's own `examples/index.html`: a filter box and a grid of cards,
//!   each pairing our frame with Three's reference screenshot.
//! * the `<!-- gallery:start -->` / `<!-- gallery:end -->` block in
//!   `README.md` — a thumbnail grid with absolute raw-GitHub image URLs, so
//!   it renders on crates.io as well as on GitHub.
//!
//! **Source of truth.** The set of graded examples and their numbers come
//! from the README's "Examples graded green" table. The e2e harness has no
//! machine-readable record — `tests/e2e/main.rs` writes `actual.png`, the
//! reference, the diff strip and the steady-frame strip per example and
//! *prints* the diff count, the steady frame and the `renderer.info()`
//! counts, but never a JSON — so the README table is the only list that is
//! already maintained per rung, and this tool parses it rather than
//! inventing a third one. The table's row order is the rung order, and the
//! gallery keeps it.
//!
//! **Reference images.** Nothing derived from Three's reference screenshots
//! is ever written into the tree. The committed thumbnails are downscales of
//! our own frames. The local `target/gallery/index.html` *does* point at the
//! vendored reference JPEGs, by absolute `file://` path into the three.js
//! checkout — that page lives under `target/`, which is git-ignored, so it
//! never leaves this machine and no reference byte is ever committed.
//!
//! ```sh
//! cargo test --release --test e2e          # fills target/e2e/<name>/
//! cargo run --release --example gallery    # then this
//! cargo run --release --example gallery -- --vendor /path/to/three.js
//! ```

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

/// The repository the README's absolute URLs point into.
const REPO: &str = "https://github.com/oneilltomhq/three-rs";
/// Raw file URLs, so crates.io (which does not rewrite relative links)
/// renders the grid's images.
const RAW: &str = "https://raw.githubusercontent.com/oneilltomhq/three-rs/main";

/// The markers the README grid is regenerated between. A rung worker adding
/// a row to the graded table never touches what is inside them.
const GALLERY_START: &str = "<!-- gallery:start -->";
const GALLERY_END: &str = "<!-- gallery:end -->";
/// The heading the block is inserted under the first time it is written.
const GALLERY_SECTION: &str = "## Examples graded green";

/// Thumbnails: 400 px wide (the graded frame is 800x500, so an exact 2:1
/// area average), JPEG quality 80. That lands each one well under 40 KB.
const THUMB_WIDTH: u32 = 400;
const JPEG_QUALITY: u8 = 80;
/// Cells per row in the README grid.
const COLUMNS: usize = 4;
/// The width the README's `<img>` tags ask for, so four fit across.
const README_IMG_WIDTH: u32 = 200;

// ---------------------------------------------------------------------------
// The source of truth: the README's graded table.
// ---------------------------------------------------------------------------

/// One row of the README's "Examples graded green" table.
#[derive(Debug, Clone, PartialEq)]
struct Row {
    name: String,
    /// The leading integer of the "different pixels" cell.
    diff_pixels: u32,
    /// Whatever the cell says after that integer, e.g. "(Three itself scores
    /// 60 against the same JPEG)". Empty for most rows.
    diff_note: String,
    steady_ms: f64,
    draw_calls: u64,
    /// The triangles cell verbatim; one row says "1 + 300000 points".
    triangles: String,
}

/// Parse the README's "Examples graded green" table.
///
/// Only rows inside that section are considered, and only rows whose first
/// cell is an example name; the header and the `---` separator fall out on
/// their own. A row whose numbers do not parse is skipped and named on
/// stderr rather than guessed at.
fn parse_graded_table(readme: &str) -> Vec<Row> {
    let mut rows = Vec::new();
    let mut in_section = false;

    for line in readme.lines() {
        if let Some(heading) = line.strip_prefix("## ") {
            in_section = format!("## {}", heading.trim()) == GALLERY_SECTION;
            continue;
        }
        if !in_section || !line.trim_start().starts_with('|') {
            continue;
        }

        let cells: Vec<&str> = line
            .trim()
            .trim_matches('|')
            .split('|')
            .map(str::trim)
            .collect();
        if cells.len() != 5 || !cells[0].starts_with("webgpu_") {
            continue;
        }

        // "60 (Three itself scores 60 against the same JPEG)" -> 60 + note.
        let (diff_head, diff_note) = match cells[1].split_once(' ') {
            Some((head, rest)) => (head, rest.trim().to_string()),
            None => (cells[1], String::new()),
        };

        let parsed = (|| {
            Some(Row {
                name: cells[0].to_string(),
                diff_pixels: diff_head.parse().ok()?,
                diff_note,
                steady_ms: cells[2].parse().ok()?,
                draw_calls: cells[3].parse().ok()?,
                triangles: cells[4].to_string(),
            })
        })();

        match parsed {
            Some(row) => rows.push(row),
            None => eprintln!("gallery: skipping unparseable table row: {line}"),
        }
    }

    rows
}

/// Splice `block` between the gallery markers, idempotently.
///
/// If the markers are already there, everything between them is replaced. If
/// they are not, the block is inserted just under the graded-examples
/// heading (pictures first, numbers under them). If even that heading is
/// missing, the block is appended.
fn replace_gallery_section(readme: &str, block: &str) -> String {
    let marked = format!("{GALLERY_START}\n{}\n{GALLERY_END}", block.trim_end());

    if let (Some(start), Some(end)) = (readme.find(GALLERY_START), readme.find(GALLERY_END)) {
        if start < end {
            let mut out = String::with_capacity(readme.len() + marked.len());
            out.push_str(&readme[..start]);
            out.push_str(&marked);
            out.push_str(&readme[end + GALLERY_END.len()..]);
            return out;
        }
    }

    if let Some(at) = readme.find(GALLERY_SECTION) {
        let after_heading = at + GALLERY_SECTION.len();
        let mut out = String::with_capacity(readme.len() + marked.len());
        out.push_str(&readme[..after_heading]);
        out.push_str("\n\n");
        out.push_str(&marked);
        out.push('\n');
        out.push_str(readme[after_heading..].trim_start_matches('\n'));
        return out;
    }

    format!("{}\n\n{marked}\n", readme.trim_end())
}

// ---------------------------------------------------------------------------
// The README grid.
// ---------------------------------------------------------------------------

/// A markdown table, `COLUMNS` cells across: one row of images, one row of
/// captions, repeating. Image URLs are absolute raw-GitHub ones so crates.io
/// renders them; the links are absolute too, for the same reason.
fn readme_grid(rows: &[&Entry]) -> String {
    let mut out = String::new();
    out.push_str(&format!("|{}\n", " |".repeat(COLUMNS)));
    out.push_str(&format!("|{}\n", " --- |".repeat(COLUMNS)));

    for chunk in rows.chunks(COLUMNS) {
        let mut images = String::from("|");
        let mut captions = String::from("|");
        for entry in chunk {
            let name = &entry.row.name;
            // The image links to the rung's progress note when there is one,
            // and to the ported source otherwise.
            let target = match &entry.progress_doc {
                Some(doc) => format!("{REPO}/blob/main/docs/{doc}"),
                None => format!("{REPO}/blob/main/examples/{name}.rs"),
            };
            let _ = write!(
                images,
                " [<img src=\"{RAW}/docs/gallery/{name}.jpg\" alt=\"{name}\" \
                 width=\"{README_IMG_WIDTH}\">]({target}) |"
            );
            let _ = write!(
                captions,
                " [`{name}`]({REPO}/blob/main/examples/{name}.rs) |"
            );
        }
        for _ in chunk.len()..COLUMNS {
            images.push_str("  |");
            captions.push_str("  |");
        }
        out.push_str(&images);
        out.push('\n');
        out.push_str(&captions);
        out.push('\n');
    }

    // Our own frames, and how to put them back when a rung lands.
    out.push_str(&format!(
        "\n<sub>Our own rendered frames, one per graded example. \
         See [`docs/gallery.md`]({REPO}/blob/main/docs/gallery.md).</sub>\n"
    ));

    out
}

// ---------------------------------------------------------------------------
// Images: decode our PNG, area-average it down, encode a JPEG.
// ---------------------------------------------------------------------------

/// Decode an 8-bit PNG to tightly packed RGB.
fn read_png_rgb(path: &Path) -> Result<(u32, u32, Vec<u8>), String> {
    let file = fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut reader = png::Decoder::new(file)
        .read_info()
        .map_err(|e| format!("{}: {e}", path.display()))?;

    let mut buf = vec![0u8; reader.output_buffer_size()];
    let info = reader
        .next_frame(&mut buf)
        .map_err(|e| format!("{}: {e}", path.display()))?;

    if info.bit_depth != png::BitDepth::Eight {
        return Err(format!("{}: not an 8-bit PNG", path.display()));
    }
    let stride = match info.color_type {
        png::ColorType::Rgb => 3,
        png::ColorType::Rgba => 4,
        other => {
            return Err(format!(
                "{}: unsupported colour type {other:?}",
                path.display()
            ))
        }
    };

    let (w, h) = (info.width, info.height);
    let mut rgb = Vec::with_capacity((w * h * 3) as usize);
    for px in buf[..(w * h) as usize * stride].chunks_exact(stride) {
        rgb.extend_from_slice(&px[..3]);
    }
    Ok((w, h, rgb))
}

/// Box (area-average) downscale of packed RGB. At 800x500 -> 400x250 every
/// destination pixel is an exact 2x2 source box, which is all the filtering a
/// thumbnail of a rendered frame wants.
fn downscale_rgb(src: &[u8], w: u32, h: u32, dst_w: u32) -> (u32, u32, Vec<u8>) {
    if dst_w >= w {
        return (w, h, src.to_vec());
    }
    let dst_h = ((h as f64 * dst_w as f64 / w as f64).round() as u32).max(1);
    let mut out = vec![0u8; (dst_w * dst_h * 3) as usize];

    for y in 0..dst_h {
        let y0 = (y as u64 * h as u64 / dst_h as u64) as u32;
        let y1 = ((((y + 1) as u64 * h as u64).div_ceil(dst_h as u64)) as u32).min(h);
        for x in 0..dst_w {
            let x0 = (x as u64 * w as u64 / dst_w as u64) as u32;
            let x1 = ((((x + 1) as u64 * w as u64).div_ceil(dst_w as u64)) as u32).min(w);

            let mut acc = [0u32; 3];
            let mut n = 0u32;
            for sy in y0..y1.max(y0 + 1) {
                for sx in x0..x1.max(x0 + 1) {
                    let i = ((sy * w + sx) * 3) as usize;
                    acc[0] += src[i] as u32;
                    acc[1] += src[i + 1] as u32;
                    acc[2] += src[i + 2] as u32;
                    n += 1;
                }
            }
            let o = ((y * dst_w + x) * 3) as usize;
            for c in 0..3 {
                out[o + c] = ((acc[c] + n / 2) / n) as u8;
            }
        }
    }

    (dst_w, dst_h, out)
}

/// Write `docs/gallery/<name>.jpg` from our own frame. Returns its size.
fn write_thumbnail(actual: &Path, out: &Path) -> Result<u64, String> {
    let (w, h, rgb) = read_png_rgb(actual)?;
    let (tw, th, thumb) = downscale_rgb(&rgb, w, h, THUMB_WIDTH);

    let encoder = jpeg_encoder::Encoder::new_file(out, JPEG_QUALITY)
        .map_err(|e| format!("{}: {e}", out.display()))?;
    encoder
        .encode(&thumb, tw as u16, th as u16, jpeg_encoder::ColorType::Rgb)
        .map_err(|e| format!("{}: {e}", out.display()))?;

    fs::metadata(out)
        .map(|m| m.len())
        .map_err(|e| format!("{}: {e}", out.display()))
}

// ---------------------------------------------------------------------------
// The local index page.
// ---------------------------------------------------------------------------

/// Everything the page and the grid need about one example.
struct Entry {
    row: Row,
    /// Our graded frame, full resolution (`target/e2e/<name>/actual.png`).
    actual: PathBuf,
    /// `docs/<something>-progress.md`, if a rung note names this example.
    progress_doc: Option<String>,
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// The local page. It is written under `target/`, which is git-ignored, and
/// is the only place a Three reference screenshot is ever shown: by absolute
/// `file://` path into the vendored checkout, never copied into the tree.
fn index_html(entries: &[&Entry], root: &Path, vendor: &Path, three_tag: &str) -> String {
    let mut cards = String::new();

    for entry in entries {
        let row = &entry.row;
        let name = &row.name;
        let reference = vendor.join(format!("examples/screenshots/{name}.jpg"));
        let source = root.join(format!("examples/{name}.rs"));

        let mut links = format!(
            "<a href=\"file://{}\">examples/{name}.rs</a>",
            escape(&source.display().to_string())
        );
        if let Some(doc) = &entry.progress_doc {
            let _ = write!(
                links,
                "<a href=\"file://{}\">docs/{doc}</a>",
                escape(&root.join("docs").join(doc).display().to_string())
            );
        }
        let _ = write!(
            links,
            "<a href=\"https://threejs.org/examples/#{name}\">threejs.org</a>\
             <a href=\"https://github.com/mrdoob/three.js/blob/{three_tag}/examples/{name}.html\">Three source</a>"
        );

        let note = if row.diff_note.is_empty() {
            String::new()
        } else {
            format!("<span class=\"note\">{}</span>", escape(&row.diff_note))
        };

        let _ = write!(
            cards,
            r#"
    <section class="card" data-name="{name}">
      <h2>{name}</h2>
      <div class="pair">
        <figure><img loading="lazy" src="file://{actual}" alt="three-rs: {name}"><figcaption>three-rs</figcaption></figure>
        <figure><img loading="lazy" src="file://{reference}" alt="three.js: {name}"><figcaption>three.js reference</figcaption></figure>
      </div>
      <dl>
        <div><dt>different pixels</dt><dd>{diff} <small>of 100000</small> {note}</dd></div>
        <div><dt>steady frame</dt><dd>{steady:.1} ms</dd></div>
        <div><dt>draw calls</dt><dd>{calls}</dd></div>
        <div><dt>triangles</dt><dd>{tris}</dd></div>
      </dl>
      <nav>{links}</nav>
    </section>"#,
            name = name,
            actual = escape(&entry.actual.display().to_string()),
            reference = escape(&reference.display().to_string()),
            diff = row.diff_pixels,
            note = note,
            steady = row.steady_ms,
            calls = row.draw_calls,
            tris = row.triangles,
            links = links,
        );
    }

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>three-rs examples</title>
<style>
  :root {{ color-scheme: light dark; --bg: #fff; --fg: #111; --dim: #666; --line: #ddd; }}
  @media (prefers-color-scheme: dark) {{
    :root {{ --bg: #111; --fg: #eee; --dim: #999; --line: #333; }}
  }}
  * {{ box-sizing: border-box; }}
  body {{ margin: 0; background: var(--bg); color: var(--fg); font: 14px/1.5 system-ui, sans-serif; }}
  header {{ position: sticky; top: 0; background: var(--bg); border-bottom: 1px solid var(--line);
            padding: 16px; display: flex; gap: 16px; align-items: baseline; flex-wrap: wrap; }}
  h1 {{ font-size: 18px; margin: 0; font-weight: 600; }}
  #filter {{ flex: 1; min-width: 200px; padding: 6px 10px; font: inherit;
             border: 1px solid var(--line); border-radius: 4px; background: transparent; color: inherit; }}
  #count {{ color: var(--dim); }}
  main {{ display: grid; gap: 16px; padding: 16px;
          grid-template-columns: repeat(auto-fill, minmax(480px, 1fr)); }}
  .card {{ border: 1px solid var(--line); border-radius: 6px; padding: 12px; }}
  .card h2 {{ font-size: 14px; margin: 0 0 8px; font-family: ui-monospace, monospace; font-weight: 600; }}
  .pair {{ display: grid; grid-template-columns: 1fr 1fr; gap: 8px; }}
  figure {{ margin: 0; }}
  figure img {{ width: 100%; display: block; border: 1px solid var(--line); background: #000; }}
  figcaption {{ color: var(--dim); font-size: 11px; padding-top: 3px; }}
  dl {{ display: flex; flex-wrap: wrap; gap: 4px 16px; margin: 10px 0 8px; }}
  dl div {{ display: flex; gap: 6px; }}
  dt {{ color: var(--dim); }}
  dd {{ margin: 0; font-variant-numeric: tabular-nums; }}
  small, .note {{ color: var(--dim); }}
  nav {{ display: flex; gap: 12px; flex-wrap: wrap; font-size: 12px; }}
  a {{ color: inherit; }}
  footer {{ color: var(--dim); padding: 16px; border-top: 1px solid var(--line); }}
</style>
</head>
<body>
<header>
  <h1>three-rs examples</h1>
  <input id="filter" type="text" placeholder="filter" autocomplete="off" spellcheck="false" autofocus>
  <span id="count"></span>
</header>
<main id="grid">{cards}
</main>
<footer>
  Our frames are <code>target/e2e/&lt;name&gt;/actual.png</code>; the reference
  screenshots come from the three.js checkout at <code>{vendor}</code> ({three_tag}).
  This page lives under <code>target/</code> and is never committed.
</footer>
<script>
  const filter = document.getElementById('filter');
  const cards = [...document.querySelectorAll('.card')];
  const count = document.getElementById('count');
  function apply() {{
    const q = filter.value.trim().toLowerCase();
    let n = 0;
    for (const card of cards) {{
      const hit = card.dataset.name.includes(q);
      card.hidden = !hit;
      if (hit) n++;
    }}
    count.textContent = n + ' of ' + cards.length;
  }}
  filter.addEventListener('input', apply);
  apply();
</script>
</body>
</html>
"#,
        cards = cards,
        vendor = escape(&vendor.display().to_string()),
        three_tag = three_tag,
    )
}

// ---------------------------------------------------------------------------
// Wiring.
// ---------------------------------------------------------------------------

/// `docs/<name>-progress.md` if it exists, else the rung note whose title
/// names the example (`docs/rung7-progress.md` is "rung 7 —
/// `webgpu_shadowmap`"). Rungs 1-4 predate the notes and simply have none.
fn find_progress_doc(docs: &Path, name: &str) -> Option<String> {
    let direct = format!("{name}-progress.md");
    if docs.join(&direct).is_file() {
        return Some(direct);
    }

    let mut candidates: Vec<String> = fs::read_dir(docs)
        .ok()?
        .filter_map(|e| e.ok())
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|f| f.ends_with("-progress.md"))
        .filter(|f| {
            let title = fs::read_to_string(docs.join(f))
                .ok()
                .and_then(|s| s.lines().next().map(str::to_string))
                .unwrap_or_default();
            title.contains(name)
        })
        .collect();
    candidates.sort();
    candidates.into_iter().next()
}

/// `r186` from the vendored checkout's `package.json`, for Three's own
/// source links. Falls back to `dev`.
fn three_tag(vendor: &Path) -> String {
    let parsed = fs::read_to_string(vendor.join("package.json"))
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| v["version"].as_str().map(str::to_string))
        .and_then(|v| v.split('.').nth(1).map(str::to_string));
    match parsed {
        Some(minor) => format!("r{minor}"),
        None => "dev".to_string(),
    }
}

fn vendor_dir(args: &[String]) -> PathBuf {
    if let Some(i) = args.iter().position(|a| a == "--vendor") {
        if let Some(path) = args.get(i + 1) {
            return PathBuf::from(path);
        }
    }
    if let Some(path) = args.iter().find_map(|a| a.strip_prefix("--vendor=")) {
        return PathBuf::from(path);
    }
    for var in ["THREE_VENDOR", "THREE_JS_DIR"] {
        if let Ok(path) = std::env::var(var) {
            if !path.is_empty() {
                return PathBuf::from(path);
            }
        }
    }
    let home = std::env::var("HOME").unwrap_or_default();
    PathBuf::from(home).join("src/vendor/three.js")
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "-h" || a == "--help") {
        println!(
            "usage: cargo run --release --example gallery -- [--vendor <three.js checkout>]\n\
             \n\
             Reads the README's \"Examples graded green\" table and the frames the e2e\n\
             ladder left in target/e2e/<name>/actual.png, then writes docs/gallery/*.jpg,\n\
             target/gallery/index.html and the README's gallery block."
        );
        return;
    }

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let vendor = vendor_dir(&args);
    let tag = three_tag(&vendor);

    let readme_path = root.join("README.md");
    let readme = fs::read_to_string(&readme_path).expect("gallery: README.md");
    let rows = parse_graded_table(&readme);
    if rows.is_empty() {
        eprintln!("gallery: no rows in the README's \"{GALLERY_SECTION}\" table; nothing to do");
        std::process::exit(1);
    }

    let gallery_dir = root.join("docs/gallery");
    fs::create_dir_all(&gallery_dir).expect("gallery: docs/gallery");

    let mut entries: Vec<Entry> = Vec::new();
    let mut missing: Vec<String> = Vec::new();
    let mut total_bytes = 0u64;

    for row in rows {
        let actual = root.join(format!("target/e2e/{}/actual.png", row.name));
        if !actual.is_file() {
            missing.push(row.name.clone());
            continue;
        }
        let thumb = gallery_dir.join(format!("{}.jpg", row.name));
        match write_thumbnail(&actual, &thumb) {
            Ok(bytes) => {
                total_bytes += bytes;
                println!("gallery: docs/gallery/{}.jpg ({bytes} bytes)", row.name);
            }
            Err(e) => {
                eprintln!("gallery: {e}");
                continue;
            }
        }
        let progress_doc = find_progress_doc(&root.join("docs"), &row.name);
        entries.push(Entry {
            actual,
            progress_doc,
            row,
        });
    }

    if entries.is_empty() {
        eprintln!(
            "gallery: no frames under target/e2e/; run `cargo test --release --test e2e` first"
        );
        std::process::exit(1);
    }

    // Keep the README's row order, which is the rung order.
    let ordered: Vec<&Entry> = entries.iter().collect();

    let out_dir = root.join("target/gallery");
    fs::create_dir_all(&out_dir).expect("gallery: target/gallery");
    let index = out_dir.join("index.html");
    fs::write(&index, index_html(&ordered, &root, &vendor, &tag)).expect("gallery: index.html");

    let updated = replace_gallery_section(&readme, &readme_grid(&ordered));
    if updated != readme {
        fs::write(&readme_path, updated).expect("gallery: README.md");
        println!("gallery: README.md gallery block updated");
    } else {
        println!("gallery: README.md gallery block already current");
    }

    println!(
        "gallery: {} example(s), {} KB of thumbnails, file://{}",
        ordered.len(),
        total_bytes.div_ceil(1024),
        index.display()
    );
    if !missing.is_empty() {
        println!(
            "gallery: no target/e2e frame for {} — run the ladder to include {}",
            missing.join(", "),
            if missing.len() == 1 { "it" } else { "them" }
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TABLE: &str = "\
# three-rs

## Examples graded green

| Three example | different pixels (of 100000) | steady frame (ms) | draw calls | triangles |
|---|---|---|---|---|
| webgpu_depth_texture | 0 | 11.0 | 43 | 671746 |
| webgpu_instance_mesh | 60 (Three itself scores 60 against the same JPEG) | 9.3 | 2 | 967001 |
| webgpu_rtt | 1 | 2.3 | 3 | 14 |

Measured on Intel Iris Xe.

## Building

| env var | default | needed by |
|---|---|---|
| `D3_GALLERY_DIR` | `~/src/vendor/d3-gallery` | something else |
";

    fn entry(name: &str, doc: Option<&str>) -> Entry {
        Entry {
            row: Row {
                name: name.to_string(),
                diff_pixels: 0,
                diff_note: String::new(),
                steady_ms: 1.0,
                draw_calls: 1,
                triangles: "1".to_string(),
            },
            actual: PathBuf::from("/dev/null"),
            progress_doc: doc.map(str::to_string),
        }
    }

    #[test]
    fn parses_the_graded_table() {
        let rows = parse_graded_table(TABLE);
        assert_eq!(rows.len(), 3, "three rows, header and separator dropped");
        assert_eq!(rows[0].name, "webgpu_depth_texture");
        assert_eq!(rows[0].diff_pixels, 0);
        assert_eq!(rows[0].steady_ms, 11.0);
        assert_eq!(rows[0].draw_calls, 43);
        assert_eq!(rows[0].triangles, "671746");
        // The order is the table's order, which is the rung order.
        assert_eq!(rows[2].name, "webgpu_rtt");
    }

    #[test]
    fn keeps_the_note_beside_the_diff_count() {
        let rows = parse_graded_table(TABLE);
        assert_eq!(rows[1].diff_pixels, 60);
        assert_eq!(
            rows[1].diff_note,
            "(Three itself scores 60 against the same JPEG)"
        );
    }

    #[test]
    fn ignores_tables_in_other_sections() {
        let rows = parse_graded_table(TABLE);
        assert!(
            rows.iter().all(|r| r.name.starts_with("webgpu_")),
            "the env-var table under ## Building is not a graded table"
        );
    }

    #[test]
    fn inserts_the_block_under_the_heading_then_replaces_in_place() {
        let once = replace_gallery_section(TABLE, "GRID-A");
        assert!(once.contains(GALLERY_START) && once.contains(GALLERY_END));
        assert!(once.contains("GRID-A"));
        // Inserted under the heading, above the numbers table.
        let at_heading = once.find(GALLERY_SECTION).unwrap();
        let at_block = once.find(GALLERY_START).unwrap();
        let at_table = once.find("| Three example |").unwrap();
        assert!(at_heading < at_block && at_block < at_table);
        // The rest of the README survives.
        assert!(once.contains("| webgpu_rtt | 1 | 2.3 | 3 | 14 |"));
        assert!(once.contains("## Building"));

        let twice = replace_gallery_section(&once, "GRID-B");
        assert!(!twice.contains("GRID-A"), "the old block is replaced");
        assert!(twice.contains("GRID-B"));
        assert_eq!(twice.matches(GALLERY_START).count(), 1);
        assert_eq!(twice.matches(GALLERY_END).count(), 1);

        // Regenerating with the same grid is a no-op.
        assert_eq!(replace_gallery_section(&twice, "GRID-B"), twice);
    }

    #[test]
    fn the_grid_is_a_table_of_absolute_urls() {
        let a = entry("webgpu_rtt", Some("rung4-progress.md"));
        let b = entry("webgpu_tsl_galaxy", None);
        let grid = readme_grid(&[&a, &b]);

        assert!(grid.contains(&format!("{RAW}/docs/gallery/webgpu_rtt.jpg")));
        // The image links to the progress note when there is one...
        assert!(grid.contains(&format!("{REPO}/blob/main/docs/rung4-progress.md")));
        // ...and to the source when there is not.
        assert!(grid.contains(&format!("{REPO}/blob/main/examples/webgpu_tsl_galaxy.rs")));
        // Two cells filled, the row padded out to four.
        let rows: Vec<&str> = grid.lines().filter(|l| l.starts_with('|')).collect();
        assert_eq!(rows.len(), 4, "header, separator, images, captions");
        for row in &rows {
            assert_eq!(row.matches('|').count(), COLUMNS + 1);
        }
    }

    #[test]
    fn downscale_halves_and_averages() {
        // 2x2 white/black checker -> one mid-grey pixel.
        let src = vec![
            255, 255, 255, 0, 0, 0, //
            0, 0, 0, 255, 255, 255,
        ];
        let (w, h, out) = downscale_rgb(&src, 2, 2, 1);
        assert_eq!((w, h), (1, 1));
        assert_eq!(out, vec![128, 128, 128]);
    }

    #[test]
    fn downscale_never_upscales() {
        let src = vec![1, 2, 3];
        assert_eq!(downscale_rgb(&src, 1, 1, 400), (1, 1, src));
    }
}
