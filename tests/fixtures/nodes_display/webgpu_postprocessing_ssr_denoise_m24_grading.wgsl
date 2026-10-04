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

struct objectStruct {
	nodeUniform4 : f32,
	nodeUniform5 : f32,
	nodeUniform6 : f32
};
@binding( 6 ) @group( 1 )
var<uniform> object : objectStruct;

// vars
var<private> nodeVar0 : vec4<f32>;
var<private> nodeVar1 : vec4<f32>;
var<private> nodeVar2 : vec4<f32>;
var<private> nodeVar3 : bool;

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


fn agxToneMapping ( color : vec3<f32>, exposure : f32 ) -> vec3<f32> {

	var nodeVar0 : vec3<f32>;
	var nodeVar1 : vec3<f32>;
	var nodeVar2 : vec3<f32>;
	var nodeVar3 : vec3<f32>;

	nodeVar0 = color;
	nodeVar0 = ( nodeVar0 * vec3<f32>( exposure ) );
	nodeVar0 = ( mat3x3<f32>( vec3<f32>( 0.6274, 0.0691, 0.0164 ), vec3<f32>( 0.3293, 0.9195, 0.088 ), vec3<f32>( 0.0433, 0.0113, 0.8956 ) ) * nodeVar0 );
	nodeVar0 = ( mat3x3<f32>( vec3<f32>( 0.856627153315983, 0.137318972929847, 0.11189821299995 ), vec3<f32>( 0.0951212405381588, 0.761241990602591, 0.0767994186031903 ), vec3<f32>( 0.0482516061458583, 0.101439036467562, 0.811302368396859 ) ) * nodeVar0 );
	nodeVar0 = max( nodeVar0, vec3<f32>( 1e-10 ) );
	nodeVar0 = log2( nodeVar0 );
	nodeVar0 = ( ( nodeVar0 - vec3<f32>( -12.47393 ) ) / vec3<f32>( ( 4.026069 - -12.47393 ) ) );
	nodeVar0 = clamp( nodeVar0, vec3<f32>( 0.0 ), vec3<f32>( 1.0 ) );
	nodeVar1 = nodeVar0;
	nodeVar2 = ( nodeVar1 * nodeVar1 );
	nodeVar3 = ( nodeVar2 * nodeVar2 );
	nodeVar0 = ( ( ( vec3<f32>( 15.5 ) * ( nodeVar3 * nodeVar2 ) ) - ( vec3<f32>( 40.14 ) * ( nodeVar3 * nodeVar1 ) ) ) + ( ( ( vec3<f32>( 31.96 ) * nodeVar3 ) - ( vec3<f32>( 6.868 ) * ( nodeVar2 * nodeVar1 ) ) ) + ( ( vec3<f32>( 0.4298 ) * nodeVar2 ) + ( ( vec3<f32>( 0.1191 ) * nodeVar1 ) - vec3<f32>( 0.00232 ) ) ) ) );
	nodeVar0 = ( mat3x3<f32>( vec3<f32>( 1.1271005818144368, -0.1413297634984383, -0.14132976349843826 ), vec3<f32>( -0.11060664309660323, 1.157823702216272, -0.11060664309660294 ), vec3<f32>( -0.016493938717834573, -0.016493938717834257, 1.2519364065950405 ) ) * nodeVar0 );
	nodeVar0 = pow( max( vec3<f32>( 0.0, 0.0, 0.0 ), nodeVar0 ), vec3<f32>( 2.2, 2.2, 2.2 ) );
	nodeVar0 = ( mat3x3<f32>( vec3<f32>( 1.6605, -0.1246, -0.0182 ), vec3<f32>( -0.5876, 1.1329, -0.1006 ), vec3<f32>( -0.0728, -0.0083, 1.1187 ) ) * nodeVar0 );
	nodeVar0 = clamp( nodeVar0, vec3<f32>( 0.0 ), vec3<f32>( 1.0 ) );

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
	let nodeVar0 = nodeVar0;
	nodeVar1 = textureSample( nodeUniform1, nodeUniform1_sampler, nodeVarying0 );
	let nodeConst0 = nodeVar1;
	nodeVar2 = textureSample( nodeUniform2, nodeUniform2_sampler, nodeVarying0 );
	let nodeConst1 = nodeVar2;
	nodeVar3 = ( nodeConst1.w > 0.0 );
	let nodeConst2 = vec4<f32>( vec4<f32>( ( nodeVar0.xyz + vec4<f32>( nodeConst0.xyz, f32( nodeVar3 ) ).xyz ), 1.0 ).xyz, 1.0 );
	let nodeConst3 = fn1( vec4<f32>( nodeConst2.xyz, clamp( nodeConst2.w, 0.0, 1.0 ) ) );
	let nodeConst4 = vec4<f32>( agxToneMapping( nodeConst3.xyz, render.nodeUniform3 ), nodeConst3.w );

	// result

	output.color = vec4<f32>( pow( max( max( mix( vec3<f32>( dot( ( ( ( fn0( vec4<f32>( sRGBTransferOETF( nodeConst4.xyz ), nodeConst4.w ) ).xyz - vec3<f32>( 0.5 ) ) * vec3<f32>( object.nodeUniform4 ) ) + vec3<f32>( 0.5 ) ), vec3<f32>( 0.2126, 0.7152, 0.0722 ) ) ), ( ( ( fn0( vec4<f32>( sRGBTransferOETF( nodeConst4.xyz ), nodeConst4.w ) ).xyz - vec3<f32>( 0.5 ) ) * vec3<f32>( object.nodeUniform4 ) ) + vec3<f32>( 0.5 ) ), object.nodeUniform5 ), vec3<f32>( 0.0 ) ), vec3<f32>( 0.0 ) ), vec3<f32>( ( 1.0 / object.nodeUniform6 ) ) ), 1.0 );

	return output;

}
