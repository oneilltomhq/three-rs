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
@binding( 0 ) @group( 0 ) var nodeUniform0 : texture_depth_2d;
@binding( 1 ) @group( 0 ) var nodeUniform1_sampler : sampler;
@binding( 2 ) @group( 0 ) var nodeUniform1 : texture_2d<f32>;
@binding( 3 ) @group( 0 ) var nodeUniform2_sampler : sampler;
@binding( 4 ) @group( 0 ) var nodeUniform2 : texture_2d<f32>;
@binding( 6 ) @group( 0 ) var nodeUniform4 : texture_2d<f32>;

struct NodeBuffer_911Struct {
	value : array< vec4<f32>, 16 >
};
@binding( 7 ) @group( 0 )
var<uniform> NodeBuffer_911 : NodeBuffer_911Struct;

struct objectStruct {
	nodeUniform3 : mat4x4<f32>,
	nodeUniform5 : mat3x3<f32>,
	nodeUniform6 : vec2<f32>,
	nodeUniform7 : f32,
	nodeUniform9 : f32,
	nodeUniform10 : f32,
	nodeUniform11 : f32,
	nodeUniform12 : f32
};
@binding( 5 ) @group( 0 )
var<uniform> object : objectStruct;

// vars
var<private> nodeVar0 : f32;
var<private> nodeVar1 : vec2<u32>;
var<private> nodeVar2 : f32;
var<private> nodeVar3 : vec4<f32>;
var<private> nodeVar4 : vec3<f32>;
var<private> nodeVar5 : vec4<f32>;
var<private> nodeVar6 : vec4<f32>;
var<private> nodeVar7 : vec4<f32>;
var<private> nodeVar8 : vec4<f32>;
var<private> nodeVar9 : vec2<u32>;
var<private> nodeVar10 : vec4<f32>;
var<private> nodeVar11 : f32;
var<private> nodeVar12 : vec3<f32>;
var<private> nodeVar13 : vec4<f32>;
var<private> nodeVar14 : vec4<f32>;
var<private> nodeVar15 : f32;
var<private> nodeVar16 : f32;
var<private> nodeVar17 : vec4<f32>;
var<private> nodeVar18 : vec3<f32>;
var<private> nodeVar19 : vec3<f32>;
var<private> nodeVar20 : f32;
var<private> nodeVar21 : f32;
var<private> nodeVar22 : f32;
var<private> nodeVar23 : f32;
var<private> nodeVar24 : f32;

// codes
fn tsl_clampWrapping_float( coord: f32 ) -> f32 { return clamp( coord, 0.0, 1.0 ); }
fn tsl_coord_clampS_clampT_2d( coord : vec2f ) -> vec2f {

	return vec2f(
		tsl_clampWrapping_float( coord.x ),
		tsl_clampWrapping_float( coord.y )
	);

}

fn tsl_repeatWrapping_float( coord: f32 ) -> f32 { return fract( coord ); }
fn tsl_coord_repeatS_repeatT_2d( coord : vec2f ) -> vec2f {

	return vec2f(
		tsl_repeatWrapping_float( coord.x ),
		tsl_repeatWrapping_float( coord.y )
	);

}

fn tsl_mod_float( x : f32, y : f32 ) -> f32 { return x - y * floor( x / y ); }


@fragment
fn main( @location( 0 ) nodeVarying0 : vec2<f32> ) -> OutputStruct {

	// flow
	// code

	nodeVar1 = textureDimensions( nodeUniform0, u32( 0 ) );
	nodeVar0 = textureLoad( nodeUniform0, vec2<u32>( clamp( floor( tsl_coord_clampS_clampT_2d( nodeVarying0 ) * vec2<f32>( nodeVar1 ) ), vec2<f32>( 0 ), vec2<f32>( nodeVar1 - vec2<u32>( 1, 1 ) ) ) ), u32( 0 ) );
	nodeVar2 = nodeVar0;
	nodeVar3 = textureSample( nodeUniform1, nodeUniform1_sampler, nodeVarying0 );
	nodeVar4 = normalize( nodeVar3.xyz );
	nodeVar5 = textureSample( nodeUniform2, nodeUniform2_sampler, nodeVarying0 );
	nodeVar6 = nodeVar5;

	if ( ( ( nodeVar2 >= 1.0 ) || ( dot( nodeVar4, nodeVar4 ) == 0.0 ) ) ) {

		nodeVar7 = nodeVar6;
		

	} else {

		let nodeConst0 = ( object.nodeUniform3 * vec4<f32>( vec3<f32>( ( ( vec2<f32>( nodeVarying0.x, ( 1.0 - nodeVarying0.y ) ) * vec2<f32>( 2.0 ) ) - vec2<f32>( 1.0 ) ), nodeVar2 ), 1.0 ) );
		let nodeConst1 = ( nodeConst0.xyz / vec3<f32>( nodeConst0.w ) );
		nodeVar9 = textureDimensions( nodeUniform4, u32( 0 ) );
		nodeVar8 = textureLoad( nodeUniform4, vec2<u32>( clamp( floor( tsl_coord_repeatS_repeatT_2d( ( object.nodeUniform5 * vec3<f32>( ( vec2<f32>( nodeVarying0.x, ( 1.0 - nodeVarying0.y ) ) * ( object.nodeUniform6 / vec2<f32>( textureDimensions( nodeUniform4, 0 ) ) ) ), 1.0 ) ).xy ) * vec2<f32>( nodeVar9 ) ), vec2<f32>( 0 ), vec2<f32>( nodeVar9 - vec2<u32>( 1, 1 ) ) ) ), u32( 0 ) );
		nodeVar10 = nodeVar8;
		nodeVar11 = 1.0;
		nodeVar12 = nodeVar6.xyz;

		for ( var i : i32 = 0; i < 16; i ++ ) {

			let nodeConst2 = vec2<f32>( sin( nodeVar10[ u32( ( ( tsl_mod_float( object.nodeUniform7, 4.0 ) * 2.0 ) * 3.141592653589793 ) ) ] ), cos( nodeVar10[ u32( ( ( tsl_mod_float( object.nodeUniform7, 4.0 ) * 2.0 ) * 3.141592653589793 ) ) ] ) );
			let nodeConst3 = ( nodeVarying0 + ( ( mat2x2<f32>( nodeConst2.x, ( - nodeConst2.y ), nodeConst2.x, nodeConst2.y ) * ( NodeBuffer_911.value[ i ].xyz.xy * vec2<f32>( ( 1.0 + ( NodeBuffer_911.value[ i ].xyz.z * ( object.nodeUniform9 - 1.0 ) ) ) ) ) ) / object.nodeUniform6 ) );
			nodeVar13 = textureSample( nodeUniform2, nodeUniform2_sampler, nodeConst3 );
			nodeVar14 = nodeVar13;
			nodeVar15 = textureLoad( nodeUniform0, vec2<u32>( clamp( floor( tsl_coord_clampS_clampT_2d( nodeConst3 ) * vec2<f32>( nodeVar1 ) ), vec2<f32>( 0 ), vec2<f32>( nodeVar1 - vec2<u32>( 1, 1 ) ) ) ), u32( 0 ) );
			nodeVar16 = nodeVar15;
			nodeVar17 = textureSample( nodeUniform1, nodeUniform1_sampler, nodeConst3 );
			nodeVar18 = normalize( nodeVar17.xyz );
			let nodeConst4 = ( object.nodeUniform3 * vec4<f32>( vec3<f32>( ( ( vec2<f32>( nodeConst3.x, ( 1.0 - nodeConst3.y ) ) * vec2<f32>( 2.0 ) ) - vec2<f32>( 1.0 ) ), nodeVar16 ), 1.0 ) );
			nodeVar19 = ( nodeConst4.xyz / vec3<f32>( nodeConst4.w ) );
			nodeVar20 = dot( nodeVar4, nodeVar18 );
			nodeVar21 = pow( max( nodeVar20, 0.0 ), object.nodeUniform10 );
			nodeVar22 = abs( ( dot( nodeVar14.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) ) - dot( nodeVar6.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) ) ) );
			nodeVar23 = max( ( 1.0 - ( nodeVar22 / object.nodeUniform11 ) ), 0.0 );
			nodeVar24 = abs( dot( ( nodeConst1 - nodeVar19 ), nodeVar4 ) );
			let nodeConst5 = ( ( nodeVar23 * max( ( 1.0 - ( nodeVar24 / object.nodeUniform12 ) ), 0.0 ) ) * nodeVar21 );
			let nodeConst6 = vec4<f32>( ( nodeVar14.xyz * vec3<f32>( nodeConst5 ) ), nodeConst5 );
			nodeVar12 = ( nodeVar12 + nodeConst6.xyz );
			nodeVar11 = ( nodeVar11 + nodeConst6.w );

		}


		if ( ( nodeVar11 > 0.0 ) ) {

			nodeVar12 = ( nodeVar12 / vec3<f32>( nodeVar11 ) );
			

		}

		nodeVar7 = vec4<f32>( nodeVar12, nodeVar6.w );
		

	}


	// result

	output.color = nodeVar7;

	return output;

}
