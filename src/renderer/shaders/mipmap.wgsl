// Port of the `mipmap` shader module in
// `three.js/src/renderers/webgpu/utils/WebGPUTexturePassUtils.js`, used by
// `WebGPUTextureUtils.generateMipmaps()`.
//
// Each level is produced by drawing one oversized triangle over the destination
// mip level while sampling the level above with a `minFilter: linear` sampler —
// a 2x bilinear (box) downsample. Only the `main_2d_array` entry point is kept:
// a dump of the real example shows three.js creating the pipeline
// `mipmap-rgba8unorm-srgb-2d-array` for the pisa cube map, because
// `getTransferPipeline()` reads `GPUTexture.textureBindingViewDimension`, which
// Chrome does not expose on the texture object, so it falls back to
// `'2d-array'`. Each cube face is therefore downsampled as an independent 2D
// layer, with clamp-to-edge at the face borders and no cross-face filtering.

struct VarysStruct {
	@builtin( position ) Position : vec4<f32>,
	@location( 0 ) vTex : vec2<f32>,
	@location( 1 ) @interpolate(flat, either) vBaseArrayLayer : u32,
};

@group( 0 ) @binding ( 2 )
var<uniform> flipY : u32;

@vertex
fn mainVS(
		@builtin( vertex_index ) vertexIndex : u32,
		@builtin( instance_index ) instanceIndex : u32 ) -> VarysStruct {

	var Varys : VarysStruct;

	var pos = array(
		vec2<f32>( -1.0, -1.0 ),
		vec2<f32>( -1.0,  3.0 ),
		vec2<f32>(  3.0, -1.0 ),
	);

	let p = pos[ vertexIndex ];
	let mult = select( vec2<f32>( 0.5, -0.5 ), vec2<f32>( 0.5, 0.5 ), flipY != 0u );
	Varys.vTex = p * mult + vec2<f32>( 0.5 );
	Varys.Position = vec4<f32>( p, 0.0, 1.0 );
	Varys.vBaseArrayLayer = instanceIndex;

	return Varys;

}

@group( 0 ) @binding( 0 )
var imgSampler : sampler;

@group( 0 ) @binding( 1 )
var img2dArray : texture_2d_array<f32>;

@fragment
fn main_2d_array( Varys : VarysStruct ) -> @location( 0 ) vec4<f32> {

	return textureSample( img2dArray, imgSampler, Varys.vTex, Varys.vBaseArrayLayer );

}
