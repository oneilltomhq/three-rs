// Three.js r186 - Node System

// global
diagnostic( off, derivative_uniformity );


// structs

struct OutputStruct {
	@location( 0 ) color: vec4<f32>
};
var<private> output : OutputStruct;

// uniforms
@binding( 1 ) @group( 1 ) var nodeUniform11_sampler : sampler;
@binding( 2 ) @group( 1 ) var nodeUniform11 : texture_2d<f32>;
@binding( 3 ) @group( 1 ) var nodeUniform22_sampler : sampler;
@binding( 4 ) @group( 1 ) var nodeUniform22 : texture_2d<f32>;

struct objectStruct {
	nodeUniform0 : vec3<f32>,
	nodeUniform1 : f32,
	nodeUniform2 : f32,
	nodeUniform3 : f32,
	nodeUniform5 : mat3x3<f32>,
	nodeUniform6 : f32,
	nodeUniform7 : vec3<f32>,
	nodeUniform8 : f32,
	nodeUniform9 : vec3<f32>,
	nodeUniform10 : f32,
	nodeUniform12 : mat4x4<f32>,
	nodeUniform17 : f32,
	nodeUniform18 : mat4x4<f32>,
	nodeUniform20 : f32,
	nodeUniform21 : f32,
	nodeUniform23 : f32
};
@binding( 0 ) @group( 1 )
var<uniform> object : objectStruct;

struct renderStruct {
	cameraProjectionMatrix : mat4x4<f32>,
	cameraViewMatrix : mat4x4<f32>,
	nodeUniform16 : vec3<f32>,
	nodeUniform14 : vec3<f32>,
	nodeUniform15 : vec3<f32>,
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
var<private> IOR : f32;
var<private> SpecularColor : vec3<f32>;
var<private> nodeVar1 : f32;
var<private> SpecularColorBlended : vec3<f32>;
var<private> SpecularF90 : f32;
var<private> DiffuseContribution : vec3<f32>;
var<private> EmissiveColor : vec3<f32>;
var<private> Output : vec4<f32>;
var<private> NORMAL_normalView : vec3<f32>;
var<private> normalView : vec3<f32>;
var<private> positionViewDirection : vec3<f32>;
var<private> nodeVar2 : f32;
var<private> nodeVar3 : vec2<f32>;
var<private> nodeVar4 : f32;
var<private> nodeVar5 : f32;
var<private> nodeVar6 : f32;
var<private> nodeVar7 : f32;
var<private> nodeVar8 : vec3<f32>;
var<private> nodeVar9 : vec3<f32>;
var<private> nodeVar10 : vec3<f32>;
var<private> nodeVar11 : vec4<f32>;
var<private> nodeVar12 : vec4<f32>;
var<private> nodeVar13 : vec3<f32>;
var<private> nodeVar14 : vec3<f32>;
var<private> nodeVar15 : f32;
var<private> nodeVar16 : vec3<f32>;
var<private> nodeVar17 : vec3<f32>;
var<private> directDiffuse : vec3<f32>;
var<private> nodeVar18 : vec3<f32>;
var<private> nodeVar19 : vec3<f32>;
var<private> nodeVar20 : vec3<f32>;
var<private> nodeVar21 : vec3<f32>;
var<private> nodeVar22 : f32;
var<private> nodeVar23 : f32;
var<private> nodeVar24 : f32;
var<private> nodeVar25 : vec3<f32>;
var<private> nodeVar26 : vec3<f32>;
var<private> nodeVar27 : vec3<f32>;
var<private> nodeVar28 : vec3<f32>;
var<private> nodeVar29 : vec3<f32>;
var<private> directSpecular : vec3<f32>;
var<private> nodeVar30 : vec3<f32>;
var<private> nodeVar31 : f32;
var<private> nodeVar32 : f32;
var<private> nodeVar33 : f32;
var<private> nodeVar34 : vec3<f32>;
var<private> nodeVar35 : vec3<f32>;
var<private> nodeVar36 : vec3<f32>;
var<private> nodeVar37 : vec3<f32>;
var<private> radiance : vec3<f32>;
var<private> nodeVar38 : f32;
var<private> nodeVar39 : f32;
var<private> nodeVar40 : f32;
var<private> nodeVar41 : vec3<f32>;
var<private> nodeVar42 : f32;
var<private> nodeVar43 : f32;
var<private> nodeVar44 : f32;
var<private> nodeVar45 : vec2<f32>;
var<private> nodeVar46 : vec4<f32>;
var<private> nodeVar47 : vec3<f32>;
var<private> nodeVar48 : f32;
var<private> nodeVar49 : f32;
var<private> nodeVar50 : f32;
var<private> nodeVar51 : f32;
var<private> nodeVar52 : f32;
var<private> nodeVar53 : vec2<f32>;
var<private> nodeVar54 : vec4<f32>;
var<private> nodeVar55 : vec3<f32>;
var<private> nodeVar56 : vec3<f32>;
var<private> iblIrradiance : vec3<f32>;
var<private> nodeVar57 : f32;
var<private> nodeVar58 : f32;
var<private> nodeVar59 : f32;
var<private> normalWorld : vec3<f32>;
var<private> nodeVar60 : f32;
var<private> nodeVar61 : f32;
var<private> nodeVar62 : f32;
var<private> nodeVar63 : vec2<f32>;
var<private> nodeVar64 : vec4<f32>;
var<private> nodeVar65 : vec3<f32>;
var<private> nodeVar66 : f32;
var<private> nodeVar67 : f32;
var<private> nodeVar68 : f32;
var<private> nodeVar69 : f32;
var<private> nodeVar70 : f32;
var<private> nodeVar71 : vec2<f32>;
var<private> nodeVar72 : vec4<f32>;
var<private> nodeVar73 : vec3<f32>;
var<private> nodeVar74 : vec3<f32>;
var<private> nodeVar75 : vec3<f32>;
var<private> nodeVar76 : vec3<f32>;
var<private> nodeVar77 : vec3<f32>;
var<private> nodeVar78 : f32;
var<private> nodeVar79 : vec3<f32>;
var<private> nodeVar80 : vec3<f32>;
var<private> nodeVar81 : vec3<f32>;
var<private> nodeVar82 : vec3<f32>;
var<private> nodeVar83 : vec3<f32>;
var<private> nodeVar84 : vec3<f32>;
var<private> nodeVar85 : vec3<f32>;
var<private> nodeVar86 : f32;
var<private> nodeVar87 : f32;
var<private> nodeVar88 : f32;
var<private> nodeVar89 : vec3<f32>;
var<private> nodeVar90 : vec3<f32>;
var<private> nodeVar91 : vec3<f32>;
var<private> nodeVar92 : vec3<f32>;
var<private> nodeVar93 : vec3<f32>;
var<private> nodeVar94 : vec3<f32>;
var<private> irradiance : vec3<f32>;
var<private> nodeVar95 : vec3<f32>;
var<private> nodeVar96 : vec3<f32>;
var<private> nodeVar97 : vec3<f32>;
var<private> nodeVar98 : vec3<f32>;
var<private> nodeVar99 : vec3<f32>;
var<private> nodeVar100 : vec3<f32>;
var<private> nodeVar101 : vec3<f32>;
var<private> indirectDiffuse : vec3<f32>;
var<private> nodeVar102 : vec3<f32>;
var<private> singleScatteringDielectric : vec3<f32>;
var<private> multiScatteringDielectric : vec3<f32>;
var<private> singleScatteringMetallic : vec3<f32>;
var<private> multiScatteringMetallic : vec3<f32>;
var<private> nodeVar103 : vec3<f32>;
var<private> nodeVar104 : f32;
var<private> nodeVar105 : vec3<f32>;
var<private> nodeVar106 : vec3<f32>;
var<private> nodeVar107 : vec3<f32>;
var<private> nodeVar108 : vec3<f32>;
var<private> nodeVar109 : vec3<f32>;
var<private> nodeVar110 : vec3<f32>;
var<private> nodeVar111 : vec3<f32>;
var<private> nodeVar112 : f32;
var<private> nodeVar113 : f32;
var<private> nodeVar114 : f32;
var<private> nodeVar115 : vec3<f32>;
var<private> nodeVar116 : vec3<f32>;
var<private> nodeVar117 : vec3<f32>;
var<private> nodeVar118 : vec3<f32>;
var<private> nodeVar119 : vec3<f32>;
var<private> nodeVar120 : vec3<f32>;
var<private> nodeVar121 : vec3<f32>;
var<private> nodeVar122 : f32;
var<private> nodeVar123 : vec3<f32>;
var<private> nodeVar124 : vec3<f32>;
var<private> nodeVar125 : vec3<f32>;
var<private> nodeVar126 : vec3<f32>;
var<private> nodeVar127 : vec3<f32>;
var<private> nodeVar128 : vec3<f32>;
var<private> nodeVar129 : vec3<f32>;
var<private> nodeVar130 : f32;
var<private> nodeVar131 : f32;
var<private> nodeVar132 : f32;
var<private> nodeVar133 : vec3<f32>;
var<private> nodeVar134 : vec3<f32>;
var<private> nodeVar135 : vec3<f32>;
var<private> nodeVar136 : vec3<f32>;
var<private> nodeVar137 : vec3<f32>;
var<private> nodeVar138 : vec3<f32>;
var<private> nodeVar139 : vec3<f32>;
var<private> nodeVar140 : vec3<f32>;
var<private> nodeVar141 : vec3<f32>;
var<private> nodeVar142 : vec3<f32>;
var<private> nodeVar143 : vec3<f32>;
var<private> nodeVar144 : vec3<f32>;
var<private> nodeVar145 : vec3<f32>;
var<private> nodeVar146 : vec3<f32>;
var<private> nodeVar147 : vec3<f32>;
var<private> nodeVar148 : vec3<f32>;
var<private> nodeVar149 : vec3<f32>;
var<private> nodeVar150 : vec3<f32>;
var<private> nodeVar151 : vec3<f32>;
var<private> indirectSpecular : vec3<f32>;
var<private> nodeVar152 : vec3<f32>;
var<private> nodeVar153 : vec3<f32>;
var<private> ambientOcclusion : f32;
var<private> nodeVar154 : vec3<f32>;
var<private> nodeVar155 : f32;
var<private> nodeVar156 : f32;
var<private> nodeVar157 : f32;
var<private> nodeVar158 : f32;
var<private> nodeVar159 : f32;
var<private> nodeVar160 : f32;
var<private> nodeVar161 : f32;
var<private> nodeVar162 : f32;
var<private> nodeVar163 : f32;
var<private> nodeVar164 : f32;
var<private> nodeVar165 : f32;
var<private> nodeVar166 : vec3<f32>;
var<private> totalDiffuse : vec3<f32>;
var<private> nodeVar167 : vec3<f32>;
var<private> totalSpecular : vec3<f32>;
var<private> nodeVar168 : vec3<f32>;
var<private> outgoingLight : vec3<f32>;
var<private> nodeVar169 : vec3<f32>;
var<private> nodeVar170 : vec4<f32>;

// codes
fn V_GGX_SmithCorrelated ( alpha : f32, dotNL : f32, dotNV : f32 ) -> f32 {

	var nodeVar0 : f32;

	nodeVar0 = ( alpha * alpha );

	return ( 0.5 / max( ( ( dotNL * sqrt( ( nodeVar0 + ( ( 1.0 - nodeVar0 ) * ( dotNV * dotNV ) ) ) ) ) + ( dotNV * sqrt( ( nodeVar0 + ( ( 1.0 - nodeVar0 ) * ( dotNL * dotNL ) ) ) ) ) ), 0.000001 ) );

}


fn D_GGX ( alpha : f32, dotNH : f32 ) -> f32 {

	var nodeVar0 : f32;
	var nodeVar1 : f32;

	nodeVar0 = ( alpha * alpha );
	nodeVar1 = ( 1.0 - ( ( dotNH * dotNH ) * ( 1.0 - nodeVar0 ) ) );

	return ( ( nodeVar0 / ( nodeVar1 * nodeVar1 ) ) * 0.3183098861837907 );

}


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
	IOR = object.nodeUniform6;
	nodeVar1 = ( ( IOR - 1.0 ) / ( IOR + 1.0 ) );
	SpecularColor = ( min( ( vec3<f32>( ( nodeVar1 * nodeVar1 ) ) * object.nodeUniform7 ), vec3<f32>( 1.0, 1.0, 1.0 ) ) * vec3<f32>( object.nodeUniform8 ) );
	SpecularColorBlended = mix( SpecularColor, DiffuseColor.xyz, Metalness );
	SpecularF90 = mix( object.nodeUniform8, 1.0, Metalness );
	DiffuseContribution = ( DiffuseColor.xyz * vec3<f32>( ( 1.0 - object.nodeUniform2 ) ) );
	EmissiveColor = ( object.nodeUniform9 * vec3<f32>( object.nodeUniform10 ) );
	NORMAL_normalView = normalViewGeometry;
	normalView = NORMAL_normalView;
	positionViewDirection = normalize( v_positionViewDirection );
	nodeVar2 = dot( normalView, positionViewDirection );
	nodeVar3 = textureSample( nodeUniform11, nodeUniform11_sampler, vec2<f32>( Roughness, clamp( nodeVar2, 0.0, 1.0 ) ) ).xy;
	let dfg = nodeVar3;
	nodeVar4 = ( dfg.x + dfg.y );
	nodeVar5 = ( 1.0 / nodeVar4 );
	nodeVar6 = nodeVar5;
	nodeVar7 = ( nodeVar6 - 1.0 );
	nodeVar8 = ( SpecularColorBlended * vec3<f32>( nodeVar7 ) );
	nodeVar9 = ( nodeVar8 + vec3<f32>( 1.0 ) );
	let multiScatteringCompensation = nodeVar9;
	nodeVar10 = ( render.nodeUniform14 - render.nodeUniform15 );
	nodeVar11 = vec4<f32>( nodeVar10, 0.0 );
	nodeVar12 = ( render.cameraViewMatrix * nodeVar11 );
	nodeVar13 = normalize( nodeVar12.xyz );
	nodeVar14 = nodeVar13;
	nodeVar15 = dot( normalView, nodeVar14 );
	nodeVar16 = ( vec3<f32>( clamp( nodeVar15, 0.0, 1.0 ) ) * render.nodeUniform16 );
	nodeVar17 = nodeVar16;
	directDiffuse = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar18 = ( DiffuseContribution * vec3<f32>( 0.3183098861837907 ) );
	nodeVar19 = ( nodeVar17 * nodeVar18 );
	nodeVar20 = ( nodeVar14 + positionViewDirection );
	nodeVar21 = normalize( nodeVar20 );
	nodeVar22 = dot( positionViewDirection, nodeVar21 );
	nodeVar23 = clamp( nodeVar22, 0.0, 1.0 );
	nodeVar24 = exp2( ( ( ( nodeVar23 * -5.55473 ) - 6.98316 ) * nodeVar23 ) );
	nodeVar25 = ( ( SpecularColor * vec3<f32>( ( 1.0 - nodeVar24 ) ) ) + vec3<f32>( ( SpecularF90 * nodeVar24 ) ) );
	nodeVar26 = ( vec3<f32>( 1.0 ) - nodeVar25 );
	nodeVar27 = nodeVar26;
	nodeVar28 = ( nodeVar19 * nodeVar27 );
	nodeVar29 = ( directDiffuse + nodeVar28 );
	directDiffuse = nodeVar29;
	directSpecular = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar30 = normalize( ( nodeVar14 + positionViewDirection ) );
	nodeVar31 = clamp( dot( positionViewDirection, nodeVar30 ), 0.0, 1.0 );
	nodeVar32 = exp2( ( ( ( nodeVar31 * -5.55473 ) - 6.98316 ) * nodeVar31 ) );
	nodeVar33 = ( Roughness * Roughness );
	nodeVar34 = ( ( ( ( SpecularColorBlended * vec3<f32>( ( 1.0 - nodeVar32 ) ) ) + vec3<f32>( ( 1.0 * nodeVar32 ) ) ) * vec3<f32>( V_GGX_SmithCorrelated( nodeVar33, clamp( dot( normalView, nodeVar14 ), 0.0, 1.0 ), clamp( dot( normalView, positionViewDirection ), 0.0, 1.0 ) ) ) ) * vec3<f32>( D_GGX( nodeVar33, clamp( dot( normalView, nodeVar30 ), 0.0, 1.0 ) ) ) );
	nodeVar35 = ( nodeVar17 * nodeVar34 );
	nodeVar36 = ( nodeVar35 * multiScatteringCompensation );
	nodeVar37 = ( directSpecular + nodeVar36 );
	directSpecular = nodeVar37;
	radiance = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar38 = clamp( roughnessToMip( Roughness ), -2.0, object.nodeUniform17 );
	nodeVar39 = floor( nodeVar38 );
	nodeVar40 = nodeVar39;
	nodeVar41 = normalize( ( render.cameraWorldMatrix * vec4<f32>( normalize( mix( reflect( ( - positionViewDirection ), normalView ), normalView, ( ( ( Roughness * Roughness ) * Roughness ) * Roughness ) ) ), 0.0 ) ).xyz );
	nodeVar42 = getFace( ( object.nodeUniform18 * vec4<f32>( vec3<f32>( nodeVar41.x, ( - nodeVar41.y ), nodeVar41.z ), 1.0 ) ).xyz );
	nodeVar43 = max( ( 4.0 - nodeVar40 ), 0.0 );
	nodeVar40 = max( nodeVar40, 4.0 );
	nodeVar44 = exp2( nodeVar40 );
	nodeVar45 = ( ( getUV( ( object.nodeUniform18 * vec4<f32>( vec3<f32>( nodeVar41.x, ( - nodeVar41.y ), nodeVar41.z ), 1.0 ) ).xyz, nodeVar42 ) * vec2<f32>( ( nodeVar44 - 2.0 ) ) ) + vec2<f32>( 1.0 ) );

	if ( ( nodeVar42 > 2.0 ) ) {

		nodeVar45.y = ( nodeVar45.y + nodeVar44 );
		nodeVar42 = ( nodeVar42 - 3.0 );
		

	}

	nodeVar45.x = ( nodeVar45.x + ( nodeVar42 * nodeVar44 ) );
	nodeVar45.x = ( nodeVar45.x + ( nodeVar43 * ( 3.0 * 16.0 ) ) );
	nodeVar45.y = ( nodeVar45.y + ( 4.0 * ( exp2( object.nodeUniform17 ) - nodeVar44 ) ) );
	nodeVar45.x = ( nodeVar45.x * object.nodeUniform20 );
	nodeVar45.y = ( nodeVar45.y * object.nodeUniform21 );
	nodeVar46 = textureSampleGrad( nodeUniform22, nodeUniform22_sampler, nodeVar45, vec2<f32>( 0.0, 0.0 ), vec2<f32>( 0.0, 0.0 ) );
	nodeVar47 = nodeVar46.xyz;
	nodeVar48 = fract( nodeVar38 );

	if ( ( nodeVar48 != 0.0 ) ) {

		nodeVar49 = ( nodeVar39 + 1.0 );
		nodeVar50 = getFace( ( object.nodeUniform18 * vec4<f32>( vec3<f32>( nodeVar41.x, ( - nodeVar41.y ), nodeVar41.z ), 1.0 ) ).xyz );
		nodeVar51 = max( ( 4.0 - nodeVar49 ), 0.0 );
		nodeVar49 = max( nodeVar49, 4.0 );
		nodeVar52 = exp2( nodeVar49 );
		nodeVar53 = ( ( getUV( ( object.nodeUniform18 * vec4<f32>( vec3<f32>( nodeVar41.x, ( - nodeVar41.y ), nodeVar41.z ), 1.0 ) ).xyz, nodeVar50 ) * vec2<f32>( ( nodeVar52 - 2.0 ) ) ) + vec2<f32>( 1.0 ) );

		if ( ( nodeVar50 > 2.0 ) ) {

			nodeVar53.y = ( nodeVar53.y + nodeVar52 );
			nodeVar50 = ( nodeVar50 - 3.0 );
			

		}

		nodeVar53.x = ( nodeVar53.x + ( nodeVar50 * nodeVar52 ) );
		nodeVar53.x = ( nodeVar53.x + ( nodeVar51 * ( 3.0 * 16.0 ) ) );
		nodeVar53.y = ( nodeVar53.y + ( 4.0 * ( exp2( object.nodeUniform17 ) - nodeVar52 ) ) );
		nodeVar53.x = ( nodeVar53.x * object.nodeUniform20 );
		nodeVar53.y = ( nodeVar53.y * object.nodeUniform21 );
		nodeVar54 = textureSampleGrad( nodeUniform22, nodeUniform22_sampler, nodeVar53, vec2<f32>( 0.0, 0.0 ), vec2<f32>( 0.0, 0.0 ) );
		nodeVar55 = nodeVar54.xyz;
		nodeVar47 = mix( nodeVar47, nodeVar55, nodeVar48 );
		

	}

	nodeVar56 = ( radiance + ( nodeVar47 * vec3<f32>( object.nodeUniform23 ) ) );
	radiance = nodeVar56;
	iblIrradiance = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar57 = clamp( roughnessToMip( 1.0 ), -2.0, object.nodeUniform17 );
	nodeVar58 = floor( nodeVar57 );
	nodeVar59 = nodeVar58;
	normalWorld = normalize( ( vec4<f32>( normalView, 0.0 ) * render.cameraViewMatrix ).xyz );
	nodeVar60 = getFace( ( object.nodeUniform18 * vec4<f32>( vec3<f32>( normalWorld.x, ( - normalWorld.y ), normalWorld.z ), 1.0 ) ).xyz );
	nodeVar61 = max( ( 4.0 - nodeVar59 ), 0.0 );
	nodeVar59 = max( nodeVar59, 4.0 );
	nodeVar62 = exp2( nodeVar59 );
	nodeVar63 = ( ( getUV( ( object.nodeUniform18 * vec4<f32>( vec3<f32>( normalWorld.x, ( - normalWorld.y ), normalWorld.z ), 1.0 ) ).xyz, nodeVar60 ) * vec2<f32>( ( nodeVar62 - 2.0 ) ) ) + vec2<f32>( 1.0 ) );

	if ( ( nodeVar60 > 2.0 ) ) {

		nodeVar63.y = ( nodeVar63.y + nodeVar62 );
		nodeVar60 = ( nodeVar60 - 3.0 );
		

	}

	nodeVar63.x = ( nodeVar63.x + ( nodeVar60 * nodeVar62 ) );
	nodeVar63.x = ( nodeVar63.x + ( nodeVar61 * ( 3.0 * 16.0 ) ) );
	nodeVar63.y = ( nodeVar63.y + ( 4.0 * ( exp2( object.nodeUniform17 ) - nodeVar62 ) ) );
	nodeVar63.x = ( nodeVar63.x * object.nodeUniform20 );
	nodeVar63.y = ( nodeVar63.y * object.nodeUniform21 );
	nodeVar64 = textureSampleGrad( nodeUniform22, nodeUniform22_sampler, nodeVar63, vec2<f32>( 0.0, 0.0 ), vec2<f32>( 0.0, 0.0 ) );
	nodeVar65 = nodeVar64.xyz;
	nodeVar66 = fract( nodeVar57 );

	if ( ( nodeVar66 != 0.0 ) ) {

		nodeVar67 = ( nodeVar58 + 1.0 );
		nodeVar68 = getFace( ( object.nodeUniform18 * vec4<f32>( vec3<f32>( normalWorld.x, ( - normalWorld.y ), normalWorld.z ), 1.0 ) ).xyz );
		nodeVar69 = max( ( 4.0 - nodeVar67 ), 0.0 );
		nodeVar67 = max( nodeVar67, 4.0 );
		nodeVar70 = exp2( nodeVar67 );
		nodeVar71 = ( ( getUV( ( object.nodeUniform18 * vec4<f32>( vec3<f32>( normalWorld.x, ( - normalWorld.y ), normalWorld.z ), 1.0 ) ).xyz, nodeVar68 ) * vec2<f32>( ( nodeVar70 - 2.0 ) ) ) + vec2<f32>( 1.0 ) );

		if ( ( nodeVar68 > 2.0 ) ) {

			nodeVar71.y = ( nodeVar71.y + nodeVar70 );
			nodeVar68 = ( nodeVar68 - 3.0 );
			

		}

		nodeVar71.x = ( nodeVar71.x + ( nodeVar68 * nodeVar70 ) );
		nodeVar71.x = ( nodeVar71.x + ( nodeVar69 * ( 3.0 * 16.0 ) ) );
		nodeVar71.y = ( nodeVar71.y + ( 4.0 * ( exp2( object.nodeUniform17 ) - nodeVar70 ) ) );
		nodeVar71.x = ( nodeVar71.x * object.nodeUniform20 );
		nodeVar71.y = ( nodeVar71.y * object.nodeUniform21 );
		nodeVar72 = textureSampleGrad( nodeUniform22, nodeUniform22_sampler, nodeVar71, vec2<f32>( 0.0, 0.0 ), vec2<f32>( 0.0, 0.0 ) );
		nodeVar73 = nodeVar72.xyz;
		nodeVar65 = mix( nodeVar65, nodeVar73, nodeVar66 );
		

	}

	nodeVar74 = ( iblIrradiance + ( ( nodeVar65 * vec3<f32>( 3.141592653589793 ) ) * vec3<f32>( object.nodeUniform23 ) ) );
	iblIrradiance = nodeVar74;
	nodeVar75 = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar76 = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar77 = ( SpecularColor * vec3<f32>( dfg.x ) );
	nodeVar78 = ( SpecularF90 * dfg.y );
	nodeVar79 = ( nodeVar77 + vec3<f32>( nodeVar78 ) );
	nodeVar80 = ( nodeVar75 + nodeVar79 );
	nodeVar75 = nodeVar80;
	nodeVar81 = ( vec3<f32>( 1.0 ) - SpecularColor );
	nodeVar82 = nodeVar81;
	nodeVar83 = ( nodeVar82 * vec3<f32>( 0.047619 ) );
	nodeVar84 = ( SpecularColor + nodeVar83 );
	nodeVar85 = ( nodeVar79 * nodeVar84 );
	nodeVar86 = ( dfg.x + dfg.y );
	nodeVar87 = ( 1.0 - nodeVar86 );
	nodeVar88 = nodeVar87;
	nodeVar89 = ( vec3<f32>( nodeVar88 ) * nodeVar84 );
	nodeVar90 = ( vec3<f32>( 1.0 ) - nodeVar89 );
	nodeVar91 = nodeVar90;
	nodeVar92 = ( nodeVar85 / nodeVar91 );
	nodeVar93 = ( nodeVar92 * vec3<f32>( nodeVar88 ) );
	nodeVar94 = ( nodeVar76 + nodeVar93 );
	nodeVar76 = nodeVar94;
	irradiance = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar95 = ( DiffuseContribution * vec3<f32>( 0.3183098861837907 ) );
	nodeVar96 = ( irradiance * nodeVar95 );
	nodeVar97 = ( nodeVar75 + nodeVar76 );
	nodeVar98 = ( vec3<f32>( 1.0 ) - nodeVar97 );
	nodeVar99 = nodeVar98;
	nodeVar100 = ( nodeVar96 * nodeVar99 );
	nodeVar101 = nodeVar100;
	indirectDiffuse = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar102 = ( indirectDiffuse + nodeVar101 );
	indirectDiffuse = nodeVar102;
	singleScatteringDielectric = vec3<f32>( 0.0, 0.0, 0.0 );
	multiScatteringDielectric = vec3<f32>( 0.0, 0.0, 0.0 );
	singleScatteringMetallic = vec3<f32>( 0.0, 0.0, 0.0 );
	multiScatteringMetallic = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar103 = ( SpecularColor * vec3<f32>( dfg.x ) );
	nodeVar104 = ( SpecularF90 * dfg.y );
	nodeVar105 = ( nodeVar103 + vec3<f32>( nodeVar104 ) );
	nodeVar106 = ( singleScatteringDielectric + nodeVar105 );
	singleScatteringDielectric = nodeVar106;
	nodeVar107 = ( vec3<f32>( 1.0 ) - SpecularColor );
	nodeVar108 = nodeVar107;
	nodeVar109 = ( nodeVar108 * vec3<f32>( 0.047619 ) );
	nodeVar110 = ( SpecularColor + nodeVar109 );
	nodeVar111 = ( nodeVar105 * nodeVar110 );
	nodeVar112 = ( dfg.x + dfg.y );
	nodeVar113 = ( 1.0 - nodeVar112 );
	nodeVar114 = nodeVar113;
	nodeVar115 = ( vec3<f32>( nodeVar114 ) * nodeVar110 );
	nodeVar116 = ( vec3<f32>( 1.0 ) - nodeVar115 );
	nodeVar117 = nodeVar116;
	nodeVar118 = ( nodeVar111 / nodeVar117 );
	nodeVar119 = ( nodeVar118 * vec3<f32>( nodeVar114 ) );
	nodeVar120 = ( multiScatteringDielectric + nodeVar119 );
	multiScatteringDielectric = nodeVar120;
	nodeVar121 = ( DiffuseColor.xyz * vec3<f32>( dfg.x ) );
	nodeVar122 = ( SpecularF90 * dfg.y );
	nodeVar123 = ( nodeVar121 + vec3<f32>( nodeVar122 ) );
	nodeVar124 = ( singleScatteringMetallic + nodeVar123 );
	singleScatteringMetallic = nodeVar124;
	nodeVar125 = ( vec3<f32>( 1.0 ) - DiffuseColor.xyz );
	nodeVar126 = nodeVar125;
	nodeVar127 = ( nodeVar126 * vec3<f32>( 0.047619 ) );
	nodeVar128 = ( DiffuseColor.xyz + nodeVar127 );
	nodeVar129 = ( nodeVar123 * nodeVar128 );
	nodeVar130 = ( dfg.x + dfg.y );
	nodeVar131 = ( 1.0 - nodeVar130 );
	nodeVar132 = nodeVar131;
	nodeVar133 = ( vec3<f32>( nodeVar132 ) * nodeVar128 );
	nodeVar134 = ( vec3<f32>( 1.0 ) - nodeVar133 );
	nodeVar135 = nodeVar134;
	nodeVar136 = ( nodeVar129 / nodeVar135 );
	nodeVar137 = ( nodeVar136 * vec3<f32>( nodeVar132 ) );
	nodeVar138 = ( multiScatteringMetallic + nodeVar137 );
	multiScatteringMetallic = nodeVar138;
	nodeVar139 = mix( singleScatteringDielectric, singleScatteringMetallic, Metalness );
	nodeVar140 = ( radiance * nodeVar139 );
	nodeVar141 = mix( multiScatteringDielectric, multiScatteringMetallic, Metalness );
	nodeVar142 = ( iblIrradiance * vec3<f32>( 0.3183098861837907 ) );
	nodeVar143 = ( nodeVar141 * nodeVar142 );
	nodeVar144 = ( nodeVar140 + nodeVar143 );
	nodeVar145 = nodeVar144;
	nodeVar146 = ( singleScatteringDielectric + multiScatteringDielectric );
	nodeVar147 = ( vec3<f32>( 1.0 ) - nodeVar146 );
	nodeVar148 = nodeVar147;
	nodeVar149 = ( DiffuseContribution * nodeVar148 );
	nodeVar150 = ( nodeVar149 * nodeVar142 );
	nodeVar151 = nodeVar150;
	indirectSpecular = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar152 = ( indirectSpecular + nodeVar145 );
	indirectSpecular = nodeVar152;
	nodeVar153 = ( indirectDiffuse + nodeVar151 );
	indirectDiffuse = nodeVar153;
	ambientOcclusion = 1.0;
	nodeVar154 = ( indirectDiffuse * vec3<f32>( ambientOcclusion ) );
	indirectDiffuse = nodeVar154;
	nodeVar155 = dot( normalView, positionViewDirection );
	nodeVar156 = ( clamp( nodeVar155, 0.0, 1.0 ) + ambientOcclusion );
	nodeVar157 = ( Roughness * -16.0 );
	nodeVar158 = ( 1.0 - nodeVar157 );
	nodeVar159 = nodeVar158;
	nodeVar160 = ( - nodeVar159 );
	nodeVar161 = exp2( nodeVar160 );
	nodeVar162 = pow( nodeVar156, nodeVar161 );
	nodeVar163 = ( 1.0 - nodeVar162 );
	nodeVar164 = nodeVar163;
	nodeVar165 = ( ambientOcclusion - nodeVar164 );
	nodeVar166 = ( indirectSpecular * vec3<f32>( clamp( nodeVar165, 0.0, 1.0 ) ) );
	indirectSpecular = nodeVar166;
	nodeVar167 = ( directDiffuse + indirectDiffuse );
	totalDiffuse = nodeVar167;
	nodeVar168 = ( directSpecular + indirectSpecular );
	totalSpecular = nodeVar168;
	nodeVar169 = ( totalDiffuse + totalSpecular );
	outgoingLight = nodeVar169;
	nodeVar170 = max( vec4<f32>( ( outgoingLight + EmissiveColor ), DiffuseColor.w ), vec4<f32>( 0.0 ) );
	Output = nodeVar170;

	// result

	output.color = nodeVar170;

	return output;

}
