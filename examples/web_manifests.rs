//! Generates the browser shell's per-example asset manifests (issue #128).
//!
//! ```sh
//! cargo run --release --example web_manifests            # write them
//! cargo run --release --example web_manifests -- --check # fail if stale
//! ```
//!
//! **Why a generator.** The shell has to fetch every file an example reads
//! before it can call that example's `init()`, and the list of files is a
//! property of the code, not of anybody's memory of it. So it is never written
//! by hand: this tool switches the I/O seam's recorder on
//! ([`three_rs::io::start_recording`]), runs each graded example's `init()`
//! natively, and writes down exactly what the loaders asked for. An example
//! that gains a texture gains it in its manifest on the next run, and
//! `--check` fails the build until someone does that run.
//!
//! Same shape as `examples/gallery.rs`, and for the same reason: one tool, one
//! source of truth, output committed. Like the gallery's thumbnails it needs a
//! GPU, because `init()` builds a `Renderer`; that is why the freshness gate
//! CI runs is the `#[test]` at the bottom, which needs neither GPU nor
//! checkout and only asserts that the committed set of manifests is exactly
//! the README's set of graded examples.
//!
//! **Nothing of Three's is committed.** A manifest holds paths relative to the
//! three.js checkout, which is what the shell appends to a pinned
//! `raw.githubusercontent.com` URL; no asset byte ever enters this repository,
//! the same promise `examples/gallery.rs` keeps.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

#[path = "webgpu_depth_texture.rs"]
#[allow(dead_code)] // only `init()` is called here
mod webgpu_depth_texture;

#[path = "webgpu_instance_mesh.rs"]
#[allow(dead_code)] // only `init()` is called here
mod webgpu_instance_mesh;

#[path = "webgpu_materials_basic.rs"]
#[allow(dead_code)] // only `init()` is called here
mod webgpu_materials_basic;

#[path = "webgpu_rtt.rs"]
#[allow(dead_code)] // only `init()` is called here
mod webgpu_rtt;

#[path = "webgpu_lights_phong.rs"]
#[allow(dead_code)] // only `init()` is called here
mod webgpu_lights_phong;

#[path = "webgpu_morphtargets.rs"]
#[allow(dead_code)] // only `init()` is called here
mod webgpu_morphtargets;

#[path = "webgpu_shadowmap.rs"]
#[allow(dead_code)] // only `init()` is called here
mod webgpu_shadowmap;

#[path = "webgpu_lights_physical.rs"]
#[allow(dead_code)] // only `init()` is called here
mod webgpu_lights_physical;

#[path = "webgpu_postprocessing_masking.rs"]
#[allow(dead_code)] // only `init()` is called here
mod webgpu_postprocessing_masking;

#[path = "webgpu_tsl_galaxy.rs"]
#[allow(dead_code)] // only `init()` is called here
mod webgpu_tsl_galaxy;

#[path = "webgpu_skinning.rs"]
#[allow(dead_code)] // only `init()` is called here
mod webgpu_skinning;

#[path = "webgpu_mesh_batch.rs"]
#[allow(dead_code)] // only `init()` is called here
mod webgpu_mesh_batch;

#[path = "webgpu_postprocessing_radial_blur.rs"]
#[allow(dead_code)] // only `init()` is called here
mod webgpu_postprocessing_radial_blur;

#[path = "webgpu_materials.rs"]
#[allow(dead_code)] // only `init()` is called here
mod webgpu_materials;

#[path = "webgpu_postprocessing_ssaa.rs"]
#[allow(dead_code)] // only `init()` is called here
mod webgpu_postprocessing_ssaa;

#[path = "webgpu_pmrem_cubemap.rs"]
#[allow(dead_code)] // only `init()` is called here
mod webgpu_pmrem_cubemap;

#[path = "webgpu_postprocessing_bloom_selective.rs"]
#[allow(dead_code)] // only `init()` is called here
mod webgpu_postprocessing_bloom_selective;

#[path = "webgpu_compute_points.rs"]
#[allow(dead_code)] // only `init()` is called here
mod webgpu_compute_points;

#[path = "webgpu_lines_fat.rs"]
#[allow(dead_code)] // only `init()` is called here
mod webgpu_lines_fat;

#[path = "webgpu_pmrem_test.rs"]
#[allow(dead_code)] // only `init()` is called here
mod webgpu_pmrem_test;

#[path = "webgpu_postprocessing_difference.rs"]
#[allow(dead_code)] // only `init()` is called here
mod webgpu_postprocessing_difference;

#[path = "webgpu_postprocessing_direct.rs"]
#[allow(dead_code)] // only `init()` is called here
mod webgpu_postprocessing_direct;

#[path = "webgpu_furnace_test.rs"]
#[allow(dead_code)] // only `init()` is called here
mod webgpu_furnace_test;

#[path = "webgpu_postprocessing_anamorphic.rs"]
#[allow(dead_code)] // only `init()` is called here
mod webgpu_postprocessing_anamorphic;

#[path = "webgpu_pmrem_scene.rs"]
#[allow(dead_code)] // only `init()` is called here
mod webgpu_pmrem_scene;

#[path = "webgpu_postprocessing_bloom.rs"]
#[allow(dead_code)] // only `init()` is called here
mod webgpu_postprocessing_bloom;

#[path = "webgpu_materials_envmaps.rs"]
#[allow(dead_code)] // only `init()` is called here
mod webgpu_materials_envmaps;

#[path = "webgpu_materials_cubemap_mipmaps.rs"]
#[allow(dead_code)] // only `init()` is called here
mod webgpu_materials_cubemap_mipmaps;

#[path = "webgpu_postprocessing_bloom_emissive.rs"]
#[allow(dead_code)] // only `init()` is called here
mod webgpu_postprocessing_bloom_emissive;

#[path = "webgpu_instance_uniform.rs"]
#[allow(dead_code)] // only `init()` is called here
mod webgpu_instance_uniform;

#[path = "webgpu_tsl_interoperability.rs"]
#[allow(dead_code)] // only `init()` is called here
mod webgpu_tsl_interoperability;

#[path = "webgpu_pmrem_equirectangular.rs"]
#[allow(dead_code)] // only `init()` is called here
mod webgpu_pmrem_equirectangular;

#[path = "webgpu_postprocessing_ca.rs"]
#[allow(dead_code)] // only `init()` is called here
mod webgpu_postprocessing_ca;

#[path = "webgpu_loader_gltf.rs"]
#[allow(dead_code)] // only `init()` is called here
mod webgpu_loader_gltf;

#[path = "webgpu_mrt.rs"]
#[allow(dead_code)] // only `init()` is called here
mod webgpu_mrt;

#[path = "webgpu_custom_fog_background.rs"]
#[allow(dead_code)] // only `init()` is called here
mod webgpu_custom_fog_background;

#[path = "webgpu_loader_gltf_sheen.rs"]
#[allow(dead_code)] // only `init()` is called here
mod webgpu_loader_gltf_sheen;

#[path = "webgpu_deferred.rs"]
#[allow(dead_code)] // only `init()` is called here
mod webgpu_deferred;

#[path = "webgpu_loader_gltf_anisotropy.rs"]
#[allow(dead_code)] // only `init()` is called here
mod webgpu_loader_gltf_anisotropy;

#[path = "webgpu_materials_texture_manualmipmap.rs"]
#[allow(dead_code)] // only `init()` is called here
mod webgpu_materials_texture_manualmipmap;
#[path = "webgpu_tsl_vfx_flames.rs"]
#[allow(dead_code)] // only `init()` is called here
mod webgpu_tsl_vfx_flames;

/// Every graded example, in the README's order, paired with a call of its
/// `init()`.
///
/// Hand-maintained, like the viewer's table next door, and checked against the
/// README by [`tests::the_table_is_the_readme_table`] — a `#[path]` attribute
/// takes a string literal, so the module includes above cannot be generated
/// from a list and the list cannot be generated from them.
///
/// The `App` each `init()` returns is dropped immediately: all this tool wants
/// is what the loaders read on the way.
#[allow(clippy::type_complexity)]
const GRADED: &[(&str, fn())] = &[
    (
        "webgpu_depth_texture",
        || drop(webgpu_depth_texture::init()),
    ),
    (
        "webgpu_instance_mesh",
        || drop(webgpu_instance_mesh::init()),
    ),
    ("webgpu_materials_basic", || {
        drop(webgpu_materials_basic::init())
    }),
    ("webgpu_rtt", || drop(webgpu_rtt::init())),
    ("webgpu_lights_phong", || drop(webgpu_lights_phong::init())),
    ("webgpu_morphtargets", || drop(webgpu_morphtargets::init())),
    ("webgpu_shadowmap", || drop(webgpu_shadowmap::init())),
    ("webgpu_lights_physical", || {
        drop(webgpu_lights_physical::init())
    }),
    ("webgpu_postprocessing_masking", || {
        drop(webgpu_postprocessing_masking::init())
    }),
    ("webgpu_tsl_galaxy", || drop(webgpu_tsl_galaxy::init())),
    ("webgpu_skinning", || drop(webgpu_skinning::init())),
    ("webgpu_mesh_batch", || drop(webgpu_mesh_batch::init())),
    ("webgpu_postprocessing_radial_blur", || {
        drop(webgpu_postprocessing_radial_blur::init())
    }),
    ("webgpu_materials", || drop(webgpu_materials::init())),
    ("webgpu_postprocessing_ssaa", || {
        drop(webgpu_postprocessing_ssaa::init())
    }),
    (
        "webgpu_pmrem_cubemap",
        || drop(webgpu_pmrem_cubemap::init()),
    ),
    ("webgpu_postprocessing_bloom_selective", || {
        drop(webgpu_postprocessing_bloom_selective::init())
    }),
    ("webgpu_compute_points", || {
        drop(webgpu_compute_points::init())
    }),
    ("webgpu_lines_fat", || drop(webgpu_lines_fat::init())),
    ("webgpu_pmrem_test", || drop(webgpu_pmrem_test::init())),
    ("webgpu_postprocessing_difference", || {
        drop(webgpu_postprocessing_difference::init())
    }),
    ("webgpu_postprocessing_direct", || {
        drop(webgpu_postprocessing_direct::init())
    }),
    ("webgpu_furnace_test", || drop(webgpu_furnace_test::init())),
    ("webgpu_postprocessing_anamorphic", || {
        drop(webgpu_postprocessing_anamorphic::init())
    }),
    ("webgpu_pmrem_scene", || drop(webgpu_pmrem_scene::init())),
    ("webgpu_postprocessing_bloom", || {
        drop(webgpu_postprocessing_bloom::init())
    }),
    ("webgpu_materials_envmaps", || {
        drop(webgpu_materials_envmaps::init())
    }),
    ("webgpu_materials_cubemap_mipmaps", || {
        drop(webgpu_materials_cubemap_mipmaps::init())
    }),
    ("webgpu_postprocessing_bloom_emissive", || {
        drop(webgpu_postprocessing_bloom_emissive::init())
    }),
    ("webgpu_instance_uniform", || {
        drop(webgpu_instance_uniform::init())
    }),
    ("webgpu_tsl_interoperability", || {
        drop(webgpu_tsl_interoperability::init())
    }),
    ("webgpu_pmrem_equirectangular", || {
        drop(webgpu_pmrem_equirectangular::init())
    }),
    ("webgpu_postprocessing_ca", || {
        drop(webgpu_postprocessing_ca::init())
    }),
    ("webgpu_loader_gltf", || drop(webgpu_loader_gltf::init())),
    ("webgpu_mrt", || drop(webgpu_mrt::init())),
    ("webgpu_custom_fog_background", || {
        drop(webgpu_custom_fog_background::init())
    }),
    ("webgpu_loader_gltf_sheen", || {
        drop(webgpu_loader_gltf_sheen::init())
    }),
    ("webgpu_deferred", || drop(webgpu_deferred::init())),
    ("webgpu_loader_gltf_anisotropy", || {
        drop(webgpu_loader_gltf_anisotropy::init())
    }),
    ("webgpu_materials_texture_manualmipmap", || {
        drop(webgpu_materials_texture_manualmipmap::init())
    }),
    ("webgpu_tsl_vfx_flames", || {
        drop(webgpu_tsl_vfx_flames::init())
    }),
];

/// Where the committed manifests live, one `<example>.json` each.
fn manifests_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("web/manifests")
}

/// One example's manifest: the paths its `init()` read, relative to the
/// three.js checkout, sorted and deduplicated.
///
/// Sorted because a manifest is a set and a diff of one should show what
/// changed rather than what moved; deduplicated because a loader may read the
/// same file twice (a glTF sharing one texture between two materials) and the
/// shell fetches each file once.
fn record(name: &str, init: fn()) -> Vec<String> {
    let three = three_rs::testing::three_js_dir();

    three_rs::io::start_recording();
    init();
    let read = three_rs::io::stop_recording();

    let mut assets = BTreeSet::new();
    for path in read {
        let relative = path.strip_prefix(&three).unwrap_or_else(|_| {
            panic!(
                "{name}: read {}, which is not under THREE_JS_DIR ({}). \n\
                 The shell fetches assets from three.js by relative path, so \n\
                 an example that reads anything else cannot run in the browser.",
                path.display(),
                three.display()
            )
        });
        // Manifest paths are URL fragments on the other side, so `/` — which
        // is also what `Path` gives on every platform this builds on.
        assets.insert(relative.to_string_lossy().replace('\\', "/"));
    }

    assets.into_iter().collect()
}

/// A JSON array of strings, one per line — the shell reads it with
/// `serde_json`, and a human reads the diff.
fn render(assets: &[String]) -> String {
    let mut out = String::from("[\n");
    for (index, asset) in assets.iter().enumerate() {
        let comma = if index + 1 == assets.len() { "" } else { "," };
        out.push_str(&format!("  {}{comma}\n", serde_json::json!(asset)));
    }
    out.push_str("]\n");
    out
}

fn main() {
    let check = std::env::args().any(|arg| arg == "--check");
    let dir = manifests_dir();
    std::fs::create_dir_all(&dir).expect("web_manifests: cannot create web/manifests");

    let mut stale = Vec::new();

    for (name, init) in GRADED {
        let assets = record(name, *init);
        let rendered = render(&assets);
        let path = dir.join(format!("{name}.json"));

        let committed = std::fs::read_to_string(&path).ok();
        let fresh = committed.as_deref() == Some(rendered.as_str());

        if check {
            if !fresh {
                stale.push(name.to_string());
            }
            println!(
                "{name}: {} asset(s) — {}",
                assets.len(),
                if fresh { "up to date" } else { "STALE" }
            );
        } else {
            std::fs::write(&path, &rendered)
                .unwrap_or_else(|e| panic!("web_manifests: cannot write {}: {e}", path.display()));
            println!("{name}: {} asset(s) -> {}", assets.len(), path.display());
        }
    }

    if !stale.is_empty() {
        eprintln!(
            "\nweb_manifests: {} manifest(s) are stale: {}\n\
             Re-run `cargo run --release --example web_manifests` and commit the result.",
            stale.len(),
            stale.join(", ")
        );
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The README's "Examples graded green" table, by name — the same source
    /// of truth `examples/gallery.rs` parses, read the same way.
    fn readme_graded() -> Vec<String> {
        let readme =
            std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("README.md"))
                .expect("web_manifests: cannot read README.md");

        let mut names = Vec::new();
        let mut in_section = false;
        for line in readme.lines() {
            if let Some(heading) = line.strip_prefix("## ") {
                in_section = heading.trim() == "Examples graded green";
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
            if (5..=6).contains(&cells.len()) && cells[0].starts_with("webgpu_") {
                names.push(cells[0].to_string());
            }
        }
        assert!(!names.is_empty(), "no graded rows parsed out of the README");
        names
    }

    /// The hand-maintained table above is the README's graded list, exactly.
    /// This is the gate that catches a rung landing a new graded example
    /// without a manifest for the browser shell.
    #[test]
    fn the_table_is_the_readme_table() {
        let listed: Vec<String> = GRADED.iter().map(|(name, _)| name.to_string()).collect();
        assert_eq!(listed, readme_graded());
    }

    /// Every graded example has a committed manifest, and nothing else does.
    /// Needs no GPU and no three.js checkout, so CI runs it; what it cannot
    /// check is a manifest's *contents*, which is `--check`'s job on a machine
    /// with both.
    #[test]
    fn every_graded_example_has_a_committed_manifest() {
        let dir = manifests_dir();
        let mut committed: Vec<String> = std::fs::read_dir(&dir)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()))
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .filter_map(|name| name.strip_suffix(".json").map(str::to_string))
            .collect();
        committed.sort();

        let mut expected: Vec<String> = GRADED.iter().map(|(name, _)| name.to_string()).collect();
        expected.sort();

        assert_eq!(committed, expected);
    }

    /// A manifest is a sorted, deduplicated JSON array of relative paths, and
    /// nothing in it escapes the three.js checkout — the shell turns each
    /// entry into a URL under a pinned r186 tag.
    #[test]
    fn manifests_are_sorted_relative_paths() {
        for (name, _) in GRADED {
            let path = manifests_dir().join(format!("{name}.json"));
            let text = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
            let assets: Vec<String> =
                serde_json::from_str(&text).unwrap_or_else(|e| panic!("{name}: bad JSON: {e}"));

            let mut sorted = assets.clone();
            sorted.sort();
            sorted.dedup();
            assert_eq!(
                assets, sorted,
                "{name}: manifest is not sorted and deduplicated"
            );

            for asset in &assets {
                assert!(
                    !asset.starts_with('/') && !asset.contains("..") && !asset.contains('\\'),
                    "{name}: {asset} is not a relative forward-slash path"
                );
            }

            // And it round-trips through the renderer, so `--check` compares
            // like with like.
            assert_eq!(
                render(&assets),
                text,
                "{name}: manifest is not in the generator's format"
            );
        }
    }
}
