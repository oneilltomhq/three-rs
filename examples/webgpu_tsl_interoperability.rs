//! Port of `three.js/examples/webgpu_tsl_interoperability.html`, calling the
//! three-rs API in the same order the page's `init()` / `animate()` do.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1.
//!
//! Two full-width quads showing the same CRT shader written twice: the top one
//! out of two `wgslFn()` blocks — the page's WGSL copied through verbatim,
//! tabs and all — and the bottom one out of TSL nodes. The two are meant to
//! look alike but are not identical: the TSL half scrolls its sample point
//! down by 1.5 and multiplies the pulse into all three channels through one
//! `vec4`, where the WGSL half writes `.r`, `.b` and `.g` separately.
//!
//! `renderer.outputColorSpace = LinearSRGBColorSpace` is the working space, so
//! `needsFrameBufferTarget` is false and the scene is drawn straight into the
//! canvas: three's dump has no colour-transform pass behind these two quads.
//!
//! `renderer.inspector.createParameters()` builds a GUI over seven of the
//! uniforms. It draws no `Math.random()` and the graded frame is the first, so
//! every uniform still holds its constructor value.

use std::rc::Rc;

// `PerspectiveCamera` only names the type in `controls_and_camera`'s signature,
// which is uniform across the examples; this page's camera is orthographic and it
// creates no controls, so the function returns `None`.
use three_rs::addons::controls::OrbitControls;
use three_rs::materials::MeshBasicNodeMaterial;
use three_rs::math::ColorSpace;
use three_rs::nodes::tsl::{
    attribute, block, call_wgsl, float, floor, fract, mod_float, position_geometry, texture,
    texture_uv, time, to_var, uniform_value, varying_property, vec2, vec2_join, vec3_join, wgsl_fn,
};
use three_rs::nodes::Type;
use three_rs::textures::Wrapping;
use three_rs::PerspectiveCamera;
use three_rs::{
    plane_geometry, Mesh, OrthographicCamera, Renderer, RendererParameters, Scene, TextureLoader,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `const dpr = window.devicePixelRatio`
pub const DPR: f64 = 1.0;

/// The page's `wgslVertexShader` source, verbatim — the six tabs of
/// indentation and the trailing newline are the template literal's own and
/// land in the generated `// codes` block unchanged.
const CRT_VERTEX_WGSL: &str = "
					fn crtVertex(
	 					position: vec3f,
						uv: vec2f
					) -> vec3<f32> {
						varyings.vUv = uv;
						return position;
					}
				";

/// The page's `wgslFragmentShader` source, verbatim.
const CRT_FRAGMENT_WGSL: &str = "
					fn crtFragment(
						vUv: vec2f,
						tex: texture_2d<f32>,
						texSampler: sampler,
						crtWidth: f32,
						crtHeight: f32,
						cellOffset: f32,
						cellSize: f32,
						borderMask: f32,
						time: f32,
						speed: f32,
						pulseIntensity: f32,
						pulseWidth: f32,
						pulseRate: f32
					) -> vec3<f32> {
						// Convert uv into map of pixels
						var pixel = ( vUv * 0.5 + 0.5 ) * vec2<f32>(
							crtWidth,
							crtHeight
						);
						// Coordinate for each cell in the pixel map
						let coord = pixel / cellSize;
						// Three color values for each cell (r, g, b)
						let subcoord = coord * vec2f( 3.0, 1.0 );
						let offset = vec2<f32>( 0, fract( floor( coord.x ) * cellOffset ) );

						let maskCoord = floor( coord + offset ) * cellSize;

						var samplePoint = maskCoord / vec2<f32>(crtWidth, crtHeight);
						samplePoint.x += fract( time * speed / 20 );

						var color = textureSample(
							tex,
							texSampler,
							samplePoint
						).xyz;

						// Current implementation does not give an even amount of space to each r, g, b unit of a cell
						// Fix/hack this by multiplying subCoord.x by cellSize at cellSizes below 6
						let ind = floor( subcoord.x ) % 3;

						var maskColor = vec3<f32>(
							f32( ind == 0.0 ),
							f32( ind == 1.0 ),
							f32( ind == 2.0 )
						) * 3.0;

						let cellUV = fract( subcoord + offset ) * 2.0 - 1.0;
						var border: vec2<f32> = 1.0 - cellUV * cellUV * borderMask;

						maskColor *= vec3f( clamp( border.x, 0.0, 1.0 ) * clamp( border.y, 0.0, 1.0) );

						color *= maskColor;

						color.r *= 1.0 + pulseIntensity * sin( pixel.y / pulseWidth + time * pulseRate );
						color.b *= 1.0 + pulseIntensity * sin( pixel.y / pulseWidth + time * pulseRate );
						color.g *= 1.0 + pulseIntensity * sin( pixel.y / pulseWidth + time * pulseRate );

						return color;
					}
				";

pub struct App {
    pub renderer: Renderer,
    pub scene: Scene,
    pub camera: OrthographicCamera,
}

fn examples_dir() -> std::path::PathBuf {
    three_rs::testing::three_js_dir().join("examples")
}

/// The page's two materials, which `dump_wgsl` builds without a renderer.
///
/// Everything from the module-scope uniform pair down to
/// `tslShaderMaterial.colorNode`, in the page's order.
pub fn materials() -> (MeshBasicNodeMaterial, MeshBasicNodeMaterial) {
    // Module scope on the page: one pair of uniform nodes shared by both
    // materials.
    let crt_width = uniform_value(Type::F32, vec![1608.0]);
    let crt_height = uniform_value(Type::F32, vec![1608.0]);

    // `varyingProperty( 'vec2', 'vUv' )` — a varying named by the application
    // rather than numbered, because the hand-written WGSL writes it by name.
    let v_uv = varying_property("vUv", Type::Vec2, false);

    // In WGSL, access varying properties from the varying struct.
    let wgsl_vertex_shader = wgsl_fn(CRT_VERTEX_WGSL, vec![v_uv.clone()]);

    // Only wgsl vertex shaders take varyings arguments when defined. For a
    // wgsl fragment shader, pass the varyingProperty node to the fragment
    // shader's constructor to access the varying value computed by the vertex
    // shader.
    let wgsl_fragment_shader = wgsl_fn(CRT_FRAGMENT_WGSL, vec![]);

    let planet_texture = TextureLoader::new()
        .load(examples_dir().join("textures/planets/earth_lights_2048.png"))
        .unwrap();
    planet_texture.set_wrapping(Wrapping::Repeat, Wrapping::Repeat);

    // Node Uniforms: passed to WGSL functions, manipulated directly in TSL
    // functions.
    let cell_offset = uniform_value(Type::F32, vec![0.5]);
    let cell_size = uniform_value(Type::F32, vec![6.0]);
    let border_mask = uniform_value(Type::F32, vec![1.0]);
    let pulse_intensity = uniform_value(Type::F32, vec![0.06]);
    let pulse_width = uniform_value(Type::F32, vec![60.0]);
    let pulse_rate = uniform_value(Type::F32, vec![20.0]);
    let wgsl_shader_speed = uniform_value(Type::F32, vec![1.0]);
    let tsl_shader_speed = uniform_value(Type::F32, vec![1.0]);

    //

    let mut wgsl_shader_material = MeshBasicNodeMaterial::new();

    // Accessed attributes correspond to a Mesh or BufferGeometry's
    // `setAttribute()` calls.
    wgsl_shader_material.position_node = Some(call_wgsl(
        &wgsl_vertex_shader,
        vec![
            ("position", attribute("position", Type::Vec3)),
            ("uv", attribute("uv", Type::Vec2)),
        ],
    ));

    // One `texture( planetTexture )` node bound to both the `texture_2d<f32>`
    // parameter and the `sampler` one, which is how three's example writes it.
    let planet_node = texture(&planet_texture);
    wgsl_shader_material.fragment_node = Some(call_wgsl(
        &wgsl_fragment_shader,
        vec![
            ("vUv", v_uv.clone()),
            ("tex", planet_node.clone()),
            ("texSampler", planet_node),
            ("crtWidth", crt_width.clone()),
            ("crtHeight", crt_height.clone()),
            ("cellOffset", cell_offset.clone()),
            ("cellSize", cell_size.clone()),
            ("borderMask", border_mask.clone()),
            ("time", time()),
            ("speed", wgsl_shader_speed),
            ("pulseIntensity", pulse_intensity.clone()),
            ("pulseWidth", pulse_width.clone()),
            ("pulseRate", pulse_rate.clone()),
        ],
    ));

    //

    // `Fn( () => { vUv.assign( uv() ); return positionGeometry; } )`. Three's
    // `uv()` is `attribute( 'uv' )`; the port's is that attribute already
    // routed through a varying, which is the fragment stage's reading of it
    // and not what a vertex shader assigning to another varying wants.
    let tsl_vertex_shader = block(
        vec![v_uv.assign(attribute("uv", Type::Vec2))],
        position_geometry(),
    );

    let tsl_fragment_shader = {
        let dimensions = vec2_join(vec![crt_width, crt_height]);
        let translated_uv = v_uv.mul(0.5).add(0.5);
        let pixel = translated_uv.mul(dimensions.clone());

        let coord = pixel.clone().div(cell_size.clone());
        let sub_coord = coord.clone().mul(vec2(3.0, 1.0));

        let cell_offset = vec2_join(vec![float(0.0), fract(floor(coord.x()).mul(cell_offset))]);

        let mask_coord = floor(coord.add(cell_offset.clone())).mul(cell_size);
        let sample_point = to_var(None, mask_coord.div(dimensions));
        let scaled_time = time().mul(tsl_shader_speed);

        // `samplePoint.x = …` / `samplePoint.y = …`: two assignments to a
        // var, which is why the quotient above is one.
        let statements = vec![
            sample_point
                .x()
                .assign(sample_point.x().add(fract(scaled_time.div(20.0)))),
            sample_point.y().assign(sample_point.y().sub(1.5)),
        ];

        let color = texture_uv(&planet_texture, sample_point);

        let ind = mod_float(floor(sub_coord.x()), float(3.0));

        let mask_color = vec3_join(vec![ind.equal(0.0), ind.equal(1.0), ind.equal(2.0)]).mul(3.0);

        let sub_coord_offset = fract(sub_coord.add(cell_offset));
        let cell_uv = sub_coord_offset.mul(2.0).sub(1.0);

        let border = float(1.0).sub(cell_uv.clone().mul(cell_uv).mul(border_mask));

        let border_clamp = border.x().clamp(0.0, 1.0).mul(border.y().clamp(0.0, 1.0));
        let mask_color = mask_color.mul(border_clamp);

        let color = color.mul(mask_color);

        let pixel_dampen = pixel.y().div(pulse_width);
        let pulse = pixel_dampen.add(time().mul(pulse_rate)).sin();
        let pulse = pulse.mul(pulse_intensity);
        let color = color.mul(float(1.0).add(pulse));

        block(statements, color)
    };

    let mut tsl_shader_material = MeshBasicNodeMaterial::new();
    tsl_shader_material.position_node = Some(tsl_vertex_shader);
    tsl_shader_material.color_node = Some(tsl_fragment_shader);

    (wgsl_shader_material, tsl_shader_material)
}

pub fn init() -> App {
    let (wgsl_shader_material, tsl_shader_material) = materials();

    let camera = OrthographicCamera::new(-1.0, 1.0, 1.0, -1.0, 0.0, 1.0);
    let scene = Scene::new();

    let geometry = Rc::new(plane_geometry(2.0, 1.0, 1, 1));

    let wgsl_quad = Mesh::new(geometry.clone(), wgsl_shader_material);
    wgsl_quad.borrow_mut().position.y += 0.5;
    scene.add(&wgsl_quad);

    let tsl_quad = Mesh::new(geometry, tsl_shader_material);
    tsl_quad.borrow_mut().position.y -= 0.5;
    scene.add(&tsl_quad);

    let mut renderer = Renderer::new(RendererParameters { antialias: true }).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);
    renderer.set_output_color_space(ColorSpace::LinearSRGB);

    App {
        renderer,
        scene,
        camera,
    }
}

/// The page's `animate()`.
pub fn animate(app: &mut App) {
    app.renderer.render(&mut app.scene, &mut app.camera);
}

/// The page's `onWindowResize()`.
///
/// The rung harness never calls this — the graded frame is always
/// 800 x 500 — but the viewer and the browser shell do, so the example
/// owns its own reaction to a resized canvas instead of the host
/// guessing at one.
///
/// The handler is only `renderer.setSize( window.innerWidth, window.innerHeight )`:
/// the camera is orthographic and covers the quads whatever the canvas is, so
/// the page has no `aspect` line to transcribe.
pub fn resize(app: &mut App, width: f64, height: f64) {
    app.renderer.set_size(width, height);
}

/// The example's controls, for a host that has a pointer. `None` here:
/// the page creates none.
pub fn controls(_app: &mut App) -> Option<&mut OrbitControls> {
    None
}

/// The controls and the camera at once, for a host delivering pointer events.
/// `None` here: the page creates no controls.
pub fn controls_and_camera(_app: &mut App) -> Option<(&mut OrbitControls, &mut PerspectiveCamera)> {
    None
}

fn main() {
    // Pin both clocks to zero, as three.js' `test/e2e/deterministic-injection.js`
    // does to the page, so that the frame this writes is the frame the rung
    // grades no matter how long `init()` took.
    three_rs::testing::pin_time(Some(0.0));
    let mut app = init();
    println!("adapter: {:?}", app.renderer.adapter_info());
    animate(&mut app);

    let (width, height, pixels) = app.renderer.read_canvas_pixels().unwrap();
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "target/webgpu_tsl_interoperability.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
