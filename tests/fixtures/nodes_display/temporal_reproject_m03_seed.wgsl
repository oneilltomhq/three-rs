// Three.js r187dev - Node System

// global
diagnostic( off, derivative_uniformity );


// directives


// structs

struct OutputStruct {
	@location( 0 ) color: vec4<f32>
};
var<private> output : OutputStruct;

// uniforms
@binding( 0 ) @group( 0 ) var nodeUniform0_sampler : sampler;
@binding( 1 ) @group( 0 ) var nodeUniform0 : texture_2d<f32>;

struct objectStruct {
	nodeUniform1 : vec2<f32>
};
@binding( 2 ) @group( 0 )
var<uniform> object : objectStruct;

// vars
var<private> nodeVar0 : vec4<f32>;

// codes
fn beautyTexelFromScreen ( screenTexel : vec2<i32>, beautySize : vec2<f32>, resolveSize : vec2<f32> ) -> vec2<i32> {

	


	return vec2<i32>( floor( ( ( vec2<f32>( screenTexel ) * beautySize ) / resolveSize ) ) );

}




@fragment
fn main( @builtin( position ) fragCoord : vec4<f32> ) -> OutputStruct {

	// flow
	// code

	nodeVar0 = textureLoad( nodeUniform0, beautyTexelFromScreen( vec2<i32>( floor( ( fragCoord.xy - vec2<f32>( 0.5 ) ) ) ), vec2<f32>( textureDimensions( nodeUniform0, 0 ) ), object.nodeUniform1 ), u32( 0u ) );

	// result

	output.color = max( nodeVar0, vec4<f32>( 0.0 ) );

	return output;

}
