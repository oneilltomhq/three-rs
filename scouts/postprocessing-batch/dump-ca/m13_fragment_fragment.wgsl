// Three.js r186 - Node System

// global
diagnostic( off, derivative_uniformity );


// structs

struct OutputStruct {
	@location( 0 ) color: vec4<f32>
};
var<private> output : OutputStruct;

// uniforms
@binding( 1 ) @group( 1 ) var nodeUniform8_sampler : sampler;
@binding( 2 ) @group( 1 ) var nodeUniform8 : texture_2d<f32>;
@binding( 3 ) @group( 1 ) var nodeUniform15_sampler : sampler;
@binding( 4 ) @group( 1 ) var nodeUniform15 : texture_2d<f32>;

struct objectStruct {
	nodeUniform0 : vec3<f32>,
	nodeUniform1 : f32,
	nodeUniform2 : f32,
	nodeUniform3 : f32,
	nodeUniform5 : mat3x3<f32>,
	nodeUniform6 : vec3<f32>,
	nodeUniform7 : f32,
	nodeUniform9 : mat4x4<f32>,
	nodeUniform10 : f32,
	nodeUniform11 : mat4x4<f32>,
	nodeUniform13 : f32,
	nodeUniform14 : f32,
	nodeUniform16 : f32
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
var<private> Metalness : f32;
var<private> Roughness : f32;
var<private> normalViewGeometry : vec3<f32>;
var<private> nodeVar0 : vec3<f32>;
var<private> SpecularColor : vec3<f32>;
var<private> SpecularColorBlended : vec3<f32>;
var<private> SpecularF90 : f32;
var<private> DiffuseContribution : vec3<f32>;
var<private> EmissiveColor : vec3<f32>;
var<private> Output : vec4<f32>;
var<private> NORMAL_normalView : vec3<f32>;
var<private> normalView : vec3<f32>;
var<private> positionViewDirection : vec3<f32>;
var<private> nodeVar1 : f32;
var<private> nodeVar2 : vec2<f32>;
var<private> nodeVar3 : f32;
var<private> nodeVar4 : f32;
var<private> nodeVar5 : f32;
var<private> nodeVar6 : f32;
var<private> nodeVar7 : vec3<f32>;
var<private> nodeVar8 : vec3<f32>;
var<private> radiance : vec3<f32>;
var<private> nodeVar9 : f32;
var<private> nodeVar10 : f32;
var<private> nodeVar11 : f32;
var<private> nodeVar12 : vec3<f32>;
var<private> nodeVar13 : f32;
var<private> nodeVar14 : f32;
var<private> nodeVar15 : f32;
var<private> nodeVar16 : vec2<f32>;
var<private> nodeVar17 : vec4<f32>;
var<private> nodeVar18 : vec3<f32>;
var<private> nodeVar19 : f32;
var<private> nodeVar20 : f32;
var<private> nodeVar21 : f32;
var<private> nodeVar22 : f32;
var<private> nodeVar23 : f32;
var<private> nodeVar24 : vec2<f32>;
var<private> nodeVar25 : vec4<f32>;
var<private> nodeVar26 : vec3<f32>;
var<private> nodeVar27 : vec3<f32>;
var<private> iblIrradiance : vec3<f32>;
var<private> nodeVar28 : f32;
var<private> nodeVar29 : f32;
var<private> nodeVar30 : f32;
var<private> normalWorld : vec3<f32>;
var<private> nodeVar31 : f32;
var<private> nodeVar32 : f32;
var<private> nodeVar33 : f32;
var<private> nodeVar34 : vec2<f32>;
var<private> nodeVar35 : vec4<f32>;
var<private> nodeVar36 : vec3<f32>;
var<private> nodeVar37 : f32;
var<private> nodeVar38 : f32;
var<private> nodeVar39 : f32;
var<private> nodeVar40 : f32;
var<private> nodeVar41 : f32;
var<private> nodeVar42 : vec2<f32>;
var<private> nodeVar43 : vec4<f32>;
var<private> nodeVar44 : vec3<f32>;
var<private> nodeVar45 : vec3<f32>;
var<private> nodeVar46 : vec3<f32>;
var<private> nodeVar47 : vec3<f32>;
var<private> nodeVar48 : vec3<f32>;
var<private> nodeVar49 : f32;
var<private> nodeVar50 : vec3<f32>;
var<private> nodeVar51 : vec3<f32>;
var<private> nodeVar52 : vec3<f32>;
var<private> nodeVar53 : vec3<f32>;
var<private> nodeVar54 : vec3<f32>;
var<private> nodeVar55 : vec3<f32>;
var<private> nodeVar56 : vec3<f32>;
var<private> nodeVar57 : f32;
var<private> nodeVar58 : f32;
var<private> nodeVar59 : f32;
var<private> nodeVar60 : vec3<f32>;
var<private> nodeVar61 : vec3<f32>;
var<private> nodeVar62 : vec3<f32>;
var<private> nodeVar63 : vec3<f32>;
var<private> nodeVar64 : vec3<f32>;
var<private> nodeVar65 : vec3<f32>;
var<private> irradiance : vec3<f32>;
var<private> nodeVar66 : vec3<f32>;
var<private> nodeVar67 : vec3<f32>;
var<private> nodeVar68 : vec3<f32>;
var<private> nodeVar69 : vec3<f32>;
var<private> nodeVar70 : vec3<f32>;
var<private> nodeVar71 : vec3<f32>;
var<private> nodeVar72 : vec3<f32>;
var<private> indirectDiffuse : vec3<f32>;
var<private> nodeVar73 : vec3<f32>;
var<private> singleScatteringDielectric : vec3<f32>;
var<private> multiScatteringDielectric : vec3<f32>;
var<private> singleScatteringMetallic : vec3<f32>;
var<private> multiScatteringMetallic : vec3<f32>;
var<private> nodeVar74 : vec3<f32>;
var<private> nodeVar75 : f32;
var<private> nodeVar76 : vec3<f32>;
var<private> nodeVar77 : vec3<f32>;
var<private> nodeVar78 : vec3<f32>;
var<private> nodeVar79 : vec3<f32>;
var<private> nodeVar80 : vec3<f32>;
var<private> nodeVar81 : vec3<f32>;
var<private> nodeVar82 : vec3<f32>;
var<private> nodeVar83 : f32;
var<private> nodeVar84 : f32;
var<private> nodeVar85 : f32;
var<private> nodeVar86 : vec3<f32>;
var<private> nodeVar87 : vec3<f32>;
var<private> nodeVar88 : vec3<f32>;
var<private> nodeVar89 : vec3<f32>;
var<private> nodeVar90 : vec3<f32>;
var<private> nodeVar91 : vec3<f32>;
var<private> nodeVar92 : vec3<f32>;
var<private> nodeVar93 : f32;
var<private> nodeVar94 : vec3<f32>;
var<private> nodeVar95 : vec3<f32>;
var<private> nodeVar96 : vec3<f32>;
var<private> nodeVar97 : vec3<f32>;
var<private> nodeVar98 : vec3<f32>;
var<private> nodeVar99 : vec3<f32>;
var<private> nodeVar100 : vec3<f32>;
var<private> nodeVar101 : f32;
var<private> nodeVar102 : f32;
var<private> nodeVar103 : f32;
var<private> nodeVar104 : vec3<f32>;
var<private> nodeVar105 : vec3<f32>;
var<private> nodeVar106 : vec3<f32>;
var<private> nodeVar107 : vec3<f32>;
var<private> nodeVar108 : vec3<f32>;
var<private> nodeVar109 : vec3<f32>;
var<private> nodeVar110 : vec3<f32>;
var<private> nodeVar111 : vec3<f32>;
var<private> nodeVar112 : vec3<f32>;
var<private> nodeVar113 : vec3<f32>;
var<private> nodeVar114 : vec3<f32>;
var<private> nodeVar115 : vec3<f32>;
var<private> nodeVar116 : vec3<f32>;
var<private> nodeVar117 : vec3<f32>;
var<private> nodeVar118 : vec3<f32>;
var<private> nodeVar119 : vec3<f32>;
var<private> nodeVar120 : vec3<f32>;
var<private> nodeVar121 : vec3<f32>;
var<private> nodeVar122 : vec3<f32>;
var<private> indirectSpecular : vec3<f32>;
var<private> nodeVar123 : vec3<f32>;
var<private> nodeVar124 : vec3<f32>;
var<private> ambientOcclusion : f32;
var<private> nodeVar125 : vec3<f32>;
var<private> nodeVar126 : f32;
var<private> nodeVar127 : f32;
var<private> nodeVar128 : f32;
var<private> nodeVar129 : f32;
var<private> nodeVar130 : f32;
var<private> nodeVar131 : f32;
var<private> nodeVar132 : f32;
var<private> nodeVar133 : f32;
var<private> nodeVar134 : f32;
var<private> nodeVar135 : f32;
var<private> nodeVar136 : f32;
var<private> nodeVar137 : vec3<f32>;
var<private> totalDiffuse : vec3<f32>;
var<private> directDiffuse : vec3<f32>;
var<private> nodeVar138 : vec3<f32>;
var<private> totalSpecular : vec3<f32>;
var<private> directSpecular : vec3<f32>;
var<private> nodeVar139 : vec3<f32>;
var<private> outgoingLight : vec3<f32>;
var<private> nodeVar140 : vec3<f32>;
var<private> nodeVar141 : vec4<f32>;

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
fn main( @location( 0 ) v_normalViewGeometry : vec3<f32>,
	@location( 1 ) v_positionViewDirection : vec3<f32> ) -> OutputStruct {

	// flow
	// code

	DiffuseColor = vec4<f32>( object.nodeUniform0, 1.0 );
	DiffuseColor.w = ( DiffuseColor.w * object.nodeUniform1 );
	DiffuseColor.w = 1.0;
	Metalness = object.nodeUniform2;
	normalViewGeometry = normalize( v_normalViewGeometry );
	nodeVar0 = max( abs( dpdx( normalViewGeometry ) ), abs( - dpdy( normalViewGeometry ) ) );
	Roughness = min( ( max( object.nodeUniform3, 0.0525 ) + max( max( nodeVar0.x, nodeVar0.y ), nodeVar0.z ) ), 1.0 );
	SpecularColor = vec3<f32>( 0.04, 0.04, 0.04 );
	SpecularColorBlended = mix( vec3<f32>( 0.04, 0.04, 0.04 ), DiffuseColor.xyz, Metalness );
	SpecularF90 = 1.0;
	DiffuseContribution = ( DiffuseColor.xyz * vec3<f32>( ( 1.0 - object.nodeUniform2 ) ) );
	EmissiveColor = ( object.nodeUniform6 * vec3<f32>( object.nodeUniform7 ) );
	NORMAL_normalView = normalViewGeometry;
	normalView = NORMAL_normalView;
	positionViewDirection = normalize( v_positionViewDirection );
	nodeVar1 = dot( normalView, positionViewDirection );
	nodeVar2 = textureSample( nodeUniform8, nodeUniform8_sampler, vec2<f32>( Roughness, clamp( nodeVar1, 0.0, 1.0 ) ) ).xy;
	let dfg = nodeVar2;
	nodeVar3 = ( dfg.x + dfg.y );
	nodeVar4 = ( 1.0 / nodeVar3 );
	nodeVar5 = nodeVar4;
	nodeVar6 = ( nodeVar5 - 1.0 );
	nodeVar7 = ( SpecularColorBlended * vec3<f32>( nodeVar6 ) );
	nodeVar8 = ( nodeVar7 + vec3<f32>( 1.0 ) );
	let multiScatteringCompensation = nodeVar8;
	radiance = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar9 = clamp( roughnessToMip( Roughness ), -2.0, object.nodeUniform10 );
	nodeVar10 = floor( nodeVar9 );
	nodeVar11 = nodeVar10;
	nodeVar12 = normalize( ( render.cameraWorldMatrix * vec4<f32>( normalize( mix( reflect( ( - positionViewDirection ), normalView ), normalView, ( ( ( Roughness * Roughness ) * Roughness ) * Roughness ) ) ), 0.0 ) ).xyz );
	nodeVar13 = getFace( ( object.nodeUniform11 * vec4<f32>( vec3<f32>( nodeVar12.x, ( - nodeVar12.y ), nodeVar12.z ), 1.0 ) ).xyz );
	nodeVar14 = max( ( 4.0 - nodeVar11 ), 0.0 );
	nodeVar11 = max( nodeVar11, 4.0 );
	nodeVar15 = exp2( nodeVar11 );
	nodeVar16 = ( ( getUV( ( object.nodeUniform11 * vec4<f32>( vec3<f32>( nodeVar12.x, ( - nodeVar12.y ), nodeVar12.z ), 1.0 ) ).xyz, nodeVar13 ) * vec2<f32>( ( nodeVar15 - 2.0 ) ) ) + vec2<f32>( 1.0 ) );

	if ( ( nodeVar13 > 2.0 ) ) {

		nodeVar16.y = ( nodeVar16.y + nodeVar15 );
		nodeVar13 = ( nodeVar13 - 3.0 );
		

	}

	nodeVar16.x = ( nodeVar16.x + ( nodeVar13 * nodeVar15 ) );
	nodeVar16.x = ( nodeVar16.x + ( nodeVar14 * ( 3.0 * 16.0 ) ) );
	nodeVar16.y = ( nodeVar16.y + ( 4.0 * ( exp2( object.nodeUniform10 ) - nodeVar15 ) ) );
	nodeVar16.x = ( nodeVar16.x * object.nodeUniform13 );
	nodeVar16.y = ( nodeVar16.y * object.nodeUniform14 );
	nodeVar17 = textureSampleGrad( nodeUniform15, nodeUniform15_sampler, nodeVar16, vec2<f32>( 0.0, 0.0 ), vec2<f32>( 0.0, 0.0 ) );
	nodeVar18 = nodeVar17.xyz;
	nodeVar19 = fract( nodeVar9 );

	if ( ( nodeVar19 != 0.0 ) ) {

		nodeVar20 = ( nodeVar10 + 1.0 );
		nodeVar21 = getFace( ( object.nodeUniform11 * vec4<f32>( vec3<f32>( nodeVar12.x, ( - nodeVar12.y ), nodeVar12.z ), 1.0 ) ).xyz );
		nodeVar22 = max( ( 4.0 - nodeVar20 ), 0.0 );
		nodeVar20 = max( nodeVar20, 4.0 );
		nodeVar23 = exp2( nodeVar20 );
		nodeVar24 = ( ( getUV( ( object.nodeUniform11 * vec4<f32>( vec3<f32>( nodeVar12.x, ( - nodeVar12.y ), nodeVar12.z ), 1.0 ) ).xyz, nodeVar21 ) * vec2<f32>( ( nodeVar23 - 2.0 ) ) ) + vec2<f32>( 1.0 ) );

		if ( ( nodeVar21 > 2.0 ) ) {

			nodeVar24.y = ( nodeVar24.y + nodeVar23 );
			nodeVar21 = ( nodeVar21 - 3.0 );
			

		}

		nodeVar24.x = ( nodeVar24.x + ( nodeVar21 * nodeVar23 ) );
		nodeVar24.x = ( nodeVar24.x + ( nodeVar22 * ( 3.0 * 16.0 ) ) );
		nodeVar24.y = ( nodeVar24.y + ( 4.0 * ( exp2( object.nodeUniform10 ) - nodeVar23 ) ) );
		nodeVar24.x = ( nodeVar24.x * object.nodeUniform13 );
		nodeVar24.y = ( nodeVar24.y * object.nodeUniform14 );
		nodeVar25 = textureSampleGrad( nodeUniform15, nodeUniform15_sampler, nodeVar24, vec2<f32>( 0.0, 0.0 ), vec2<f32>( 0.0, 0.0 ) );
		nodeVar26 = nodeVar25.xyz;
		nodeVar18 = mix( nodeVar18, nodeVar26, nodeVar19 );
		

	}

	nodeVar27 = ( radiance + ( nodeVar18 * vec3<f32>( object.nodeUniform16 ) ) );
	radiance = nodeVar27;
	iblIrradiance = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar28 = clamp( roughnessToMip( 1.0 ), -2.0, object.nodeUniform10 );
	nodeVar29 = floor( nodeVar28 );
	nodeVar30 = nodeVar29;
	normalWorld = normalize( ( vec4<f32>( normalView, 0.0 ) * render.cameraViewMatrix ).xyz );
	nodeVar31 = getFace( ( object.nodeUniform11 * vec4<f32>( vec3<f32>( normalWorld.x, ( - normalWorld.y ), normalWorld.z ), 1.0 ) ).xyz );
	nodeVar32 = max( ( 4.0 - nodeVar30 ), 0.0 );
	nodeVar30 = max( nodeVar30, 4.0 );
	nodeVar33 = exp2( nodeVar30 );
	nodeVar34 = ( ( getUV( ( object.nodeUniform11 * vec4<f32>( vec3<f32>( normalWorld.x, ( - normalWorld.y ), normalWorld.z ), 1.0 ) ).xyz, nodeVar31 ) * vec2<f32>( ( nodeVar33 - 2.0 ) ) ) + vec2<f32>( 1.0 ) );

	if ( ( nodeVar31 > 2.0 ) ) {

		nodeVar34.y = ( nodeVar34.y + nodeVar33 );
		nodeVar31 = ( nodeVar31 - 3.0 );
		

	}

	nodeVar34.x = ( nodeVar34.x + ( nodeVar31 * nodeVar33 ) );
	nodeVar34.x = ( nodeVar34.x + ( nodeVar32 * ( 3.0 * 16.0 ) ) );
	nodeVar34.y = ( nodeVar34.y + ( 4.0 * ( exp2( object.nodeUniform10 ) - nodeVar33 ) ) );
	nodeVar34.x = ( nodeVar34.x * object.nodeUniform13 );
	nodeVar34.y = ( nodeVar34.y * object.nodeUniform14 );
	nodeVar35 = textureSampleGrad( nodeUniform15, nodeUniform15_sampler, nodeVar34, vec2<f32>( 0.0, 0.0 ), vec2<f32>( 0.0, 0.0 ) );
	nodeVar36 = nodeVar35.xyz;
	nodeVar37 = fract( nodeVar28 );

	if ( ( nodeVar37 != 0.0 ) ) {

		nodeVar38 = ( nodeVar29 + 1.0 );
		nodeVar39 = getFace( ( object.nodeUniform11 * vec4<f32>( vec3<f32>( normalWorld.x, ( - normalWorld.y ), normalWorld.z ), 1.0 ) ).xyz );
		nodeVar40 = max( ( 4.0 - nodeVar38 ), 0.0 );
		nodeVar38 = max( nodeVar38, 4.0 );
		nodeVar41 = exp2( nodeVar38 );
		nodeVar42 = ( ( getUV( ( object.nodeUniform11 * vec4<f32>( vec3<f32>( normalWorld.x, ( - normalWorld.y ), normalWorld.z ), 1.0 ) ).xyz, nodeVar39 ) * vec2<f32>( ( nodeVar41 - 2.0 ) ) ) + vec2<f32>( 1.0 ) );

		if ( ( nodeVar39 > 2.0 ) ) {

			nodeVar42.y = ( nodeVar42.y + nodeVar41 );
			nodeVar39 = ( nodeVar39 - 3.0 );
			

		}

		nodeVar42.x = ( nodeVar42.x + ( nodeVar39 * nodeVar41 ) );
		nodeVar42.x = ( nodeVar42.x + ( nodeVar40 * ( 3.0 * 16.0 ) ) );
		nodeVar42.y = ( nodeVar42.y + ( 4.0 * ( exp2( object.nodeUniform10 ) - nodeVar41 ) ) );
		nodeVar42.x = ( nodeVar42.x * object.nodeUniform13 );
		nodeVar42.y = ( nodeVar42.y * object.nodeUniform14 );
		nodeVar43 = textureSampleGrad( nodeUniform15, nodeUniform15_sampler, nodeVar42, vec2<f32>( 0.0, 0.0 ), vec2<f32>( 0.0, 0.0 ) );
		nodeVar44 = nodeVar43.xyz;
		nodeVar36 = mix( nodeVar36, nodeVar44, nodeVar37 );
		

	}

	nodeVar45 = ( iblIrradiance + ( ( nodeVar36 * vec3<f32>( 3.141592653589793 ) ) * vec3<f32>( object.nodeUniform16 ) ) );
	iblIrradiance = nodeVar45;
	nodeVar46 = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar47 = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar48 = ( SpecularColor * vec3<f32>( dfg.x ) );
	nodeVar49 = ( SpecularF90 * dfg.y );
	nodeVar50 = ( nodeVar48 + vec3<f32>( nodeVar49 ) );
	nodeVar51 = ( nodeVar46 + nodeVar50 );
	nodeVar46 = nodeVar51;
	nodeVar52 = ( vec3<f32>( 1.0 ) - SpecularColor );
	nodeVar53 = nodeVar52;
	nodeVar54 = ( nodeVar53 * vec3<f32>( 0.047619 ) );
	nodeVar55 = ( SpecularColor + nodeVar54 );
	nodeVar56 = ( nodeVar50 * nodeVar55 );
	nodeVar57 = ( dfg.x + dfg.y );
	nodeVar58 = ( 1.0 - nodeVar57 );
	nodeVar59 = nodeVar58;
	nodeVar60 = ( vec3<f32>( nodeVar59 ) * nodeVar55 );
	nodeVar61 = ( vec3<f32>( 1.0 ) - nodeVar60 );
	nodeVar62 = nodeVar61;
	nodeVar63 = ( nodeVar56 / nodeVar62 );
	nodeVar64 = ( nodeVar63 * vec3<f32>( nodeVar59 ) );
	nodeVar65 = ( nodeVar47 + nodeVar64 );
	nodeVar47 = nodeVar65;
	irradiance = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar66 = ( DiffuseContribution * vec3<f32>( 0.3183098861837907 ) );
	nodeVar67 = ( irradiance * nodeVar66 );
	nodeVar68 = ( nodeVar46 + nodeVar47 );
	nodeVar69 = ( vec3<f32>( 1.0 ) - nodeVar68 );
	nodeVar70 = nodeVar69;
	nodeVar71 = ( nodeVar67 * nodeVar70 );
	nodeVar72 = nodeVar71;
	indirectDiffuse = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar73 = ( indirectDiffuse + nodeVar72 );
	indirectDiffuse = nodeVar73;
	singleScatteringDielectric = vec3<f32>( 0.0, 0.0, 0.0 );
	multiScatteringDielectric = vec3<f32>( 0.0, 0.0, 0.0 );
	singleScatteringMetallic = vec3<f32>( 0.0, 0.0, 0.0 );
	multiScatteringMetallic = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar74 = ( SpecularColor * vec3<f32>( dfg.x ) );
	nodeVar75 = ( SpecularF90 * dfg.y );
	nodeVar76 = ( nodeVar74 + vec3<f32>( nodeVar75 ) );
	nodeVar77 = ( singleScatteringDielectric + nodeVar76 );
	singleScatteringDielectric = nodeVar77;
	nodeVar78 = ( vec3<f32>( 1.0 ) - SpecularColor );
	nodeVar79 = nodeVar78;
	nodeVar80 = ( nodeVar79 * vec3<f32>( 0.047619 ) );
	nodeVar81 = ( SpecularColor + nodeVar80 );
	nodeVar82 = ( nodeVar76 * nodeVar81 );
	nodeVar83 = ( dfg.x + dfg.y );
	nodeVar84 = ( 1.0 - nodeVar83 );
	nodeVar85 = nodeVar84;
	nodeVar86 = ( vec3<f32>( nodeVar85 ) * nodeVar81 );
	nodeVar87 = ( vec3<f32>( 1.0 ) - nodeVar86 );
	nodeVar88 = nodeVar87;
	nodeVar89 = ( nodeVar82 / nodeVar88 );
	nodeVar90 = ( nodeVar89 * vec3<f32>( nodeVar85 ) );
	nodeVar91 = ( multiScatteringDielectric + nodeVar90 );
	multiScatteringDielectric = nodeVar91;
	nodeVar92 = ( DiffuseColor.xyz * vec3<f32>( dfg.x ) );
	nodeVar93 = ( SpecularF90 * dfg.y );
	nodeVar94 = ( nodeVar92 + vec3<f32>( nodeVar93 ) );
	nodeVar95 = ( singleScatteringMetallic + nodeVar94 );
	singleScatteringMetallic = nodeVar95;
	nodeVar96 = ( vec3<f32>( 1.0 ) - DiffuseColor.xyz );
	nodeVar97 = nodeVar96;
	nodeVar98 = ( nodeVar97 * vec3<f32>( 0.047619 ) );
	nodeVar99 = ( DiffuseColor.xyz + nodeVar98 );
	nodeVar100 = ( nodeVar94 * nodeVar99 );
	nodeVar101 = ( dfg.x + dfg.y );
	nodeVar102 = ( 1.0 - nodeVar101 );
	nodeVar103 = nodeVar102;
	nodeVar104 = ( vec3<f32>( nodeVar103 ) * nodeVar99 );
	nodeVar105 = ( vec3<f32>( 1.0 ) - nodeVar104 );
	nodeVar106 = nodeVar105;
	nodeVar107 = ( nodeVar100 / nodeVar106 );
	nodeVar108 = ( nodeVar107 * vec3<f32>( nodeVar103 ) );
	nodeVar109 = ( multiScatteringMetallic + nodeVar108 );
	multiScatteringMetallic = nodeVar109;
	nodeVar110 = mix( singleScatteringDielectric, singleScatteringMetallic, Metalness );
	nodeVar111 = ( radiance * nodeVar110 );
	nodeVar112 = mix( multiScatteringDielectric, multiScatteringMetallic, Metalness );
	nodeVar113 = ( iblIrradiance * vec3<f32>( 0.3183098861837907 ) );
	nodeVar114 = ( nodeVar112 * nodeVar113 );
	nodeVar115 = ( nodeVar111 + nodeVar114 );
	nodeVar116 = nodeVar115;
	nodeVar117 = ( singleScatteringDielectric + multiScatteringDielectric );
	nodeVar118 = ( vec3<f32>( 1.0 ) - nodeVar117 );
	nodeVar119 = nodeVar118;
	nodeVar120 = ( DiffuseContribution * nodeVar119 );
	nodeVar121 = ( nodeVar120 * nodeVar113 );
	nodeVar122 = nodeVar121;
	indirectSpecular = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar123 = ( indirectSpecular + nodeVar116 );
	indirectSpecular = nodeVar123;
	nodeVar124 = ( indirectDiffuse + nodeVar122 );
	indirectDiffuse = nodeVar124;
	ambientOcclusion = 1.0;
	nodeVar125 = ( indirectDiffuse * vec3<f32>( ambientOcclusion ) );
	indirectDiffuse = nodeVar125;
	nodeVar126 = dot( normalView, positionViewDirection );
	nodeVar127 = ( clamp( nodeVar126, 0.0, 1.0 ) + ambientOcclusion );
	nodeVar128 = ( Roughness * -16.0 );
	nodeVar129 = ( 1.0 - nodeVar128 );
	nodeVar130 = nodeVar129;
	nodeVar131 = ( - nodeVar130 );
	nodeVar132 = exp2( nodeVar131 );
	nodeVar133 = pow( nodeVar127, nodeVar132 );
	nodeVar134 = ( 1.0 - nodeVar133 );
	nodeVar135 = nodeVar134;
	nodeVar136 = ( ambientOcclusion - nodeVar135 );
	nodeVar137 = ( indirectSpecular * vec3<f32>( clamp( nodeVar136, 0.0, 1.0 ) ) );
	indirectSpecular = nodeVar137;
	directDiffuse = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar138 = ( directDiffuse + indirectDiffuse );
	totalDiffuse = nodeVar138;
	directSpecular = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar139 = ( directSpecular + indirectSpecular );
	totalSpecular = nodeVar139;
	nodeVar140 = ( totalDiffuse + totalSpecular );
	outgoingLight = nodeVar140;
	nodeVar141 = max( vec4<f32>( ( outgoingLight + EmissiveColor ), DiffuseColor.w ), vec4<f32>( 0.0 ) );
	Output = nodeVar141;

	// result

	output.color = nodeVar141;

	return output;

}
