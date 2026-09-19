// Three.js r186 - Node System

// global
diagnostic( off, derivative_uniformity );


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

struct renderStruct {
	nodeUniform2 : f32
};
@binding( 0 ) @group( 0 )
var<uniform> render : renderStruct;

// vars
var<private> nodeVar0 : vec4<f32>;
var<private> nodeVar1 : vec4<f32>;
var<private> nodeVar2 : vec3<f32>;
var<private> nodeVar3 : vec4<f32>;
var<private> nodeVar4 : vec4<f32>;

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


fn neutralToneMapping ( color : vec3<f32>, exposure : f32 ) -> vec3<f32> {

	var nodeVar0 : vec3<f32>;
	var nodeVar1 : f32;
	var nodeVar2 : f32;
	var nodeVar3 : f32;
	var nodeVar4 : f32;
	var nodeVar5 : f32;

	nodeVar0 = ( color * vec3<f32>( exposure ) );
	nodeVar2 = min( nodeVar0.x, min( nodeVar0.y, nodeVar0.z ) );

	if ( ( nodeVar2 < 0.08 ) ) {

		nodeVar1 = ( nodeVar2 - ( 6.25 * ( nodeVar2 * nodeVar2 ) ) );

	} else {

		nodeVar1 = 0.04;

	}

	nodeVar0 = ( nodeVar0 - vec3<f32>( nodeVar1 ) );
	nodeVar3 = max( nodeVar0.x, max( nodeVar0.y, nodeVar0.z ) );

	if ( ( nodeVar3 < 0.76 ) ) {

		return nodeVar0;

	}

	nodeVar4 = ( 1.0 - 0.76 );
	nodeVar5 = ( 1.0 - ( ( nodeVar4 * nodeVar4 ) / ( nodeVar3 + ( nodeVar4 - 0.76 ) ) ) );
	nodeVar0 = ( nodeVar0 * vec3<f32>( ( nodeVar5 / nodeVar3 ) ) );

	return mix( nodeVar0, vec3<f32>( nodeVar5 ), ( 1.0 - ( 1.0 / ( ( 0.15 * ( nodeVar3 - nodeVar5 ) ) + 1.0 ) ) ) );

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
	nodeVar1 = textureSample( nodeUniform1, nodeUniform1_sampler, nodeVarying0 );
	nodeVar2 = max( mix( vec3<f32>( dot( nodeVar0.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) ) ), nodeVar0.xyz, clamp( ( dot( abs( ( nodeVar1 - nodeVar0 ) ), vec4<f32>( vec3<f32>( 0.2126, 0.7152, 0.0722 ), 1.0 ) ) * 1000.0 ), 0.0, 3.0 ) ), vec3<f32>( 0.0 ) );
	nodeVar3 = fn1( vec4<f32>( nodeVar2, clamp( vec4<f32>( nodeVar2, 1.0 ).w, 0.0, 1.0 ) ) );
	nodeVar4 = vec4<f32>( neutralToneMapping( nodeVar3.xyz, render.nodeUniform2 ), nodeVar3.w );

	// result

	output.color = fn0( vec4<f32>( sRGBTransferOETF( nodeVar4.xyz ), nodeVar4.w ) );

	return output;

}
