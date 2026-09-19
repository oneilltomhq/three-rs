// Three.js r186 - Node System

// global
diagnostic( off, derivative_uniformity );


// structs

struct OutputStruct {
	@location( 0 ) color: vec4<f32>
};
var<private> output : OutputStruct;

// uniforms
@binding( 1 ) @group( 1 ) var nodeUniform1_sampler : sampler;
@binding( 2 ) @group( 1 ) var nodeUniform1 : texture_2d<f32>;
@binding( 3 ) @group( 1 ) var nodeUniform4_sampler : sampler;
@binding( 4 ) @group( 1 ) var nodeUniform4 : texture_2d<f32>;
@binding( 5 ) @group( 1 ) var nodeUniform8_sampler : sampler;
@binding( 6 ) @group( 1 ) var nodeUniform8 : texture_2d<f32>;
@binding( 7 ) @group( 1 ) var nodeUniform16_sampler : sampler;
@binding( 8 ) @group( 1 ) var nodeUniform16 : texture_2d<f32>;
@binding( 9 ) @group( 1 ) var nodeUniform18_sampler : sampler;
@binding( 10 ) @group( 1 ) var nodeUniform18 : texture_2d<f32>;
@binding( 11 ) @group( 1 ) var nodeUniform20_sampler : sampler;
@binding( 12 ) @group( 1 ) var nodeUniform20 : texture_2d<f32>;
@binding( 13 ) @group( 1 ) var nodeUniform28_sampler : sampler;
@binding( 14 ) @group( 1 ) var nodeUniform28 : texture_2d<f32>;

struct objectStruct {
	nodeUniform0 : vec3<f32>,
	nodeUniform2 : mat3x3<f32>,
	nodeUniform3 : f32,
	nodeUniform5 : mat3x3<f32>,
	nodeUniform6 : f32,
	nodeUniform7 : f32,
	nodeUniform9 : mat3x3<f32>,
	nodeUniform10 : f32,
	nodeUniform11 : mat3x3<f32>,
	nodeUniform13 : mat3x3<f32>,
	nodeUniform14 : vec3<f32>,
	nodeUniform15 : f32,
	nodeUniform17 : mat3x3<f32>,
	nodeUniform19 : mat4x4<f32>,
	nodeUniform21 : mat3x3<f32>,
	nodeUniform22 : vec2<f32>,
	nodeUniform23 : f32,
	nodeUniform24 : mat4x4<f32>,
	nodeUniform26 : f32,
	nodeUniform27 : f32,
	nodeUniform29 : f32
};
@binding( 0 ) @group( 1 )
var<uniform> object : objectStruct;

struct renderStruct {
	cameraProjectionMatrix : mat4x4<f32>,
	cameraViewMatrix : mat4x4<f32>,
	cameraWorldMatrix : mat4x4<f32>
};
@binding( 0 ) @group( 0 )
var<uniform> render : renderStruct;

// vars
var<private> DiffuseColor : vec4<f32>;
var<private> nodeVar0 : vec4<f32>;
var<private> AmbientOcclusion : f32;
var<private> nodeVar1 : vec4<f32>;
var<private> Metalness : f32;
var<private> nodeVar2 : vec4<f32>;
var<private> Roughness : f32;
var<private> nodeVar3 : vec4<f32>;
var<private> normalViewGeometry : vec3<f32>;
var<private> nodeVar4 : vec3<f32>;
var<private> SpecularColor : vec3<f32>;
var<private> SpecularColorBlended : vec3<f32>;
var<private> SpecularF90 : f32;
var<private> DiffuseContribution : vec3<f32>;
var<private> EmissiveColor : vec3<f32>;
var<private> nodeVar5 : vec4<f32>;
var<private> Output : vec4<f32>;
var<private> NORMAL_normalView : vec3<f32>;
var<private> nodeVar6 : vec3<f32>;
var<private> nodeVar7 : vec2<f32>;
var<private> nodeVar8 : vec3<f32>;
var<private> nodeVar9 : vec2<f32>;
var<private> nodeVar10 : vec3<f32>;
var<private> nodeVar11 : f32;
var<private> nodeVar12 : vec3<f32>;
var<private> nodeVar13 : f32;
var<private> tangentViewFrame : vec3<f32>;
var<private> NORMAL_tangentView : vec3<f32>;
var<private> bitangentViewFrame : vec3<f32>;
var<private> NORMAL_bitangentView : vec3<f32>;
var<private> NORMAL_TBNViewMatrix : mat3x3<f32>;
var<private> nodeVar14 : vec4<f32>;
var<private> nodeVar15 : vec4<f32>;
var<private> normalView : vec3<f32>;
var<private> positionViewDirection : vec3<f32>;
var<private> nodeVar16 : f32;
var<private> nodeVar17 : vec2<f32>;
var<private> nodeVar18 : f32;
var<private> nodeVar19 : f32;
var<private> nodeVar20 : f32;
var<private> nodeVar21 : f32;
var<private> nodeVar22 : vec3<f32>;
var<private> nodeVar23 : vec3<f32>;
var<private> radiance : vec3<f32>;
var<private> nodeVar24 : f32;
var<private> nodeVar25 : f32;
var<private> nodeVar26 : f32;
var<private> nodeVar27 : vec3<f32>;
var<private> nodeVar28 : f32;
var<private> nodeVar29 : f32;
var<private> nodeVar30 : f32;
var<private> nodeVar31 : vec2<f32>;
var<private> nodeVar32 : vec4<f32>;
var<private> nodeVar33 : vec3<f32>;
var<private> nodeVar34 : f32;
var<private> nodeVar35 : f32;
var<private> nodeVar36 : f32;
var<private> nodeVar37 : f32;
var<private> nodeVar38 : f32;
var<private> nodeVar39 : vec2<f32>;
var<private> nodeVar40 : vec4<f32>;
var<private> nodeVar41 : vec3<f32>;
var<private> nodeVar42 : vec3<f32>;
var<private> iblIrradiance : vec3<f32>;
var<private> nodeVar43 : f32;
var<private> nodeVar44 : f32;
var<private> nodeVar45 : f32;
var<private> normalWorld : vec3<f32>;
var<private> nodeVar46 : f32;
var<private> nodeVar47 : f32;
var<private> nodeVar48 : f32;
var<private> nodeVar49 : vec2<f32>;
var<private> nodeVar50 : vec4<f32>;
var<private> nodeVar51 : vec3<f32>;
var<private> nodeVar52 : f32;
var<private> nodeVar53 : f32;
var<private> nodeVar54 : f32;
var<private> nodeVar55 : f32;
var<private> nodeVar56 : f32;
var<private> nodeVar57 : vec2<f32>;
var<private> nodeVar58 : vec4<f32>;
var<private> nodeVar59 : vec3<f32>;
var<private> nodeVar60 : vec3<f32>;
var<private> ambientOcclusion : f32;
var<private> nodeVar61 : f32;
var<private> nodeVar62 : vec3<f32>;
var<private> nodeVar63 : vec3<f32>;
var<private> nodeVar64 : vec3<f32>;
var<private> nodeVar65 : f32;
var<private> nodeVar66 : vec3<f32>;
var<private> nodeVar67 : vec3<f32>;
var<private> nodeVar68 : vec3<f32>;
var<private> nodeVar69 : vec3<f32>;
var<private> nodeVar70 : vec3<f32>;
var<private> nodeVar71 : vec3<f32>;
var<private> nodeVar72 : vec3<f32>;
var<private> nodeVar73 : f32;
var<private> nodeVar74 : f32;
var<private> nodeVar75 : f32;
var<private> nodeVar76 : vec3<f32>;
var<private> nodeVar77 : vec3<f32>;
var<private> nodeVar78 : vec3<f32>;
var<private> nodeVar79 : vec3<f32>;
var<private> nodeVar80 : vec3<f32>;
var<private> nodeVar81 : vec3<f32>;
var<private> irradiance : vec3<f32>;
var<private> nodeVar82 : vec3<f32>;
var<private> nodeVar83 : vec3<f32>;
var<private> nodeVar84 : vec3<f32>;
var<private> nodeVar85 : vec3<f32>;
var<private> nodeVar86 : vec3<f32>;
var<private> nodeVar87 : vec3<f32>;
var<private> nodeVar88 : vec3<f32>;
var<private> indirectDiffuse : vec3<f32>;
var<private> nodeVar89 : vec3<f32>;
var<private> singleScatteringDielectric : vec3<f32>;
var<private> multiScatteringDielectric : vec3<f32>;
var<private> singleScatteringMetallic : vec3<f32>;
var<private> multiScatteringMetallic : vec3<f32>;
var<private> nodeVar90 : vec3<f32>;
var<private> nodeVar91 : f32;
var<private> nodeVar92 : vec3<f32>;
var<private> nodeVar93 : vec3<f32>;
var<private> nodeVar94 : vec3<f32>;
var<private> nodeVar95 : vec3<f32>;
var<private> nodeVar96 : vec3<f32>;
var<private> nodeVar97 : vec3<f32>;
var<private> nodeVar98 : vec3<f32>;
var<private> nodeVar99 : f32;
var<private> nodeVar100 : f32;
var<private> nodeVar101 : f32;
var<private> nodeVar102 : vec3<f32>;
var<private> nodeVar103 : vec3<f32>;
var<private> nodeVar104 : vec3<f32>;
var<private> nodeVar105 : vec3<f32>;
var<private> nodeVar106 : vec3<f32>;
var<private> nodeVar107 : vec3<f32>;
var<private> nodeVar108 : vec3<f32>;
var<private> nodeVar109 : f32;
var<private> nodeVar110 : vec3<f32>;
var<private> nodeVar111 : vec3<f32>;
var<private> nodeVar112 : vec3<f32>;
var<private> nodeVar113 : vec3<f32>;
var<private> nodeVar114 : vec3<f32>;
var<private> nodeVar115 : vec3<f32>;
var<private> nodeVar116 : vec3<f32>;
var<private> nodeVar117 : f32;
var<private> nodeVar118 : f32;
var<private> nodeVar119 : f32;
var<private> nodeVar120 : vec3<f32>;
var<private> nodeVar121 : vec3<f32>;
var<private> nodeVar122 : vec3<f32>;
var<private> nodeVar123 : vec3<f32>;
var<private> nodeVar124 : vec3<f32>;
var<private> nodeVar125 : vec3<f32>;
var<private> nodeVar126 : vec3<f32>;
var<private> nodeVar127 : vec3<f32>;
var<private> nodeVar128 : vec3<f32>;
var<private> nodeVar129 : vec3<f32>;
var<private> nodeVar130 : vec3<f32>;
var<private> nodeVar131 : vec3<f32>;
var<private> nodeVar132 : vec3<f32>;
var<private> nodeVar133 : vec3<f32>;
var<private> nodeVar134 : vec3<f32>;
var<private> nodeVar135 : vec3<f32>;
var<private> nodeVar136 : vec3<f32>;
var<private> nodeVar137 : vec3<f32>;
var<private> nodeVar138 : vec3<f32>;
var<private> indirectSpecular : vec3<f32>;
var<private> nodeVar139 : vec3<f32>;
var<private> nodeVar140 : vec3<f32>;
var<private> nodeVar141 : vec3<f32>;
var<private> nodeVar142 : f32;
var<private> nodeVar143 : f32;
var<private> nodeVar144 : f32;
var<private> nodeVar145 : f32;
var<private> nodeVar146 : f32;
var<private> nodeVar147 : f32;
var<private> nodeVar148 : f32;
var<private> nodeVar149 : f32;
var<private> nodeVar150 : f32;
var<private> nodeVar151 : f32;
var<private> nodeVar152 : f32;
var<private> nodeVar153 : vec3<f32>;
var<private> totalDiffuse : vec3<f32>;
var<private> directDiffuse : vec3<f32>;
var<private> nodeVar154 : vec3<f32>;
var<private> totalSpecular : vec3<f32>;
var<private> directSpecular : vec3<f32>;
var<private> nodeVar155 : vec3<f32>;
var<private> outgoingLight : vec3<f32>;
var<private> nodeVar156 : vec3<f32>;
var<private> nodeVar157 : vec4<f32>;

// codes
fn roughnessToMip ( roughness : f32 ) -> f32 {

	var nodeVar0 : f32;

	nodeVar0 = 0.0;

	if ( ( roughness >= 0.8 ) ) {

		nodeVar0 = ( ( ( ( 1.0 - roughness ) * ( -1.0 - -2.0 ) ) / ( 1.0 - 0.8 ) ) + -2.0 );
		

	} else {


		if ( ( roughness >= 0.4 ) ) {

			nodeVar0 = ( ( ( ( 0.8 - roughness ) * ( 2.0 - -1.0 ) ) / ( 0.8 - 0.4 ) ) + -1.0 );
			

		} else {


			if ( ( roughness >= 0.305 ) ) {

				nodeVar0 = ( ( ( ( 0.4 - roughness ) * ( 3.0 - 2.0 ) ) / ( 0.4 - 0.305 ) ) + 2.0 );
				

			} else {


				if ( ( roughness >= 0.21 ) ) {

					nodeVar0 = ( ( ( ( 0.305 - roughness ) * ( 4.0 - 3.0 ) ) / ( 0.305 - 0.21 ) ) + 3.0 );
					

				} else {

					nodeVar0 = ( -2.0 * log2( ( 1.16 * roughness ) ) );
					

				}

				

			}

			

		}

		

	}


	return nodeVar0;

}


fn getFace ( direction : vec3<f32> ) -> f32 {

	var nodeVar0 : vec3<f32>;
	var nodeVar1 : f32;
	var nodeVar2 : f32;
	var nodeVar3 : f32;
	var nodeVar4 : f32;
	var nodeVar5 : f32;

	nodeVar0 = abs( direction );
	nodeVar1 = -1.0;

	if ( ( nodeVar0.x > nodeVar0.z ) ) {


		if ( ( nodeVar0.x > nodeVar0.y ) ) {


			if ( ( direction.x > 0.0 ) ) {

				nodeVar2 = 0.0;

			} else {

				nodeVar2 = 3.0;

			}

			nodeVar1 = nodeVar2;
			

		} else {


			if ( ( direction.y > 0.0 ) ) {

				nodeVar3 = 1.0;

			} else {

				nodeVar3 = 4.0;

			}

			nodeVar1 = nodeVar3;
			

		}

		

	} else {


		if ( ( nodeVar0.z > nodeVar0.y ) ) {


			if ( ( direction.z > 0.0 ) ) {

				nodeVar4 = 2.0;

			} else {

				nodeVar4 = 5.0;

			}

			nodeVar1 = nodeVar4;
			

		} else {


			if ( ( direction.y > 0.0 ) ) {

				nodeVar5 = 1.0;

			} else {

				nodeVar5 = 4.0;

			}

			nodeVar1 = nodeVar5;
			

		}

		

	}


	return nodeVar1;

}


fn getUV ( direction : vec3<f32>, face : f32 ) -> vec2<f32> {

	var nodeVar0 : vec2<f32>;

	nodeVar0 = vec2<f32>( 0.0, 0.0 );

	if ( ( face == 0.0 ) ) {

		nodeVar0 = ( vec2<f32>( direction.z, direction.y ) / vec2<f32>( abs( direction.x ) ) );
		

	} else {


		if ( ( face == 1.0 ) ) {

			nodeVar0 = ( vec2<f32>( ( - direction.x ), ( - direction.z ) ) / vec2<f32>( abs( direction.y ) ) );
			

		} else {


			if ( ( face == 2.0 ) ) {

				nodeVar0 = ( vec2<f32>( ( - direction.x ), direction.y ) / vec2<f32>( abs( direction.z ) ) );
				

			} else {


				if ( ( face == 3.0 ) ) {

					nodeVar0 = ( vec2<f32>( ( - direction.z ), direction.y ) / vec2<f32>( abs( direction.x ) ) );
					

				} else {


					if ( ( face == 4.0 ) ) {

						nodeVar0 = ( vec2<f32>( ( - direction.x ), direction.z ) / vec2<f32>( abs( direction.y ) ) );
						

					} else {

						nodeVar0 = ( vec2<f32>( direction.x, direction.y ) / vec2<f32>( abs( direction.z ) ) );
						

					}

					

				}

				

			}

			

		}

		

	}


	return ( vec2<f32>( 0.5 ) * ( nodeVar0 + vec2<f32>( 1.0 ) ) );

}




@fragment
fn main( @location( 0 ) v_positionView : vec3<f32>,
	@location( 1 ) v_normalViewGeometry : vec3<f32>,
	@location( 2 ) v_positionViewDirection : vec3<f32>,
	@location( 3 ) nodeVarying6 : vec2<f32> ) -> OutputStruct {

	// flow
	// code

	nodeVar0 = textureSample( nodeUniform1, nodeUniform1_sampler, ( object.nodeUniform2 * vec3<f32>( nodeVarying6, 1.0 ) ).xy );
	DiffuseColor = ( vec4<f32>( object.nodeUniform0, 1.0 ) * nodeVar0 );
	DiffuseColor.w = ( DiffuseColor.w * object.nodeUniform3 );
	DiffuseColor.w = 1.0;
	nodeVar1 = textureSample( nodeUniform4, nodeUniform4_sampler, ( object.nodeUniform5 * vec3<f32>( nodeVarying6, 1.0 ) ).xy );
	AmbientOcclusion = ( ( ( nodeVar1.x - 1.0 ) * object.nodeUniform6 ) + 1.0 );
	nodeVar2 = textureSample( nodeUniform8, nodeUniform8_sampler, ( object.nodeUniform9 * vec3<f32>( nodeVarying6, 1.0 ) ).xy );
	Metalness = ( object.nodeUniform7 * nodeVar2.z );
	nodeVar3 = textureSample( nodeUniform8, nodeUniform8_sampler, ( object.nodeUniform11 * vec3<f32>( nodeVarying6, 1.0 ) ).xy );
	normalViewGeometry = normalize( v_normalViewGeometry );
	nodeVar4 = max( abs( dpdx( normalViewGeometry ) ), abs( - dpdy( normalViewGeometry ) ) );
	Roughness = min( ( max( ( object.nodeUniform10 * nodeVar3.y ), 0.0525 ) + max( max( nodeVar4.x, nodeVar4.y ), nodeVar4.z ) ), 1.0 );
	SpecularColor = vec3<f32>( 0.04, 0.04, 0.04 );
	SpecularColorBlended = mix( vec3<f32>( 0.04, 0.04, 0.04 ), DiffuseColor.xyz, Metalness );
	SpecularF90 = 1.0;
	DiffuseContribution = ( DiffuseColor.xyz * vec3<f32>( ( 1.0 - ( object.nodeUniform7 * nodeVar2.z ) ) ) );
	nodeVar5 = textureSample( nodeUniform16, nodeUniform16_sampler, ( object.nodeUniform17 * vec3<f32>( nodeVarying6, 1.0 ) ).xy );
	EmissiveColor = ( vec4<f32>( ( object.nodeUniform14 * vec3<f32>( object.nodeUniform15 ) ), 1.0 ) * nodeVar5 ).xyz;
	NORMAL_normalView = normalViewGeometry;
	nodeVar6 = cross( - dpdy( v_positionView ), NORMAL_normalView );
	nodeVar7 = dpdx( nodeVarying6 );
	nodeVar8 = cross( NORMAL_normalView, dpdx( v_positionView ) );
	nodeVar9 = - dpdy( nodeVarying6 );
	nodeVar10 = ( ( nodeVar6 * vec3<f32>( nodeVar7.x ) ) + ( nodeVar8 * vec3<f32>( nodeVar9.x ) ) );
	nodeVar12 = ( ( nodeVar6 * vec3<f32>( nodeVar7.y ) ) + ( nodeVar8 * vec3<f32>( nodeVar9.y ) ) );
	nodeVar13 = max( dot( nodeVar10, nodeVar10 ), dot( nodeVar12, nodeVar12 ) );

	if ( ( nodeVar13 == 0.0 ) ) {

		nodeVar11 = 0.0;

	} else {

		nodeVar11 = inverseSqrt( nodeVar13 );

	}

	tangentViewFrame = ( nodeVar10 * vec3<f32>( nodeVar11 ) );
	NORMAL_tangentView = tangentViewFrame;
	bitangentViewFrame = ( nodeVar12 * nodeVar11 );
	NORMAL_bitangentView = bitangentViewFrame;
	NORMAL_TBNViewMatrix = mat3x3<f32>( NORMAL_tangentView, NORMAL_bitangentView, NORMAL_normalView );
	nodeVar14 = textureSample( nodeUniform20, nodeUniform20_sampler, ( object.nodeUniform21 * vec3<f32>( nodeVarying6, 1.0 ) ).xy );
	nodeVar15 = ( ( nodeVar14 * vec4<f32>( 2.0 ) ) - vec4<f32>( 1.0 ) );
	normalView = normalize( ( NORMAL_TBNViewMatrix * vec3<f32>( ( nodeVar15.xy * object.nodeUniform22 ), nodeVar15.z ) ) );
	positionViewDirection = normalize( v_positionViewDirection );
	nodeVar16 = dot( normalView, positionViewDirection );
	nodeVar17 = textureSample( nodeUniform18, nodeUniform18_sampler, vec2<f32>( Roughness, clamp( nodeVar16, 0.0, 1.0 ) ) ).xy;
	let dfg = nodeVar17;
	nodeVar18 = ( dfg.x + dfg.y );
	nodeVar19 = ( 1.0 / nodeVar18 );
	nodeVar20 = nodeVar19;
	nodeVar21 = ( nodeVar20 - 1.0 );
	nodeVar22 = ( SpecularColorBlended * vec3<f32>( nodeVar21 ) );
	nodeVar23 = ( nodeVar22 + vec3<f32>( 1.0 ) );
	let multiScatteringCompensation = nodeVar23;
	radiance = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar24 = clamp( roughnessToMip( Roughness ), -2.0, object.nodeUniform23 );
	nodeVar25 = floor( nodeVar24 );
	nodeVar26 = nodeVar25;
	nodeVar27 = normalize( ( render.cameraWorldMatrix * vec4<f32>( normalize( mix( reflect( ( - positionViewDirection ), normalView ), normalView, ( ( ( Roughness * Roughness ) * Roughness ) * Roughness ) ) ), 0.0 ) ).xyz );
	nodeVar28 = getFace( ( object.nodeUniform24 * vec4<f32>( vec3<f32>( nodeVar27.x, ( - nodeVar27.y ), nodeVar27.z ), 1.0 ) ).xyz );
	nodeVar29 = max( ( 4.0 - nodeVar26 ), 0.0 );
	nodeVar26 = max( nodeVar26, 4.0 );
	nodeVar30 = exp2( nodeVar26 );
	nodeVar31 = ( ( getUV( ( object.nodeUniform24 * vec4<f32>( vec3<f32>( nodeVar27.x, ( - nodeVar27.y ), nodeVar27.z ), 1.0 ) ).xyz, nodeVar28 ) * vec2<f32>( ( nodeVar30 - 2.0 ) ) ) + vec2<f32>( 1.0 ) );

	if ( ( nodeVar28 > 2.0 ) ) {

		nodeVar31.y = ( nodeVar31.y + nodeVar30 );
		nodeVar28 = ( nodeVar28 - 3.0 );
		

	}

	nodeVar31.x = ( nodeVar31.x + ( nodeVar28 * nodeVar30 ) );
	nodeVar31.x = ( nodeVar31.x + ( nodeVar29 * ( 3.0 * 16.0 ) ) );
	nodeVar31.y = ( nodeVar31.y + ( 4.0 * ( exp2( object.nodeUniform23 ) - nodeVar30 ) ) );
	nodeVar31.x = ( nodeVar31.x * object.nodeUniform26 );
	nodeVar31.y = ( nodeVar31.y * object.nodeUniform27 );
	nodeVar32 = textureSampleGrad( nodeUniform28, nodeUniform28_sampler, nodeVar31, vec2<f32>( 0.0, 0.0 ), vec2<f32>( 0.0, 0.0 ) );
	nodeVar33 = nodeVar32.xyz;
	nodeVar34 = fract( nodeVar24 );

	if ( ( nodeVar34 != 0.0 ) ) {

		nodeVar35 = ( nodeVar25 + 1.0 );
		nodeVar36 = getFace( ( object.nodeUniform24 * vec4<f32>( vec3<f32>( nodeVar27.x, ( - nodeVar27.y ), nodeVar27.z ), 1.0 ) ).xyz );
		nodeVar37 = max( ( 4.0 - nodeVar35 ), 0.0 );
		nodeVar35 = max( nodeVar35, 4.0 );
		nodeVar38 = exp2( nodeVar35 );
		nodeVar39 = ( ( getUV( ( object.nodeUniform24 * vec4<f32>( vec3<f32>( nodeVar27.x, ( - nodeVar27.y ), nodeVar27.z ), 1.0 ) ).xyz, nodeVar36 ) * vec2<f32>( ( nodeVar38 - 2.0 ) ) ) + vec2<f32>( 1.0 ) );

		if ( ( nodeVar36 > 2.0 ) ) {

			nodeVar39.y = ( nodeVar39.y + nodeVar38 );
			nodeVar36 = ( nodeVar36 - 3.0 );
			

		}

		nodeVar39.x = ( nodeVar39.x + ( nodeVar36 * nodeVar38 ) );
		nodeVar39.x = ( nodeVar39.x + ( nodeVar37 * ( 3.0 * 16.0 ) ) );
		nodeVar39.y = ( nodeVar39.y + ( 4.0 * ( exp2( object.nodeUniform23 ) - nodeVar38 ) ) );
		nodeVar39.x = ( nodeVar39.x * object.nodeUniform26 );
		nodeVar39.y = ( nodeVar39.y * object.nodeUniform27 );
		nodeVar40 = textureSampleGrad( nodeUniform28, nodeUniform28_sampler, nodeVar39, vec2<f32>( 0.0, 0.0 ), vec2<f32>( 0.0, 0.0 ) );
		nodeVar41 = nodeVar40.xyz;
		nodeVar33 = mix( nodeVar33, nodeVar41, nodeVar34 );
		

	}

	nodeVar42 = ( radiance + ( nodeVar33 * vec3<f32>( object.nodeUniform29 ) ) );
	radiance = nodeVar42;
	iblIrradiance = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar43 = clamp( roughnessToMip( 1.0 ), -2.0, object.nodeUniform23 );
	nodeVar44 = floor( nodeVar43 );
	nodeVar45 = nodeVar44;
	normalWorld = normalize( ( vec4<f32>( normalView, 0.0 ) * render.cameraViewMatrix ).xyz );
	nodeVar46 = getFace( ( object.nodeUniform24 * vec4<f32>( vec3<f32>( normalWorld.x, ( - normalWorld.y ), normalWorld.z ), 1.0 ) ).xyz );
	nodeVar47 = max( ( 4.0 - nodeVar45 ), 0.0 );
	nodeVar45 = max( nodeVar45, 4.0 );
	nodeVar48 = exp2( nodeVar45 );
	nodeVar49 = ( ( getUV( ( object.nodeUniform24 * vec4<f32>( vec3<f32>( normalWorld.x, ( - normalWorld.y ), normalWorld.z ), 1.0 ) ).xyz, nodeVar46 ) * vec2<f32>( ( nodeVar48 - 2.0 ) ) ) + vec2<f32>( 1.0 ) );

	if ( ( nodeVar46 > 2.0 ) ) {

		nodeVar49.y = ( nodeVar49.y + nodeVar48 );
		nodeVar46 = ( nodeVar46 - 3.0 );
		

	}

	nodeVar49.x = ( nodeVar49.x + ( nodeVar46 * nodeVar48 ) );
	nodeVar49.x = ( nodeVar49.x + ( nodeVar47 * ( 3.0 * 16.0 ) ) );
	nodeVar49.y = ( nodeVar49.y + ( 4.0 * ( exp2( object.nodeUniform23 ) - nodeVar48 ) ) );
	nodeVar49.x = ( nodeVar49.x * object.nodeUniform26 );
	nodeVar49.y = ( nodeVar49.y * object.nodeUniform27 );
	nodeVar50 = textureSampleGrad( nodeUniform28, nodeUniform28_sampler, nodeVar49, vec2<f32>( 0.0, 0.0 ), vec2<f32>( 0.0, 0.0 ) );
	nodeVar51 = nodeVar50.xyz;
	nodeVar52 = fract( nodeVar43 );

	if ( ( nodeVar52 != 0.0 ) ) {

		nodeVar53 = ( nodeVar44 + 1.0 );
		nodeVar54 = getFace( ( object.nodeUniform24 * vec4<f32>( vec3<f32>( normalWorld.x, ( - normalWorld.y ), normalWorld.z ), 1.0 ) ).xyz );
		nodeVar55 = max( ( 4.0 - nodeVar53 ), 0.0 );
		nodeVar53 = max( nodeVar53, 4.0 );
		nodeVar56 = exp2( nodeVar53 );
		nodeVar57 = ( ( getUV( ( object.nodeUniform24 * vec4<f32>( vec3<f32>( normalWorld.x, ( - normalWorld.y ), normalWorld.z ), 1.0 ) ).xyz, nodeVar54 ) * vec2<f32>( ( nodeVar56 - 2.0 ) ) ) + vec2<f32>( 1.0 ) );

		if ( ( nodeVar54 > 2.0 ) ) {

			nodeVar57.y = ( nodeVar57.y + nodeVar56 );
			nodeVar54 = ( nodeVar54 - 3.0 );
			

		}

		nodeVar57.x = ( nodeVar57.x + ( nodeVar54 * nodeVar56 ) );
		nodeVar57.x = ( nodeVar57.x + ( nodeVar55 * ( 3.0 * 16.0 ) ) );
		nodeVar57.y = ( nodeVar57.y + ( 4.0 * ( exp2( object.nodeUniform23 ) - nodeVar56 ) ) );
		nodeVar57.x = ( nodeVar57.x * object.nodeUniform26 );
		nodeVar57.y = ( nodeVar57.y * object.nodeUniform27 );
		nodeVar58 = textureSampleGrad( nodeUniform28, nodeUniform28_sampler, nodeVar57, vec2<f32>( 0.0, 0.0 ), vec2<f32>( 0.0, 0.0 ) );
		nodeVar59 = nodeVar58.xyz;
		nodeVar51 = mix( nodeVar51, nodeVar59, nodeVar52 );
		

	}

	nodeVar60 = ( iblIrradiance + ( ( nodeVar51 * vec3<f32>( 3.141592653589793 ) ) * vec3<f32>( object.nodeUniform29 ) ) );
	iblIrradiance = nodeVar60;
	ambientOcclusion = 1.0;
	nodeVar61 = ( ambientOcclusion * AmbientOcclusion );
	ambientOcclusion = nodeVar61;
	nodeVar62 = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar63 = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar64 = ( SpecularColor * vec3<f32>( dfg.x ) );
	nodeVar65 = ( SpecularF90 * dfg.y );
	nodeVar66 = ( nodeVar64 + vec3<f32>( nodeVar65 ) );
	nodeVar67 = ( nodeVar62 + nodeVar66 );
	nodeVar62 = nodeVar67;
	nodeVar68 = ( vec3<f32>( 1.0 ) - SpecularColor );
	nodeVar69 = nodeVar68;
	nodeVar70 = ( nodeVar69 * vec3<f32>( 0.047619 ) );
	nodeVar71 = ( SpecularColor + nodeVar70 );
	nodeVar72 = ( nodeVar66 * nodeVar71 );
	nodeVar73 = ( dfg.x + dfg.y );
	nodeVar74 = ( 1.0 - nodeVar73 );
	nodeVar75 = nodeVar74;
	nodeVar76 = ( vec3<f32>( nodeVar75 ) * nodeVar71 );
	nodeVar77 = ( vec3<f32>( 1.0 ) - nodeVar76 );
	nodeVar78 = nodeVar77;
	nodeVar79 = ( nodeVar72 / nodeVar78 );
	nodeVar80 = ( nodeVar79 * vec3<f32>( nodeVar75 ) );
	nodeVar81 = ( nodeVar63 + nodeVar80 );
	nodeVar63 = nodeVar81;
	irradiance = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar82 = ( DiffuseContribution * vec3<f32>( 0.3183098861837907 ) );
	nodeVar83 = ( irradiance * nodeVar82 );
	nodeVar84 = ( nodeVar62 + nodeVar63 );
	nodeVar85 = ( vec3<f32>( 1.0 ) - nodeVar84 );
	nodeVar86 = nodeVar85;
	nodeVar87 = ( nodeVar83 * nodeVar86 );
	nodeVar88 = nodeVar87;
	indirectDiffuse = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar89 = ( indirectDiffuse + nodeVar88 );
	indirectDiffuse = nodeVar89;
	singleScatteringDielectric = vec3<f32>( 0.0, 0.0, 0.0 );
	multiScatteringDielectric = vec3<f32>( 0.0, 0.0, 0.0 );
	singleScatteringMetallic = vec3<f32>( 0.0, 0.0, 0.0 );
	multiScatteringMetallic = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar90 = ( SpecularColor * vec3<f32>( dfg.x ) );
	nodeVar91 = ( SpecularF90 * dfg.y );
	nodeVar92 = ( nodeVar90 + vec3<f32>( nodeVar91 ) );
	nodeVar93 = ( singleScatteringDielectric + nodeVar92 );
	singleScatteringDielectric = nodeVar93;
	nodeVar94 = ( vec3<f32>( 1.0 ) - SpecularColor );
	nodeVar95 = nodeVar94;
	nodeVar96 = ( nodeVar95 * vec3<f32>( 0.047619 ) );
	nodeVar97 = ( SpecularColor + nodeVar96 );
	nodeVar98 = ( nodeVar92 * nodeVar97 );
	nodeVar99 = ( dfg.x + dfg.y );
	nodeVar100 = ( 1.0 - nodeVar99 );
	nodeVar101 = nodeVar100;
	nodeVar102 = ( vec3<f32>( nodeVar101 ) * nodeVar97 );
	nodeVar103 = ( vec3<f32>( 1.0 ) - nodeVar102 );
	nodeVar104 = nodeVar103;
	nodeVar105 = ( nodeVar98 / nodeVar104 );
	nodeVar106 = ( nodeVar105 * vec3<f32>( nodeVar101 ) );
	nodeVar107 = ( multiScatteringDielectric + nodeVar106 );
	multiScatteringDielectric = nodeVar107;
	nodeVar108 = ( DiffuseColor.xyz * vec3<f32>( dfg.x ) );
	nodeVar109 = ( SpecularF90 * dfg.y );
	nodeVar110 = ( nodeVar108 + vec3<f32>( nodeVar109 ) );
	nodeVar111 = ( singleScatteringMetallic + nodeVar110 );
	singleScatteringMetallic = nodeVar111;
	nodeVar112 = ( vec3<f32>( 1.0 ) - DiffuseColor.xyz );
	nodeVar113 = nodeVar112;
	nodeVar114 = ( nodeVar113 * vec3<f32>( 0.047619 ) );
	nodeVar115 = ( DiffuseColor.xyz + nodeVar114 );
	nodeVar116 = ( nodeVar110 * nodeVar115 );
	nodeVar117 = ( dfg.x + dfg.y );
	nodeVar118 = ( 1.0 - nodeVar117 );
	nodeVar119 = nodeVar118;
	nodeVar120 = ( vec3<f32>( nodeVar119 ) * nodeVar115 );
	nodeVar121 = ( vec3<f32>( 1.0 ) - nodeVar120 );
	nodeVar122 = nodeVar121;
	nodeVar123 = ( nodeVar116 / nodeVar122 );
	nodeVar124 = ( nodeVar123 * vec3<f32>( nodeVar119 ) );
	nodeVar125 = ( multiScatteringMetallic + nodeVar124 );
	multiScatteringMetallic = nodeVar125;
	nodeVar126 = mix( singleScatteringDielectric, singleScatteringMetallic, Metalness );
	nodeVar127 = ( radiance * nodeVar126 );
	nodeVar128 = mix( multiScatteringDielectric, multiScatteringMetallic, Metalness );
	nodeVar129 = ( iblIrradiance * vec3<f32>( 0.3183098861837907 ) );
	nodeVar130 = ( nodeVar128 * nodeVar129 );
	nodeVar131 = ( nodeVar127 + nodeVar130 );
	nodeVar132 = nodeVar131;
	nodeVar133 = ( singleScatteringDielectric + multiScatteringDielectric );
	nodeVar134 = ( vec3<f32>( 1.0 ) - nodeVar133 );
	nodeVar135 = nodeVar134;
	nodeVar136 = ( DiffuseContribution * nodeVar135 );
	nodeVar137 = ( nodeVar136 * nodeVar129 );
	nodeVar138 = nodeVar137;
	indirectSpecular = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar139 = ( indirectSpecular + nodeVar132 );
	indirectSpecular = nodeVar139;
	nodeVar140 = ( indirectDiffuse + nodeVar138 );
	indirectDiffuse = nodeVar140;
	nodeVar141 = ( indirectDiffuse * vec3<f32>( ambientOcclusion ) );
	indirectDiffuse = nodeVar141;
	nodeVar142 = dot( normalView, positionViewDirection );
	nodeVar143 = ( clamp( nodeVar142, 0.0, 1.0 ) + ambientOcclusion );
	nodeVar144 = ( Roughness * -16.0 );
	nodeVar145 = ( 1.0 - nodeVar144 );
	nodeVar146 = nodeVar145;
	nodeVar147 = ( - nodeVar146 );
	nodeVar148 = exp2( nodeVar147 );
	nodeVar149 = pow( nodeVar143, nodeVar148 );
	nodeVar150 = ( 1.0 - nodeVar149 );
	nodeVar151 = nodeVar150;
	nodeVar152 = ( ambientOcclusion - nodeVar151 );
	nodeVar153 = ( indirectSpecular * vec3<f32>( clamp( nodeVar152, 0.0, 1.0 ) ) );
	indirectSpecular = nodeVar153;
	directDiffuse = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar154 = ( directDiffuse + indirectDiffuse );
	totalDiffuse = nodeVar154;
	directSpecular = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar155 = ( directSpecular + indirectSpecular );
	totalSpecular = nodeVar155;
	nodeVar156 = ( totalDiffuse + totalSpecular );
	outgoingLight = nodeVar156;
	nodeVar157 = max( vec4<f32>( ( outgoingLight + EmissiveColor ), DiffuseColor.w ), vec4<f32>( 0.0 ) );
	Output = nodeVar157;

	// result

	output.color = nodeVar157;

	return output;

}
