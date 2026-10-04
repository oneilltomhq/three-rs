// Three.js r187dev - Node System

// global
diagnostic( off, derivative_uniformity );


// directives


// structs

struct OutputStruct {
	@location( 0 ) color: f32
};
var<private> output : OutputStruct;

// uniforms
@binding( 1 ) @group( 0 ) var nodeUniform1_sampler : sampler;
@binding( 2 ) @group( 0 ) var nodeUniform1 : texture_depth_2d;
@binding( 3 ) @group( 0 ) var nodeUniform3_sampler : sampler;
@binding( 4 ) @group( 0 ) var nodeUniform3 : texture_2d<f32>;
@binding( 5 ) @group( 0 ) var nodeUniform7 : texture_2d<f32>;

struct objectStruct {
	nodeUniform0 : f32,
	nodeUniform2 : mat4x4<f32>,
	nodeUniform4 : f32,
	nodeUniform5 : mat4x4<f32>,
	nodeUniform6 : f32,
	nodeUniform8 : mat3x3<f32>,
	nodeUniform9 : vec2<f32>,
	nodeUniform10 : f32,
	nodeUniform11 : f32,
	nodeUniform12 : f32
};
@binding( 0 ) @group( 0 )
var<uniform> object : objectStruct;

// vars
var<private> nodeVar0 : f32;
var<private> nodeVar1 : vec4<f32>;
var<private> nodeVar2 : f32;
var<private> nodeVar3 : vec2<u32>;
var<private> nodeVar4 : vec4<f32>;
var<private> nodeVar5 : f32;
var<private> nodeVar6 : vec4<f32>;
var<private> nodeVar7 : vec2<u32>;
var<private> nodeVar8 : f32;
var<private> nodeVar9 : vec2<f32>;
var<private> nodeVar10 : f32;
var<private> nodeVar11 : vec2<u32>;
var<private> nodeVar12 : f32;

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

fn interleavedGradientNoise ( position : vec2<f32> ) -> f32 {

	


	return fract( ( 52.9829189 * fract( dot( position, vec2<f32>( 0.06711056, 0.00583715 ) ) ) ) );

}


fn tsl_mod_float( x : f32, y : f32 ) -> f32 { return x - y * floor( x / y ); }
fn getScreenPositionFromClip ( clipPosition : vec4<f32> ) -> vec2<f32> {

	var nodeVar0 : vec2<f32>;

	nodeVar0 = ( ( ( clipPosition.xy / vec2<f32>( clipPosition.w ) ) * vec2<f32>( 0.5 ) ) + vec2<f32>( 0.5 ) );

	return vec2<f32>( nodeVar0.x, ( 1.0 - nodeVar0.y ) );

}




@fragment
fn main( @location( 0 ) nodeVarying0 : vec2<f32>,
	@builtin( position ) fragCoord : vec4<f32> ) -> OutputStruct {

	// flow
	// code


	if ( ( object.nodeUniform0 < 1.0 ) ) {

		nodeVar1 = vec4<f32>( textureGather( nodeUniform1, nodeUniform1_sampler, nodeVarying0) );
		let nodeVar1 = nodeVar1;
		nodeVar0 = min( min( nodeVar1.x, nodeVar1.y ), min( nodeVar1.z, nodeVar1.w ) );

	} else {

		nodeVar3 = textureDimensions( nodeUniform1, u32( 0 ) );
		nodeVar2 = textureLoad( nodeUniform1, vec2<u32>( clamp( floor( tsl_coord_clampS_clampT_2d( nodeVarying0 ) * vec2<f32>( nodeVar3 ) ), vec2<f32>( 0 ), vec2<f32>( nodeVar3 - vec2<u32>( 1, 1 ) ) ) ), u32( 0 ) );
		nodeVar0 = nodeVar2;

	}

	let nodeConst0 = nodeVar0;

	if ( ( nodeConst0 >= 1.0 ) ) {

		discard;
		

	}

	let nodeConst1 = ( object.nodeUniform2 * vec4<f32>( vec3<f32>( ( ( vec2<f32>( nodeVarying0.x, ( 1.0 - nodeVarying0.y ) ) * vec2<f32>( 2.0 ) ) - vec2<f32>( 1.0 ) ), nodeConst0 ), 1.0 ) );
	let nodeConst2 = ( nodeConst1.xyz / vec3<f32>( nodeConst1.w ) );
	nodeVar4 = textureSample( nodeUniform3, nodeUniform3_sampler, nodeVarying0 );
	let nodeConst3 = normalize( ( ( nodeVar4 * vec4<f32>( 2.0 ) ) - vec4<f32>( 1.0 ) ).xyz );
	let nodeConst4 = ( 1.0 / object.nodeUniform4 );
	let nodeConst5 = normalize( ( - nodeConst2 ) );
	let nodeConst6 = ( object.nodeUniform5 * vec4<f32>( nodeConst2, 1.0 ) );
	nodeVar5 = 0.0;

	for ( var i : i32 = 0; i < 3; i ++ ) {

		let nodeConst7 = ( ( ( f32( i ) / 3.0 ) * 3.141592653589793 ) + object.nodeUniform6 );
		nodeVar7 = textureDimensions( nodeUniform7, u32( 0 ) );
		nodeVar6 = textureLoad( nodeUniform7, vec2<u32>( clamp( floor( tsl_coord_repeatS_repeatT_2d( ( object.nodeUniform8 * vec3<f32>( ( vec2<f32>( nodeVarying0.x, ( 1.0 - nodeVarying0.y ) ) * ( object.nodeUniform9 / vec2<f32>( textureDimensions( nodeUniform7, 0 ) ) ) ), 1.0 ) ).xy ) * vec2<f32>( nodeVar7 ) ), vec2<f32>( 0 ), vec2<f32>( nodeVar7 - vec2<u32>( 1, 1 ) ) ) ), u32( 0 ) );
		let nodeConst8 = normalize( vec3<f32>( ( ( nodeVar6.xyz * vec3<f32>( 2.0 ) ) - vec3<f32>( 1.0 ) ).xy, 0.0 ) );
		let nodeConst9 = ( mat3x3<f32>( nodeConst8, vec3<f32>( ( nodeConst8.y * -1.0 ), nodeConst8.x, 0.0 ), vec3<f32>( 0.0, 0.0, 1.0 ) ) * vec3<f32>( cos( nodeConst7 ), sin( nodeConst7 ), 0.0 ) );
		let nodeConst10 = ( ( object.nodeUniform5 * vec4<f32>( nodeConst9, 0.0 ) ) * vec4<f32>( object.nodeUniform4 ) );
		let nodeConst11 = normalize( cross( nodeConst9, nodeConst5 ) );
		let nodeConst12 = cross( nodeConst11, nodeConst5 );
		let nodeConst13 = ( nodeConst3 - ( nodeConst11 * vec3<f32>( dot( nodeConst3, nodeConst11 ) ) ) );
		let nodeConst14 = length( nodeConst13 );
		let nodeConst15 = ( nodeConst13 / vec3<f32>( max( nodeConst14, 0.0001 ) ) );
		let nodeConst16 = dot( nodeConst15, nodeConst12 );
		let nodeConst17 = clamp( dot( nodeConst15, nodeConst5 ), 0.0, 1.0 );

		if ( ( nodeConst16 >= 0.0 ) ) {

			nodeVar8 = 1.0;

		} else {

			nodeVar8 = -1.0;

		}

		let nodeConst18 = ( nodeVar8 * acos( nodeConst17 ) );
		let nodeConst19 = cross( nodeConst15, nodeConst11 );
		let nodeConst20 = dot( nodeConst5, nodeConst19 );
		nodeVar9 = vec2<f32>( nodeConst20, ( - nodeConst20 ) );

		for ( var j : i32 = 0; j < 6; j ++ ) {

			let nodeConst21 = ( ( ( f32( j ) + 1.0 ) + ( interleavedGradientNoise( ( fragCoord.xy + vec2<f32>( object.nodeUniform10 ) ) ) + fract( ( sin( tsl_mod_float( dot( ( ( ( nodeVarying0 + vec2<f32>( ( object.nodeUniform6 * 0.02 ) ) ) * vec2<f32>( 2.0 ) ) - vec2<f32>( 1.0 ) ), vec2<f32>( 12.9898, 78.233 ) ), 3.141592653589793 ) ) * 43758.5453 ) ) ) ) * 0.16666666666666666 );
			let nodeConst22 = ( nodeConst10 * vec4<f32>( ( nodeConst21 * nodeConst21 ) ) );
			let nodeConst23 = getScreenPositionFromClip( ( nodeConst6 + nodeConst22 ) );
			nodeVar11 = textureDimensions( nodeUniform1, u32( 0 ) );
			nodeVar10 = textureLoad( nodeUniform1, vec2<u32>( clamp( floor( tsl_coord_clampS_clampT_2d( nodeConst23 ) * vec2<f32>( nodeVar11 ) ), vec2<f32>( 0 ), vec2<f32>( nodeVar11 - vec2<u32>( 1, 1 ) ) ) ), u32( 0 ) );
			let nodeConst24 = nodeVar10;
			let nodeConst25 = ( object.nodeUniform2 * vec4<f32>( vec3<f32>( ( ( vec2<f32>( nodeConst23.x, ( 1.0 - nodeConst23.y ) ) * vec2<f32>( 2.0 ) ) - vec2<f32>( 1.0 ) ), nodeConst24 ), 1.0 ) );
			let nodeConst26 = ( nodeConst25.xyz / vec3<f32>( nodeConst25.w ) );
			let nodeConst27 = ( nodeConst26 - nodeConst2 );
			let nodeConst28 = length( nodeConst27 );

			if ( ( abs( nodeConst27.z ) < object.nodeUniform11 ) ) {

				let nodeConst29 = min( ( nodeConst28 * nodeConst4 ), 1.0 );
				nodeVar9.x = mix( max( nodeVar9.x, ( dot( nodeConst5, nodeConst27 ) / max( nodeConst28, 0.0001 ) ) ), nodeVar9.x, ( nodeConst29 * nodeConst29 ) );
				

			}

			let nodeConst30 = getScreenPositionFromClip( ( nodeConst6 - nodeConst22 ) );
			nodeVar12 = textureLoad( nodeUniform1, vec2<u32>( clamp( floor( tsl_coord_clampS_clampT_2d( nodeConst30 ) * vec2<f32>( nodeVar11 ) ), vec2<f32>( 0 ), vec2<f32>( nodeVar11 - vec2<u32>( 1, 1 ) ) ) ), u32( 0 ) );
			let nodeConst31 = nodeVar12;
			let nodeConst32 = ( object.nodeUniform2 * vec4<f32>( vec3<f32>( ( ( vec2<f32>( nodeConst30.x, ( 1.0 - nodeConst30.y ) ) * vec2<f32>( 2.0 ) ) - vec2<f32>( 1.0 ) ), nodeConst31 ), 1.0 ) );
			let nodeConst33 = ( nodeConst32.xyz / vec3<f32>( nodeConst32.w ) );
			let nodeConst34 = ( nodeConst33 - nodeConst2 );
			let nodeConst35 = length( nodeConst34 );

			if ( ( abs( nodeConst34.z ) < object.nodeUniform11 ) ) {

				let nodeConst36 = min( ( nodeConst35 * nodeConst4 ), 1.0 );
				nodeVar9.y = mix( max( nodeVar9.y, ( dot( nodeConst5, nodeConst34 ) / max( nodeConst35, 0.0001 ) ) ), nodeVar9.y, ( nodeConst36 * nodeConst36 ) );
				

			}


		}

		let nodeConst37 = acos( nodeVar9.y );
		let nodeConst38 = ( - acos( nodeVar9.x ) );
		nodeVar5 = ( nodeVar5 + ( nodeConst14 * ( ( ( ( ( - cos( ( ( nodeConst37 * 2.0 ) - nodeConst18 ) ) ) + nodeConst17 ) + ( ( nodeConst37 * 2.0 ) * nodeConst16 ) ) + ( ( ( - cos( ( ( nodeConst38 * 2.0 ) - nodeConst18 ) ) ) + nodeConst17 ) + ( ( nodeConst38 * 2.0 ) * nodeConst16 ) ) ) * 0.25 ) ) );

	}

	nodeVar5 = clamp( ( nodeVar5 / 3.0 ), 0.0, 1.0 );
	nodeVar5 = pow( nodeVar5, object.nodeUniform12 );

	// result

	output.color = nodeVar5;

	return output;

}
