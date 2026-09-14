//! Port of lib3's `examples/sdf-text-vector/main.js` (`?mode=vector`) — step 5
//! of the sdf-text ladder: `BatchedText` assembling a multi-member, multi-size
//! label block.
//!
//! The page's own geometry is `window.innerWidth` × `innerHeight`; here it is
//! the ladder's 800 × 500 at `setPixelRatio( 1 )`, which is what the e2e harness
//! gives every other example. Everything that decides *where the glyphs land* —
//! the orthographic view height of 8, the four strings, their font sizes, their
//! anchors, their y positions and their colours — is the page's, unchanged.
//!
//! The page's `VectorFont.load( fontURL )` is a `fetch`; here the same font file
//! is read from disk (`VectorFont::load` is in the crate's skip register, and
//! the bytes are byte-identical to lib3's and d33's copies).
//!
//! Two `sync()` + render rounds, as the page does: in vector mode the first
//! `sync` is what rasterises the atlas, so the page renders once more to make
//! the `DataTexture` upload visible. Here the upload is not deferred, but the
//! two rounds are kept because the gate asserts they produce the *same* frame —
//! `sync()` must be idempotent, and a second `sync` that re-packed into
//! different slots or re-rasterised into a different atlas would show up as a
//! different image.

use std::rc::Rc;

use sdf_text::{Anchor, BatchedText, BatchedTextOptions, Text, VectorFont};
use three_rs::core::Object3DNode;
use three_rs::{Color, OrthographicCamera, Renderer, RendererParameters, Scene};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( 1 )` — the page pins it, unlike the three.js
/// examples which use `devicePixelRatio`.
pub const DPR: f64 = 1.0;

/// `const viewH = 8` — the orthographic view height in world units, so
/// pixels-per-world-unit is `H / 8`. At H = 500 that is 62.5 px/unit, so
/// `fontSize 0.12` is ≈ 7.5 px text and `fontSize 1.2` is ≈ 75 px.
const VIEW_H: f64 = 8.0;

const KERN_STR: &str = "Kerning: AV To Wa AW LT";
const BIG_STR: &str = "AVToWa";

pub struct App {
    pub renderer: Renderer,
    pub scene: Scene,
    pub camera: OrthographicCamera,
    pub batched: BatchedText,
}

fn font_path() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/assets/Roboto-Regular.ttf")
}

pub fn load_font() -> VectorFont {
    let path = font_path();
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    VectorFont::parse(bytes, "Roboto-Regular.ttf").expect("parse Roboto-Regular.ttf")
}

pub fn init() -> App {
    let aspect = INNER_WIDTH / INNER_HEIGHT;
    let view_w = VIEW_H * aspect;
    let mut camera = OrthographicCamera::new(
        -view_w / 2.0,
        view_w / 2.0,
        VIEW_H / 2.0,
        -VIEW_H / 2.0,
        -10.0,
        10.0,
    );
    camera.object.position.z = 5.0;
    camera.update_projection_matrix();

    let mut scene = Scene::new();
    // `scene.background = new THREE.Color( 0x0b0d12 )`. `Color::from_hex`
    // applies `SRGBToLinear`, matching `THREE.Color`'s own constructor.
    scene.set_background(Color::from_hex(0x0b0d12));

    // `new BatchedText( 8, 2048, undefined, { font, outlineWidth: 0.04 } )`.
    // `outlineColor` is unset on this page, so it is the legacy same-colour
    // outline: the halo mix uniform stays 0 and `haloRGB` is `aColor`.
    let mut batched = BatchedText::new(
        8,
        2048,
        BatchedTextOptions {
            outline_width: 0.04,
            outline_color: None,
            ..Default::default()
        },
    );
    batched.set_font(Rc::new(load_font()));
    scene.add(batched.node());

    // `makeText( str, fontSize, x, y, color, align )`, in the page's order —
    // which is also the order the glyph slots and the atlas slots are assigned
    // in, so it is load-bearing.
    make_text(&mut batched, KERN_STR, 0.12, 0.0, 3.2, 0x9ae6b4);
    make_text(&mut batched, KERN_STR, 0.12, 0.0, 2.4, 0xffffff);
    make_text(&mut batched, BIG_STR, 1.2, 0.0, -0.2, 0x7dd3fc);
    make_text(&mut batched, "Roboto", 0.6, 0.0, -2.6, 0xf472b6);

    let mut renderer = Renderer::new(RendererParameters { antialias: true });
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);

    App {
        renderer,
        scene,
        camera,
        batched,
    }
}

/// ```js
/// const t = new Text();
/// t.text = str; t.fontSize = fontSize;
/// t.anchorX = align; t.anchorY = 'middle';
/// t.color.set( color );
/// t.position.set( x, y, 0 );
/// scene.add( t ); batched.addText( t );
/// ```
///
/// `scene.add( t )` is the member's own `Object3D`, which the batch owns here,
/// so the position is set on `member_node` after `add_text` rather than on the
/// `Text`. Every call site on this page passes `align = 'center'`.
fn make_text(batched: &mut BatchedText, str: &str, font_size: f64, x: f64, y: f64, color: u32) {
    let mut text = Text::new();
    text.set_text(str);
    text.set_font_size(font_size);
    text.set_anchor_x(Anchor::named("center"));
    text.set_anchor_y(Anchor::named("middle"));
    let id = batched.add_text(text) as usize;

    // `t.color.set( hex )` — `THREE.Color.set` decodes sRGB, which is what
    // `Color::from_hex` does, and `Text::color` is documented as linear.
    batched.set_color_at(id, Color::from_hex(color));

    batched
        .member_node(id)
        .expect("the member was just added")
        .borrow_mut()
        .position
        .set(x, y, 0.0);
}

/// The page's `main()` after `renderer.init()` and `await batched.ready`: two
/// `sync()` + render rounds.
pub fn animate(app: &mut App) {
    app.batched.sync();
    render_once(app);
    app.batched.sync();
    render_once(app);
}

/// One `await renderer.renderAsync( scene, camera )`.
pub fn render_once(app: &mut App) {
    // `scene.add( t )` put every member under the scene in the JS, so the
    // renderer's own `scene.updateMatrixWorld()` composed their world matrices.
    // Here the members hang off the batch, whose `sync()` composed them already;
    // this is the batch node's own matrix.
    app.batched.node().update_matrix_world(true);
    app.renderer.render(&mut app.scene, &mut app.camera);
}

fn main() {
    let mut app = init();
    println!("adapter: {:?}", app.renderer.adapter_info());
    animate(&mut app);
    println!(
        "members: {}, glyph instances: {}",
        app.batched.member_count(),
        app.batched.count()
    );

    let (width, height, pixels) = app.renderer.read_canvas_pixels();
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "target/sdf_text_block.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
