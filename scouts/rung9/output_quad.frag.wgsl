// Three.js r186dev - Node System

// global
diagnostic( off, derivative_uniformity );


// structs

struct OutputStruct {
	@location( 0 ) color: vec4<f32>
};
var<private> output : OutputStruct;

// uniforms
@binding( 0 ) @group( 0 ) var nodeUniform0_sampler : sampler;
@binding( 1 ) @group( 0 ) var nodeUniform0 : texture_2d<f32>;
@binding( 2 ) @group( 0 ) var nodeUniform1_sampler : sampler;
@binding( 3 ) @group( 0 ) var nodeUniform1 : texture_2d<f32>;
@binding( 5 ) @group( 0 ) var nodeUniform3_sampler : sampler;
@binding( 6 ) @group( 0 ) var nodeUniform3 : texture_2d<f32>;
@binding( 7 ) @group( 0 ) var nodeUniform4_sampler : sampler;
@binding( 8 ) @group( 0 ) var nodeUniform4 : texture_2d<f32>;
@binding( 9 ) @group( 0 ) var nodeUniform6_sampler : sampler;
@binding( 10 ) @group( 0 ) var nodeUniform6 : texture_2d<f32>;

struct objectStruct {
	nodeUniform2 : mat3x3<f32>,
	nodeUniform5 : mat3x3<f32>
};
@binding( 4 ) @group( 0 )
var<uniform> object : objectStruct;

// vars
var<private> nodeVar0 : vec4<f32>;
var<private> nodeVar1 : vec4<f32>;
var<private> nodeVar2 : vec4<f32>;
var<private> nodeVar3 : vec4<f32>;
var<private> nodeVar4 : vec4<f32>;
var<private> nodeVar5 : vec4<f32>;
var<private> nodeVar6 : vec4<f32>;
var<private> nodeVar7 : vec4<f32>;
var<private> nodeVar8 : vec4<f32>;
var<private> nodeVar9 : vec4<f32>;

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

	nodeVar0 = textureSample( nodeUniform0, nodeUniform0_sampler, nodeVarying0 );
	nodeVar1 = nodeVar0;
	nodeVar2 = textureSample( nodeUniform1, nodeUniform1_sampler, ( object.nodeUniform2 * vec3<f32>( nodeVarying0, 1.0 ) ).xy );
	nodeVar3 = textureSample( nodeUniform3, nodeUniform3_sampler, nodeVarying0 );
	nodeVar4 = nodeVar3;
	nodeVar5 = textureSample( nodeUniform4, nodeUniform4_sampler, ( object.nodeUniform5 * vec3<f32>( nodeVarying0, 1.0 ) ).xy );
	nodeVar6 = textureSample( nodeUniform6, nodeUniform6_sampler, nodeVarying0 );
	nodeVar7 = nodeVar6;
	nodeVar8 = mix( mix( nodeVar1, nodeVar2, nodeVar4.w ), nodeVar5, nodeVar7.w );
	nodeVar9 = fn1( vec4<f32>( nodeVar8.xyz, clamp( nodeVar8.w, 0.0, 1.0 ) ) );

	// result

	output.color = fn0( vec4<f32>( sRGBTransferOETF( nodeVar9.xyz ), nodeVar9.w ) );

	return output;

}
