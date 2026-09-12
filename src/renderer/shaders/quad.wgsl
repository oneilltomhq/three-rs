// Hand-written stand-in for the WGSL that a `QuadMesh` with
// `MeshBasicNodeMaterial.colorNode = texture( depthTexture )` generates.
// Rung 4 replaces this file with the ported TSL path.
//
// Vertex: `QuadMesh.render()` swaps in a vertexNode of
//   vec4( array( [ -1, -1, 3 ] ).element( vertexIndex ),
//         array( [ 3, -1, -1 ] ).element( vertexIndex ), 0, 1 )
// and `QuadGeometry` supplies uv ( 0, -1 ), ( 0, 1 ), ( 2, 1 ).
//
// Fragment: sampling a depth texture yields one f32 (`TextureNode` uses the
// snippet type `float` when `texture.isDepthTexture`), `vec4( colorNode )`
// splats it, the opaque path forces alpha to 1, and because this pass targets
// the canvas `DirectRenderPipeline` wraps the result in
// `renderOutput( output, NoToneMapping, SRGBColorSpace )` — i.e. just the sRGB
// transfer function, applied to rgb only.
//
// `builder.isFlipY()` is false for WGSL, so the flipY uniform that
// `TextureNode.setupUV()` would otherwise insert for a depth texture never
// appears in the WebGPU backend.

@group(0) @binding(0) var depthTexture : texture_depth_2d;
@group(0) @binding(1) var depthTexture_sampler : sampler;

struct VertexOutput {
	@builtin(position) position : vec4<f32>,
	@location(0) vUv : vec2<f32>,
};

@vertex
fn main_vertex( @builtin(vertex_index) vertexIndex : u32 ) -> VertexOutput {

	var xs = array<f32, 3>( -1.0, -1.0, 3.0 );
	var ys = array<f32, 3>( 3.0, -1.0, -1.0 );
	var uvs = array<vec2<f32>, 3>( vec2<f32>( 0.0, -1.0 ), vec2<f32>( 0.0, 1.0 ), vec2<f32>( 2.0, 1.0 ) );

	var output : VertexOutput;
	output.position = vec4<f32>( xs[ vertexIndex ], ys[ vertexIndex ], 0.0, 1.0 );
	output.vUv = uvs[ vertexIndex ];
	return output;

}

// `sRGBTransferOETF` from src/nodes/display/ColorSpaceFunctions.js, constants included.
fn sRGBTransferOETF( color : vec3<f32> ) -> vec3<f32> {

	let a = pow( color, vec3<f32>( 0.41666 ) ) * 1.055 - 0.055;
	let b = color * 12.92;
	return select( a, b, color <= vec3<f32>( 0.0031308 ) );

}

@fragment
fn main_fragment( input : VertexOutput ) -> @location(0) vec4<f32> {

	let depth = textureSample( depthTexture, depthTexture_sampler, input.vUv );

	let diffuseColor = vec4<f32>( vec3<f32>( depth ), 1.0 );
	let outputNode = max( diffuseColor, vec4<f32>( 0.0 ) );

	return vec4<f32>( sRGBTransferOETF( outputNode.rgb ), outputNode.a );

}
