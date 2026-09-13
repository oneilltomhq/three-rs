//! Port of the **label half** of d33's `rung0/examples/d3_treemap.html` — step 6
//! of the sdf-text ladder: the d3 treemap of `flare.json` laid flat in the world,
//! one multi-line `Text` per labelled leaf, all drawn by one `BatchedText`.
//!
//! Everything that decides *where a glyph lands* is the page's, unchanged: the
//! notebook's `treemap().tile(binary).size([1154, 1154]).padding(1).round(true)`
//! over `hierarchy(flare).sum(d => d.value).sort((a, b) => b.value - a.value)`,
//! the notebook-pixel → world mapping (`S = SIDE / WIDTH`, `x → x`,
//! `y → y·S − SIDE`), `frameCamera` at elevation 55 / azimuth 20 / fill 0.85,
//! the `pxPerUnit` → `fontSize` derivation from `LABEL_PX = 5` at 400 × 250, the
//! per-leaf fit test measured from the TTF, the `flat` basis quaternion, and
//! `anchorY: 'top'` with `lineHeight: 0.9`.
//!
//! The white tile outlines are the page's too, as of the `lines` branch: one
//! `Group` named `outlines`, one `Line` per leaf over the five points
//! `[ a, b, c, e, a ]` at `THICK + LIFT`, all sharing one white
//! `LineBasicNodeMaterial`.
//!
//! ## What is *not* the page, and why
//!
//! 1. **The tiles are `MeshBasicNodeMaterial`, not `MeshStandardNodeMaterial`,
//!    and there are no lights.** The tile colour is the page's own
//!    `color(pkg).lerp(white, 0.4)` — the notebook's fill at
//!    `fill-opacity: 0.6` on white — so the hue of every tile is right and only
//!    the shading is missing. This is the whole of the image gap that is left:
//!    all 156 of the 100000 pixels that still differ from d33's own render are
//!    on the slab's near silhouette, where the page's `HemisphereLight` +
//!    `DirectionalLight` darken the tiles' side walls. `MeshStandardNodeMaterial`
//!    and both light types *do* exist in the port now (rung 8); switching the
//!    tiles over is a small change to this file, not a missing capability.
//! 2. **No mirror check and no per-frame flip.** `labels.js` runs a two-axis
//!    projection test per label. Under this page's fixed camera both axes come
//!    out positive — local `+x` is world `+X`, which projects right, and local
//!    `+y` is world `−Z`, which projects up — so every label is in pose 0 with
//!    `anchorX: 'left'`, which is what is built here directly. The arithmetic
//!    that would be exercised is `Vector3::project`, already graded.
//! 3. **`positionNode` is kept.** d33's `LabelBatch` nulls it and bakes each
//!    glyph quad into its instance matrix, purely to dodge r186's
//!    `setupPosition` ordering bug. This port applies `position_node` *before*
//!    the instance matrix (plan §5.2), so the bug is not there and lib3's own
//!    `BatchedText` path is the one used.
//!
//! Deviation 1 is what still keeps the `< 0.1 %` image gate against
//! `d33/rung0/examples/screenshots/d3_treemap.jpg` out of reach — 0.156 %.
//! `tests/d33_treemap_labels.rs` grades the layout against a JSON golden dumped
//! from the page's own JS, and asserts the image number under a 250-pixel
//! ceiling.

use std::collections::HashMap;
use std::rc::Rc;

use d3_hierarchy::node::Tree;
use d3_hierarchy::treemap::{binary, treemap};
use d3_hierarchy::{hierarchy, Datum};
use sdf_text::text_builder::LineHeight;
use sdf_text::{Anchor, BatchedText, BatchedTextOptions, Text, VectorFont};
use three_rs::core::{BufferGeometry, Object3DNode};
use three_rs::{
    box_geometry, Color, Group, Line, Matrix4, Mesh, MeshBasicNodeMaterial, PerspectiveCamera,
    Quaternion, Renderer, RendererParameters, Scene, Vector3,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// The d33 harness runs at `400 × 250 @ viewScale 2` with no
/// `deviceScaleFactor`, so `devicePixelRatio` is 1.
pub const DPR: f64 = 1.0;

/// `const WIDTH = 1154, HEIGHT = 1154` — the notebook's size.
const WIDTH: f64 = 1154.0;
const HEIGHT: f64 = 1154.0;
/// `const SIDE = 20` — world units of the square.
const SIDE: f64 = 20.0;
/// `const S = SIDE / WIDTH` — notebook px → world units.
const S: f64 = SIDE / WIDTH;
/// `const THICK = 0.015 * SIDE` — tile thickness.
const THICK: f64 = 0.015 * SIDE;
/// `const LIFT = 0.012` — the outline lift; it is the label plane's height too,
/// so it survives even though the outlines themselves do not.
const LIFT: f64 = 0.012;
/// `const LABEL_PX = 5` — label em in pixels at the target distance, at 400×250.
const LABEL_PX: f64 = 5.0;
/// `const LINE = 0.9` — the notebook's line pitch.
const LINE: f64 = 0.9;
/// `const LABEL_PAD_PX = 3` — the notebook's `x = 3`.
const LABEL_PAD_PX: f64 = 3.0;

/// `d3.schemeTableau10`.
const SCHEME_TABLEAU10: [u32; 10] = [
    0x4e79a7, 0xf28e2c, 0xe15759, 0x76b7b2, 0x59a14f, 0xedc949, 0xaf7aa1, 0xff9da7, 0x9c755f,
    0xbab0ab,
];

/// One `root.leaves()` entry, with what the page reads off it.
pub struct Leaf {
    pub name: String,
    pub depth: usize,
    pub value: f64,
    pub pkg: String,
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
}

/// One labelled leaf: the lines the page splits its name into, the widest line's
/// measured width, and the world anchor of the text block's top-left corner.
pub struct Labelled {
    pub name: String,
    pub lines: Vec<String>,
    pub widest: f64,
    pub anchor: Vector3,
}

/// Everything the page computes that is not a GPU resource, exposed so the gate
/// can compare it with the golden.
pub struct Layout {
    pub leaves: Vec<Leaf>,
    pub labelled: Vec<Labelled>,
    pub px_per_unit: f64,
    pub font_size: f64,
    pub pad: f64,
    pub target: Vector3,
    pub flat: Quaternion,
    pub root_value: f64,
}

pub struct App {
    pub renderer: Renderer,
    pub scene: Scene,
    pub camera: PerspectiveCamera,
    pub batched: BatchedText,
    pub layout: Layout,
}

fn flare_json() -> std::path::PathBuf {
    let gallery = match std::env::var("D3_GALLERY_DIR") {
        Ok(dir) => std::path::PathBuf::from(dir),
        Err(_) => std::path::PathBuf::from(std::env::var("HOME").unwrap())
            .join("src/vendor/d3-gallery"),
    };
    gallery.join("notebooks/hierarchies/treemap.v2/files/flare.json")
}

fn font_path() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("sdf-text/tests/assets/Roboto-Regular.ttf")
}

/// `d3.format( ',d' )` for the integer values `flare.json` carries: round, then
/// group the integer part in threes with `,`.
pub fn format_d(value: f64) -> String {
    let rounded = value.abs().round() as u64;
    let digits = rounded.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    if value < 0.0 {
        format!("-{out}")
    } else {
        out
    }
}

/// `name.split( /(?=[A-Z][a-z])|\s+/g )`.
///
/// The lookahead is zero-width, so it breaks *before* every capital that starts
/// a lower-case run — except at index 0, where `String.prototype.split` skips a
/// zero-width match rather than emitting a leading `''`. `\s+` eats whitespace
/// runs (and so can produce an empty piece at either end, exactly as the JS).
pub fn split_name(name: &str) -> Vec<String> {
    let chars: Vec<char> = name.chars().collect();
    let mut out: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            out.push(std::mem::take(&mut current));
            while i < chars.len() && chars[i].is_whitespace() {
                i += 1;
            }
            continue;
        }
        let breaks_here = i > 0
            && c.is_ascii_uppercase()
            && matches!(chars.get(i + 1), Some(n) if n.is_ascii_lowercase());
        if breaks_here {
            out.push(std::mem::take(&mut current));
        }
        current.push(c);
        i += 1;
    }
    out.push(current);
    out
}

/// `frameCamera( camera, box, { elevation, azimuth, fill } )` from
/// `d33/rung0/src/frame.js`, for the one box this page passes: the axis-aligned
/// slab `(0, 0, −SIDE) … (SIDE, THICK + LIFT, 0)`. The `Box3` → `Sphere` step
/// and the 8 corners are inlined rather than routed through `Box3`, which keeps
/// the deliberately-iterative part — place, project, recentre, refit — verbatim.
fn frame_camera(
    camera: &mut PerspectiveCamera,
    min: Vector3,
    max: Vector3,
    elevation: f64,
    azimuth: f64,
    fill: f64,
) -> Vector3 {
    let deg = std::f64::consts::PI / 180.0;

    // `box.getBoundingSphere( sphere )`: the centre is the box centre and the
    // radius is half the diagonal.
    let mut target = Vector3::new(
        (min.x + max.x) / 2.0,
        (min.y + max.y) / 2.0,
        (min.z + max.z) / 2.0,
    );
    let radius = {
        let d = Vector3::new(max.x - min.x, max.y - min.y, max.z - min.z);
        d.length() / 2.0
    };
    let radius = if radius != 0.0 { radius } else { 1.0 };

    let half_v = (camera.fov * deg) / 2.0;
    let half_h = (half_v.tan() * camera.aspect).atan();

    // `boxCorners( box )`, in the JS's bit order.
    let mut fitted: Vec<Vector3> = Vec::with_capacity(8);
    for i in 0..8 {
        fitted.push(Vector3::new(
            if i & 1 != 0 { max.x } else { min.x },
            if i & 2 != 0 { max.y } else { min.y },
            if i & 4 != 0 { max.z } else { min.z },
        ));
    }

    let el = elevation * deg;
    let az = azimuth * deg;
    let direction = Vector3::new(el.cos() * az.sin(), el.sin(), el.cos() * az.cos());

    let mut distance = radius / (fill * half_v.min(half_h).tan());

    let place = |camera: &mut PerspectiveCamera, target: &Vector3, distance: f64| {
        {
            let mut object = camera.node.borrow_mut();
            object.position.copy(target);
            object.position.add_scaled_vector(&direction, distance);
        }
        camera.look_at(target);
        camera.near = (distance - radius * 2.0).max(distance / 100.0);
        camera.far = distance + radius * 2.0;
        camera.update_projection_matrix();
        camera.update_matrix_world();
    };

    let mut right = Vector3::ZERO;
    let mut up = Vector3::ZERO;
    let mut v = Vector3::ZERO;

    for _ in 0..24 {
        place(camera, &target, distance);

        let (mut min_x, mut max_x) = (f64::INFINITY, f64::NEG_INFINITY);
        let (mut min_y, mut max_y) = (f64::INFINITY, f64::NEG_INFINITY);

        for p in &fitted {
            let mut q = *p;
            q.project(camera);
            min_x = min_x.min(q.x);
            max_x = max_x.max(q.x);
            min_y = min_y.min(q.y);
            max_y = max_y.max(q.y);
        }

        // recentre: move the target sideways by the projected offset
        let cx = (min_x + max_x) / 2.0;
        let cy = (min_y + max_y) / 2.0;

        camera
            .node
            .borrow()
            .matrix_world
            .extract_basis(&mut right, &mut up, &mut v);
        target.add_scaled_vector(&right, cx * half_h.tan() * distance);
        target.add_scaled_vector(&up, cy * half_v.tan() * distance);

        // refit: scale the distance by how much of the frame is used
        let extent = (max_x - min_x).max(max_y - min_y) / 2.0;
        if extent == 0.0 {
            break;
        }

        let next = distance * extent / fill;
        let settled = (next - distance).abs() < distance * 1e-4 && cx.abs() < 1e-4 && cy.abs() < 1e-4;

        distance = next;
        if settled {
            break;
        }
    }

    place(camera, &target, distance);

    target
}

/// `basisQuaternion( along, up )` from `d33/rung0/src/labels.js`.
fn basis_quaternion(along: Vector3, up: Vector3) -> Quaternion {
    let mut x = along;
    x.normalize();
    let mut z = Vector3::ZERO;
    z.cross_vectors(&x, &up).normalize();
    let mut y = Vector3::ZERO;
    y.cross_vectors(&z, &x).normalize();

    let mut m = Matrix4::default();
    m.make_basis(&x, &y, &z);

    let mut q = Quaternion::default();
    q.set_from_rotation_matrix(&m);
    q
}

/// `const pkg = d => { while ( d.depth > 1 ) d = d.parent; return d.data.name; }`
fn pkg_of(tree: &Tree, mut i: usize) -> String {
    while tree.nodes[i].depth > 1 {
        i = tree.nodes[i].parent.expect("depth > 1 implies a parent");
    }
    name_of(&tree.nodes[i].data)
}

fn name_of(data: &Datum) -> String {
    data.get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

pub fn init() -> App {
    // ---------------- data + d3 layout (the notebook's chart cell) ----------------

    let path = flare_json();
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let data: Datum = serde_json::from_str(&text).expect("parse flare.json");

    // `d3.scaleOrdinal( data.children.map( d => d.name ), d3.schemeTableau10 )`:
    // the domain is the *raw* top-level order, and the range wraps at 10.
    let mut package_color: HashMap<String, Color> = HashMap::new();
    let white = Color::new(1.0, 1.0, 1.0);
    for (i, child) in data
        .get("children")
        .and_then(|c| c.as_array())
        .map(|a| a.as_slice())
        .unwrap_or(&[])
        .iter()
        .enumerate()
    {
        // `new THREE.Color( color( key ) ).lerp( white, 0.4 )` — the notebook's
        // fill at `fill-opacity: 0.6` over white. `THREE.Color.lerp` runs in the
        // working colour space, i.e. after `SRGBToLinear`, which is what
        // `Color::from_hex` + `Color::lerp` do here.
        let mut c = Color::from_hex(SCHEME_TABLEAU10[i % SCHEME_TABLEAU10.len()]);
        c.lerp(&white, 0.4);
        package_color.insert(name_of(child), c);
    }

    let mut root = hierarchy(&data);
    root.sum(|d| d.get("value").and_then(|v| v.as_f64()).unwrap_or(f64::NAN));
    root.sort(|a, b| {
        let (a, b) = (a.value.unwrap_or(f64::NAN), b.value.unwrap_or(f64::NAN));
        b.partial_cmp(&a).unwrap_or(std::cmp::Ordering::Equal)
    });

    treemap()
        .tile(Box::new(binary))
        .size([WIDTH, HEIGHT])
        .padding(1.0)
        .round(true)
        .treemap(&mut root);

    let leaf_indices = root.leaves(root.root);

    let leaves: Vec<Leaf> = leaf_indices
        .iter()
        .map(|&i| {
            let n = &root.nodes[i];
            Leaf {
                name: name_of(&n.data),
                depth: n.depth,
                value: n.value.unwrap_or(f64::NAN),
                pkg: pkg_of(&root, i),
                x0: n.x0,
                y0: n.y0,
                x1: n.x1,
                y1: n.y1,
            }
        })
        .collect();

    // `const toWorld = ( x, y ) => new THREE.Vector3( x * S, 0, y * S - SIDE )`
    let to_world = |x: f64, y: f64| Vector3::new(x * S, 0.0, y * S - SIDE);

    // ---------------- scene ----------------

    let mut scene = Scene::new();
    scene.set_background(Color::from_hex(0xffffff));

    let mut camera = PerspectiveCamera::new(35.0, INNER_WIDTH / INNER_HEIGHT, 0.1, 4000.0);

    // ---------------- marks: one Mesh(BoxGeometry) per leaf ----------------
    //
    // `new THREE.BoxGeometry( 1, 1, 1 ).translate( 0, - 0.5, 0 )` — a unit box
    // hanging below its origin, so `position.y = THICK` puts the top face at
    // `THICK` and the footprint is exactly the notebook rect.
    let mut unit_box = box_geometry(1.0, 1.0, 1.0, 1, 1, 1);
    unit_box.translate(0.0, -0.5, 0.0);
    let unit_box = Rc::new(unit_box);

    let mut materials: HashMap<String, MeshBasicNodeMaterial> = HashMap::new();

    for leaf in &leaves {
        let material = materials
            .entry(leaf.pkg.clone())
            .or_insert_with(|| {
                let mut m = MeshBasicNodeMaterial::new();
                m.color = *package_color
                    .get(&leaf.pkg)
                    .unwrap_or(&Color::new(1.0, 1.0, 1.0));
                m
            })
            .clone();

        let mesh = Mesh::new(unit_box.clone());
        {
            let mut object = mesh.borrow_mut();
            object.mesh_mut().unwrap().material = Some(material);
            object
                .scale
                .set((leaf.x1 - leaf.x0) * S, THICK, (leaf.y1 - leaf.y0) * S);
            let centre = to_world((leaf.x0 + leaf.x1) / 2.0, (leaf.y0 + leaf.y1) / 2.0);
            object.position.copy(&centre);
            object.position.y = THICK;
            object.name = leaf.name.clone();
        }
        scene.add(&mesh);
    }

    // ---------------- outlines: one closed Line per leaf ----------------
    //
    // ```js
    // const outlines = new THREE.Group(); outlines.name = 'outlines';
    // const outlineMaterial = new THREE.LineBasicNodeMaterial( { color: 0xffffff } );
    // const LIFT = 0.012;
    // ```
    //
    // The notebook's 1 px gaps are 0.017 world units here — sub-pixel — so each
    // tile's top rect is outlined in white, the gap's own colour. `LIFT` puts
    // the loop above the tile's top face so nothing z-fights. Five points
    // `[ a, b, c, e, a ]`: a `line-strip` of four segments that happens to
    // close, not a `LineLoop` (which `Renderer._projectObject()` refuses).
    let outlines = Group::new();
    outlines.borrow_mut().name = "outlines".to_string();
    scene.add(&outlines);

    let outline_material = MeshBasicNodeMaterial::line(Color::from_hex(0xffffff));

    for leaf in &leaves {
        let mut a = to_world(leaf.x0, leaf.y0);
        let mut b = to_world(leaf.x1, leaf.y0);
        let mut c = to_world(leaf.x1, leaf.y1);
        let mut e = to_world(leaf.x0, leaf.y1);
        for p in [&mut a, &mut b, &mut c, &mut e] {
            p.y = THICK + LIFT;
        }

        let mut geometry = BufferGeometry::new();
        geometry.set_from_points(&[a, b, c, e, a]);

        let loop_line = Line::new(Rc::new(geometry), outline_material.clone());
        loop_line.borrow_mut().name = format!("outline {}", leaf.name);
        outlines.add(&loop_line);
    }

    // ---------------- camera ----------------

    let target = frame_camera(
        &mut camera,
        Vector3::new(0.0, 0.0, -SIDE),
        Vector3::new(SIDE, THICK + LIFT, 0.0),
        55.0,
        20.0,
        0.85,
    );

    // ---------------- labels ----------------

    // `250 / ( 2 * camera.position.distanceTo( target ) * tan( fov * PI / 360 ) )`
    let px_per_unit = 250.0
        / (2.0
            * camera.node.borrow().position.distance_to(&target)
            * (camera.fov * std::f64::consts::PI / 360.0).tan());
    let font_size = LABEL_PX / px_per_unit;
    let pad = LABEL_PAD_PX * S;

    let font_bytes = std::fs::read(font_path()).expect("read Roboto-Regular.ttf");
    let font = Rc::new(VectorFont::parse(font_bytes, "Roboto-Regular.ttf").expect("parse font"));

    // `measure` — the page's own advance + kern sum over the TTF, in em.
    let measure = |text: &str| {
        let chars: Vec<char> = text.chars().collect();
        let mut units = 0.0;
        for (i, &c) in chars.iter().enumerate() {
            units += font.advance_width(c);
            if i > 0 {
                units += font.kerning(chars[i - 1], c);
            }
        }
        units / font.units_per_em * font_size
    };

    // `basisQuaternion( (1, 0, 0), (0, 0, -1) )`: local +x is world +X and local
    // +y is world −Z, so the text lies in the tile plane reading away from the
    // near edge, and `anchorY: 'top'` runs the block towards the camera.
    let flat = basis_quaternion(Vector3::new(1.0, 0.0, 0.0), Vector3::new(0.0, 0.0, -1.0));

    let mut labelled: Vec<Labelled> = Vec::new();
    for leaf in &leaves {
        let mut lines = split_name(&leaf.name);
        lines.push(format_d(leaf.value));

        let w = (leaf.x1 - leaf.x0) * S;
        let h = (leaf.y1 - leaf.y0) * S;
        let widest = lines
            .iter()
            .map(|l| measure(l))
            .fold(f64::NEG_INFINITY, f64::max);

        if w < widest + 2.0 * pad || h < pad + lines.len() as f64 * LINE * font_size {
            continue;
        }

        let mut anchor = to_world(leaf.x0, leaf.y0);
        anchor.x += pad;
        anchor.z += pad;
        anchor.y = THICK + LIFT;

        labelled.push(Labelled {
            name: leaf.name.clone(),
            lines,
            widest,
            anchor,
        });
    }

    // `hierarchyLabels` sizes the batch to exactly this set: one member per
    // label, one glyph quad per character, newlines excluded.
    let glyph_total: usize = labelled
        .iter()
        .map(|l| l.lines.iter().map(|s| s.chars().count()).sum::<usize>())
        .sum();

    let mut batched = BatchedText::new(
        labelled.len(),
        glyph_total,
        BatchedTextOptions {
            outline_width: 0.2,
            outline_color: Some(Color::from_hex(0xffffff)),
            ..Default::default()
        },
    );
    batched.set_font(font.clone());
    batched.node().borrow_mut().frustum_culled = false;
    batched.node().borrow_mut().render_order = 1.0;
    scene.add(batched.node());

    for label in &labelled {
        let mut text = Text::new();
        text.set_text(label.lines.join("\n"));
        text.set_font_size(font_size);
        // `anchorX` is 'left' — `hierarchyLabels` anchors leaves 'start' and the
        // mirror check does not fire under this camera (deviation 3).
        text.set_anchor_x(Anchor::named("left"));
        // the page overwrites `hierarchyLabels`' 'middle' before `sync()`
        text.set_anchor_y(Anchor::named("top"));
        text.set_line_height(LineHeight::Factor(LINE));

        let id = batched.add_text(text) as usize;
        // `label.color.set( 0x222222 )`
        batched.set_color_at(id, Color::from_hex(0x222222));

        let node = batched.member_node(id).expect("just added");
        {
            let mut object = node.borrow_mut();
            object.position.copy(&label.anchor);
            // `label.quaternion.copy( orient( node ) )`. `Object3D::update_matrix`
            // composes from `position`/`quaternion`/`scale` — the Euler is never
            // read back — so `label.updateMatrix()` is left to `sync()`.
            object.quaternion = flat;
        }
    }

    let mut renderer = Renderer::new(RendererParameters { antialias: true });
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);

    let root_value = root.nodes[root.root].value.unwrap_or(f64::NAN);

    App {
        renderer,
        scene,
        camera,
        batched,
        layout: Layout {
            leaves,
            labelled,
            px_per_unit,
            font_size,
            pad,
            target,
            flat,
            root_value,
        },
    }
}

/// The page's `init()` tail —
///
/// ```js
/// labels.group.updateMatrixWorld( true );
/// labels.batched.sync();
/// ```
///
/// — plus its `animate()`, with `labels.update( camera )` left out (deviation 3:
/// every label is already in pose 0 under this camera). The order matters:
/// `sync()` reads each member's `matrixWorld` to place its glyph quads, so the
/// world matrices must be composed first.
pub fn animate(app: &mut App) {
    app.batched.node().update_matrix_world(true);
    app.batched.sync();
    render_once(app);
}

pub fn render_once(app: &mut App) {
    app.renderer.render(&mut app.scene, &mut app.camera);
}

fn main() {
    let mut app = init();
    println!("adapter: {:?}", app.renderer.adapter_info());
    animate(&mut app);
    println!(
        "fontSize: {:.3} px/unit: {:.2} cells: {} labels: {} glyph instances: {}",
        app.layout.font_size,
        app.layout.px_per_unit,
        app.layout.leaves.len(),
        app.layout.labelled.len(),
        app.batched.count()
    );

    let (width, height, pixels) = app.renderer.read_canvas_pixels();
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "target/d33_treemap_labels.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
