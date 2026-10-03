//! Port of `three.js/src/cameras/CubeCamera.js`.
//!
//! Six [`PerspectiveCamera`]s of a -90° field of view, children of one
//! `Object3D`, each rendering the scene into one face of a cube render target
//! ([`CubeTexture::render_target`]). Moving the cube camera's [`node`] moves
//! the point the cube is captured from.
//!
//! **What this port does differently.** As in
//! `renderer::cube_render_target`, [`Renderer`] has no layered colour
//! attachment: each face is drawn into one 2-D render target of the cube's size
//! and type, then copied into its array layer. The copy is the exact move of
//! the texels, so the cube is the one three's `setRenderTarget( target, face )`
//! leaves. `activeMipmapLevel` is always 0, and the face cameras do not take
//! the cube camera's `layers`.
//!
//! [`node`]: CubeCamera::node

use crate::core::{Node, Object3D};
use crate::error::Error;
use crate::math::Vector3;
use crate::objects::Scene;
use crate::renderer::cube_render_target::{FACES, FOV};
use crate::renderer::{RenderTarget, RenderTargetOptions, Renderer};
use crate::textures::{CubeTexture, TextureFilter};

use super::PerspectiveCamera;

/// `new CubeCamera( near, far, renderTarget )`.
pub struct CubeCamera {
    /// The cube camera's own `Object3D` (`type = 'CubeCamera'`), parent of the
    /// six face cameras. Position it, or add it to the scene, as three's.
    pub node: Node,
    /// `cubeCamera.renderTarget.texture`: the cube [`update`](Self::update)
    /// renders into, from [`CubeTexture::render_target`].
    pub render_target: CubeTexture,
    /// px, nx, py, ny, pz, nz.
    cameras: Vec<PerspectiveCamera>,
    /// The 2-D target each face is drawn into before it is copied into its
    /// layer, made on the first `update()`.
    face_target: Option<RenderTarget>,
}

impl CubeCamera {
    /// `new CubeCamera( near, far, renderTarget )`, with
    /// `updateCoordinateSystem()`'s WebGPU branch already applied: the
    /// renderer is always WebGPU here.
    pub fn new(near: f64, far: f64, render_target: CubeTexture) -> Self {
        let node = Object3D {
            object_type: "CubeCamera",
            ..Default::default()
        }
        .into_node();

        // `updateCoordinateSystem()` takes each camera out of the cube camera,
        // points it from the origin, and adds it back: the face directions are
        // local to the cube camera.
        let cameras = FACES
            .iter()
            .map(|(up, look_at)| {
                let mut camera = PerspectiveCamera::new(FOV, 1.0, near, far);
                camera.node.borrow_mut().up = Vector3::new(up[0], up[1], up[2]);
                camera.look_at(&Vector3::new(look_at[0], look_at[1], look_at[2]));
                node.add(&camera.node);
                camera.node.update_matrix_world(false);
                camera
            })
            .collect();

        Self {
            node,
            render_target,
            cameras,
            face_target: None,
        }
    }

    /// `cubeCamera.update( renderer, scene )`: the scene rendered six times,
    /// once into each face of [`render_target`](Self::render_target), from the
    /// cube camera's world position. The renderer's render target and MRT
    /// are put back afterwards.
    pub fn update(&mut self, renderer: &mut Renderer, scene: &mut Scene) -> Result<(), Error> {
        if self.node.parent().is_none() {
            self.node.update_matrix_world(false);
        }

        let (size, texture_type) = {
            let inner = self.render_target.borrow();
            (inner.images[0].width, inner.texture_type)
        };
        let face_target = match &self.face_target {
            Some(target) if target.size() == (size, size) => target.clone(),
            _ => {
                let target = RenderTarget::new_with_options(
                    size,
                    size,
                    RenderTargetOptions {
                        texture_type,
                        samples: 0,
                        depth_buffer: true,
                        min_filter: TextureFilter::Linear,
                        mag_filter: TextureFilter::Linear,
                    },
                )?;
                self.face_target = Some(target.clone());
                target
            }
        };
        face_target
            .texture()
            .set_color_space(self.render_target.borrow().color_space);

        let previous_target = renderer.render_target();
        let previous_mrt = renderer.mrt();
        renderer.set_mrt(None);
        renderer.set_render_target(Some(face_target.clone()));

        for (layer, camera) in self.cameras.iter_mut().enumerate() {
            renderer.render(scene, camera);
            renderer.copy_to_cube_layer(&face_target, &self.render_target, layer as u32, 0);
        }
        // `mipmapsAutoUpdate` is restored before the last face, so the
        // mipmaps (if the cube has any) are built once all six are defined.
        renderer.generate_cube_mipmaps(&self.render_target);

        renderer.set_render_target(previous_target);
        renderer.set_mrt(previous_mrt);
        Ok(())
    }
}
