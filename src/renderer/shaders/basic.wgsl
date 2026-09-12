// Hand-written stand-in for the WGSL that `MeshBasicNodeMaterial` (no colorNode)
// generates through three.js' node system. Rung 4 replaces this file with the
// ported TSL path. It follows the same flow three.js emits:
//
//   modelViewMatrix = cameraViewMatrix * modelWorldMatrix   (in the shader, f32;
//       three.js' `mediumpModelViewMatrix`, the default for `modelViewMatrix`)
//   positionView    = ( modelViewMatrix * vec4( positionLocal, 1 ) ).xyz
//   clip            = cameraProjectionMatrix * vec4( positionView, 1 )
//   diffuseColor    = materialColor (white), alpha forced to 1 (opaque)
//   output          = vec4( diffuseColor.rgb, diffuseColor.a ).max( 0 )
//
// No output colour transform: this material only ever draws into the render
// target, and `DirectRenderPipeline` applies `renderOutput()` to canvas passes
// only.

struct Camera {
	projectionMatrix : mat4x4<f32>,
	viewMatrix : mat4x4<f32>,
};

struct ObjectUniforms {
	worldMatrix : mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> camera : Camera;
@group(0) @binding(1) var<uniform> object : ObjectUniforms;

struct VertexOutput {
	@builtin(position) position : vec4<f32>,
};

@vertex
fn main_vertex( @location(0) positionLocal : vec3<f32> ) -> VertexOutput {

	let modelViewMatrix = camera.viewMatrix * object.worldMatrix;
	let positionView = ( modelViewMatrix * vec4<f32>( positionLocal, 1.0 ) ).xyz;

	var output : VertexOutput;
	output.position = camera.projectionMatrix * vec4<f32>( positionView, 1.0 );
	return output;

}

@fragment
fn main_fragment() -> @location(0) vec4<f32> {

	return max( vec4<f32>( 1.0, 1.0, 1.0, 1.0 ), vec4<f32>( 0.0 ) );

}
