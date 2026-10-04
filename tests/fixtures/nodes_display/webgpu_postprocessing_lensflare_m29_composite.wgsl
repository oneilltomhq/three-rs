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
@binding( 0 ) @group( 1 ) var nodeUniform0_sampler : sampler;
@binding( 1 ) @group( 1 ) var nodeUniform0 : texture_2d<f32>;
@binding( 2 ) @group( 1 ) var nodeUniform1_sampler : sampler;
@binding( 3 ) @group( 1 ) var nodeUniform1 : texture_2d<f32>;
@binding( 4 ) @group( 1 ) var nodeUniform2_sampler : sampler;
@binding( 5 ) @group( 1 ) var nodeUniform2 : texture_2d<f32>;

struct renderStruct {
	nodeUniform3 : f32
};
@binding( 0 ) @group( 0 )
var<uniform> render : renderStruct;

// vars
var<private> nodeVar0 : vec4<f32>;
var<private> nodeVar1 : vec4<f32>;
var<private> nodeVar2 : vec4<f32>;

// codes
fn fn1 ( color : vec4<f32> ) -> vec4<f32> {

	var nodeVar0 : vec4<f32>;


	if ( ( color.w == 0.0 ) ) {

		nodeVar0 = vec4<f32>( 0.0, 0.0, 0.0, 0.0 );

	} else {

		nodeVar0 = vec4<f32>( ( color.xyz / vec3<f32>( color.w ) ), color.w );

	}


	return nodeVar0;

}


fn acesFilmicToneMapping ( color : vec3<f32>, exposure : f32 ) -> vec3<f32> {

	

	let nodeConst0 = ( mat3x3<f32>( 0.59719, 0.076, 0.0284, 0.35458, 0.90834, 0.13383, 0.04823, 0.01566, 0.83777 ) * ( ( color * vec3<f32>( exposure ) ) / vec3<f32>( 0.6 ) ) );

	return clamp( ( mat3x3<f32>( 1.60475, -0.10208, -0.00327, -0.53108, 1.10813, -0.07276, -0.07367, -0.00605, 1.07602 ) * ( ( ( nodeConst0 * ( nodeConst0 + vec3<f32>( 0.0245786 ) ) ) - vec3<f32>( 0.000090537 ) ) / ( ( nodeConst0 * ( ( nodeConst0 + vec3<f32>( 0.432951 ) ) * vec3<f32>( 0.983729 ) ) ) + vec3<f32>( 0.238081 ) ) ) ), vec3<f32>( 0.0 ), vec3<f32>( 1.0 ) );

}


fn sRGBTransferOETF ( color : vec3<f32> ) -> vec3<f32> {

	


	return mix( ( ( pow( color, vec3<f32>( 0.41666 ) ) * vec3<f32>( 1.055 ) ) - vec3<f32>( 0.055 ) ), ( color * vec3<f32>( 12.92 ) ), vec3<f32>( ( color <= vec3<f32>( 0.0031308 ) ) ) );

}


fn fn0 ( color : vec4<f32> ) -> vec4<f32> {

	


	return vec4<f32>( ( color.xyz * vec3<f32>( color.w ) ), color.w );

}




@fragment
fn main( @location( 0 ) nodeVarying0 : vec2<f32> ) -> OutputStruct {

	// flow
	// code

	nodeVar0 = textureSample( nodeUniform0, nodeUniform0_sampler, nodeVarying0 );
	let nodeVar0 = nodeVar0;
	nodeVar1 = textureSample( nodeUniform1, nodeUniform1_sampler, nodeVarying0 );
	let nodeConst0 = nodeVar1;
	nodeVar2 = textureSample( nodeUniform2, nodeUniform2_sampler, nodeVarying0 );
	let nodeConst1 = nodeVar2;
	let nodeConst2 = ( ( nodeVar0 + nodeConst0 ) + nodeConst1 );
	let nodeConst3 = fn1( vec4<f32>( nodeConst2.xyz, clamp( nodeConst2.w, 0.0, 1.0 ) ) );
	let nodeConst4 = vec4<f32>( acesFilmicToneMapping( nodeConst3.xyz, render.nodeUniform3 ), nodeConst3.w );

	// result

	output.color = fn0( vec4<f32>( sRGBTransferOETF( nodeConst4.xyz ), nodeConst4.w ) );

	return output;

}
