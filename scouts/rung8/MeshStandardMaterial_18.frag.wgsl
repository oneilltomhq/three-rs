// Three.js r186dev - Node System

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
@binding( 3 ) @group( 1 ) var nodeUniform6_sampler : sampler;
@binding( 4 ) @group( 1 ) var nodeUniform6 : texture_2d<f32>;
@binding( 5 ) @group( 1 ) var nodeUniform12_sampler : sampler;
@binding( 6 ) @group( 1 ) var nodeUniform12 : texture_2d<f32>;
@binding( 7 ) @group( 1 ) var nodeUniform14_sampler : sampler;
@binding( 8 ) @group( 1 ) var nodeUniform14 : texture_2d<f32>;
@binding( 9 ) @group( 1 ) var nodeUniform25_sampler : sampler_comparison;
@binding( 10 ) @group( 1 ) var nodeUniform25 : texture_depth_cube;

struct objectStruct {
	nodeUniform0 : vec3<f32>,
	nodeUniform2 : mat3x3<f32>,
	nodeUniform3 : f32,
	nodeUniform4 : f32,
	nodeUniform5 : f32,
	nodeUniform7 : mat3x3<f32>,
	nodeUniform9 : mat3x3<f32>,
	nodeUniform10 : vec3<f32>,
	nodeUniform11 : f32,
	nodeUniform13 : mat4x4<f32>,
	nodeUniform15 : mat3x3<f32>,
	nodeUniform16 : f32
};
@binding( 0 ) @group( 1 )
var<uniform> object : objectStruct;

struct renderStruct {
	cameraProjectionMatrix : mat4x4<f32>,
	cameraViewMatrix : mat4x4<f32>,
	nodeUniform18 : vec3<f32>,
	nodeUniform29 : f32,
	nodeUniform30 : f32,
	nodeUniform32 : vec3<f32>,
	nodeUniform33 : vec3<f32>,
	nodeUniform31 : vec3<f32>,
	nodeUniform17 : vec3<f32>,
	nodeUniform19 : mat4x4<f32>,
	nodeUniform23 : f32,
	nodeUniform22 : f32,
	nodeUniform21 : f32,
	nodeUniform24 : f32,
	nodeUniform26 : f32,
	nodeUniform27 : vec2<f32>,
	nodeUniform28 : f32
};
@binding( 0 ) @group( 0 )
var<uniform> render : renderStruct;

// vars
var<private> DiffuseColor : vec4<f32>;
var<private> nodeVar0 : vec4<f32>;
var<private> Metalness : f32;
var<private> Roughness : f32;
var<private> nodeVar1 : vec4<f32>;
var<private> normalViewGeometry : vec3<f32>;
var<private> nodeVar2 : vec3<f32>;
var<private> SpecularColor : vec3<f32>;
var<private> SpecularColorBlended : vec3<f32>;
var<private> SpecularF90 : f32;
var<private> DiffuseContribution : vec3<f32>;
var<private> EmissiveColor : vec3<f32>;
var<private> Output : vec4<f32>;
var<private> nodeVar3 : vec3<f32>;
var<private> NORMAL_normalView : vec3<f32>;
var<private> nodeVar4 : vec3<f32>;
var<private> nodeVar5 : f32;
var<private> nodeVar6 : f32;
var<private> nodeVar7 : vec4<f32>;
var<private> nodeVar8 : vec4<f32>;
var<private> nodeVar9 : vec4<f32>;
var<private> nodeVar10 : vec2<f32>;
var<private> normalView : vec3<f32>;
var<private> positionViewDirection : vec3<f32>;
var<private> nodeVar11 : f32;
var<private> nodeVar12 : vec2<f32>;
var<private> nodeVar13 : f32;
var<private> nodeVar14 : f32;
var<private> nodeVar15 : f32;
var<private> nodeVar16 : f32;
var<private> nodeVar17 : vec3<f32>;
var<private> nodeVar18 : vec3<f32>;
var<private> nodeVar19 : vec3<f32>;
var<private> nodeVar20 : vec3<f32>;
var<private> nodeVar21 : f32;
var<private> shadowPositionWorld : vec3<f32>;
var<private> normalWorld : vec3<f32>;
var<private> nodeVar22 : f32;
var<private> nodeVar23 : f32;
var<private> nodeVar24 : f32;
var<private> nodeVar25 : f32;
var<private> nodeVar26 : vec3<f32>;
var<private> nodeVar27 : vec3<f32>;
var<private> nodeVar28 : vec3<f32>;
var<private> nodeVar29 : vec3<f32>;
var<private> nodeVar30 : f32;
var<private> nodeVar31 : vec2<f32>;
var<private> nodeVar32 : vec3<f32>;
var<private> nodeVar33 : f32;
var<private> nodeVar34 : vec3<f32>;
var<private> nodeVar35 : f32;
var<private> nodeVar36 : vec2<f32>;
var<private> nodeVar37 : vec3<f32>;
var<private> nodeVar38 : f32;
var<private> nodeVar39 : vec2<f32>;
var<private> nodeVar40 : vec3<f32>;
var<private> nodeVar41 : f32;
var<private> nodeVar42 : vec2<f32>;
var<private> nodeVar43 : vec3<f32>;
var<private> nodeVar44 : f32;
var<private> nodeVar45 : vec2<f32>;
var<private> nodeVar46 : vec3<f32>;
var<private> nodeVar47 : f32;
var<private> nodeVar48 : f32;
var<private> nodeVar49 : vec3<f32>;
var<private> nodeVar50 : f32;
var<private> nodeVar51 : f32;
var<private> nodeVar52 : f32;
var<private> nodeVar53 : f32;
var<private> nodeVar54 : vec3<f32>;
var<private> nodeVar55 : vec3<f32>;
var<private> nodeVar56 : vec3<f32>;
var<private> directDiffuse : vec3<f32>;
var<private> nodeVar57 : vec3<f32>;
var<private> nodeVar58 : vec3<f32>;
var<private> nodeVar59 : vec3<f32>;
var<private> nodeVar60 : vec3<f32>;
var<private> nodeVar61 : f32;
var<private> nodeVar62 : f32;
var<private> nodeVar63 : f32;
var<private> nodeVar64 : vec3<f32>;
var<private> nodeVar65 : vec3<f32>;
var<private> nodeVar66 : vec3<f32>;
var<private> nodeVar67 : vec3<f32>;
var<private> nodeVar68 : vec3<f32>;
var<private> directSpecular : vec3<f32>;
var<private> nodeVar69 : vec3<f32>;
var<private> nodeVar70 : f32;
var<private> nodeVar71 : f32;
var<private> nodeVar72 : f32;
var<private> nodeVar73 : vec3<f32>;
var<private> nodeVar74 : vec3<f32>;
var<private> nodeVar75 : vec3<f32>;
var<private> nodeVar76 : vec3<f32>;
var<private> irradiance : vec3<f32>;
var<private> nodeVar77 : f32;
var<private> nodeVar78 : f32;
var<private> nodeVar79 : f32;
var<private> nodeVar80 : vec3<f32>;
var<private> nodeVar81 : vec3<f32>;
var<private> nodeVar82 : vec3<f32>;
var<private> nodeVar83 : vec3<f32>;
var<private> nodeVar84 : vec3<f32>;
var<private> nodeVar85 : f32;
var<private> nodeVar86 : vec3<f32>;
var<private> nodeVar87 : vec3<f32>;
var<private> nodeVar88 : vec3<f32>;
var<private> nodeVar89 : vec3<f32>;
var<private> nodeVar90 : vec3<f32>;
var<private> nodeVar91 : vec3<f32>;
var<private> nodeVar92 : vec3<f32>;
var<private> nodeVar93 : f32;
var<private> nodeVar94 : f32;
var<private> nodeVar95 : f32;
var<private> nodeVar96 : vec3<f32>;
var<private> nodeVar97 : vec3<f32>;
var<private> nodeVar98 : vec3<f32>;
var<private> nodeVar99 : vec3<f32>;
var<private> nodeVar100 : vec3<f32>;
var<private> nodeVar101 : vec3<f32>;
var<private> nodeVar102 : vec3<f32>;
var<private> nodeVar103 : vec3<f32>;
var<private> nodeVar104 : vec3<f32>;
var<private> nodeVar105 : vec3<f32>;
var<private> nodeVar106 : vec3<f32>;
var<private> nodeVar107 : vec3<f32>;
var<private> nodeVar108 : vec3<f32>;
var<private> indirectDiffuse : vec3<f32>;
var<private> nodeVar109 : vec3<f32>;
var<private> singleScatteringDielectric : vec3<f32>;
var<private> multiScatteringDielectric : vec3<f32>;
var<private> singleScatteringMetallic : vec3<f32>;
var<private> multiScatteringMetallic : vec3<f32>;
var<private> nodeVar110 : vec3<f32>;
var<private> nodeVar111 : f32;
var<private> nodeVar112 : vec3<f32>;
var<private> nodeVar113 : vec3<f32>;
var<private> nodeVar114 : vec3<f32>;
var<private> nodeVar115 : vec3<f32>;
var<private> nodeVar116 : vec3<f32>;
var<private> nodeVar117 : vec3<f32>;
var<private> nodeVar118 : vec3<f32>;
var<private> nodeVar119 : f32;
var<private> nodeVar120 : f32;
var<private> nodeVar121 : f32;
var<private> nodeVar122 : vec3<f32>;
var<private> nodeVar123 : vec3<f32>;
var<private> nodeVar124 : vec3<f32>;
var<private> nodeVar125 : vec3<f32>;
var<private> nodeVar126 : vec3<f32>;
var<private> nodeVar127 : vec3<f32>;
var<private> nodeVar128 : vec3<f32>;
var<private> nodeVar129 : f32;
var<private> nodeVar130 : vec3<f32>;
var<private> nodeVar131 : vec3<f32>;
var<private> nodeVar132 : vec3<f32>;
var<private> nodeVar133 : vec3<f32>;
var<private> nodeVar134 : vec3<f32>;
var<private> nodeVar135 : vec3<f32>;
var<private> nodeVar136 : vec3<f32>;
var<private> nodeVar137 : f32;
var<private> nodeVar138 : f32;
var<private> nodeVar139 : f32;
var<private> nodeVar140 : vec3<f32>;
var<private> nodeVar141 : vec3<f32>;
var<private> nodeVar142 : vec3<f32>;
var<private> nodeVar143 : vec3<f32>;
var<private> nodeVar144 : vec3<f32>;
var<private> nodeVar145 : vec3<f32>;
var<private> radiance : vec3<f32>;
var<private> nodeVar146 : vec3<f32>;
var<private> nodeVar147 : vec3<f32>;
var<private> nodeVar148 : vec3<f32>;
var<private> iblIrradiance : vec3<f32>;
var<private> nodeVar149 : vec3<f32>;
var<private> nodeVar150 : vec3<f32>;
var<private> nodeVar151 : vec3<f32>;
var<private> nodeVar152 : vec3<f32>;
var<private> nodeVar153 : vec3<f32>;
var<private> nodeVar154 : vec3<f32>;
var<private> nodeVar155 : vec3<f32>;
var<private> nodeVar156 : vec3<f32>;
var<private> nodeVar157 : vec3<f32>;
var<private> nodeVar158 : vec3<f32>;
var<private> indirectSpecular : vec3<f32>;
var<private> nodeVar159 : vec3<f32>;
var<private> nodeVar160 : vec3<f32>;
var<private> ambientOcclusion : f32;
var<private> nodeVar161 : vec3<f32>;
var<private> nodeVar162 : f32;
var<private> nodeVar163 : f32;
var<private> nodeVar164 : f32;
var<private> nodeVar165 : f32;
var<private> nodeVar166 : f32;
var<private> nodeVar167 : f32;
var<private> nodeVar168 : f32;
var<private> nodeVar169 : f32;
var<private> nodeVar170 : f32;
var<private> nodeVar171 : f32;
var<private> nodeVar172 : f32;
var<private> nodeVar173 : vec3<f32>;
var<private> totalDiffuse : vec3<f32>;
var<private> nodeVar174 : vec3<f32>;
var<private> totalSpecular : vec3<f32>;
var<private> nodeVar175 : vec3<f32>;
var<private> outgoingLight : vec3<f32>;
var<private> nodeVar176 : vec3<f32>;
var<private> nodeVar177 : vec4<f32>;

// codes
fn interleavedGradientNoise ( position : vec2<f32> ) -> f32 {

	


	return fract( ( 52.9829189 * fract( dot( position, vec2<f32>( 0.06711056, 0.00583715 ) ) ) ) );

}


fn vogelDiskSample ( sampleIndex : i32, samplesCount : i32, phi : f32 ) -> vec2<f32> {

	var nodeVar0 : f32;

	nodeVar0 = ( ( f32( sampleIndex ) * 2.399963229728653 ) + phi );

	return ( vec2<f32>( cos( nodeVar0 ), sin( nodeVar0 ) ) * vec2<f32>( sqrt( ( ( f32( sampleIndex ) + 0.5 ) / f32( samplesCount ) ) ) ) );

}


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




@fragment
fn main( @location( 0 ) v_positionView : vec3<f32>,
	@location( 1 ) v_normalViewGeometry : vec3<f32>,
	@location( 2 ) v_positionViewDirection : vec3<f32>,
	@location( 3 ) v_positionWorld : vec3<f32>,
	@location( 4 ) nodeVarying7 : vec2<f32>,
	@builtin( front_facing ) isFront : bool,
	@builtin( position ) fragCoord : vec4<f32> ) -> OutputStruct {

	// flow
	// code

	nodeVar0 = textureSample( nodeUniform1, nodeUniform1_sampler, ( object.nodeUniform2 * vec3<f32>( nodeVarying7, 1.0 ) ).xy );
	DiffuseColor = ( vec4<f32>( object.nodeUniform0, 1.0 ) * nodeVar0 );
	DiffuseColor.w = ( DiffuseColor.w * object.nodeUniform3 );
	DiffuseColor.w = 1.0;
	Metalness = object.nodeUniform4;
	nodeVar1 = textureSample( nodeUniform6, nodeUniform6_sampler, ( object.nodeUniform7 * vec3<f32>( nodeVarying7, 1.0 ) ).xy );
	normalViewGeometry = normalize( v_normalViewGeometry );
	nodeVar2 = max( abs( dpdx( normalViewGeometry ) ), abs( - dpdy( normalViewGeometry ) ) );
	Roughness = min( ( max( ( object.nodeUniform5 * nodeVar1.y ), 0.0525 ) + max( max( nodeVar2.x, nodeVar2.y ), nodeVar2.z ) ), 1.0 );
	SpecularColor = vec3<f32>( 0.04, 0.04, 0.04 );
	SpecularColorBlended = mix( vec3<f32>( 0.04, 0.04, 0.04 ), DiffuseColor.xyz, Metalness );
	SpecularF90 = 1.0;
	DiffuseContribution = ( DiffuseColor.xyz * vec3<f32>( ( 1.0 - object.nodeUniform4 ) ) );
	EmissiveColor = ( object.nodeUniform10 * vec3<f32>( object.nodeUniform11 ) );
	nodeVar3 = normalize( dpdx( v_positionView ) );
	NORMAL_normalView = normalViewGeometry;
	nodeVar4 = cross( normalize( - dpdy( v_positionView ) ), NORMAL_normalView );
	nodeVar5 = ( ( f32( isFront ) * 2.0 ) - 1.0 );
	nodeVar6 = ( dot( nodeVar3, nodeVar4 ) * nodeVar5 );
	nodeVar7 = textureSample( nodeUniform14, nodeUniform14_sampler, ( object.nodeUniform15 * vec3<f32>( ( nodeVarying7 + dpdx( nodeVarying7 ) ), 1.0 ) ).xy );
	nodeVar8 = textureSample( nodeUniform14, nodeUniform14_sampler, ( object.nodeUniform15 * vec3<f32>( nodeVarying7, 1.0 ) ).xy );
	nodeVar9 = textureSample( nodeUniform14, nodeUniform14_sampler, ( object.nodeUniform15 * vec3<f32>( ( nodeVarying7 + - dpdy( nodeVarying7 ) ), 1.0 ) ).xy );
	nodeVar10 = ( vec2<f32>( ( nodeVar7.x - nodeVar8.x ), ( nodeVar9.x - nodeVar8.x ) ) * vec2<f32>( object.nodeUniform16 ) );
	normalView = normalize( ( ( vec3<f32>( abs( nodeVar6 ) ) * NORMAL_normalView ) - ( vec3<f32>( sign( nodeVar6 ) ) * ( ( vec3<f32>( nodeVar10.x ) * nodeVar4 ) + ( vec3<f32>( nodeVar10.y ) * cross( NORMAL_normalView, nodeVar3 ) ) ) ) ) );
	positionViewDirection = normalize( v_positionViewDirection );
	nodeVar11 = dot( normalView, positionViewDirection );
	nodeVar12 = textureSample( nodeUniform12, nodeUniform12_sampler, vec2<f32>( Roughness, clamp( nodeVar11, 0.0, 1.0 ) ) ).xy;
	let dfg = nodeVar12;
	nodeVar13 = ( dfg.x + dfg.y );
	nodeVar14 = ( 1.0 / nodeVar13 );
	nodeVar15 = nodeVar14;
	nodeVar16 = ( nodeVar15 - 1.0 );
	nodeVar17 = ( SpecularColorBlended * vec3<f32>( nodeVar16 ) );
	nodeVar18 = ( nodeVar17 + vec3<f32>( 1.0 ) );
	let multiScatteringCompensation = nodeVar18;
	nodeVar19 = ( render.nodeUniform17 - v_positionView );
	nodeVar20 = normalize( nodeVar19 );
	nodeVar21 = dot( normalView, nodeVar20 );
	shadowPositionWorld = v_positionWorld;
	normalWorld = normalize( ( vec4<f32>( normalView, 0.0 ) * render.cameraViewMatrix ).xyz );
	let nodeConst0 = ( render.nodeUniform19 * vec4<f32>( ( shadowPositionWorld + ( normalWorld * vec3<f32>( render.nodeUniform21 ) ) ), 1.0 ) ).xyz;
	let nodeConst1 = abs( nodeConst0 );
	nodeVar22 = 1.0;
	nodeVar23 = max( max( nodeConst1.x, nodeConst1.y ), nodeConst1.z );

	if ( ( ( ( nodeVar23 - render.nodeUniform22 ) <= 0.0 ) && ( ( nodeVar23 - render.nodeUniform23 ) >= 0.0 ) ) ) {

		nodeVar24 = ( - nodeVar23 );
		nodeVar25 = ( ( ( render.nodeUniform23 + nodeVar24 ) * render.nodeUniform22 ) / ( ( render.nodeUniform22 - render.nodeUniform23 ) * nodeVar24 ) );
		nodeVar25 = ( nodeVar25 + render.nodeUniform24 );
		nodeVar26 = normalize( nodeConst0 );
		nodeVar28 = abs( nodeVar26 );

		if ( ( nodeVar28.x > nodeVar28.z ) ) {

			nodeVar27 = vec3<f32>( 0.0, 1.0, 0.0 );

		} else {

			nodeVar27 = vec3<f32>( 1.0, 0.0, 0.0 );

		}

		nodeVar29 = normalize( cross( nodeVar26, nodeVar27 ) );
		nodeVar30 = ( interleavedGradientNoise( fragCoord.xy ) * 6.28318530718 );
		nodeVar31 = vogelDiskSample( 0, 5, nodeVar30 );
		nodeVar32 = cross( nodeVar26, nodeVar29 );
		nodeVar33 = ( render.nodeUniform26 / render.nodeUniform27.x );
		nodeVar34 = ( nodeVar26 + ( ( ( nodeVar29 * vec3<f32>( nodeVar31.x ) ) + ( nodeVar32 * vec3<f32>( nodeVar31.y ) ) ) * vec3<f32>( nodeVar33 ) ) );
		nodeVar35 = textureSampleCompare( nodeUniform25, nodeUniform25_sampler, vec3<f32>( nodeVar34.x, ( - nodeVar34.y ), nodeVar34.z ), nodeVar25 );
		nodeVar36 = vogelDiskSample( 1, 5, nodeVar30 );
		nodeVar37 = ( nodeVar26 + ( ( ( nodeVar29 * vec3<f32>( nodeVar36.x ) ) + ( nodeVar32 * vec3<f32>( nodeVar36.y ) ) ) * vec3<f32>( nodeVar33 ) ) );
		nodeVar38 = textureSampleCompare( nodeUniform25, nodeUniform25_sampler, vec3<f32>( nodeVar37.x, ( - nodeVar37.y ), nodeVar37.z ), nodeVar25 );
		nodeVar39 = vogelDiskSample( 2, 5, nodeVar30 );
		nodeVar40 = ( nodeVar26 + ( ( ( nodeVar29 * vec3<f32>( nodeVar39.x ) ) + ( nodeVar32 * vec3<f32>( nodeVar39.y ) ) ) * vec3<f32>( nodeVar33 ) ) );
		nodeVar41 = textureSampleCompare( nodeUniform25, nodeUniform25_sampler, vec3<f32>( nodeVar40.x, ( - nodeVar40.y ), nodeVar40.z ), nodeVar25 );
		nodeVar42 = vogelDiskSample( 3, 5, nodeVar30 );
		nodeVar43 = ( nodeVar26 + ( ( ( nodeVar29 * vec3<f32>( nodeVar42.x ) ) + ( nodeVar32 * vec3<f32>( nodeVar42.y ) ) ) * vec3<f32>( nodeVar33 ) ) );
		nodeVar44 = textureSampleCompare( nodeUniform25, nodeUniform25_sampler, vec3<f32>( nodeVar43.x, ( - nodeVar43.y ), nodeVar43.z ), nodeVar25 );
		nodeVar45 = vogelDiskSample( 4, 5, nodeVar30 );
		nodeVar46 = ( nodeVar26 + ( ( ( nodeVar29 * vec3<f32>( nodeVar45.x ) ) + ( nodeVar32 * vec3<f32>( nodeVar45.y ) ) ) * vec3<f32>( nodeVar33 ) ) );
		nodeVar47 = textureSampleCompare( nodeUniform25, nodeUniform25_sampler, vec3<f32>( nodeVar46.x, ( - nodeVar46.y ), nodeVar46.z ), nodeVar25 );
		nodeVar22 = ( ( ( ( ( nodeVar35 + nodeVar38 ) + nodeVar41 ) + nodeVar44 ) + nodeVar47 ) * 0.2 );
		

	}

	nodeVar48 = mix( 1.0, nodeVar22, render.nodeUniform28 );
	nodeVar49 = ( render.nodeUniform18 * vec3<f32>( nodeVar48 ) );

	if ( ( render.nodeUniform29 > 0.0 ) ) {

		nodeVar51 = length( nodeVar19 );
		nodeVar52 = ( nodeVar51 / render.nodeUniform29 );
		nodeVar53 = clamp( ( 1.0 - ( ( ( nodeVar52 * nodeVar52 ) * nodeVar52 ) * nodeVar52 ) ), 0.0, 1.0 );
		nodeVar50 = ( ( 1.0 / max( pow( nodeVar51, render.nodeUniform30 ), 0.01 ) ) * ( nodeVar53 * nodeVar53 ) );

	} else {

		nodeVar50 = ( 1.0 / max( pow( length( nodeVar19 ), render.nodeUniform30 ), 0.01 ) );

	}

	nodeVar54 = ( nodeVar49 * vec3<f32>( nodeVar50 ) );
	nodeVar55 = ( vec3<f32>( clamp( nodeVar21, 0.0, 1.0 ) ) * nodeVar54 );
	nodeVar56 = nodeVar55;
	directDiffuse = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar57 = ( DiffuseContribution * vec3<f32>( 0.3183098861837907 ) );
	nodeVar58 = ( nodeVar56 * nodeVar57 );
	nodeVar59 = ( nodeVar20 + positionViewDirection );
	nodeVar60 = normalize( nodeVar59 );
	nodeVar61 = dot( positionViewDirection, nodeVar60 );
	nodeVar62 = clamp( nodeVar61, 0.0, 1.0 );
	nodeVar63 = exp2( ( ( ( nodeVar62 * -5.55473 ) - 6.98316 ) * nodeVar62 ) );
	nodeVar64 = ( ( SpecularColor * vec3<f32>( ( 1.0 - nodeVar63 ) ) ) + vec3<f32>( ( SpecularF90 * nodeVar63 ) ) );
	nodeVar65 = ( vec3<f32>( 1.0 ) - nodeVar64 );
	nodeVar66 = nodeVar65;
	nodeVar67 = ( nodeVar58 * nodeVar66 );
	nodeVar68 = ( directDiffuse + nodeVar67 );
	directDiffuse = nodeVar68;
	directSpecular = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar69 = normalize( ( nodeVar20 + positionViewDirection ) );
	nodeVar70 = clamp( dot( positionViewDirection, nodeVar69 ), 0.0, 1.0 );
	nodeVar71 = exp2( ( ( ( nodeVar70 * -5.55473 ) - 6.98316 ) * nodeVar70 ) );
	nodeVar72 = ( Roughness * Roughness );
	nodeVar73 = ( ( ( ( SpecularColorBlended * vec3<f32>( ( 1.0 - nodeVar71 ) ) ) + vec3<f32>( ( 1.0 * nodeVar71 ) ) ) * vec3<f32>( V_GGX_SmithCorrelated( nodeVar72, clamp( dot( normalView, nodeVar20 ), 0.0, 1.0 ), clamp( dot( normalView, positionViewDirection ), 0.0, 1.0 ) ) ) ) * vec3<f32>( D_GGX( nodeVar72, clamp( dot( normalView, nodeVar69 ), 0.0, 1.0 ) ) ) );
	nodeVar74 = ( nodeVar56 * nodeVar73 );
	nodeVar75 = ( nodeVar74 * multiScatteringCompensation );
	nodeVar76 = ( directSpecular + nodeVar75 );
	directSpecular = nodeVar76;
	irradiance = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar77 = dot( normalWorld, normalize( render.nodeUniform33 ) );
	nodeVar78 = ( nodeVar77 * 0.5 );
	nodeVar79 = ( nodeVar78 + 0.5 );
	nodeVar80 = mix( render.nodeUniform31, render.nodeUniform32, nodeVar79 );
	nodeVar81 = ( irradiance + nodeVar80 );
	irradiance = nodeVar81;
	nodeVar82 = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar83 = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar84 = ( SpecularColor * vec3<f32>( dfg.x ) );
	nodeVar85 = ( SpecularF90 * dfg.y );
	nodeVar86 = ( nodeVar84 + vec3<f32>( nodeVar85 ) );
	nodeVar87 = ( nodeVar82 + nodeVar86 );
	nodeVar82 = nodeVar87;
	nodeVar88 = ( vec3<f32>( 1.0 ) - SpecularColor );
	nodeVar89 = nodeVar88;
	nodeVar90 = ( nodeVar89 * vec3<f32>( 0.047619 ) );
	nodeVar91 = ( SpecularColor + nodeVar90 );
	nodeVar92 = ( nodeVar86 * nodeVar91 );
	nodeVar93 = ( dfg.x + dfg.y );
	nodeVar94 = ( 1.0 - nodeVar93 );
	nodeVar95 = nodeVar94;
	nodeVar96 = ( vec3<f32>( nodeVar95 ) * nodeVar91 );
	nodeVar97 = ( vec3<f32>( 1.0 ) - nodeVar96 );
	nodeVar98 = nodeVar97;
	nodeVar99 = ( nodeVar92 / nodeVar98 );
	nodeVar100 = ( nodeVar99 * vec3<f32>( nodeVar95 ) );
	nodeVar101 = ( nodeVar83 + nodeVar100 );
	nodeVar83 = nodeVar101;
	nodeVar102 = ( DiffuseContribution * vec3<f32>( 0.3183098861837907 ) );
	nodeVar103 = ( irradiance * nodeVar102 );
	nodeVar104 = ( nodeVar82 + nodeVar83 );
	nodeVar105 = ( vec3<f32>( 1.0 ) - nodeVar104 );
	nodeVar106 = nodeVar105;
	nodeVar107 = ( nodeVar103 * nodeVar106 );
	nodeVar108 = nodeVar107;
	indirectDiffuse = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar109 = ( indirectDiffuse + nodeVar108 );
	indirectDiffuse = nodeVar109;
	singleScatteringDielectric = vec3<f32>( 0.0, 0.0, 0.0 );
	multiScatteringDielectric = vec3<f32>( 0.0, 0.0, 0.0 );
	singleScatteringMetallic = vec3<f32>( 0.0, 0.0, 0.0 );
	multiScatteringMetallic = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar110 = ( SpecularColor * vec3<f32>( dfg.x ) );
	nodeVar111 = ( SpecularF90 * dfg.y );
	nodeVar112 = ( nodeVar110 + vec3<f32>( nodeVar111 ) );
	nodeVar113 = ( singleScatteringDielectric + nodeVar112 );
	singleScatteringDielectric = nodeVar113;
	nodeVar114 = ( vec3<f32>( 1.0 ) - SpecularColor );
	nodeVar115 = nodeVar114;
	nodeVar116 = ( nodeVar115 * vec3<f32>( 0.047619 ) );
	nodeVar117 = ( SpecularColor + nodeVar116 );
	nodeVar118 = ( nodeVar112 * nodeVar117 );
	nodeVar119 = ( dfg.x + dfg.y );
	nodeVar120 = ( 1.0 - nodeVar119 );
	nodeVar121 = nodeVar120;
	nodeVar122 = ( vec3<f32>( nodeVar121 ) * nodeVar117 );
	nodeVar123 = ( vec3<f32>( 1.0 ) - nodeVar122 );
	nodeVar124 = nodeVar123;
	nodeVar125 = ( nodeVar118 / nodeVar124 );
	nodeVar126 = ( nodeVar125 * vec3<f32>( nodeVar121 ) );
	nodeVar127 = ( multiScatteringDielectric + nodeVar126 );
	multiScatteringDielectric = nodeVar127;
	nodeVar128 = ( DiffuseColor.xyz * vec3<f32>( dfg.x ) );
	nodeVar129 = ( SpecularF90 * dfg.y );
	nodeVar130 = ( nodeVar128 + vec3<f32>( nodeVar129 ) );
	nodeVar131 = ( singleScatteringMetallic + nodeVar130 );
	singleScatteringMetallic = nodeVar131;
	nodeVar132 = ( vec3<f32>( 1.0 ) - DiffuseColor.xyz );
	nodeVar133 = nodeVar132;
	nodeVar134 = ( nodeVar133 * vec3<f32>( 0.047619 ) );
	nodeVar135 = ( DiffuseColor.xyz + nodeVar134 );
	nodeVar136 = ( nodeVar130 * nodeVar135 );
	nodeVar137 = ( dfg.x + dfg.y );
	nodeVar138 = ( 1.0 - nodeVar137 );
	nodeVar139 = nodeVar138;
	nodeVar140 = ( vec3<f32>( nodeVar139 ) * nodeVar135 );
	nodeVar141 = ( vec3<f32>( 1.0 ) - nodeVar140 );
	nodeVar142 = nodeVar141;
	nodeVar143 = ( nodeVar136 / nodeVar142 );
	nodeVar144 = ( nodeVar143 * vec3<f32>( nodeVar139 ) );
	nodeVar145 = ( multiScatteringMetallic + nodeVar144 );
	multiScatteringMetallic = nodeVar145;
	radiance = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar146 = mix( singleScatteringDielectric, singleScatteringMetallic, Metalness );
	nodeVar147 = ( radiance * nodeVar146 );
	nodeVar148 = mix( multiScatteringDielectric, multiScatteringMetallic, Metalness );
	iblIrradiance = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar149 = ( iblIrradiance * vec3<f32>( 0.3183098861837907 ) );
	nodeVar150 = ( nodeVar148 * nodeVar149 );
	nodeVar151 = ( nodeVar147 + nodeVar150 );
	nodeVar152 = nodeVar151;
	nodeVar153 = ( singleScatteringDielectric + multiScatteringDielectric );
	nodeVar154 = ( vec3<f32>( 1.0 ) - nodeVar153 );
	nodeVar155 = nodeVar154;
	nodeVar156 = ( DiffuseContribution * nodeVar155 );
	nodeVar157 = ( nodeVar156 * nodeVar149 );
	nodeVar158 = nodeVar157;
	indirectSpecular = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar159 = ( indirectSpecular + nodeVar152 );
	indirectSpecular = nodeVar159;
	nodeVar160 = ( indirectDiffuse + nodeVar158 );
	indirectDiffuse = nodeVar160;
	ambientOcclusion = 1.0;
	nodeVar161 = ( indirectDiffuse * vec3<f32>( ambientOcclusion ) );
	indirectDiffuse = nodeVar161;
	nodeVar162 = dot( normalView, positionViewDirection );
	nodeVar163 = ( clamp( nodeVar162, 0.0, 1.0 ) + ambientOcclusion );
	nodeVar164 = ( Roughness * -16.0 );
	nodeVar165 = ( 1.0 - nodeVar164 );
	nodeVar166 = nodeVar165;
	nodeVar167 = ( - nodeVar166 );
	nodeVar168 = exp2( nodeVar167 );
	nodeVar169 = pow( nodeVar163, nodeVar168 );
	nodeVar170 = ( 1.0 - nodeVar169 );
	nodeVar171 = nodeVar170;
	nodeVar172 = ( ambientOcclusion - nodeVar171 );
	nodeVar173 = ( indirectSpecular * vec3<f32>( clamp( nodeVar172, 0.0, 1.0 ) ) );
	indirectSpecular = nodeVar173;
	nodeVar174 = ( directDiffuse + indirectDiffuse );
	totalDiffuse = nodeVar174;
	nodeVar175 = ( directSpecular + indirectSpecular );
	totalSpecular = nodeVar175;
	nodeVar176 = ( totalDiffuse + totalSpecular );
	outgoingLight = nodeVar176;
	nodeVar177 = max( vec4<f32>( ( outgoingLight + EmissiveColor ), DiffuseColor.w ), vec4<f32>( 0.0 ) );
	Output = nodeVar177;

	// result

	output.color = nodeVar177;

	return output;

}
