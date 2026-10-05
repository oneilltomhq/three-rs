//! Port of `three.js/src/cameras/CubeCamera.js`.

use crate::core::{Object3D, ObjectRef};
use crate::math::Vector3;
use crate::objects::Scene;
use crate::renderer::cube_render_target::{CubeRenderTarget, FACES, FOV};
use crate::renderer::Renderer;

use super::PerspectiveCamera;

/// `new CubeCamera( near, far, renderTarget )` — six 90° cameras at one point,
/// one per cube face, which [`CubeCamera::update`] renders a scene through
/// into [`CubeCamera::render_target`].
///
/// As in three, the face cameras are children of the cube camera's own
/// [`ObjectRef`], so moving or rotating `node` moves the six views with it.
///
/// Three builds the face orientations lazily, in `updateCoordinateSystem()`,
/// the first time `update()` sees a renderer, because a WebGL and a WebGPU
/// renderer want different ones. The port has only the WebGPU coordinate
/// system, so the constructor sets them once, from the same table
/// `fromEquirectangularTexture()` uses.
///
/// Three's `activeMipmapLevel` is not ported: the port renders a face into a
/// 2-D target and copies it into mip 0, and nothing on the ladder renders a
/// cube camera into a lower level.
pub struct CubeCamera {
    /// The `CubeCamera`'s `Object3D` — the parent of the six face cameras.
    pub node: ObjectRef,
    /// `cubeCamera.renderTarget`.
    pub render_target: CubeRenderTarget,
    /// px, nx, py, ny, pz, nz — `this.children` in three.
    cameras: Vec<PerspectiveCamera>,
}

impl CubeCamera {
    /// `new CubeCamera( near, far, renderTarget )`.
    pub fn new(near: f64, far: f64, render_target: CubeRenderTarget) -> Self {
        let node = Object3D {
            object_type: "CubeCamera",
            ..Default::default()
        }
        .into_node();

        let mut cameras = Vec::with_capacity(6);
        for (up, look_at) in FACES {
            // `new PerspectiveCamera( fov, aspect, near, far )` with `aspect = 1`,
            // `camera.layers = this.layers`, `this.add( camera )`.
            let mut camera = PerspectiveCamera::new(FOV, 1.0, near, far);
            camera.node.borrow_mut().layers = node.borrow().layers;
            // `updateCoordinateSystem()`'s `WebGPUCoordinateSystem` branch.
            camera.node.borrow_mut().up = Vector3::new(up[0], up[1], up[2]);
            camera.look_at(&Vector3::new(look_at[0], look_at[1], look_at[2]));
            node.add(&camera.node);
            cameras.push(camera);
        }
        node.update_matrix_world(false);

        Self {
            node,
            render_target,
            cameras,
        }
    }

    /// `cubeCamera.update( renderer, scene )` — render `scene` once per face
    /// into the render target's cube.
    ///
    /// The renderer's current render target and MRT are restored afterwards,
    /// as three restores them (`setMRT( null )` for the capture). Each face is drawn into the target's 2-D face buffer and
    /// copied into its layer (`renderer.setRenderTarget( renderTarget, i )`
    /// in three); the cube carries no mipmaps unless it was made with them,
    /// and then they are regenerated after the last face, which is when
    /// three's `mipmapsAutoUpdate` dance has them built.
    pub fn update(&mut self, renderer: &mut Renderer, scene: &mut Scene) {
        // `if ( this.parent === null ) this.updateMatrixWorld();`
        if self.node.parent().is_none() {
            self.node.update_matrix_world(false);
        }

        let mut renderer = renderer.save_state();
        renderer.set_mrt(None);
        renderer.set_render_target(Some(self.render_target.face.clone()));

        for (layer, camera) in self.cameras.iter_mut().enumerate() {
            renderer.render(scene, camera);
            renderer.copy_to_cube_layer(
                &self.render_target.face,
                &self.render_target.texture,
                layer as u32,
                0,
            );
        }

        let mips = self.render_target.texture.mip_level_count();
        if mips > 1 {
            renderer.generate_cube_mipmaps(&self.render_target.texture);
        }
    }
}
