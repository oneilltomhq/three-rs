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

	let nodeConst0 = ( vec2<f32>( cos( 0.0 ), sin( 0.0 ) ) * vec2<f32>( 0.001 ) );
	nodeVar0 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 + nodeConst0 ) );
	nodeVar1 = textureSample( nodeUniform0, nodeUniform0_sampler, nodeVarying0 );
	nodeVar2 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 - nodeConst0 ) );
	let nodeConst1 = vec4<f32>( nodeVar0.x, nodeVar1.y, nodeVar2.z, nodeVar1.w );
	let nodeConst2 = fn1( vec4<f32>( nodeConst1.xyz, clamp( nodeConst1.w, 0.0, 1.0 ) ) );

	// result

	output.color = fn0( vec4<f32>( sRGBTransferOETF( nodeConst2.xyz ), nodeConst2.w ) );

	return output;

}
