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
@binding( 4 ) @group( 0 ) var nodeUniform4_sampler : sampler;
@binding( 5 ) @group( 0 ) var nodeUniform4 : texture_2d<f32>;
@binding( 6 ) @group( 0 ) var nodeUniform12_sampler : sampler;
@binding( 7 ) @group( 0 ) var nodeUniform12 : texture_2d<f32>;

struct objectStruct {
	nodeUniform1 : mat4x4<f32>,
	nodeUniform2 : mat4x4<f32>,
	nodeUniform5 : f32,
	nodeUniform6 : f32,
	nodeUniform7 : vec2<f32>,
	nodeUniform8 : mat4x4<f32>,
	nodeUniform9 : f32,
	nodeUniform10 : f32,
	nodeUniform11 : f32,
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
var<private> nodeVar9 : vec4<f32>;
var<private> nodeVar10 : vec3<f32>;
var<private> nodeVar11 : f32;
var<private> nodeVar12 : vec3<f32>;
var<private> nodeVar13 : vec2<f32>;
var<private> nodeVar14 : vec2<f32>;
var<private> nodeVar15 : vec2<f32>;
var<private> nodeVar16 : f32;
var<private> nodeVar17 : f32;
var<private> nodeVar18 : f32;
var<private> nodeVar19 : f32;
var<private> nodeVar20 : vec2<f32>;
var<private> nodeVar21 : vec2<f32>;
var<private> nodeVar22 : vec2<f32>;
var<private> nodeVar23 : vec4<f32>;
var<private> nodeVar24 : f32;
var<private> nodeVar25 : bool;
var<private> nodeVar26 : vec2<f32>;
var<private> nodeVar27 : f32;
var<private> nodeVar28 : f32;
var<private> nodeVar29 : vec2<f32>;
var<private> nodeVar30 : vec2<f32>;
var<private> nodeVar31 : f32;
var<private> nodeVar32 : f32;
var<private> nodeVar33 : f32;
var<private> nodeVar34 : f32;
var<private> nodeVar35 : vec3<f32>;
var<private> nodeVar36 : f32;
var<private> nodeVar37 : vec2<f32>;
var<private> nodeVar38 : vec3<f32>;
var<private> nodeVar39 : f32;
var<private> nodeVar40 : f32;
var<private> nodeVar41 : vec4<f32>;
var<private> nodeVar42 : vec3<f32>;
var<private> nodeVar43 : f32;
var<private> nodeVar44 : f32;
var<private> nodeVar45 : vec3<f32>;
var<private> nodeVar46 : f32;
var<private> nodeVar47 : f32;
var<private> nodeVar48 : vec3<f32>;
var<private> nodeVar49 : f32;
var<private> nodeVar50 : vec4<f32>;
var<private> nodeVar51 : vec4<f32>;
var<private> nodeVar52 : vec3<f32>;
var<private> nodeVar53 : f32;
var<private> nodeVar54 : f32;
var<private> nodeVar55 : f32;
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
	nodeVar7 = normalize( ( ( nodeVar6 * vec4<f32>( 2.0 ) ) - vec4<f32>( 1.0 ) ).xyz );
	nodeVar8 = normalize( nodeVar4 );
	nodeVar9 = textureSample( nodeUniform4, nodeUniform4_sampler, nodeVarying0 );
	let nodeVar9 = nodeVar9;
	let nodeConst1 = nodeVar9.x;

	if ( ( nodeConst1 <= 0.0 ) ) {

		discard;
		

	}

	nodeVar10 = normalize( reflect( nodeVar8, nodeVar7 ) );
	nodeVar11 = ( object.nodeUniform5 / dot( ( - nodeVar8 ), nodeVar7 ) );
	nodeVar12 = ( nodeVar4 + ( nodeVar10 * vec3<f32>( nodeVar11 ) ) );

	if ( ( nodeVar12.z > ( - object.nodeUniform6 ) ) ) {

		nodeVar12 = ( nodeVar4 + ( nodeVar10 * vec3<f32>( ( ( ( - object.nodeUniform6 ) - nodeVar4.z ) / nodeVar10.z ) ) ) );
		

	}

	nodeVar13 = ( nodeVar0 * object.nodeUniform7 );
	let nodeConst2 = ( object.nodeUniform8 * vec4<f32>( nodeVar12, 1.0 ) );
	nodeVar14 = ( ( ( nodeConst2.xy / vec2<f32>( nodeConst2.w ) ) * vec2<f32>( 0.5 ) ) + vec2<f32>( 0.5 ) );
	nodeVar15 = ( vec2<f32>( nodeVar14.x, ( 1.0 - nodeVar14.y ) ) * object.nodeUniform7 );
	nodeVar16 = ( nodeVar15.x - nodeVar13.x );
	nodeVar17 = ( nodeVar15.y - nodeVar13.y );
	let nodeConst3 = max( i32( trunc( ( max( abs( nodeVar16 ), abs( nodeVar17 ) ) * clamp( object.nodeUniform9, 0.0, 1.0 ) ) ) ), 1 );
	let nodeConst4 = nodeConst3;
	nodeVar18 = ( nodeVar16 / f32( nodeConst4 ) );
	nodeVar19 = ( nodeVar17 / f32( nodeConst4 ) );
	nodeVar20 = vec2<f32>( nodeVar18, nodeVar19 );
	nodeVar21 = ( vec2<f32>( 1.0, 1.0 ) / object.nodeUniform7 );
	nodeVar22 = vec2<f32>( nodeVar21.x, 0.0 );
	nodeVar23 = vec4<f32>( 0.0, 0.0, 0.0, 0.0 );
	nodeVar24 = 0.0;
	let nodeConst5 = ( 1.0 / nodeVar4.z );
	let nodeConst6 = ( 1.0 / nodeVar12.z );
	nodeVar25 = false;
	nodeVar26 = vec2<f32>( 0.0, 0.0 );
	nodeVar27 = 0.0;

	for ( var i : i32 = 1; i < nodeConst4; i ++ ) {

		nodeVar28 = ( f32( i ) / f32( nodeConst4 ) );
		nodeVar29 = ( nodeVar13 + ( nodeVar20 * vec2<f32>( ( nodeVar28 * f32( nodeConst4 ) ) ) ) );

		if ( ( ( ( ( nodeVar29.x < 0.0 ) || ( nodeVar29.x > object.nodeUniform7.x ) ) || ( nodeVar29.y < 0.0 ) ) || ( nodeVar29.y > object.nodeUniform7.y ) ) ) {

			break;
			

		}

		nodeVar30 = ( nodeVar29 * nodeVar21 );
		nodeVar31 = textureLoad( nodeUniform0, vec2<u32>( clamp( floor( tsl_coord_clampS_clampT_2d( nodeVar30 ) * vec2<f32>( nodeVar2 ) ), vec2<f32>( 0 ), vec2<f32>( nodeVar2 - vec2<u32>( 1, 1 ) ) ) ), u32( 0 ) );
		nodeVar32 = nodeVar31;
		nodeVar33 = ( ( object.nodeUniform6 * object.nodeUniform10 ) / ( ( ( object.nodeUniform10 - object.nodeUniform6 ) * nodeVar32 ) - object.nodeUniform10 ) );
		nodeVar34 = ( 1.0 / ( nodeConst5 + ( nodeVar28 * ( nodeConst6 - nodeConst5 ) ) ) );

		if ( ( nodeVar34 <= nodeVar33 ) ) {

			let nodeConst7 = ( object.nodeUniform1 * vec4<f32>( vec3<f32>( ( ( vec2<f32>( nodeVar30.x, ( 1.0 - nodeVar30.y ) ) * vec2<f32>( 2.0 ) ) - vec2<f32>( 1.0 ) ), nodeVar32 ), 1.0 ) );
			nodeVar35 = ( nodeConst7.xyz / vec3<f32>( nodeConst7.w ) );
			nodeVar36 = ( length( cross( ( nodeVar35 - nodeVar4 ), ( nodeVar35 - nodeVar12 ) ) ) / length( ( nodeVar12 - nodeVar4 ) ) );
			nodeVar37 = ( nodeVar30 + nodeVar22 );
			let nodeConst8 = ( object.nodeUniform1 * vec4<f32>( vec3<f32>( ( ( vec2<f32>( nodeVar37.x, ( 1.0 - nodeVar37.y ) ) * vec2<f32>( 2.0 ) ) - vec2<f32>( 1.0 ) ), nodeVar32 ), 1.0 ) );
			nodeVar38 = ( nodeConst8.xyz / vec3<f32>( nodeConst8.w ) );
			nodeVar39 = ( ( nodeVar38.x - nodeVar35.x ) * 3.0 );
			nodeVar40 = max( nodeVar39, object.nodeUniform11 );

			if ( ( nodeVar36 <= nodeVar40 ) ) {

				nodeVar41 = textureSample( nodeUniform3, nodeUniform3_sampler, nodeVar30 );
				nodeVar42 = normalize( ( ( nodeVar41 * vec4<f32>( 2.0 ) ) - vec4<f32>( 1.0 ) ).xyz );

				if ( ( dot( nodeVar10, nodeVar42 ) >= 0.0 ) ) {

					continue;
					

				}

				nodeVar43 = ( - ( ( ( nodeVar7.x * nodeVar4.x ) + ( nodeVar7.y * nodeVar4.y ) ) + ( nodeVar7.z * nodeVar4.z ) ) );
				nodeVar44 = ( ( ( ( nodeVar7.x * nodeVar35.x ) + ( nodeVar7.y * nodeVar35.y ) ) + ( nodeVar7.z * nodeVar35.z ) ) + nodeVar43 );

				if ( ( nodeVar44 > object.nodeUniform5 ) ) {

					break;
					

				}

				nodeVar25 = true;
				nodeVar26 = nodeVar30;
				nodeVar27 = nodeVar32;
				break;
				

			}

			

		}


	}


	if ( nodeVar25 ) {

		let nodeConst9 = ( object.nodeUniform1 * vec4<f32>( vec3<f32>( ( ( vec2<f32>( nodeVar26.x, ( 1.0 - nodeVar26.y ) ) * vec2<f32>( 2.0 ) ) - vec2<f32>( 1.0 ) ), nodeVar27 ), 1.0 ) );
		nodeVar45 = ( nodeConst9.xyz / vec3<f32>( nodeConst9.w ) );
		nodeVar46 = ( - ( ( ( nodeVar7.x * nodeVar4.x ) + ( nodeVar7.y * nodeVar4.y ) ) + ( nodeVar7.z * nodeVar4.z ) ) );
		nodeVar47 = ( ( ( ( nodeVar7.x * nodeVar45.x ) + ( nodeVar7.y * nodeVar45.y ) ) + ( nodeVar7.z * nodeVar45.z ) ) + nodeVar46 );

		if ( ( nodeVar47 <= object.nodeUniform5 ) ) {

			nodeVar48 = ( object.nodeUniform2 * vec4<f32>( nodeVar45, 1.0 ) ).xyz;
			nodeVar49 = ( distance( nodeVar5, nodeVar48 ) * 1.0 );
			nodeVar50 = textureSample( nodeUniform12, nodeUniform12_sampler, nodeVar26 );
			nodeVar51 = nodeVar50;
			nodeVar52 = nodeVar51.xyz;
			nodeVar51.x = nodeVar52[ 0 ];
			nodeVar51.y = nodeVar52[ 1 ];
			nodeVar51.z = nodeVar52[ 2 ];
			nodeVar53 = ( 1.0 - ( nodeVar47 / object.nodeUniform5 ) );
			nodeVar54 = ( nodeVar53 * nodeVar53 );
			nodeVar55 = ( ( dot( nodeVar8, nodeVar10 ) + 1.0 ) / 2.0 );
			nodeVar24 = 1.0;
			nodeVar23 = vec4<f32>( ( ( nodeVar51.xyz * vec3<f32>( nodeConst1 ) ) * vec3<f32>( ( nodeVar54 * nodeVar55 ) ) ), nodeVar49 );
			

		}

		

	}


	if ( ( nodeVar24 == 0.0 ) ) {

		

	}

	nodeVar56 = max( dot( nodeVar23.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) ), 0.0001 );
	nodeVar57 = ( nodeVar23.xyz * vec3<f32>( min( ( object.nodeUniform13 / nodeVar56 ), 1.0 ) ) );
	nodeVar23.x = nodeVar57[ 0 ];
	nodeVar23.y = nodeVar57[ 1 ];
	nodeVar23.z = nodeVar57[ 2 ];
	nodeVar58 = ( nodeVar23.xyz * vec3<f32>( object.nodeUniform14 ) );
	nodeVar23.x = nodeVar58[ 0 ];
	nodeVar23.y = nodeVar58[ 1 ];
	nodeVar23.z = nodeVar58[ 2 ];

	// result

	output.color = max( nodeVar23, vec4<f32>( 0.0 ) );

	return output;

}
