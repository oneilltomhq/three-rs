// Hand-written stand-in for the WGSL that `Renderer._renderOutput()` builds:
// a `QuadMesh` with `NodeMaterial.fragmentNode = nodes.getOutputNode( texture )`,
// i.e. `RenderOutputNode` → `renderOutput( color, NoToneMapping, SRGBColorSpace )`.
// Rung 4 replaces this file with the ported TSL path.
//
// Why it exists at all: `Renderer.needsFrameBufferTarget` is true whenever the
// output colour space differs from the working colour space, which is the
// default (`SRGBColorSpace` vs `LinearSRGBColorSpace`). The scene is therefore
// rendered into an internal `RenderTarget` (`_getFrameBufferTarget()`:
// `HalfFloatType`, `RGBAFormat`, `LinearFilter`, `samples = renderer.samples`)
// in *linear* colour, MSAA-resolved there, and only then does this full-screen
// pass convert to sRGB into the canvas. Doing the conversion inside the scene
// pass instead would average sRGB-encoded samples at every MSAA edge.
//
// Vertex: `QuadGeometry` is one oversized triangle at
//   ( -1, 3, 0 ), ( -1, -1, 0 ), ( 3, -1, 0 )
// rendered with `OrthographicCamera( -1, 1, 1, -1, 0, 1 )` and an identity
// world matrix, so the clip-space positions are the attribute values.
//
// Fragment: `RenderOutputNode.setup()` →
//   color = clamp alpha, unpremultiply, sRGBTransferOETF( rgb ), premultiply
// and `TextureNode`'s uv for a `viewportTexture`-style read is
// `screenCoordinate / viewportSize`, which three.js emits verbatim as
// `fragCoord.xy / viewportSize`.

struct RenderUniforms {
	projectionMatrix : mat4x4<f32>,
	viewMatrix : mat4x4<f32>,
	viewportSize : vec2<f32>,
};

@group(0) @binding(0) var<uniform> render : RenderUniforms;
@group(0) @binding(1) var outputTexture : texture_2d<f32>;
@group(0) @binding(2) var outputTexture_sampler : sampler;

@vertex
fn main_vertex( @builtin(vertex_index) vertexIndex : u32 ) -> @builtin(position) vec4<f32> {

	var xs = array<f32, 3>( -1.0, -1.0, 3.0 );
	var ys = array<f32, 3>( 3.0, -1.0, -1.0 );

	return vec4<f32>( xs[ vertexIndex ], ys[ vertexIndex ], 0.0, 1.0 );

}

// `unpremultiplyAlpha` / `premultiplyAlpha` from src/nodes/display/ColorSpaceNode.js
fn unpremultiplyAlpha( color : vec4<f32> ) -> vec4<f32> {

	if ( color.w == 0.0 ) {

		return vec4<f32>( 0.0, 0.0, 0.0, 0.0 );

	}

	return vec4<f32>( color.xyz / vec3<f32>( color.w ), color.w );

}

fn premultiplyAlpha( color : vec4<f32> ) -> vec4<f32> {

	return vec4<f32>( color.xyz * vec3<f32>( color.w ), color.w );

}

// `sRGBTransferOETF` from src/nodes/display/ColorSpaceFunctions.js.
fn sRGBTransferOETF( color : vec3<f32> ) -> vec3<f32> {

	return mix(
		( pow( color, vec3<f32>( 0.41666 ) ) * vec3<f32>( 1.055 ) ) - vec3<f32>( 0.055 ),
		color * vec3<f32>( 12.92 ),
		vec3<f32>( color <= vec3<f32>( 0.0031308 ) )
	);

}

@fragment
fn main_fragment( @builtin(position) fragCoord : vec4<f32> ) -> @location(0) vec4<f32> {

	let sampled = textureSample( outputTexture, outputTexture_sampler, fragCoord.xy / render.viewportSize );
	let color = unpremultiplyAlpha( vec4<f32>( sampled.xyz, clamp( sampled.w, 0.0, 1.0 ) ) );

	return premultiplyAlpha( vec4<f32>( sRGBTransferOETF( color.xyz ), color.w ) );

}
