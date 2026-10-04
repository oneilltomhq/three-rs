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
@binding( 2 ) @group( 0 ) var nodeUniform3_sampler : sampler;
@binding( 3 ) @group( 0 ) var nodeUniform3 : texture_2d<f32>;
@binding( 4 ) @group( 0 ) var nodeUniform11_sampler : sampler;
@binding( 5 ) @group( 0 ) var nodeUniform11 : texture_2d<f32>;
@binding( 6 ) @group( 0 ) var nodeUniform12_sampler : sampler;
@binding( 7 ) @group( 0 ) var nodeUniform12 : texture_2d<f32>;

struct objectStruct {
	nodeUniform1 : mat4x4<f32>,
	nodeUniform2 : mat4x4<f32>,
	nodeUniform4 : f32,
	nodeUniform5 : f32,
	nodeUniform6 : vec2<f32>,
	nodeUniform7 : mat4x4<f32>,
	nodeUniform8 : f32,
	nodeUniform9 : f32,
	nodeUniform10 : f32,
	nodeUniform13 : f32,
	nodeUniform14 : f32
};
@binding( 1 ) @group( 0 )
var<uniform> object : objectStruct;

// vars
var<private> nodeVar0 : vec2<f32>;
var<private> nodeVar1 : f32;
var<private> nodeVar2 : vec2<u32>;
var<private> nodeVar3 : f32;
var<private> nodeVar4 : vec3<f32>;
var<private> nodeVar5 : vec3<f32>;
var<private> nodeVar6 : vec4<f32>;
var<private> nodeVar7 : vec3<f32>;
var<private> nodeVar8 : vec3<f32>;
var<private> nodeVar9 : vec3<f32>;
var<private> nodeVar10 : f32;
var<private> nodeVar11 : vec3<f32>;
var<private> nodeVar12 : vec2<f32>;
var<private> nodeVar13 : vec2<f32>;
var<private> nodeVar14 : vec2<f32>;
var<private> nodeVar15 : f32;
var<private> nodeVar16 : f32;
var<private> nodeVar17 : f32;
var<private> nodeVar18 : f32;
var<private> nodeVar19 : vec2<f32>;
var<private> nodeVar20 : vec2<f32>;
var<private> nodeVar21 : vec2<f32>;
var<private> nodeVar22 : vec4<f32>;
var<private> nodeVar23 : f32;
var<private> nodeVar24 : bool;
var<private> nodeVar25 : vec2<f32>;
var<private> nodeVar26 : f32;
var<private> nodeVar27 : f32;
var<private> nodeVar28 : vec2<f32>;
var<private> nodeVar29 : vec2<f32>;
var<private> nodeVar30 : f32;
var<private> nodeVar31 : f32;
var<private> nodeVar32 : f32;
var<private> nodeVar33 : f32;
var<private> nodeVar34 : vec3<f32>;
var<private> nodeVar35 : f32;
var<private> nodeVar36 : vec2<f32>;
var<private> nodeVar37 : vec3<f32>;
var<private> nodeVar38 : f32;
var<private> nodeVar39 : f32;
var<private> nodeVar40 : vec4<f32>;
var<private> nodeVar41 : vec3<f32>;
var<private> nodeVar42 : f32;
var<private> nodeVar43 : f32;
var<private> nodeVar44 : vec3<f32>;
var<private> nodeVar45 : f32;
var<private> nodeVar46 : f32;
var<private> nodeVar47 : vec3<f32>;
var<private> nodeVar48 : f32;
var<private> nodeVar49 : vec4<f32>;
var<private> nodeVar50 : vec4<f32>;
var<private> nodeVar51 : vec3<f32>;
var<private> nodeVar52 : f32;
var<private> nodeVar53 : f32;
var<private> nodeVar54 : f32;
var<private> nodeVar55 : vec4<f32>;
var<private> nodeVar56 : f32;
var<private> nodeVar57 : vec3<f32>;
var<private> nodeVar58 : vec3<f32>;

// codes
fn tsl_clampWrapping_float( coord: f32 ) -> f32 { return clamp( coord, 0.0, 1.0 ); }
fn tsl_coord_clampS_clampT_2d( coord : vec2f ) -> vec2f {

	return vec2f(
		tsl_clampWrapping_float( coord.x ),
		tsl_clampWrapping_float( coord.y )
	);

}



@fragment
fn main( @location( 0 ) nodeVarying0 : vec2<f32> ) -> OutputStruct {

	// flow
	// code

	nodeVar0 = nodeVarying0;
	nodeVar2 = textureDimensions( nodeUniform0, u32( 0 ) );
	nodeVar1 = textureLoad( nodeUniform0, vec2<u32>( clamp( floor( tsl_coord_clampS_clampT_2d( nodeVar0 ) * vec2<f32>( nodeVar2 ) ), vec2<f32>( 0 ), vec2<f32>( nodeVar2 - vec2<u32>( 1, 1 ) ) ) ), u32( 0 ) );
	nodeVar3 = nodeVar1;

	if ( ( nodeVar3 >= 1.0 ) ) {

		discard;
		

	}

	let nodeConst0 = ( object.nodeUniform1 * vec4<f32>( vec3<f32>( ( ( vec2<f32>( nodeVar0.x, ( 1.0 - nodeVar0.y ) ) * vec2<f32>( 2.0 ) ) - vec2<f32>( 1.0 ) ), nodeVar3 ), 1.0 ) );
	nodeVar4 = ( nodeConst0.xyz / vec3<f32>( nodeConst0.w ) );
	nodeVar5 = ( object.nodeUniform2 * vec4<f32>( nodeVar4, 1.0 ) ).xyz;
	nodeVar6 = textureSample( nodeUniform3, nodeUniform3_sampler, nodeVarying0 );
	nodeVar7 = normalize( ( ( nodeVar6.xyz * vec3<f32>( 2.0 ) ) - vec3<f32>( 1.0 ) ) );
	nodeVar8 = normalize( nodeVar4 );
	nodeVar9 = normalize( reflect( nodeVar8, nodeVar7 ) );
	nodeVar10 = ( object.nodeUniform4 / dot( ( - nodeVar8 ), nodeVar7 ) );
	nodeVar11 = ( nodeVar4 + ( nodeVar9 * vec3<f32>( nodeVar10 ) ) );

	if ( ( nodeVar11.z > ( - object.nodeUniform5 ) ) ) {

		nodeVar11 = ( nodeVar4 + ( nodeVar9 * vec3<f32>( ( ( ( - object.nodeUniform5 ) - nodeVar4.z ) / nodeVar9.z ) ) ) );
		

	}

	nodeVar12 = ( nodeVar0 * object.nodeUniform6 );
	let nodeConst1 = ( object.nodeUniform7 * vec4<f32>( nodeVar11, 1.0 ) );
	nodeVar13 = ( ( ( nodeConst1.xy / vec2<f32>( nodeConst1.w ) ) * vec2<f32>( 0.5 ) ) + vec2<f32>( 0.5 ) );
	nodeVar14 = ( vec2<f32>( nodeVar13.x, ( 1.0 - nodeVar13.y ) ) * object.nodeUniform6 );
	nodeVar15 = ( nodeVar14.x - nodeVar12.x );
	nodeVar16 = ( nodeVar14.y - nodeVar12.y );
	let nodeConst2 = max( i32( trunc( ( max( abs( nodeVar15 ), abs( nodeVar16 ) ) * clamp( object.nodeUniform8, 0.0, 1.0 ) ) ) ), 1 );
	let nodeConst3 = nodeConst2;
	nodeVar17 = ( nodeVar15 / f32( nodeConst3 ) );
	nodeVar18 = ( nodeVar16 / f32( nodeConst3 ) );
	nodeVar19 = vec2<f32>( nodeVar17, nodeVar18 );
	nodeVar20 = ( vec2<f32>( 1.0, 1.0 ) / object.nodeUniform6 );
	nodeVar21 = vec2<f32>( nodeVar20.x, 0.0 );
	nodeVar22 = vec4<f32>( 0.0, 0.0, 0.0, 0.0 );
	nodeVar23 = 0.0;
	let nodeConst4 = ( 1.0 / nodeVar4.z );
	let nodeConst5 = ( 1.0 / nodeVar11.z );
	nodeVar24 = false;
	nodeVar25 = vec2<f32>( 0.0, 0.0 );
	nodeVar26 = 0.0;

	for ( var i : i32 = 1; i < nodeConst3; i ++ ) {

		nodeVar27 = ( f32( i ) / f32( nodeConst3 ) );
		nodeVar28 = ( nodeVar12 + ( nodeVar19 * vec2<f32>( ( nodeVar27 * f32( nodeConst3 ) ) ) ) );

		if ( ( ( ( ( nodeVar28.x < 0.0 ) || ( nodeVar28.x > object.nodeUniform6.x ) ) || ( nodeVar28.y < 0.0 ) ) || ( nodeVar28.y > object.nodeUniform6.y ) ) ) {

			break;
			

		}

		nodeVar29 = ( nodeVar28 * nodeVar20 );
		nodeVar30 = textureLoad( nodeUniform0, vec2<u32>( clamp( floor( tsl_coord_clampS_clampT_2d( nodeVar29 ) * vec2<f32>( nodeVar2 ) ), vec2<f32>( 0 ), vec2<f32>( nodeVar2 - vec2<u32>( 1, 1 ) ) ) ), u32( 0 ) );
		nodeVar31 = nodeVar30;
		nodeVar32 = ( ( object.nodeUniform5 * object.nodeUniform9 ) / ( ( ( object.nodeUniform9 - object.nodeUniform5 ) * nodeVar31 ) - object.nodeUniform9 ) );
		nodeVar33 = ( 1.0 / ( nodeConst4 + ( nodeVar27 * ( nodeConst5 - nodeConst4 ) ) ) );

		if ( ( nodeVar33 <= nodeVar32 ) ) {

			let nodeConst6 = ( object.nodeUniform1 * vec4<f32>( vec3<f32>( ( ( vec2<f32>( nodeVar29.x, ( 1.0 - nodeVar29.y ) ) * vec2<f32>( 2.0 ) ) - vec2<f32>( 1.0 ) ), nodeVar31 ), 1.0 ) );
			nodeVar34 = ( nodeConst6.xyz / vec3<f32>( nodeConst6.w ) );
			nodeVar35 = ( length( cross( ( nodeVar34 - nodeVar4 ), ( nodeVar34 - nodeVar11 ) ) ) / length( ( nodeVar11 - nodeVar4 ) ) );
			nodeVar36 = ( nodeVar29 + nodeVar21 );
			let nodeConst7 = ( object.nodeUniform1 * vec4<f32>( vec3<f32>( ( ( vec2<f32>( nodeVar36.x, ( 1.0 - nodeVar36.y ) ) * vec2<f32>( 2.0 ) ) - vec2<f32>( 1.0 ) ), nodeVar31 ), 1.0 ) );
			nodeVar37 = ( nodeConst7.xyz / vec3<f32>( nodeConst7.w ) );
			nodeVar38 = ( ( nodeVar37.x - nodeVar34.x ) * 3.0 );
			nodeVar39 = max( nodeVar38, object.nodeUniform10 );

			if ( ( nodeVar35 <= nodeVar39 ) ) {

				nodeVar40 = textureSample( nodeUniform3, nodeUniform3_sampler, nodeVar29 );
				nodeVar41 = normalize( ( ( nodeVar40.xyz * vec3<f32>( 2.0 ) ) - vec3<f32>( 1.0 ) ) );

				if ( ( dot( nodeVar9, nodeVar41 ) >= 0.0 ) ) {

					continue;
					

				}

				nodeVar42 = ( - ( ( ( nodeVar7.x * nodeVar4.x ) + ( nodeVar7.y * nodeVar4.y ) ) + ( nodeVar7.z * nodeVar4.z ) ) );
				nodeVar43 = ( ( ( ( nodeVar7.x * nodeVar34.x ) + ( nodeVar7.y * nodeVar34.y ) ) + ( nodeVar7.z * nodeVar34.z ) ) + nodeVar42 );

				if ( ( nodeVar43 > object.nodeUniform4 ) ) {

					break;
					

				}

				nodeVar24 = true;
				nodeVar25 = nodeVar29;
				nodeVar26 = nodeVar31;
				break;
				

			}

			

		}


	}


	if ( nodeVar24 ) {

		let nodeConst8 = ( object.nodeUniform1 * vec4<f32>( vec3<f32>( ( ( vec2<f32>( nodeVar25.x, ( 1.0 - nodeVar25.y ) ) * vec2<f32>( 2.0 ) ) - vec2<f32>( 1.0 ) ), nodeVar26 ), 1.0 ) );
		nodeVar44 = ( nodeConst8.xyz / vec3<f32>( nodeConst8.w ) );
		nodeVar45 = ( - ( ( ( nodeVar7.x * nodeVar4.x ) + ( nodeVar7.y * nodeVar4.y ) ) + ( nodeVar7.z * nodeVar4.z ) ) );
		nodeVar46 = ( ( ( ( nodeVar7.x * nodeVar44.x ) + ( nodeVar7.y * nodeVar44.y ) ) + ( nodeVar7.z * nodeVar44.z ) ) + nodeVar45 );

		if ( ( nodeVar46 <= object.nodeUniform4 ) ) {

			nodeVar47 = ( object.nodeUniform2 * vec4<f32>( nodeVar44, 1.0 ) ).xyz;
			nodeVar48 = ( distance( nodeVar5, nodeVar47 ) * 1.0 );
			nodeVar49 = textureSample( nodeUniform11, nodeUniform11_sampler, nodeVar25 );
			nodeVar50 = nodeVar49;
			nodeVar51 = nodeVar50.xyz;
			nodeVar50.x = nodeVar51[ 0 ];
			nodeVar50.y = nodeVar51[ 1 ];
			nodeVar50.z = nodeVar51[ 2 ];
			nodeVar52 = ( 1.0 - ( nodeVar46 / object.nodeUniform4 ) );
			nodeVar53 = ( nodeVar52 * nodeVar52 );
			nodeVar54 = ( ( dot( nodeVar8, nodeVar9 ) + 1.0 ) / 2.0 );
			nodeVar23 = 1.0;
			nodeVar55 = textureSample( nodeUniform12, nodeUniform12_sampler, nodeVarying0 );
			nodeVar22 = vec4<f32>( ( ( nodeVar50.xyz * vec3<f32>( nodeVar55.w ) ) * vec3<f32>( ( nodeVar53 * nodeVar54 ) ) ), nodeVar48 );
			

		}

		

	}


	if ( ( nodeVar23 == 0.0 ) ) {

		

	}

	nodeVar56 = max( dot( nodeVar22.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) ), 0.0001 );
	nodeVar57 = ( nodeVar22.xyz * vec3<f32>( min( ( object.nodeUniform13 / nodeVar56 ), 1.0 ) ) );
	nodeVar22.x = nodeVar57[ 0 ];
	nodeVar22.y = nodeVar57[ 1 ];
	nodeVar22.z = nodeVar57[ 2 ];
	nodeVar58 = ( nodeVar22.xyz * vec3<f32>( object.nodeUniform14 ) );
	nodeVar22.x = nodeVar58[ 0 ];
	nodeVar22.y = nodeVar58[ 1 ];
	nodeVar22.z = nodeVar58[ 2 ];

	// result

	output.color = max( nodeVar22, vec4<f32>( 0.0 ) );

	return output;

}
