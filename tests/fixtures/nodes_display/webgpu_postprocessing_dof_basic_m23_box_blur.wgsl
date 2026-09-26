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
@binding( 3 ) @group( 1 ) var nodeUniform7 : texture_depth_2d;

struct objectStruct {
	nodeUniform1 : f32,
	nodeUniform2 : f32,
	nodeUniform3 : f32,
	nodeUniform4 : f32,
	nodeUniform5 : f32,
	nodeUniform6 : f32,
	nodeUniform8 : vec3<f32>
};
@binding( 2 ) @group( 1 )
var<uniform> object : objectStruct;

struct renderStruct {
	nodeUniform9 : f32
};
@binding( 0 ) @group( 0 )
var<uniform> render : renderStruct;

// vars
var<private> nodeVar0 : vec4<f32>;
var<private> nodeVar3 : vec4<f32>;
var<private> nodeVar4 : f32;
var<private> nodeVar5 : vec2<u32>;

// codes
fn tsl_clampWrapping_float( coord: f32 ) -> f32 { return clamp( coord, 0.0, 1.0 ); }
fn tsl_coord_clampS_clampT_2d( coord : vec2f ) -> vec2f {

	return vec2f(
		tsl_clampWrapping_float( coord.x ),
		tsl_clampWrapping_float( coord.y )
	);

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

	var nodeVar1 : f32;

	var nodeVar0 : vec3<f32> = ( color * vec3<f32>( exposure ) );
	let nodeConst0 = min( nodeVar0.x, min( nodeVar0.y, nodeVar0.z ) );

	if ( ( nodeConst0 < 0.08 ) ) {

		nodeVar1 = ( nodeConst0 - ( 6.25 * ( nodeConst0 * nodeConst0 ) ) );

	} else {

		nodeVar1 = 0.04;

	}

	nodeVar0 = ( nodeVar0 - vec3<f32>( nodeVar1 ) );
	let nodeConst1 = max( nodeVar0.x, max( nodeVar0.y, nodeVar0.z ) );

	if ( ( nodeConst1 < 0.76 ) ) {

		return nodeVar0;

	}

	let nodeConst2 = ( 1.0 - 0.76 );
	let nodeConst3 = ( 1.0 - ( ( nodeConst2 * nodeConst2 ) / ( nodeConst1 + ( nodeConst2 - 0.76 ) ) ) );
	nodeVar0 = ( nodeVar0 * vec3<f32>( ( nodeConst3 / nodeConst1 ) ) );

	return mix( nodeVar0, vec3<f32>( nodeConst3 ), ( 1.0 - ( 1.0 / ( ( 0.15 * ( nodeConst1 - nodeConst3 ) ) + 1.0 ) ) ) );

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
	var nodeVar1 : vec4<f32> = vec4<f32>( 0.0, 0.0, 0.0, 0.0 );
	var nodeVar2 : i32 = 0;

	for ( var i : i32 = i32( ( - object.nodeUniform1 ) ); i <= i32( object.nodeUniform1 ); i ++ ) {


		for ( var j : i32 = i32( ( - object.nodeUniform1 ) ); j <= i32( object.nodeUniform1 ); j ++ ) {

			nodeVar3 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 + ( ( vec2<f32>( f32( i ), f32( j ) ) * ( vec2<f32>( 1.0, 1.0 ) / vec2<f32>( textureDimensions( nodeUniform0, 0 ) ) ) ) * vec2<f32>( max( object.nodeUniform2, 1.0 ) ) ) ) );
			nodeVar1 = ( nodeVar1 + nodeVar3 );
			nodeVar2 = ( nodeVar2 + 1 );

		}


	}

	nodeVar1 = ( nodeVar1 / vec4<f32>( f32( nodeVar2 ) ) );
	nodeVar5 = textureDimensions( nodeUniform7, u32( 0 ) );
	nodeVar4 = textureLoad( nodeUniform7, vec2<u32>( clamp( floor( tsl_coord_clampS_clampT_2d( nodeVarying0 ) * vec2<f32>( nodeVar5 ) ), vec2<f32>( 0 ), vec2<f32>( nodeVar5 - vec2<u32>( 1, 1 ) ) ) ), u32( 0 ) );
	let nodeConst0 = mix( nodeVar0, nodeVar1, smoothstep( object.nodeUniform3, object.nodeUniform4, abs( ( ( ( object.nodeUniform5 * object.nodeUniform6 ) / ( ( ( object.nodeUniform6 - object.nodeUniform5 ) * nodeVar4 ) - object.nodeUniform6 ) ) - object.nodeUniform8.z ) ) ) );
	let nodeConst1 = fn1( vec4<f32>( nodeConst0.xyz, clamp( nodeConst0.w, 0.0, 1.0 ) ) );
	let nodeConst2 = vec4<f32>( neutralToneMapping( nodeConst1.xyz, render.nodeUniform9 ), nodeConst1.w );

	// result

	output.color = fn0( vec4<f32>( sRGBTransferOETF( nodeConst2.xyz ), nodeConst2.w ) );

	return output;

}
