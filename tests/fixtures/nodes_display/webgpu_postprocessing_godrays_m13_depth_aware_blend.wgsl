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
@binding( 3 ) @group( 0 ) var nodeUniform3 : texture_depth_2d;
@binding( 4 ) @group( 0 ) var nodeUniform6_sampler : sampler;
@binding( 5 ) @group( 0 ) var nodeUniform6 : texture_2d<f32>;

struct objectStruct {
	nodeUniform1 : f32,
	nodeUniform2 : f32,
	nodeUniform4 : f32,
	nodeUniform5 : vec3<f32>,
	nodeUniform7 : f32
};
@binding( 2 ) @group( 0 )
var<uniform> object : objectStruct;

// vars
var<private> nodeVar0 : vec2<f32>;
var<private> nodeVar1 : f32;
var<private> nodeVar2 : f32;
var<private> nodeVar3 : vec2<u32>;
var<private> nodeVar4 : f32;
var<private> nodeVar5 : f32;
var<private> nodeVar6 : vec4<f32>;
var<private> nodeVar7 : vec2<f32>;
var<private> nodeVar8 : vec4<f32>;

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

	nodeVar0 = vec2<f32>( 0.0, 0.0 );
	nodeVar1 = 0.0;
	let nodeConst0 = vec2<i32>( textureDimensions( nodeUniform0, 0 ) );

	for ( var i : i32 = 0; i < 8; i ++ ) {

		let nodeConst1 = ( array< vec2<f32>, 8 >( vec2<f32>( 0.493393, 0.394269 ), vec2<f32>( 0.798547, 0.885922 ), vec2<f32>( 0.259143, 0.650754 ), vec2<f32>( 0.605322, 0.023588 ), vec2<f32>( -0.574681, 0.137452 ), vec2<f32>( -0.430397, -0.638423 ), vec2<f32>( -0.849487, -0.366258 ), vec2<f32>( 0.170621, -0.569941 ) )[ i ] * vec2<f32>( object.nodeUniform4 ) );
		nodeVar3 = textureDimensions( nodeUniform3, u32( 0 ) );
		nodeVar2 = textureLoad( nodeUniform3, vec2<u32>( clamp( floor( tsl_coord_clampS_clampT_2d( ( nodeVarying0 + ( nodeConst1 * ( vec2<f32>( 1.0, 1.0 ) / vec2<f32>( nodeConst0 ) ) ) ) ) * vec2<f32>( nodeVar3 ) ), vec2<f32>( 0 ), vec2<f32>( nodeVar3 - vec2<u32>( 1, 1 ) ) ) ), u32( 0 ) );
		nodeVar4 = textureLoad( nodeUniform3, vec2<u32>( clamp( floor( tsl_coord_clampS_clampT_2d( nodeVarying0 ) * vec2<f32>( nodeVar3 ) ), vec2<f32>( 0 ), vec2<f32>( nodeVar3 - vec2<u32>( 1, 1 ) ) ) ), u32( 0 ) );
		let nodeVar4 = nodeVar4;
		let nodeConst2 = ( ( ( ( object.nodeUniform1 * object.nodeUniform2 ) / ( ( ( object.nodeUniform2 - object.nodeUniform1 ) * nodeVar4 ) - object.nodeUniform2 ) ) + object.nodeUniform1 ) / ( object.nodeUniform1 - object.nodeUniform2 ) );

		if ( ( abs( ( ( ( ( ( object.nodeUniform1 * object.nodeUniform2 ) / ( ( ( object.nodeUniform2 - object.nodeUniform1 ) * nodeVar2 ) - object.nodeUniform2 ) ) + object.nodeUniform1 ) / ( object.nodeUniform1 - object.nodeUniform2 ) ) - nodeConst2 ) ) < ( 0.05 * nodeConst2 ) ) ) {

			nodeVar0 = ( nodeVar0 + nodeConst1 );
			nodeVar1 = ( nodeVar1 + 1.0 );
			

		}


	}


	if ( ( nodeVar1 == 0.0 ) ) {

		nodeVar5 = 1.0;

	} else {

		nodeVar5 = nodeVar1;

	}

	nodeVar1 = nodeVar5;
	nodeVar0 = ( nodeVar0 / vec2<f32>( nodeVar1 ) );
	nodeVar6 = textureSample( nodeUniform0, nodeUniform0_sampler, nodeVarying0 );

	if ( ( length( nodeVar0 ) > 0.0 ) ) {

		nodeVar7 = ( nodeVarying0 + ( vec2<f32>( object.nodeUniform7 ) * ( nodeVar0 / vec2<f32>( nodeConst0 ) ) ) );

	} else {

		nodeVar7 = nodeVarying0;

	}

	nodeVar8 = textureSample( nodeUniform6, nodeUniform6_sampler, nodeVar7 );
	let nodeConst3 = mix( nodeVar6, vec4<f32>( object.nodeUniform5, 1.0 ), nodeVar8.x );
	let nodeConst4 = fn1( vec4<f32>( nodeConst3.xyz, clamp( nodeConst3.w, 0.0, 1.0 ) ) );

	// result

	output.color = fn0( vec4<f32>( sRGBTransferOETF( nodeConst4.xyz ), nodeConst4.w ) );

	return output;

}
