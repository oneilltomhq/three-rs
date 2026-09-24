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
//! **The `browser` column.** The table's sixth column says whether CI's
//! `web-gate` job (`tools/web_gate.mjs`: headless Chrome, software WebGPU,
//! the same screenshots at the same threshold) grades the example. It is
//! derived, never typed: `yes` when `web/manifests/<name>.json` exists and
//! `tools/web_gate.skip` does not list the name, `no (<reason>)` when the
//! skip file does, and `not yet` when there is no manifest (the example is
//! not on the Pages build, so the gate cannot see it). Every run of this
//! tool, `--readme-only` included, rewrites the column's header and cells in
//! place and nothing else in the row, so a rung worker adding a row by hand
//! may leave the cell off. The test `the_readme_browser_column_is_current`
//! fails CI when the column and the manifests drift apart.
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
//! cargo run --release --example gallery -- --readme-only
//! ```
//!
//! `--readme-only` rewrites only the README block and the `browser` column,
//! from the thumbnails already committed under `docs/gallery/` and the
//! manifests under `web/manifests/`: no ladder run, no GPU, no three.js
//! checkout. It is for changing the block's shape and refreshing the column,
//! not for adding a rung.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

/// The repository the README's absolute URLs point into.
const REPO: &str = "https://github.com/oneilltomhq/three-rs";
/// Raw file URLs, so crates.io (which does not rewrite relative links)
/// renders the grid's images.
const RAW: &str = "https://raw.githubusercontent.com/oneilltomhq/three-rs/main";
/// The browser shell (`web/`) as GitHub Pages deploys it from `main`; each
/// thumbnail opens its example running there, at `?example=<name>`.
const PAGES: &str = "https://oneilltomhq.github.io/three-rs/";

/// The markers the README grid is regenerated between. A rung worker adding
/// a row to the graded table never touches what is inside them.
const GALLERY_START: &str = "<!-- gallery:start -->";
const GALLERY_END: &str = "<!-- gallery:end -->";
/// The heading the block is inserted under the first time it is written.
const GALLERY_SECTION: &str = "## Examples graded green";
/// The first header cell of the graded table, and the header of the column
/// this tool owns in it.
const TABLE_FIRST_HEADER: &str = "Three example";
const BROWSER_HEADER: &str = "browser";
/// Cells in a graded row without, and with, the `browser` column.
const ROW_CELLS: usize = 5;
const ROW_CELLS_WITH_BROWSER: usize = 6;

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
    /// The `browser` cell verbatim, or `None` on a five-cell row that has not
    /// been through the generator yet.
    browser: Option<String>,
}

/// Parse the README's "Examples graded green" table.
///
/// Only rows inside that section are considered, and only rows whose first
/// cell is an example name; the header and the `---` separator fall out on
/// their own. A row has five cells, or six once the generator has filled in
/// its `browser` cell. A row whose numbers do not parse is skipped and named
/// on stderr rather than guessed at.
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

        let cells = table_cells(line);
        if !(cells.len() == ROW_CELLS || cells.len() == ROW_CELLS_WITH_BROWSER)
            || !cells[0].starts_with("webgpu_")
        {
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
                browser: cells.get(5).map(|c| c.to_string()),
            })
        })();

        match parsed {
            Some(row) => rows.push(row),
            None => eprintln!("gallery: skipping unparseable table row: {line}"),
        }
    }

    rows
}

/// The trimmed cells of a Markdown table line.
fn table_cells(line: &str) -> Vec<&str> {
    line.trim()
        .trim_matches('|')
        .split('|')
        .map(str::trim)
        .collect()
}

// ---------------------------------------------------------------------------
// The `browser` column: what CI's web gate grades.
// ---------------------------------------------------------------------------

/// Parse `tools/web_gate.skip` the way `tools/web_gate.mjs` does: one
/// `<example>  <reason>` per line, whitespace-separated, `#` to end of line
/// a comment, blank lines ignored.
fn parse_skip_list(text: &str) -> BTreeMap<String, String> {
    let mut skip = BTreeMap::new();
    for raw in text.lines() {
        let line = raw.split('#').next().unwrap_or("").trim();
        let mut words = line.split_whitespace();
        let Some(name) = words.next() else { continue };
        skip.insert(name.to_string(), words.collect::<Vec<_>>().join(" "));
    }
    skip
}

/// The examples `web/manifests/` builds for Pages, and therefore the ones the
/// web gate grades: one `<name>.json` each.
fn read_manifests(root: &Path) -> BTreeSet<String> {
    let dir = root.join("web/manifests");
    let Ok(read) = fs::read_dir(&dir) else {
        return BTreeSet::new();
    };
    read.filter_map(|e| e.ok())
        .filter_map(|e| {
            e.file_name()
                .to_str()
                .and_then(|f| f.strip_suffix(".json"))
                .map(str::to_string)
        })
        .collect()
}

/// `tools/web_gate.skip`, parsed; empty when the file does not exist.
fn read_skip_list(root: &Path) -> BTreeMap<String, String> {
    fs::read_to_string(root.join("tools/web_gate.skip"))
        .map(|t| parse_skip_list(&t))
        .unwrap_or_default()
}

/// The `browser` cell for one example.
fn browser_cell(
    name: &str,
    manifests: &BTreeSet<String>,
    skip: &BTreeMap<String, String>,
) -> String {
    match skip.get(name) {
        // A `|` in a reason would split the cell.
        Some(reason) if reason.is_empty() => "no (skipped)".to_string(),
        Some(reason) => format!("no ({})", reason.replace('|', "/")),
        None if manifests.contains(name) => "yes".to_string(),
        None => "not yet".to_string(),
    }
}

/// `line` (a table line with `cells` cells) with its last cell set to
/// `value` when it has `full` cells, or `value` appended when it has one
/// fewer. The other cells keep their text and spacing byte for byte.
fn with_last_cell(line: &str, cells: usize, full: usize, value: &str) -> String {
    let body = line.trim_end().strip_suffix('|').unwrap_or(line.trim_end());
    let body = if cells == full {
        body.rfind('|').map_or(body, |at| &body[..at])
    } else {
        body
    };
    format!("{} | {value} |", body.trim_end())
}

/// Rewrite the graded table's `browser` header and cells in place from the
/// manifests and the skip list. Nothing else in the README changes: not the
/// other cells, not the row order, not the gallery block.
fn rewrite_browser_column(
    readme: &str,
    manifests: &BTreeSet<String>,
    skip: &BTreeMap<String, String>,
) -> String {
    let mut out = String::with_capacity(readme.len() + 4096);
    let mut in_section = false;
    let mut in_block = false;
    let mut after_header = false;

    for line in readme.split_inclusive('\n') {
        let (text, newline) = match line.strip_suffix('\n') {
            Some(t) => (t, "\n"),
            None => (line, ""),
        };
        let was_after_header = std::mem::take(&mut after_header);

        if let Some(heading) = text.strip_prefix("## ") {
            in_section = format!("## {}", heading.trim()) == GALLERY_SECTION;
        } else if text.trim() == GALLERY_START {
            in_block = true;
        } else if text.trim() == GALLERY_END {
            in_block = false;
        } else if in_section && !in_block && text.trim_start().starts_with('|') {
            let cells = table_cells(text);
            let n = cells.len();
            let fits = n == ROW_CELLS || n == ROW_CELLS_WITH_BROWSER;
            if fits && cells[0] == TABLE_FIRST_HEADER {
                out.push_str(&with_last_cell(
                    text,
                    n,
                    ROW_CELLS_WITH_BROWSER,
                    BROWSER_HEADER,
                ));
                out.push_str(newline);
                after_header = true;
                continue;
            }
            let separator = cells
                .iter()
                .all(|c| !c.is_empty() && c.chars().all(|ch| ch == '-' || ch == ':'));
            if fits && was_after_header && separator {
                // Every cell of our table's separator is a plain `---`.
                out.push('|');
                out.push_str(&"---|".repeat(ROW_CELLS_WITH_BROWSER));
                out.push_str(newline);
                continue;
            }
            if fits && cells[0].starts_with("webgpu_") {
                let cell = browser_cell(cells[0], manifests, skip);
                out.push_str(&with_last_cell(text, n, ROW_CELLS_WITH_BROWSER, &cell));
                out.push_str(newline);
                continue;
            }
        }
        out.push_str(line);
    }
    out
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
///
/// The image opens the example running in the browser (the `web/` shell on
/// GitHub Pages); the caption under it links the ported source, and the
/// rung's progress note when there is one.
fn readme_grid(rows: &[&Entry]) -> String {
    let mut out = String::new();
    out.push_str(&format!("|{}\n", " |".repeat(COLUMNS)));
    out.push_str(&format!("|{}\n", " --- |".repeat(COLUMNS)));

    for chunk in rows.chunks(COLUMNS) {
        let mut images = String::from("|");
        let mut captions = String::from("|");
        for entry in chunk {
            let name = &entry.row.name;
            let _ = write!(
                images,
                " [<img src=\"{RAW}/docs/gallery/{name}.jpg\" alt=\"{name}\" \
                 width=\"{README_IMG_WIDTH}\">]({PAGES}?example={name}) |"
            );
            let _ = write!(captions, " [`{name}`]({REPO}/blob/main/examples/{name}.rs)");
            if let Some(doc) = &entry.progress_doc {
                let _ = write!(captions, " · [notes]({REPO}/blob/main/docs/{doc})");
            }
            captions.push_str(" |");
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
         Each thumbnail opens the example running in your browser on WebGPU \
         ([all of them]({PAGES})); the caption links the ported source. \
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

/// Write the README with a fresh gallery block and `browser` column, if
/// either changed.
fn write_readme(root: &Path, readme: &str, entries: &[&Entry]) {
    let updated = rewrite_browser_column(
        &replace_gallery_section(readme, &readme_grid(entries)),
        &read_manifests(root),
        &read_skip_list(root),
    );
    if updated != readme {
        fs::write(root.join("README.md"), updated).expect("gallery: README.md");
        println!(
            "gallery: README.md gallery block and browser column updated ({} examples)",
            entries.len()
        );
    } else {
        println!("gallery: README.md gallery block and browser column already current");
    }
}

/// `--readme-only`: the README block from the committed thumbnails alone,
/// and the `browser` column from the manifests.
fn readme_only(root: &Path) {
    let readme = fs::read_to_string(root.join("README.md")).expect("gallery: README.md");
    let mut entries: Vec<Entry> = Vec::new();
    let mut missing: Vec<String> = Vec::new();
    for row in parse_graded_table(&readme) {
        let thumb = root.join(format!("docs/gallery/{}.jpg", row.name));
        if !thumb.is_file() {
            missing.push(row.name);
            continue;
        }
        let progress_doc = find_progress_doc(&root.join("docs"), &row.name);
        entries.push(Entry {
            actual: thumb,
            progress_doc,
            row,
        });
    }
    if entries.is_empty() {
        eprintln!("gallery: no committed thumbnails under docs/gallery/ for the graded table");
        std::process::exit(1);
    }
    let ordered: Vec<&Entry> = entries.iter().collect();
    write_readme(root, &readme, &ordered);
    if !missing.is_empty() {
        println!(
            "gallery: no docs/gallery thumbnail for {}; run the ladder and the full generator",
            missing.join(", ")
        );
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "-h" || a == "--help") {
        println!(
            "usage: cargo run --release --example gallery -- [--vendor <three.js checkout>]\n\
             \x20      cargo run --release --example gallery -- --readme-only\n\
             \n\
             Reads the README's \"Examples graded green\" table and the frames the e2e\n\
             ladder left in target/e2e/<name>/actual.png, then writes docs/gallery/*.jpg,\n\
             target/gallery/index.html, the README's gallery block and the table's\n\
             browser column (from web/manifests/ and tools/web_gate.skip).\n\
             \n\
             --readme-only rewrites just the README block, from the thumbnails already\n\
             committed under docs/gallery/, and the browser column; it needs no ladder\n\
             run and no GPU."
        );
        return;
    }

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    if args.iter().any(|a| a == "--readme-only") {
        readme_only(&root);
        return;
    }
    let vendor = vendor_dir(&args);
    let tag = three_tag(&vendor);

    let readme = fs::read_to_string(root.join("README.md")).expect("gallery: README.md");
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

    write_readme(&root, &readme, &ordered);

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
                browser: None,
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
    fn parses_five_and_six_cell_rows() {
        let readme = "\
## Examples graded green

| Three example | different pixels (of 100000) | steady frame (ms) | draw calls | triangles | browser |
|---|---|---|---|---|---|
| webgpu_rtt | 1 | 2.3 | 3 | 14 | yes |
| webgpu_mrt | 87 (a note) | 2.8 | 3 | 17437 | no (flaky on SwiftShader) |
| webgpu_new_rung | 5 | 1.0 | 2 | 10 |
";
        let rows = parse_graded_table(readme);
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].browser.as_deref(), Some("yes"));
        assert_eq!(rows[0].triangles, "14");
        assert_eq!(rows[1].diff_pixels, 87);
        assert_eq!(rows[1].diff_note, "(a note)");
        assert_eq!(
            rows[1].browser.as_deref(),
            Some("no (flaky on SwiftShader)")
        );
        assert_eq!(rows[2].name, "webgpu_new_rung");
        assert_eq!(rows[2].browser, None, "a hand-added row may leave it off");
    }

    #[test]
    fn parses_the_skip_file_like_web_gate_mjs() {
        let skip = parse_skip_list(
            "# one example per line, then why\n\
             \n\
             webgpu_mrt   0.12% on SwiftShader, see #999  # trailing comment\n\
             webgpu_bare\n",
        );
        assert_eq!(skip.len(), 2);
        assert_eq!(skip["webgpu_mrt"], "0.12% on SwiftShader, see");
        assert_eq!(skip["webgpu_bare"], "");
    }

    #[test]
    fn derives_the_browser_cell() {
        let manifests: BTreeSet<String> = ["webgpu_rtt", "webgpu_mrt", "webgpu_bare"]
            .map(str::to_string)
            .into();
        let skip = parse_skip_list("webgpu_mrt  0.12% on SwiftShader\nwebgpu_bare\n");
        assert_eq!(browser_cell("webgpu_rtt", &manifests, &skip), "yes");
        assert_eq!(
            browser_cell("webgpu_mrt", &manifests, &skip),
            "no (0.12% on SwiftShader)"
        );
        assert_eq!(
            browser_cell("webgpu_bare", &manifests, &skip),
            "no (skipped)"
        );
        assert_eq!(browser_cell("webgpu_unbuilt", &manifests, &skip), "not yet");
    }

    #[test]
    fn fills_the_browser_column_in_place() {
        let manifests: BTreeSet<String> = ["webgpu_depth_texture", "webgpu_instance_mesh"]
            .map(str::to_string)
            .into();
        let skip = parse_skip_list("webgpu_instance_mesh  too slow\n");
        let once = rewrite_browser_column(TABLE, &manifests, &skip);
        assert!(once.contains(
            "| Three example | different pixels (of 100000) | steady frame (ms) \
             | draw calls | triangles | browser |\n|---|---|---|---|---|---|\n"
        ));
        assert!(once.contains("| webgpu_depth_texture | 0 | 11.0 | 43 | 671746 | yes |\n"));
        assert!(once.contains(
            "| webgpu_instance_mesh | 60 (Three itself scores 60 against the same JPEG) \
             | 9.3 | 2 | 967001 | no (too slow) |\n"
        ));
        assert!(once.contains("| webgpu_rtt | 1 | 2.3 | 3 | 14 | not yet |\n"));
        // The table in the other section, and the rest, are untouched.
        assert!(once.contains("|---|---|---|\n| `D3_GALLERY_DIR` |"));
        assert_eq!(once.lines().count(), TABLE.lines().count());
        // The rows still parse, notes and all.
        let rows = parse_graded_table(&once);
        assert_eq!(rows.len(), 3);
        assert_eq!(
            rows[1].diff_note,
            "(Three itself scores 60 against the same JPEG)"
        );
        // Idempotent, and a changed verdict replaces the cell, not appends.
        assert_eq!(rewrite_browser_column(&once, &manifests, &skip), once);
        let twice = rewrite_browser_column(&once, &manifests, &BTreeMap::new());
        assert!(twice.contains("| 9.3 | 2 | 967001 | yes |\n"));
        assert_eq!(parse_graded_table(&twice).len(), 3);
    }

    #[test]
    fn leaves_the_gallery_block_alone() {
        let readme = replace_gallery_section(TABLE, "| a | b |\n|---|---|\n| webgpu_rtt | x |");
        let manifests = BTreeSet::new();
        let out = rewrite_browser_column(&readme, &manifests, &BTreeMap::new());
        let block = |s: &str| {
            let a = s.find(GALLERY_START).unwrap();
            let b = s.find(GALLERY_END).unwrap();
            s[a..b].to_string()
        };
        assert_eq!(block(&out), block(&readme));
    }

    /// The CI gate on the column: the committed README, `web/manifests/` and
    /// `tools/web_gate.skip` must agree. The `web-gate` job grades every
    /// manifest, so this is what makes the column say what CI enforces.
    #[test]
    fn the_readme_browser_column_is_current() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let readme = fs::read_to_string(root.join("README.md")).expect("README.md");
        let manifests = read_manifests(root);
        let skip = read_skip_list(root);
        let fix =
            "run `cargo run --release --example gallery -- --readme-only` and commit README.md";

        let rows = parse_graded_table(&readme);
        assert!(!rows.is_empty(), "no graded table in README.md");
        let mut drift = Vec::new();
        for row in &rows {
            let want = browser_cell(&row.name, &manifests, &skip);
            if row.browser.as_deref() != Some(want.as_str()) {
                drift.push(format!(
                    "{}: browser cell is {:?}, web/manifests + tools/web_gate.skip say {want:?}",
                    row.name, row.browser
                ));
            }
        }
        let named: BTreeSet<&str> = rows.iter().map(|r| r.name.as_str()).collect();
        for m in &manifests {
            if !named.contains(m.as_str()) {
                drift.push(format!(
                    "web/manifests/{m}.json has no row in the README's graded table; \
                     add the row (the web gate grades it)"
                ));
            }
        }
        for s in skip.keys() {
            if !manifests.contains(s) {
                drift.push(format!(
                    "tools/web_gate.skip names {s}, which has no manifest"
                ));
            }
        }
        assert!(drift.is_empty(), "{}\n\n{fix}", drift.join("\n"));
        assert_eq!(
            rewrite_browser_column(&readme, &manifests, &skip),
            readme,
            "the graded table's browser header or cells are stale; {fix}"
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
        // Every image opens its example running on Pages...
        assert!(grid.contains(&format!(
            "width=\"{README_IMG_WIDTH}\">]({PAGES}?example=webgpu_rtt)"
        )));
        assert!(grid.contains(&format!(
            "width=\"{README_IMG_WIDTH}\">]({PAGES}?example=webgpu_tsl_galaxy)"
        )));
        // ...every caption links the source...
        assert!(grid.contains(&format!("{REPO}/blob/main/examples/webgpu_rtt.rs")));
        assert!(grid.contains(&format!("{REPO}/blob/main/examples/webgpu_tsl_galaxy.rs")));
        // ...and the progress note where there is one, and only there.
        assert!(grid.contains(&format!(
            "{REPO}/blob/main/examples/webgpu_rtt.rs) · [notes]({REPO}/blob/main/docs/rung4-progress.md) |"
        )));
        assert_eq!(grid.matches("[notes]").count(), 1);
        // No image links anywhere but Pages.
        assert!(!grid.contains("\">](https://github.com"));
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
