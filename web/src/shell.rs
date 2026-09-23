//! The graded examples, in a browser, on the browser's own WebGPU.
//!
//! The examples are not reimplemented here. They are included as modules —
//! `#[path = "../../examples/<name>.rs"] mod <name>;` — exactly as
//! `tests/e2e/main.rs` and `src/bin/viewer.rs` include them, so this is the
//! third driver of one corpus and not a second copy of it (issue #128).
//!
//! # The page's flow
//!
//! `?example=<name>` names the example. Then, in order:
//!
//! 1. create a wgpu instance on [`three_rs::BACKENDS`], which is
//!    `BROWSER_WEBGPU` here, and a surface on the page's `<canvas>`;
//! 2. `request_adapter` and `request_device`, both awaited — this is the
//!    asynchrony that cannot happen inside a synchronous `init()`;
//! 3. hand the three handles to [`three_rs::renderer::adopt_device`], so that
//!    the `Renderer::new` inside the example's `init()` finds them;
//! 4. fetch every asset the example's manifest names from a pinned three.js
//!    r186 tag on raw.githubusercontent.com, and
//!    [`three_rs::io::preload`] each one under the path the example will ask
//!    for;
//! 5. call `init()`, then `animate()` once, then blit the renderer's canvas
//!    texture onto the surface.
//!
//! Step 5 is the *graded* frame and nothing more: `animate()` reads
//! `Date.now()` as 0 the way three.js' e2e harness pins it, so one call
//! reproduces the frame the ladder grades. An animation loop on the wall
//! clock is the next item on #128, which is why the presentation is factored
//! into [`present`] and the example table carries a `run` function rather
//! than the frame being inlined here.
//!
//! # Nothing of Three's is committed
//!
//! The manifests hold paths relative to a three.js checkout and the assets are
//! fetched at run time from a pinned tag, the same promise `examples/gallery.rs`
//! keeps for the README's thumbnails.

use std::path::PathBuf;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

#[path = "../../examples/webgpu_depth_texture.rs"]
#[allow(dead_code)] // the example's own `main()` is unused here
mod webgpu_depth_texture;

#[path = "../../examples/webgpu_instance_mesh.rs"]
#[allow(dead_code)] // the example's own `main()` is unused here
mod webgpu_instance_mesh;

#[path = "../../examples/webgpu_materials_basic.rs"]
#[allow(dead_code)] // the example's own `main()` is unused here
mod webgpu_materials_basic;

#[path = "../../examples/webgpu_rtt.rs"]
#[allow(dead_code)] // the example's own `main()` is unused here
mod webgpu_rtt;

#[path = "../../examples/webgpu_lights_phong.rs"]
#[allow(dead_code)] // the example's own `main()` is unused here
mod webgpu_lights_phong;

#[path = "../../examples/webgpu_morphtargets.rs"]
#[allow(dead_code)] // the example's own `main()` is unused here
mod webgpu_morphtargets;

#[path = "../../examples/webgpu_shadowmap.rs"]
#[allow(dead_code)] // the example's own `main()` is unused here
mod webgpu_shadowmap;

#[path = "../../examples/webgpu_lights_physical.rs"]
#[allow(dead_code)] // the example's own `main()` is unused here
mod webgpu_lights_physical;

#[path = "../../examples/webgpu_postprocessing_masking.rs"]
#[allow(dead_code)] // the example's own `main()` is unused here
mod webgpu_postprocessing_masking;

#[path = "../../examples/webgpu_tsl_galaxy.rs"]
#[allow(dead_code)] // the example's own `main()` is unused here
mod webgpu_tsl_galaxy;

#[path = "../../examples/webgpu_skinning.rs"]
#[allow(dead_code)] // the example's own `main()` is unused here
mod webgpu_skinning;

#[path = "../../examples/webgpu_mesh_batch.rs"]
#[allow(dead_code)] // the example's own `main()` is unused here
mod webgpu_mesh_batch;

#[path = "../../examples/webgpu_postprocessing_radial_blur.rs"]
#[allow(dead_code)] // the example's own `main()` is unused here
mod webgpu_postprocessing_radial_blur;

#[path = "../../examples/webgpu_materials.rs"]
#[allow(dead_code)] // the example's own `main()` is unused here
mod webgpu_materials;

#[path = "../../examples/webgpu_postprocessing_ssaa.rs"]
#[allow(dead_code)] // the example's own `main()` is unused here
mod webgpu_postprocessing_ssaa;

#[path = "../../examples/webgpu_pmrem_cubemap.rs"]
#[allow(dead_code)] // the example's own `main()` is unused here
mod webgpu_pmrem_cubemap;

#[path = "../../examples/webgpu_postprocessing_bloom_selective.rs"]
#[allow(dead_code)] // the example's own `main()` is unused here
mod webgpu_postprocessing_bloom_selective;

#[path = "../../examples/webgpu_compute_points.rs"]
#[allow(dead_code)] // the example's own `main()` is unused here
mod webgpu_compute_points;

#[path = "../../examples/webgpu_lines_fat.rs"]
#[allow(dead_code)] // the example's own `main()` is unused here
mod webgpu_lines_fat;

#[path = "../../examples/webgpu_pmrem_test.rs"]
#[allow(dead_code)] // the example's own `main()` is unused here
mod webgpu_pmrem_test;

#[path = "../../examples/webgpu_postprocessing_difference.rs"]
#[allow(dead_code)] // the example's own `main()` is unused here
mod webgpu_postprocessing_difference;

#[path = "../../examples/webgpu_postprocessing_direct.rs"]
#[allow(dead_code)] // the example's own `main()` is unused here
mod webgpu_postprocessing_direct;

#[path = "../../examples/webgpu_furnace_test.rs"]
#[allow(dead_code)] // the example's own `main()` is unused here
mod webgpu_furnace_test;

#[path = "../../examples/webgpu_postprocessing_anamorphic.rs"]
#[allow(dead_code)] // the example's own `main()` is unused here
mod webgpu_postprocessing_anamorphic;

#[path = "../../examples/webgpu_pmrem_scene.rs"]
#[allow(dead_code)] // the example's own `main()` is unused here
mod webgpu_pmrem_scene;

#[path = "../../examples/webgpu_postprocessing_bloom.rs"]
#[allow(dead_code)] // the example's own `main()` is unused here
mod webgpu_postprocessing_bloom;

#[path = "../../examples/webgpu_materials_envmaps.rs"]
#[allow(dead_code)] // the example's own `main()` is unused here
mod webgpu_materials_envmaps;

#[path = "../../examples/webgpu_materials_cubemap_mipmaps.rs"]
#[allow(dead_code)] // the example's own `main()` is unused here
mod webgpu_materials_cubemap_mipmaps;

#[path = "../../examples/webgpu_postprocessing_bloom_emissive.rs"]
#[allow(dead_code)] // the example's own `main()` is unused here
mod webgpu_postprocessing_bloom_emissive;

#[path = "../../examples/webgpu_instance_uniform.rs"]
#[allow(dead_code)] // the example's own `main()` is unused here
mod webgpu_instance_uniform;

#[path = "../../examples/webgpu_tsl_interoperability.rs"]
#[allow(dead_code)] // the example's own `main()` is unused here
mod webgpu_tsl_interoperability;

#[path = "../../examples/webgpu_pmrem_equirectangular.rs"]
#[allow(dead_code)] // the example's own `main()` is unused here
mod webgpu_pmrem_equirectangular;

#[path = "../../examples/webgpu_postprocessing_ca.rs"]
#[allow(dead_code)] // the example's own `main()` is unused here
mod webgpu_postprocessing_ca;

#[path = "../../examples/webgpu_loader_gltf.rs"]
#[allow(dead_code)] // the example's own `main()` is unused here
mod webgpu_loader_gltf;

#[path = "../../examples/webgpu_mrt.rs"]
#[allow(dead_code)] // the example's own `main()` is unused here
mod webgpu_mrt;

#[path = "../../examples/webgpu_custom_fog_background.rs"]
#[allow(dead_code)] // the example's own `main()` is unused here
mod webgpu_custom_fog_background;

#[path = "../../examples/webgpu_loader_gltf_sheen.rs"]
#[allow(dead_code)] // the example's own `main()` is unused here
mod webgpu_loader_gltf_sheen;

#[path = "../../examples/webgpu_deferred.rs"]
#[allow(dead_code)] // the example's own `main()` is unused here
mod webgpu_deferred;

#[path = "../../examples/webgpu_loader_gltf_anisotropy.rs"]
#[allow(dead_code)] // the example's own `main()` is unused here
mod webgpu_loader_gltf_anisotropy;

/// Where an example's assets come from: three.js at the tag this port is
/// graded against, so a browser frame reads the very bytes the native ladder
/// reads off disk. Pinned, never `main` — an asset changing upstream would
/// silently move a graded frame.
const THREE_JS_TAG: &str = "r186";
const ASSET_BASE: &str = "https://raw.githubusercontent.com/mrdoob/three.js";

/// Our own rendered frame for each example, for the browsers that cannot run
/// one. Same raw-GitHub URLs the README's gallery grid uses.
const GALLERY_BASE: &str =
    "https://raw.githubusercontent.com/oneilltomhq/three-rs/main/docs/gallery";

/// One graded example: how big its canvas is, what it needs fetched, and how
/// to run its graded frame.
struct Example {
    name: &'static str,
    /// `window.innerWidth * devicePixelRatio` as the example itself declares
    /// them — the canvas is sized to the frame the grader compares, not to the
    /// browser window, so that what the page shows is what the ladder graded.
    width: u32,
    height: u32,
    /// The committed manifest, compiled in. Embedding it rather than fetching
    /// it saves a round trip and, more to the point, makes a manifest that was
    /// never generated a build error instead of a 404.
    manifest: &'static str,
    run: fn(&wgpu::Surface<'static>, wgpu::TextureFormat) -> Result<(), String>,
}

/// Every graded example, in the README's order.
///
/// Hand-maintained, like the viewer's table and `examples/web_manifests.rs`'s,
/// and for the same reason: a `#[path]` attribute takes a string literal, so
/// the module includes cannot be generated from a list. The freshness gate is
/// `web_manifests`' `#[test]`, which asserts its own copy of this list is the
/// README's; a name here with no committed manifest fails to compile at the
/// `include_str!` above.
const EXAMPLES: &[Example] = &[
    Example {
        name: "webgpu_depth_texture",
        width: (webgpu_depth_texture::INNER_WIDTH * webgpu_depth_texture::DPR) as u32,
        height: (webgpu_depth_texture::INNER_HEIGHT * webgpu_depth_texture::DPR) as u32,
        manifest: include_str!("../manifests/webgpu_depth_texture.json"),
        run: |surface, format| {
            let mut app = webgpu_depth_texture::init();
            webgpu_depth_texture::animate(&mut app);
            present(&mut app.renderer, surface, format)
        },
    },
    Example {
        name: "webgpu_instance_mesh",
        width: (webgpu_instance_mesh::INNER_WIDTH * webgpu_instance_mesh::DPR) as u32,
        height: (webgpu_instance_mesh::INNER_HEIGHT * webgpu_instance_mesh::DPR) as u32,
        manifest: include_str!("../manifests/webgpu_instance_mesh.json"),
        run: |surface, format| {
            let mut app = webgpu_instance_mesh::init();
            webgpu_instance_mesh::animate(&mut app);
            present(&mut app.renderer, surface, format)
        },
    },
    Example {
        name: "webgpu_materials_basic",
        width: (webgpu_materials_basic::INNER_WIDTH * webgpu_materials_basic::DPR) as u32,
        height: (webgpu_materials_basic::INNER_HEIGHT * webgpu_materials_basic::DPR) as u32,
        manifest: include_str!("../manifests/webgpu_materials_basic.json"),
        run: |surface, format| {
            let mut app = webgpu_materials_basic::init();
            webgpu_materials_basic::animate(&mut app);
            present(&mut app.renderer, surface, format)
        },
    },
    Example {
        name: "webgpu_rtt",
        width: (webgpu_rtt::INNER_WIDTH * webgpu_rtt::DPR) as u32,
        height: (webgpu_rtt::INNER_HEIGHT * webgpu_rtt::DPR) as u32,
        manifest: include_str!("../manifests/webgpu_rtt.json"),
        run: |surface, format| {
            let mut app = webgpu_rtt::init();
            webgpu_rtt::animate(&mut app);
            present(&mut app.renderer, surface, format)
        },
    },
    Example {
        name: "webgpu_lights_phong",
        width: (webgpu_lights_phong::INNER_WIDTH * webgpu_lights_phong::DPR) as u32,
        height: (webgpu_lights_phong::INNER_HEIGHT * webgpu_lights_phong::DPR) as u32,
        manifest: include_str!("../manifests/webgpu_lights_phong.json"),
        run: |surface, format| {
            let mut app = webgpu_lights_phong::init();
            webgpu_lights_phong::animate(&mut app);
            present(&mut app.renderer, surface, format)
        },
    },
    Example {
        name: "webgpu_morphtargets",
        width: (webgpu_morphtargets::INNER_WIDTH * webgpu_morphtargets::DPR) as u32,
        height: (webgpu_morphtargets::INNER_HEIGHT * webgpu_morphtargets::DPR) as u32,
        manifest: include_str!("../manifests/webgpu_morphtargets.json"),
        run: |surface, format| {
            let mut app = webgpu_morphtargets::init();
            webgpu_morphtargets::animate(&mut app);
            present(&mut app.renderer, surface, format)
        },
    },
    Example {
        name: "webgpu_shadowmap",
        width: (webgpu_shadowmap::INNER_WIDTH * webgpu_shadowmap::DPR) as u32,
        height: (webgpu_shadowmap::INNER_HEIGHT * webgpu_shadowmap::DPR) as u32,
        manifest: include_str!("../manifests/webgpu_shadowmap.json"),
        run: |surface, format| {
            let mut app = webgpu_shadowmap::init();
            webgpu_shadowmap::animate(&mut app);
            present(&mut app.renderer, surface, format)
        },
    },
    Example {
        name: "webgpu_lights_physical",
        width: (webgpu_lights_physical::INNER_WIDTH * webgpu_lights_physical::DPR) as u32,
        height: (webgpu_lights_physical::INNER_HEIGHT * webgpu_lights_physical::DPR) as u32,
        manifest: include_str!("../manifests/webgpu_lights_physical.json"),
        run: |surface, format| {
            let mut app = webgpu_lights_physical::init();
            webgpu_lights_physical::animate(&mut app);
            present(&mut app.renderer, surface, format)
        },
    },
    Example {
        name: "webgpu_postprocessing_masking",
        width: (webgpu_postprocessing_masking::INNER_WIDTH * webgpu_postprocessing_masking::DPR)
            as u32,
        height: (webgpu_postprocessing_masking::INNER_HEIGHT * webgpu_postprocessing_masking::DPR)
            as u32,
        manifest: include_str!("../manifests/webgpu_postprocessing_masking.json"),
        run: |surface, format| {
            let mut app = webgpu_postprocessing_masking::init();
            webgpu_postprocessing_masking::animate(&mut app);
            present(&mut app.renderer, surface, format)
        },
    },
    Example {
        name: "webgpu_tsl_galaxy",
        width: (webgpu_tsl_galaxy::INNER_WIDTH * webgpu_tsl_galaxy::DPR) as u32,
        height: (webgpu_tsl_galaxy::INNER_HEIGHT * webgpu_tsl_galaxy::DPR) as u32,
        manifest: include_str!("../manifests/webgpu_tsl_galaxy.json"),
        run: |surface, format| {
            let mut app = webgpu_tsl_galaxy::init();
            webgpu_tsl_galaxy::animate(&mut app);
            present(&mut app.renderer, surface, format)
        },
    },
    Example {
        name: "webgpu_skinning",
        width: (webgpu_skinning::INNER_WIDTH * webgpu_skinning::DPR) as u32,
        height: (webgpu_skinning::INNER_HEIGHT * webgpu_skinning::DPR) as u32,
        manifest: include_str!("../manifests/webgpu_skinning.json"),
        run: |surface, format| {
            let mut app = webgpu_skinning::init();
            webgpu_skinning::animate(&mut app);
            present(&mut app.renderer, surface, format)
        },
    },
    Example {
        name: "webgpu_mesh_batch",
        width: (webgpu_mesh_batch::INNER_WIDTH * webgpu_mesh_batch::DPR) as u32,
        height: (webgpu_mesh_batch::INNER_HEIGHT * webgpu_mesh_batch::DPR) as u32,
        manifest: include_str!("../manifests/webgpu_mesh_batch.json"),
        run: |surface, format| {
            let mut app = webgpu_mesh_batch::init();
            webgpu_mesh_batch::animate(&mut app);
            present(&mut app.renderer, surface, format)
        },
    },
    Example {
        name: "webgpu_postprocessing_radial_blur",
        width: (webgpu_postprocessing_radial_blur::INNER_WIDTH
            * webgpu_postprocessing_radial_blur::DPR) as u32,
        height: (webgpu_postprocessing_radial_blur::INNER_HEIGHT
            * webgpu_postprocessing_radial_blur::DPR) as u32,
        manifest: include_str!("../manifests/webgpu_postprocessing_radial_blur.json"),
        run: |surface, format| {
            let mut app = webgpu_postprocessing_radial_blur::init();
            webgpu_postprocessing_radial_blur::animate(&mut app);
            present(&mut app.renderer, surface, format)
        },
    },
    Example {
        name: "webgpu_materials",
        width: (webgpu_materials::INNER_WIDTH * webgpu_materials::DPR) as u32,
        height: (webgpu_materials::INNER_HEIGHT * webgpu_materials::DPR) as u32,
        manifest: include_str!("../manifests/webgpu_materials.json"),
        run: |surface, format| {
            let mut app = webgpu_materials::init();
            webgpu_materials::animate(&mut app);
            present(&mut app.renderer, surface, format)
        },
    },
    Example {
        name: "webgpu_postprocessing_ssaa",
        width: (webgpu_postprocessing_ssaa::INNER_WIDTH * webgpu_postprocessing_ssaa::DPR) as u32,
        height: (webgpu_postprocessing_ssaa::INNER_HEIGHT * webgpu_postprocessing_ssaa::DPR) as u32,
        manifest: include_str!("../manifests/webgpu_postprocessing_ssaa.json"),
        run: |surface, format| {
            let mut app = webgpu_postprocessing_ssaa::init();
            webgpu_postprocessing_ssaa::animate(&mut app);
            present(&mut app.renderer, surface, format)
        },
    },
    Example {
        name: "webgpu_pmrem_cubemap",
        width: (webgpu_pmrem_cubemap::INNER_WIDTH * webgpu_pmrem_cubemap::DPR) as u32,
        height: (webgpu_pmrem_cubemap::INNER_HEIGHT * webgpu_pmrem_cubemap::DPR) as u32,
        manifest: include_str!("../manifests/webgpu_pmrem_cubemap.json"),
        run: |surface, format| {
            let mut app = webgpu_pmrem_cubemap::init();
            webgpu_pmrem_cubemap::animate(&mut app);
            present(&mut app.renderer, surface, format)
        },
    },
    Example {
        name: "webgpu_postprocessing_bloom_selective",
        width: (webgpu_postprocessing_bloom_selective::INNER_WIDTH
            * webgpu_postprocessing_bloom_selective::DPR) as u32,
        height: (webgpu_postprocessing_bloom_selective::INNER_HEIGHT
            * webgpu_postprocessing_bloom_selective::DPR) as u32,
        manifest: include_str!("../manifests/webgpu_postprocessing_bloom_selective.json"),
        run: |surface, format| {
            let mut app = webgpu_postprocessing_bloom_selective::init();
            webgpu_postprocessing_bloom_selective::animate(&mut app);
            present(&mut app.renderer, surface, format)
        },
    },
    Example {
        name: "webgpu_compute_points",
        width: (webgpu_compute_points::INNER_WIDTH * webgpu_compute_points::DPR) as u32,
        height: (webgpu_compute_points::INNER_HEIGHT * webgpu_compute_points::DPR) as u32,
        manifest: include_str!("../manifests/webgpu_compute_points.json"),
        run: |surface, format| {
            let mut app = webgpu_compute_points::init();
            webgpu_compute_points::animate(&mut app);
            present(&mut app.renderer, surface, format)
        },
    },
    Example {
        name: "webgpu_lines_fat",
        width: (webgpu_lines_fat::INNER_WIDTH * webgpu_lines_fat::DPR) as u32,
        height: (webgpu_lines_fat::INNER_HEIGHT * webgpu_lines_fat::DPR) as u32,
        manifest: include_str!("../manifests/webgpu_lines_fat.json"),
        run: |surface, format| {
            let mut app = webgpu_lines_fat::init();
            webgpu_lines_fat::animate(&mut app);
            present(&mut app.renderer, surface, format)
        },
    },
    Example {
        name: "webgpu_pmrem_test",
        width: (webgpu_pmrem_test::INNER_WIDTH * webgpu_pmrem_test::DPR) as u32,
        height: (webgpu_pmrem_test::INNER_HEIGHT * webgpu_pmrem_test::DPR) as u32,
        manifest: include_str!("../manifests/webgpu_pmrem_test.json"),
        run: |surface, format| {
            let mut app = webgpu_pmrem_test::init();
            webgpu_pmrem_test::animate(&mut app);
            present(&mut app.renderer, surface, format)
        },
    },
    Example {
        name: "webgpu_postprocessing_difference",
        width: (webgpu_postprocessing_difference::INNER_WIDTH
            * webgpu_postprocessing_difference::DPR) as u32,
        height: (webgpu_postprocessing_difference::INNER_HEIGHT
            * webgpu_postprocessing_difference::DPR) as u32,
        manifest: include_str!("../manifests/webgpu_postprocessing_difference.json"),
        run: |surface, format| {
            let mut app = webgpu_postprocessing_difference::init();
            webgpu_postprocessing_difference::animate(&mut app);
            present(&mut app.renderer, surface, format)
        },
    },
    Example {
        name: "webgpu_postprocessing_direct",
        width: (webgpu_postprocessing_direct::INNER_WIDTH * webgpu_postprocessing_direct::DPR)
            as u32,
        height: (webgpu_postprocessing_direct::INNER_HEIGHT * webgpu_postprocessing_direct::DPR)
            as u32,
        manifest: include_str!("../manifests/webgpu_postprocessing_direct.json"),
        run: |surface, format| {
            let mut app = webgpu_postprocessing_direct::init();
            webgpu_postprocessing_direct::animate(&mut app);
            present(&mut app.renderer, surface, format)
        },
    },
    Example {
        name: "webgpu_furnace_test",
        width: (webgpu_furnace_test::INNER_WIDTH * webgpu_furnace_test::DPR) as u32,
        height: (webgpu_furnace_test::INNER_HEIGHT * webgpu_furnace_test::DPR) as u32,
        manifest: include_str!("../manifests/webgpu_furnace_test.json"),
        run: |surface, format| {
            let mut app = webgpu_furnace_test::init();
            webgpu_furnace_test::animate(&mut app);
            present(&mut app.renderer, surface, format)
        },
    },
    Example {
        name: "webgpu_postprocessing_anamorphic",
        width: (webgpu_postprocessing_anamorphic::INNER_WIDTH
            * webgpu_postprocessing_anamorphic::DPR) as u32,
        height: (webgpu_postprocessing_anamorphic::INNER_HEIGHT
            * webgpu_postprocessing_anamorphic::DPR) as u32,
        manifest: include_str!("../manifests/webgpu_postprocessing_anamorphic.json"),
        run: |surface, format| {
            let mut app = webgpu_postprocessing_anamorphic::init();
            webgpu_postprocessing_anamorphic::animate(&mut app);
            present(&mut app.renderer, surface, format)
        },
    },
    Example {
        name: "webgpu_pmrem_scene",
        width: (webgpu_pmrem_scene::INNER_WIDTH * webgpu_pmrem_scene::DPR) as u32,
        height: (webgpu_pmrem_scene::INNER_HEIGHT * webgpu_pmrem_scene::DPR) as u32,
        manifest: include_str!("../manifests/webgpu_pmrem_scene.json"),
        run: |surface, format| {
            let mut app = webgpu_pmrem_scene::init();
            webgpu_pmrem_scene::animate(&mut app);
            present(&mut app.renderer, surface, format)
        },
    },
    Example {
        name: "webgpu_postprocessing_bloom",
        width: (webgpu_postprocessing_bloom::INNER_WIDTH * webgpu_postprocessing_bloom::DPR) as u32,
        height: (webgpu_postprocessing_bloom::INNER_HEIGHT * webgpu_postprocessing_bloom::DPR)
            as u32,
        manifest: include_str!("../manifests/webgpu_postprocessing_bloom.json"),
        run: |surface, format| {
            let mut app = webgpu_postprocessing_bloom::init();
            webgpu_postprocessing_bloom::animate(&mut app);
            present(&mut app.renderer, surface, format)
        },
    },
    Example {
        name: "webgpu_materials_envmaps",
        width: (webgpu_materials_envmaps::INNER_WIDTH * webgpu_materials_envmaps::DPR) as u32,
        height: (webgpu_materials_envmaps::INNER_HEIGHT * webgpu_materials_envmaps::DPR) as u32,
        manifest: include_str!("../manifests/webgpu_materials_envmaps.json"),
        run: |surface, format| {
            let mut app = webgpu_materials_envmaps::init();
            webgpu_materials_envmaps::animate(&mut app);
            present(&mut app.renderer, surface, format)
        },
    },
    Example {
        name: "webgpu_materials_cubemap_mipmaps",
        width: (webgpu_materials_cubemap_mipmaps::INNER_WIDTH
            * webgpu_materials_cubemap_mipmaps::DPR) as u32,
        height: (webgpu_materials_cubemap_mipmaps::INNER_HEIGHT
            * webgpu_materials_cubemap_mipmaps::DPR) as u32,
        manifest: include_str!("../manifests/webgpu_materials_cubemap_mipmaps.json"),
        run: |surface, format| {
            let mut app = webgpu_materials_cubemap_mipmaps::init();
            webgpu_materials_cubemap_mipmaps::animate(&mut app);
            present(&mut app.renderer, surface, format)
        },
    },
    Example {
        name: "webgpu_postprocessing_bloom_emissive",
        width: (webgpu_postprocessing_bloom_emissive::INNER_WIDTH
            * webgpu_postprocessing_bloom_emissive::DPR) as u32,
        height: (webgpu_postprocessing_bloom_emissive::INNER_HEIGHT
            * webgpu_postprocessing_bloom_emissive::DPR) as u32,
        manifest: include_str!("../manifests/webgpu_postprocessing_bloom_emissive.json"),
        run: |surface, format| {
            let mut app = webgpu_postprocessing_bloom_emissive::init();
            webgpu_postprocessing_bloom_emissive::animate(&mut app);
            present(&mut app.renderer, surface, format)
        },
    },
    Example {
        name: "webgpu_instance_uniform",
        width: (webgpu_instance_uniform::INNER_WIDTH * webgpu_instance_uniform::DPR) as u32,
        height: (webgpu_instance_uniform::INNER_HEIGHT * webgpu_instance_uniform::DPR) as u32,
        manifest: include_str!("../manifests/webgpu_instance_uniform.json"),
        run: |surface, format| {
            let mut app = webgpu_instance_uniform::init();
            webgpu_instance_uniform::animate(&mut app);
            present(&mut app.renderer, surface, format)
        },
    },
    Example {
        name: "webgpu_tsl_interoperability",
        width: (webgpu_tsl_interoperability::INNER_WIDTH * webgpu_tsl_interoperability::DPR) as u32,
        height: (webgpu_tsl_interoperability::INNER_HEIGHT * webgpu_tsl_interoperability::DPR)
            as u32,
        manifest: include_str!("../manifests/webgpu_tsl_interoperability.json"),
        run: |surface, format| {
            let mut app = webgpu_tsl_interoperability::init();
            webgpu_tsl_interoperability::animate(&mut app);
            present(&mut app.renderer, surface, format)
        },
    },
    Example {
        name: "webgpu_pmrem_equirectangular",
        width: (webgpu_pmrem_equirectangular::INNER_WIDTH * webgpu_pmrem_equirectangular::DPR)
            as u32,
        height: (webgpu_pmrem_equirectangular::INNER_HEIGHT * webgpu_pmrem_equirectangular::DPR)
            as u32,
        manifest: include_str!("../manifests/webgpu_pmrem_equirectangular.json"),
        run: |surface, format| {
            let mut app = webgpu_pmrem_equirectangular::init();
            webgpu_pmrem_equirectangular::animate(&mut app);
            present(&mut app.renderer, surface, format)
        },
    },
    Example {
        name: "webgpu_postprocessing_ca",
        width: (webgpu_postprocessing_ca::INNER_WIDTH * webgpu_postprocessing_ca::DPR) as u32,
        height: (webgpu_postprocessing_ca::INNER_HEIGHT * webgpu_postprocessing_ca::DPR) as u32,
        manifest: include_str!("../manifests/webgpu_postprocessing_ca.json"),
        run: |surface, format| {
            let mut app = webgpu_postprocessing_ca::init();
            webgpu_postprocessing_ca::animate(&mut app);
            present(&mut app.renderer, surface, format)
        },
    },
    Example {
        name: "webgpu_loader_gltf",
        width: (webgpu_loader_gltf::INNER_WIDTH * webgpu_loader_gltf::DPR) as u32,
        height: (webgpu_loader_gltf::INNER_HEIGHT * webgpu_loader_gltf::DPR) as u32,
        manifest: include_str!("../manifests/webgpu_loader_gltf.json"),
        run: |surface, format| {
            let mut app = webgpu_loader_gltf::init();
            webgpu_loader_gltf::animate(&mut app);
            present(&mut app.renderer, surface, format)
        },
    },
    Example {
        name: "webgpu_mrt",
        width: (webgpu_mrt::INNER_WIDTH * webgpu_mrt::DPR) as u32,
        height: (webgpu_mrt::INNER_HEIGHT * webgpu_mrt::DPR) as u32,
        manifest: include_str!("../manifests/webgpu_mrt.json"),
        run: |surface, format| {
            let mut app = webgpu_mrt::init();
            webgpu_mrt::animate(&mut app);
            present(&mut app.renderer, surface, format)
        },
    },
    Example {
        name: "webgpu_custom_fog_background",
        width: (webgpu_custom_fog_background::INNER_WIDTH * webgpu_custom_fog_background::DPR)
            as u32,
        height: (webgpu_custom_fog_background::INNER_HEIGHT * webgpu_custom_fog_background::DPR)
            as u32,
        manifest: include_str!("../manifests/webgpu_custom_fog_background.json"),
        run: |surface, format| {
            let mut app = webgpu_custom_fog_background::init();
            webgpu_custom_fog_background::animate(&mut app);
            present(&mut app.renderer, surface, format)
        },
    },
    Example {
        name: "webgpu_loader_gltf_sheen",
        width: (webgpu_loader_gltf_sheen::INNER_WIDTH * webgpu_loader_gltf_sheen::DPR) as u32,
        height: (webgpu_loader_gltf_sheen::INNER_HEIGHT * webgpu_loader_gltf_sheen::DPR) as u32,
        manifest: include_str!("../manifests/webgpu_loader_gltf_sheen.json"),
        run: |surface, format| {
            let mut app = webgpu_loader_gltf_sheen::init();
            webgpu_loader_gltf_sheen::animate(&mut app);
            present(&mut app.renderer, surface, format)
        },
    },
    Example {
        name: "webgpu_deferred",
        width: (webgpu_deferred::INNER_WIDTH * webgpu_deferred::DPR) as u32,
        height: (webgpu_deferred::INNER_HEIGHT * webgpu_deferred::DPR) as u32,
        manifest: include_str!("../manifests/webgpu_deferred.json"),
        run: |surface, format| {
            let mut app = webgpu_deferred::init();
            webgpu_deferred::animate(&mut app);
            present(&mut app.renderer, surface, format)
        },
    },
    Example {
        name: "webgpu_loader_gltf_anisotropy",
        width: (webgpu_loader_gltf_anisotropy::INNER_WIDTH * webgpu_loader_gltf_anisotropy::DPR)
            as u32,
        height: (webgpu_loader_gltf_anisotropy::INNER_HEIGHT * webgpu_loader_gltf_anisotropy::DPR)
            as u32,
        manifest: include_str!("../manifests/webgpu_loader_gltf_anisotropy.json"),
        run: |surface, format| {
            let mut app = webgpu_loader_gltf_anisotropy::init();
            webgpu_loader_gltf_anisotropy::animate(&mut app);
            present(&mut app.renderer, surface, format)
        },
    },
];

/// wasm-bindgen's entry point, called by the generated JS glue once the page
/// has its canvas.
///
/// Everything real happens in [`run`], on `spawn_local`: the adapter, the
/// device and every asset fetch are promises, and a page has one thread to
/// wait on them with.
#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
    wasm_bindgen_futures::spawn_local(async {
        if let Err(message) = run().await {
            report(&message);
        }
    });
}

/// The `?example=` query parameter, or `None` when the page is the index.
fn requested_example() -> Option<String> {
    let search = window().location().search().ok()?;
    let params = web_sys::UrlSearchParams::new_with_str(&search).ok()?;
    params.get("example")
}

fn window() -> web_sys::Window {
    web_sys::window().expect("three-rs-web: no window")
}

fn document() -> web_sys::Document {
    window().document().expect("three-rs-web: no document")
}

/// Puts one line of text in the page's status element. This is the only
/// reporting channel a page has that is not the console, and every failure
/// below ends here rather than in a panic, because a panic in a page is a
/// blank rectangle.
fn report(message: &str) {
    web_sys::console::log_1(&JsValue::from_str(message));
    if let Some(status) = document().get_element_by_id("status") {
        status.set_text_content(Some(message));
    }
}

/// The still frame and one line, for a browser with no WebGPU.
///
/// Not a panic and not an empty page: the thumbnail is what this example looks
/// like, and the sentence says why it is a picture rather than a rendering.
fn no_webgpu(name: &str, reason: &str) {
    if let Some(canvas) = document().get_element_by_id("canvas") {
        let _ = canvas
            .dyn_ref::<web_sys::HtmlElement>()
            .map(|element| element.style().set_property("display", "none"));
    }
    if let Some(still) = document().get_element_by_id("still") {
        let _ = still.set_attribute("src", &format!("{GALLERY_BASE}/{name}.jpg"));
        let _ = still
            .dyn_ref::<web_sys::HtmlElement>()
            .map(|element| element.style().set_property("display", "block"));
    }
    report(&format!(
        "This browser has no WebGPU ({reason}), so this is a still of the frame \
         three-rs renders natively."
    ));
}

async fn run() -> Result<(), String> {
    let name = requested_example().ok_or("no ?example= in the URL")?;
    let example = EXAMPLES
        .iter()
        .find(|candidate| candidate.name == name)
        .ok_or_else(|| format!("no graded example named {name}"))?;

    let canvas: web_sys::HtmlCanvasElement = document()
        .get_element_by_id("canvas")
        .ok_or("the page has no #canvas")?
        .dyn_into()
        .map_err(|_| "#canvas is not a <canvas>")?;
    canvas.set_width(example.width);
    canvas.set_height(example.height);

    // `navigator.gpu` missing is the common case on Firefox and Safari today,
    // and it is not an error: the page falls back to the still.
    if js_sys::Reflect::get(&window().navigator(), &JsValue::from_str("gpu"))
        .map(|gpu| gpu.is_undefined() || gpu.is_null())
        .unwrap_or(true)
    {
        no_webgpu(example.name, "navigator.gpu is undefined");
        return Ok(());
    }

    report(&format!("{name}: asking for a WebGPU adapter…"));

    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: three_rs::BACKENDS,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });

    let surface = instance
        .create_surface(wgpu::SurfaceTarget::Canvas(canvas))
        .map_err(|error| format!("cannot create a surface on the canvas: {error}"))?;

    let adapter = match instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(&surface),
            ..Default::default()
        })
        .await
    {
        Ok(adapter) => adapter,
        Err(error) => {
            no_webgpu(example.name, &format!("requestAdapter failed: {error}"));
            return Ok(());
        }
    };

    // The same feature request `Renderer::with_instance_async` makes: an
    // `r32float` texture is only sampled through a filtering sampler on a
    // device that asked for it. Requested when the adapter has it, exactly as
    // the native path does, so a browser device is not quietly less capable
    // than a desktop one.
    let float32_filterable = adapter
        .features()
        .contains(wgpu::Features::FLOAT32_FILTERABLE);

    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("three-rs device"),
            required_features: if float32_filterable {
                wgpu::Features::FLOAT32_FILTERABLE
            } else {
                wgpu::Features::empty()
            },
            required_limits: wgpu::Limits::default(),
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            memory_hints: wgpu::MemoryHints::MemoryUsage,
            trace: wgpu::Trace::Off,
        })
        .await
        .map_err(|error| format!("cannot create a device: {error}"))?;

    // wgpu reports validation and out-of-memory errors on this callback; in a
    // page they would otherwise only reach the devtools console, and a frame
    // that failed validation is a black canvas with no explanation.
    device.on_uncaptured_error(std::sync::Arc::new(|error| {
        web_sys::console::error_1(&JsValue::from_str(&format!("wgpu: {error}")));
    }));

    let format = {
        let caps = surface.get_capabilities(&adapter);
        let preferred = *caps
            .formats
            .first()
            .ok_or("the canvas surface supports no format")?;
        // Present to a NON-srgb format, exactly as the viewer does: the
        // renderer's canvas texture already holds sRGB-encoded bytes and an
        // `-srgb` view would encode them a second time.
        let linear = preferred.remove_srgb_suffix();
        if caps.formats.contains(&linear) {
            linear
        } else {
            preferred
        }
    };

    surface.configure(
        &device,
        &wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            color_space: wgpu::SurfaceColorSpace::Auto,
            view_formats: vec![format],
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            width: example.width,
            height: example.height,
            desired_maximum_frame_latency: 2,
            present_mode: wgpu::PresentMode::AutoVsync,
        },
    );

    // Everything the example's synchronous `init()` will need, in place before
    // it runs: the device it will "create", and the bytes it will "read".
    three_rs::renderer::adopt_device(adapter, device, queue);
    preload_assets(example).await?;

    report(&format!("{name}: rendering…"));
    (example.run)(&surface, format)?;
    report(&format!(
        "{name} — the graded frame, {}x{}, rendered in this browser",
        example.width, example.height
    ));

    Ok(())
}

/// Fetches every asset the manifest names and hands it to the I/O seam under
/// the path the example will ask for.
///
/// The key is `three_js_dir().join(relative)`: on wasm32 `three_js_dir()` is
/// the fixed virtual root `/vendor/three.js`, and the examples build their
/// asset paths through that same function, so host and example agree without
/// either knowing about the other.
async fn preload_assets(example: &Example) -> Result<(), String> {
    let assets: Vec<String> = serde_json::from_str(example.manifest)
        .map_err(|error| format!("{}: bad manifest: {error}", example.name))?;

    let root = three_rs::testing::three_js_dir();

    for (index, asset) in assets.iter().enumerate() {
        report(&format!(
            "{}: fetching asset {} of {} — {asset}",
            example.name,
            index + 1,
            assets.len()
        ));
        let url = format!("{ASSET_BASE}/{THREE_JS_TAG}/{asset}");
        let bytes = fetch(&url).await?;
        three_rs::io::preload(PathBuf::from(&root).join(asset), bytes);
    }

    Ok(())
}

/// `fetch(url)` down to bytes.
///
/// Sequential rather than concurrent on purpose for this first pass: the
/// progress line is readable, a failure names the file that failed, and the
/// heaviest example fetches a couple of dozen files.
async fn fetch(url: &str) -> Result<Vec<u8>, String> {
    let response: web_sys::Response = JsFuture::from(window().fetch_with_str(url))
        .await
        .map_err(|error| format!("fetch {url} failed: {error:?}"))?
        .dyn_into()
        .map_err(|_| format!("fetch {url} did not return a Response"))?;

    if !response.ok() {
        return Err(format!("fetch {url}: HTTP {}", response.status()));
    }

    let buffer = JsFuture::from(
        response
            .array_buffer()
            .map_err(|error| format!("{url}: no body: {error:?}"))?,
    )
    .await
    .map_err(|error| format!("{url}: cannot read the body: {error:?}"))?;

    Ok(js_sys::Uint8Array::new(&buffer).to_vec())
}

/// Blits the renderer's finished canvas texture onto the page's surface.
///
/// The mechanism is the viewer's, unchanged: `Renderer::present` is the
/// fullscreen blit `src/renderer/present.rs` already provides for exactly this
/// — the renderer draws into its own canvas texture and a caller decides where
/// that ends up. There is no second presentation path for the web.
///
/// Factored out of the per-example `run` so that the animation loop that comes
/// with the next item on #128 can call it per frame with nothing else moving.
fn present(
    renderer: &mut three_rs::Renderer,
    surface: &wgpu::Surface<'static>,
    format: wgpu::TextureFormat,
) -> Result<(), String> {
    let surface_texture = match surface.get_current_texture() {
        wgpu::CurrentSurfaceTexture::Success(texture)
        | wgpu::CurrentSurfaceTexture::Suboptimal(texture) => texture,
        other => return Err(format!("no surface texture to present to: {other:?}")),
    };

    let view = surface_texture
        .texture
        .create_view(&wgpu::TextureViewDescriptor {
            format: Some(format),
            ..Default::default()
        });

    if !renderer.present(&view, format) {
        return Err("the renderer has nothing to present".into());
    }

    renderer.queue().present(surface_texture);
    Ok(())
}
