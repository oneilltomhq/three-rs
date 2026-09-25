//! `Sprite` through the real renderer: the WGSL `SpriteNodeMaterial` builds for
//! a sprite against three.js' own dump of `webgpu_sprites`, and a few pixels
//! of a headless render worked out from the camera alone. Nothing here is
//! derived from an image.
//!
//! The fixture `tests/fixtures/webgpu_sprites/sprite.vert.wgsl` is
//! `node tools/dump-webgpu.mjs webgpu_sprites`' `m01_vertex_vertex.wgsl`,
//! verbatim. The material is the example's:
//!
//! ```js
//! const textureNode = texture( map );
//! material.colorNode = textureNode.mul( uv() ).mul( 2 ).saturate();
//! material.opacityNode = textureNode.a;
//! material.rotationNode = userData( 'rotation', 'float' );
//! scene.fogNode = fog( color( 0x0000ff ), rangeFogFactor( 1500, 2100 ) );
//! ```
//!
//! What the WGSL check pins that no pixel check on a centred sprite would:
//! the `position.xy - ( center - 0.5 )` step, which only a `Sprite` (an object
//! with a `center`) gets, and the rotation matrix it feeds.

use std::collections::HashMap;

use three_rs::materials::{setup, SetupContext, SpriteNodeMaterial};
use three_rs::nodes::tsl::{texture, uniform_object, uv};
use three_rs::nodes::{NodeBuilder, Type};
use three_rs::textures::Texture;
use three_rs::{Color, OrthographicCamera, Renderer, RendererParameters, Scene, Sprite, Vector2};

/// Renumber `prefix<digits>` by order of first appearance. three's counters
/// have been running for the whole page and the port's restart per build;
/// three r187 also spells a vertex-stage temporary `nodeConstN` where r186
/// (and the port) spell it `nodeVarN`, so both are folded into one `tmpN`.
fn renumber(wgsl: &str) -> String {
    let mut out = wgsl.replace("nodeConst", "tmp").replace("nodeVar", "tmp");
    // `nodeVarying` became `tmpying` above; put it back first.
    out = out.replace("tmpying", "nodeVarying");
    for prefix in ["nodeUniform", "nodeVarying", "tmp"] {
        let mut seen: HashMap<String, usize> = HashMap::new();
        let mut result = String::with_capacity(out.len());
        let mut rest = out.as_str();
        while let Some(at) = rest.find(prefix) {
            result.push_str(&rest[..at]);
            let after = &rest[at + prefix.len()..];
            let digits: String = after.chars().take_while(char::is_ascii_digit).collect();
            if digits.is_empty() {
                result.push_str(prefix);
                rest = after;
                continue;
            }
            let next = seen.len();
            let n = *seen.entry(digits.clone()).or_insert(next);
            result.push_str(&format!("{prefix}{n}"));
            rest = &after[digits.len()..];
        }
        result.push_str(rest);
        out = result;
    }
    out
}

/// The `objectStruct` declaration: the object group's members, in order.
fn object_struct(wgsl: &str) -> String {
    let start = wgsl.find("struct objectStruct {").expect("an object group");
    let end = start + wgsl[start..].find("};").unwrap();
    renumber(&wgsl[start..end])
}

/// The one line that builds the sprite's view-space position: the
/// `mat2x2` rotation of the centred, scaled quad corner.
fn sprite_position(wgsl: &str) -> String {
    let line = wgsl
        .lines()
        .find(|line| line.contains("mat2x2<f32>"))
        .expect("the sprite rotation");
    let rhs = line.split_once(" = ").expect("an assignment").1;
    renumber(rhs)
}

/// The example's material, built for a `Sprite` (`SetupContext::sprite`).
fn example_program() -> three_rs::nodes::NodeProgram {
    let map = Texture::new(4, 4, Some(vec![0; 64]));
    let mut material = SpriteNodeMaterial::sprite();
    let texture_node = texture(&map);
    material.color_node = Some(texture_node.mul(uv()).mul(2.0).saturate());
    material.opacity_node = Some(texture_node.w());
    // `userData( 'rotation', 'float' )`: an object-group float read off the
    // object being drawn. Every sprite in the example keeps it at 0.
    material.rotation_node = Some(uniform_object(Type::F32, |_| vec![0.0]));
    let ctx = SetupContext {
        sprite: true,
        ..SetupContext::default()
    };
    NodeBuilder::new().build(&setup(&material, &ctx, None))
}

/// Not a whole-file diff. The vendored three.js is r187dev, which differs from
/// the r186 generator the port follows in ways that have nothing to do with
/// sprites (`nodeConst` lets, `VERTEX_` sub-build temps, the render struct's
/// member order — `docs/nodes.md` §8). And the example's
/// `fog( …, rangeFogFactor( 1500, 2100 ) )` reads `positionView` inside
/// three's lazily built graph, where it is the sprite's own `v_positionView`;
/// the port builds the fog factor eagerly, before the material installs its
/// `setupPositionView`, so a sprite scene's fog would read the mesh
/// `positionView` instead (`docs/nodes.md` §24). No ladder rung has a fogged
/// sprite, so the fog is left out here and the check is on what is the
/// sprite's: the object group and the position expression.
#[test]
fn sprite_vertex_matches_three() {
    let three = include_str!("fixtures/webgpu_sprites/sprite.vert.wgsl");
    let program = example_program();
    // uvTransform, modelWorldMatrix, rotation, center.
    assert_eq!(
        object_struct(&program.vertex_wgsl),
        object_struct(three),
        "object group\n{}",
        program.vertex_wgsl
    );
    assert_eq!(
        sprite_position(&program.vertex_wgsl),
        sprite_position(three),
        "sprite position\n{}",
        program.vertex_wgsl
    );
}

/// A `SpriteNodeMaterial` on something that is not a `Sprite` has no
/// `object.center`, and three skips the offset.
#[test]
fn no_center_step_without_a_sprite() {
    let program = NodeBuilder::new().build(&setup(
        &SpriteNodeMaterial::sprite(),
        &SetupContext::default(),
        None,
    ));
    assert!(
        !program.vertex_wgsl.contains("vec2<f32>( 0.5 )"),
        "{}",
        program.vertex_wgsl
    );
}

// ---------------------------------------------------------------------------
// pixels
// ---------------------------------------------------------------------------

const W: usize = 64;
const H: usize = 64;

/// `new OrthographicCamera( 0, W, H, 0, 0.1, 10 )` at `z = 5`: one world unit
/// is one pixel, world `( x, y )` lands in column `x`, row `H - y`.
fn pixel_camera() -> OrthographicCamera {
    let mut camera = OrthographicCamera::new(0.0, W as f64, H as f64, 0.0, 0.1, 10.0);
    camera.object.position.z = 5.0;
    camera.update_matrix_world();
    camera
}

/// One red 16 x 16 sprite at world `( 32, 32 )`, anchored at `center`.
fn render_sprite(center: Vector2) -> Vec<u8> {
    let mut scene = Scene::new();
    scene.set_background(Color::from_hex(0x000000));
    let mut material = SpriteNodeMaterial::sprite();
    material.color = Color::from_hex(0xff0000);
    let sprite = Sprite::new(material);
    {
        let mut object = sprite.borrow_mut();
        object.position.set(32.0, 32.0, 0.0);
        object.scale.set(16.0, 16.0, 1.0);
        object.payload.sprite_mut().unwrap().center = center;
    }
    scene.add(&sprite);

    let mut camera = pixel_camera();
    let mut renderer = Renderer::new(RendererParameters { antialias: false }).unwrap();
    renderer.set_pixel_ratio(1.0);
    renderer.set_size(W as f64, H as f64);
    renderer.render(&mut scene, &mut camera);
    let (w, h, pixels) = renderer.read_canvas_pixels().unwrap();
    assert_eq!((w as usize, h as usize), (W, H));
    pixels
}

/// The pixel whose centre is world `( x, y )`.
fn rgb(pixels: &[u8], x: f64, y: f64) -> [u8; 3] {
    let column = x.floor() as usize;
    let row = H - 1 - y.floor() as usize;
    let at = (row * W + column) * 4;
    [pixels[at], pixels[at + 1], pixels[at + 2]]
}

const RED: [u8; 3] = [255, 0, 0];
const BLACK: [u8; 3] = [0, 0, 0];

/// The default `center = ( 0.5, 0.5 )` centres the quad on the sprite's
/// position: world x and y in `[24, 40)` are covered, one pixel outside is not.
#[test]
fn centred_sprite_covers_its_square() {
    let pixels = render_sprite(Vector2::new(0.5, 0.5));
    for (x, y) in [(32.5, 32.5), (24.5, 24.5), (39.5, 39.5), (24.5, 39.5)] {
        assert_eq!(rgb(&pixels, x, y), RED, "({x}, {y}) is inside the sprite");
    }
    for (x, y) in [(23.5, 32.5), (40.5, 32.5), (32.5, 23.5), (32.5, 40.5)] {
        assert_eq!(
            rgb(&pixels, x, y),
            BLACK,
            "({x}, {y}) is outside the sprite"
        );
    }
}

/// `center = ( 0, 0 )` puts the sprite's position at the quad's bottom-left
/// corner, so it covers `[32, 48)` on both axes — the
/// `alignedPosition.sub( center.sub( 0.5 ) )` step, per draw.
#[test]
fn center_moves_the_anchor() {
    let pixels = render_sprite(Vector2::new(0.0, 0.0));
    for (x, y) in [(32.5, 32.5), (47.5, 47.5), (40.5, 40.5)] {
        assert_eq!(rgb(&pixels, x, y), RED, "({x}, {y}) is inside the sprite");
    }
    for (x, y) in [(31.5, 32.5), (48.5, 40.5), (40.5, 31.5), (24.5, 24.5)] {
        assert_eq!(
            rgb(&pixels, x, y),
            BLACK,
            "({x}, {y}) is outside the sprite"
        );
    }
}
