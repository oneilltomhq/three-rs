// Hand-written stand-in for the WGSL that a `MeshBasicNodeMaterial` with
//
//   material.colorNode = mix( normalWorld, range( black, white ),
//                             oscSine( time.mul( .1 ) ) )
//
// generates on an `InstancedMesh`. Rung 4 replaces this file with the ported
// TSL path. It follows the flow three.js emits, in order:
//
// VERTEX — `NodeMaterial.setupPosition()` calls `instancedMesh( object )`
// (src/nodes/accessors/Instance.js), then `setupVertex()`:
//
//   instanceMatrix = buffer( instanceMatrix.array, 'mat4', count )
//                        .element( instanceIndex )        // 1000*64 B <= the
//                                                        // uniform buffer limit
//   positionLocal  = ( instanceMatrix * vec4( positionLocal, 1 ) ).xyz
//   normalLocal    = normalize( transpose( inverse( mat3( instanceMatrix ) ) )
//                               * normalLocal )          // transformNormal()
//   normalViewGeometry = normalize( ( cameraViewMatrix
//                          * vec4( modelNormalMatrix * normalLocal, 0 ) ).xyz )
//                        -> varying v_normalViewGeometry
//   modelViewMatrix = cameraViewMatrix * modelWorldMatrix  // mediump, the default
//   positionView    = ( modelViewMatrix * vec4( positionLocal, 1 ) ).xyz
//   clip            = cameraProjectionMatrix * vec4( positionView, 1 )
//
// FRAGMENT:
//
//   normalViewGeometry = normalize( v_normalViewGeometry )
//   normalView   = negateOnBackSide( normalViewGeometry )  // FrontSide: identity
//   normalWorld  = normalize( ( vec4( normalView, 0 ) * cameraViewMatrix ).xyz )
//   randomColors = range buffer[ instanceIndex ].xyz
//   colorNode    = mix( normalWorld, randomColors, oscSine( time * 0.1 ) )
//   diffuseColor = vec4( colorNode, 1.0 )                  // vec3 -> vec4 fills 1
//   diffuseColor.a = 1                                     // opaque
//   outgoingLight  = diffuseColor.rgb                      // setupOutgoingLight
//   basicOutput    = max( vec4( outgoingLight, diffuseColor.a ), 0 )
//
// and that is where the scene shader stops: `Renderer.needsFrameBufferTarget` is
// true by default, so this pass renders into an internal `HalfFloatType` render
// target in the working (linear) colour space and `output_color_transform.wgsl`
// applies `renderOutput( …, NoToneMapping, SRGBColorSpace )` in a separate
// full-screen pass. (Verified against the four shader modules Chrome's
// `createShaderModule` receives for this example: vertex, fragment,
// vertex_outputColorTransform, fragment_outputColorTransform.)
//
// INSTANCE_COUNT is substituted at shader-build time, the same way three.js
// bakes `count` into the `array<…, N>` of `buffer( array, type, count )`.

struct Camera {
	projectionMatrix : mat4x4<f32>,
	viewMatrix : mat4x4<f32>,
};

struct ObjectUniforms {
	worldMatrix : mat4x4<f32>,
	// `modelNormalMatrix`: `normalMatrix.getNormalMatrix( object.matrixWorld )`.
	normalMatrix : mat3x3<f32>,
};

// `time` — a renderGroup uniform fed from `NodeFrame.time`.
struct Frame {
	time : f32,
};

struct InstanceMatrices {
	data : array<mat4x4<f32>, INSTANCE_COUNT>,
};

struct Ranges {
	data : array<vec4<f32>, INSTANCE_COUNT>,
};

@group(0) @binding(0) var<uniform> camera : Camera;
@group(0) @binding(1) var<uniform> object : ObjectUniforms;
@group(0) @binding(2) var<uniform> frame : Frame;
@group(0) @binding(3) var<uniform> instances : InstanceMatrices;
@group(0) @binding(4) var<uniform> ranges : Ranges;

struct VertexOutput {
	@builtin(position) position : vec4<f32>,
	@location(0) v_normalViewGeometry : vec3<f32>,
	@location(1) @interpolate(flat) v_instanceIndex : u32,
};

// `tsl_inverse_mat3` from src/renderers/webgpu/nodes/WGSLNodeBuilder.js, verbatim.
fn tsl_inverse_mat3( m : mat3x3<f32> ) -> mat3x3<f32> {

	let a00 = m[ 0 ][ 0 ]; let a01 = m[ 0 ][ 1 ]; let a02 = m[ 0 ][ 2 ];
	let a10 = m[ 1 ][ 0 ]; let a11 = m[ 1 ][ 1 ]; let a12 = m[ 1 ][ 2 ];
	let a20 = m[ 2 ][ 0 ]; let a21 = m[ 2 ][ 1 ]; let a22 = m[ 2 ][ 2 ];

	let b01 = a22 * a11 - a12 * a21;
	let b11 = - a22 * a10 + a12 * a20;
	let b21 = a21 * a10 - a11 * a20;

	let det = a00 * b01 + a01 * b11 + a02 * b21;

	return mat3x3<f32>(
		b01, ( - a22 * a01 + a02 * a21 ), ( a12 * a01 - a02 * a11 ),
		b11, ( a22 * a00 - a02 * a20 ), ( - a12 * a00 + a02 * a10 ),
		b21, ( - a21 * a00 + a01 * a20 ), ( a11 * a00 - a01 * a10 )
	) * ( 1.0 / det );

}

@vertex
fn main_vertex(
	@builtin(instance_index) instanceIndex : u32,
	@location(0) position : vec3<f32>,
	@location(1) normal : vec3<f32>
) -> VertexOutput {

	let instanceMatrix = instances.data[ instanceIndex ];

	// instance() — POSITION
	let positionLocal = ( instanceMatrix * vec4<f32>( position, 1.0 ) ).xyz;

	// instance() — NORMAL: transformNormal( normalLocal, instanceMatrix )
	let instanceNormalMatrix = transpose( tsl_inverse_mat3( mat3x3<f32>(
		instanceMatrix[ 0 ].xyz, instanceMatrix[ 1 ].xyz, instanceMatrix[ 2 ].xyz ) ) );
	let normalLocal = normalize( instanceNormalMatrix * normal );

	// transformNormalToView( normalLocal )
	let transformedNormal = object.normalMatrix * normalLocal;
	let normalViewGeometry = normalize( ( camera.viewMatrix * vec4<f32>( transformedNormal, 0.0 ) ).xyz );

	let modelViewMatrix = camera.viewMatrix * object.worldMatrix;
	let positionView = ( modelViewMatrix * vec4<f32>( positionLocal, 1.0 ) ).xyz;

	var output : VertexOutput;
	output.position = camera.projectionMatrix * vec4<f32>( positionView, 1.0 );
	output.v_normalViewGeometry = normalViewGeometry;
	output.v_instanceIndex = instanceIndex;
	return output;

}

// `oscSine` from src/nodes/utils/Oscillators.js:
//   ( t ) => t.add( 0.75 ).mul( Math.PI * 2 ).sin().mul( 0.5 ).add( 0.5 )
fn oscSine( t : f32 ) -> f32 {

	return sin( ( t + 0.75 ) * 6.283185307179586 ) * 0.5 + 0.5;

}

@fragment
fn main_fragment( input : VertexOutput ) -> @location(0) vec4<f32> {

	let normalViewGeometry = normalize( input.v_normalViewGeometry );
	let normalView = normalViewGeometry;
	let normalWorld = normalize( ( vec4<f32>( normalView, 0.0 ) * camera.viewMatrix ).xyz );

	let randomColors = ranges.data[ input.v_instanceIndex ].xyz;

	let colorNode = mix( normalWorld, randomColors, oscSine( frame.time * 0.1 ) );

	let diffuseColor = vec4<f32>( colorNode, 1.0 );
	let outgoingLight = diffuseColor.rgb;
	// The last line of the generated scene fragment shader: the colour stays in
	// the working (linear) colour space, because this pass renders into the
	// renderer's internal framebuffer target. `output_color_transform.wgsl`
	// applies `renderOutput( …, NoToneMapping, SRGBColorSpace )` afterwards.
	let basicOutput = max( vec4<f32>( outgoingLight, diffuseColor.a ), vec4<f32>( 0.0 ) );

	return basicOutput;

}
