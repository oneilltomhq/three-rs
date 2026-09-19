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

struct objectStruct {
	nodeUniform1 : f32,
	nodeUniform2 : f32,
	nodeUniform3 : f32,
	nodeUniform4 : f32
};
@binding( 2 ) @group( 1 )
var<uniform> object : objectStruct;

struct renderStruct {
	nodeUniform5 : f32
};
@binding( 0 ) @group( 0 )
var<uniform> render : renderStruct;

// vars
var<private> nodeVar0 : vec2<f32>;
var<private> nodeVar1 : vec4<f32>;
var<private> nodeVar2 : vec4<f32>;
var<private> nodeVar3 : f32;
var<private> nodeVar4 : vec4<f32>;
var<private> nodeVar5 : vec4<f32>;
var<private> nodeVar6 : vec4<f32>;
var<private> nodeVar7 : vec4<f32>;

// codes
fn interleavedGradientNoise ( position : vec2<f32> ) -> f32 {

	


	return fract( ( 52.9829189 * fract( dot( position, vec2<f32>( 0.06711056, 0.00583715 ) ) ) ) );

}


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
fn main( @location( 0 ) nodeVarying0 : vec2<f32>,
	@builtin( position ) fragCoord : vec4<f32> ) -> OutputStruct {

	// flow
	// code

	nodeVar0 = nodeVarying0;
	nodeVar1 = textureSample( nodeUniform0, nodeUniform0_sampler, nodeVar0 );
	let nodeConst0 = nodeVar1;
	nodeVar2 = vec4<f32>( 0.0, 0.0, 0.0, 1.0 );
	let nodeConst1 = ( ( vec2<f32>( 0.5, 0.5 ) - nodeVar0 ) / vec2<f32>( object.nodeUniform1 ) );
	nodeVar3 = object.nodeUniform2;
	nodeVar0 = ( nodeVar0 + ( nodeConst1 * vec2<f32>( interleavedGradientNoise( fragCoord.xy ) ) ) );

	for ( var i : i32 = 0; i < i32( object.nodeUniform1 ); i ++ ) {

		nodeVar0 = ( nodeVar0 + nodeConst1 );
		nodeVar4 = textureSample( nodeUniform0, nodeUniform0_sampler, nodeVar0 );
		nodeVar2 = ( nodeVar2 + ( nodeVar4 * vec4<f32>( nodeVar3 ) ) );
		nodeVar3 = ( nodeVar3 * object.nodeUniform3 );

	}

	nodeVar2 = ( nodeVar2 / vec4<f32>( object.nodeUniform1 ) );
	nodeVar2 = ( nodeVar2 * vec4<f32>( object.nodeUniform4 ) );
	nodeVar5 = mix( nodeVar2, ( nodeConst0 * vec4<f32>( 2.0 ) ), 0.5 );
	nodeVar6 = fn1( vec4<f32>( nodeVar5.xyz, clamp( nodeVar5.w, 0.0, 1.0 ) ) );
	nodeVar7 = vec4<f32>( neutralToneMapping( nodeVar6.xyz, render.nodeUniform5 ), nodeVar6.w );

	// result

	output.color = fn0( vec4<f32>( sRGBTransferOETF( nodeVar7.xyz ), nodeVar7.w ) );

	return output;

}
