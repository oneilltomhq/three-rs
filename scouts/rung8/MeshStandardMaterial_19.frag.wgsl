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
@binding( 3 ) @group( 1 ) var nodeUniform10_sampler : sampler;
@binding( 4 ) @group( 1 ) var nodeUniform10 : texture_2d<f32>;
@binding( 5 ) @group( 1 ) var nodeUniform12_sampler : sampler;
@binding( 6 ) @group( 1 ) var nodeUniform12 : texture_2d<f32>;

struct objectStruct {
	nodeUniform0 : vec3<f32>,
	nodeUniform2 : mat3x3<f32>,
	nodeUniform3 : f32,
	nodeUniform4 : f32,
	nodeUniform5 : f32,
	nodeUniform7 : mat3x3<f32>,
	nodeUniform8 : vec3<f32>,
	nodeUniform9 : f32,
	nodeUniform11 : mat4x4<f32>,
	nodeUniform13 : mat3x3<f32>,
	nodeUniform14 : f32
};
@binding( 0 ) @group( 1 )
var<uniform> object : objectStruct;

struct renderStruct {
	cameraProjectionMatrix : mat4x4<f32>,
	cameraViewMatrix : mat4x4<f32>,
	nodeUniform16 : vec3<f32>,
	nodeUniform17 : f32,
	nodeUniform18 : f32,
	nodeUniform20 : vec3<f32>,
	nodeUniform22 : vec3<f32>,
	nodeUniform19 : vec3<f32>,
	nodeUniform15 : vec3<f32>
};
@binding( 0 ) @group( 0 )
var<uniform> render : renderStruct;

// vars
var<private> DiffuseColor : vec4<f32>;
var<private> nodeVar0 : vec4<f32>;
var<private> Metalness : f32;
var<private> Roughness : f32;
var<private> normalViewGeometry : vec3<f32>;
var<private> nodeVar1 : vec3<f32>;
var<private> SpecularColor : vec3<f32>;
var<private> SpecularColorBlended : vec3<f32>;
var<private> SpecularF90 : f32;
var<private> DiffuseContribution : vec3<f32>;
var<private> EmissiveColor : vec3<f32>;
var<private> Output : vec4<f32>;
var<private> nodeVar2 : vec3<f32>;
var<private> NORMAL_normalView : vec3<f32>;
var<private> nodeVar3 : vec3<f32>;
var<private> nodeVar4 : f32;
var<private> nodeVar5 : f32;
var<private> nodeVar6 : vec4<f32>;
var<private> nodeVar7 : vec4<f32>;
var<private> nodeVar8 : vec4<f32>;
var<private> nodeVar9 : vec2<f32>;
var<private> normalView : vec3<f32>;
var<private> positionViewDirection : vec3<f32>;
var<private> nodeVar10 : f32;
var<private> nodeVar11 : vec2<f32>;
var<private> nodeVar12 : f32;
var<private> nodeVar13 : f32;
var<private> nodeVar14 : f32;
var<private> nodeVar15 : f32;
var<private> nodeVar16 : vec3<f32>;
var<private> nodeVar17 : vec3<f32>;
var<private> nodeVar18 : vec3<f32>;
var<private> nodeVar19 : vec3<f32>;
var<private> nodeVar20 : f32;
var<private> nodeVar21 : f32;
var<private> nodeVar22 : f32;
var<private> nodeVar23 : f32;
var<private> nodeVar24 : f32;
var<private> nodeVar25 : vec3<f32>;
var<private> nodeVar26 : vec3<f32>;
var<private> nodeVar27 : vec3<f32>;
var<private> directDiffuse : vec3<f32>;
var<private> nodeVar28 : vec3<f32>;
var<private> nodeVar29 : vec3<f32>;
var<private> nodeVar30 : vec3<f32>;
var<private> nodeVar31 : vec3<f32>;
var<private> nodeVar32 : f32;
var<private> nodeVar33 : f32;
var<private> nodeVar34 : f32;
var<private> nodeVar35 : vec3<f32>;
var<private> nodeVar36 : vec3<f32>;
var<private> nodeVar37 : vec3<f32>;
var<private> nodeVar38 : vec3<f32>;
var<private> nodeVar39 : vec3<f32>;
var<private> directSpecular : vec3<f32>;
var<private> nodeVar40 : vec3<f32>;
var<private> nodeVar41 : f32;
var<private> nodeVar42 : f32;
var<private> nodeVar43 : f32;
var<private> nodeVar44 : vec3<f32>;
var<private> nodeVar45 : vec3<f32>;
var<private> nodeVar46 : vec3<f32>;
var<private> nodeVar47 : vec3<f32>;
var<private> irradiance : vec3<f32>;
var<private> normalWorld : vec3<f32>;
var<private> nodeVar48 : f32;
var<private> nodeVar49 : f32;
var<private> nodeVar50 : f32;
var<private> nodeVar51 : vec3<f32>;
var<private> nodeVar52 : vec3<f32>;
var<private> nodeVar53 : vec3<f32>;
var<private> nodeVar54 : vec3<f32>;
var<private> nodeVar55 : vec3<f32>;
var<private> nodeVar56 : f32;
var<private> nodeVar57 : vec3<f32>;
var<private> nodeVar58 : vec3<f32>;
var<private> nodeVar59 : vec3<f32>;
var<private> nodeVar60 : vec3<f32>;
var<private> nodeVar61 : vec3<f32>;
var<private> nodeVar62 : vec3<f32>;
var<private> nodeVar63 : vec3<f32>;
var<private> nodeVar64 : f32;
var<private> nodeVar65 : f32;
var<private> nodeVar66 : f32;
var<private> nodeVar67 : vec3<f32>;
var<private> nodeVar68 : vec3<f32>;
var<private> nodeVar69 : vec3<f32>;
var<private> nodeVar70 : vec3<f32>;
var<private> nodeVar71 : vec3<f32>;
var<private> nodeVar72 : vec3<f32>;
var<private> nodeVar73 : vec3<f32>;
var<private> nodeVar74 : vec3<f32>;
var<private> nodeVar75 : vec3<f32>;
var<private> nodeVar76 : vec3<f32>;
var<private> nodeVar77 : vec3<f32>;
var<private> nodeVar78 : vec3<f32>;
var<private> nodeVar79 : vec3<f32>;
var<private> indirectDiffuse : vec3<f32>;
var<private> nodeVar80 : vec3<f32>;
var<private> singleScatteringDielectric : vec3<f32>;
var<private> multiScatteringDielectric : vec3<f32>;
var<private> singleScatteringMetallic : vec3<f32>;
var<private> multiScatteringMetallic : vec3<f32>;
var<private> nodeVar81 : vec3<f32>;
var<private> nodeVar82 : f32;
var<private> nodeVar83 : vec3<f32>;
var<private> nodeVar84 : vec3<f32>;
var<private> nodeVar85 : vec3<f32>;
var<private> nodeVar86 : vec3<f32>;
var<private> nodeVar87 : vec3<f32>;
var<private> nodeVar88 : vec3<f32>;
var<private> nodeVar89 : vec3<f32>;
var<private> nodeVar90 : f32;
var<private> nodeVar91 : f32;
var<private> nodeVar92 : f32;
var<private> nodeVar93 : vec3<f32>;
var<private> nodeVar94 : vec3<f32>;
var<private> nodeVar95 : vec3<f32>;
var<private> nodeVar96 : vec3<f32>;
var<private> nodeVar97 : vec3<f32>;
var<private> nodeVar98 : vec3<f32>;
var<private> nodeVar99 : vec3<f32>;
var<private> nodeVar100 : f32;
var<private> nodeVar101 : vec3<f32>;
var<private> nodeVar102 : vec3<f32>;
var<private> nodeVar103 : vec3<f32>;
var<private> nodeVar104 : vec3<f32>;
var<private> nodeVar105 : vec3<f32>;
var<private> nodeVar106 : vec3<f32>;
var<private> nodeVar107 : vec3<f32>;
var<private> nodeVar108 : f32;
var<private> nodeVar109 : f32;
var<private> nodeVar110 : f32;
var<private> nodeVar111 : vec3<f32>;
var<private> nodeVar112 : vec3<f32>;
var<private> nodeVar113 : vec3<f32>;
var<private> nodeVar114 : vec3<f32>;
var<private> nodeVar115 : vec3<f32>;
var<private> nodeVar116 : vec3<f32>;
var<private> radiance : vec3<f32>;
var<private> nodeVar117 : vec3<f32>;
var<private> nodeVar118 : vec3<f32>;
var<private> nodeVar119 : vec3<f32>;
var<private> iblIrradiance : vec3<f32>;
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
var<private> indirectSpecular : vec3<f32>;
var<private> nodeVar130 : vec3<f32>;
var<private> nodeVar131 : vec3<f32>;
var<private> ambientOcclusion : f32;
var<private> nodeVar132 : vec3<f32>;
var<private> nodeVar133 : f32;
var<private> nodeVar134 : f32;
var<private> nodeVar135 : f32;
var<private> nodeVar136 : f32;
var<private> nodeVar137 : f32;
var<private> nodeVar138 : f32;
var<private> nodeVar139 : f32;
var<private> nodeVar140 : f32;
var<private> nodeVar141 : f32;
var<private> nodeVar142 : f32;
var<private> nodeVar143 : f32;
var<private> nodeVar144 : vec3<f32>;
var<private> totalDiffuse : vec3<f32>;
var<private> nodeVar145 : vec3<f32>;
var<private> totalSpecular : vec3<f32>;
var<private> nodeVar146 : vec3<f32>;
var<private> outgoingLight : vec3<f32>;
var<private> nodeVar147 : vec3<f32>;
var<private> nodeVar148 : vec4<f32>;

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




@fragment
fn main( @location( 0 ) v_positionView : vec3<f32>,
	@location( 1 ) v_normalViewGeometry : vec3<f32>,
	@location( 2 ) v_positionViewDirection : vec3<f32>,
	@location( 3 ) nodeVarying6 : vec2<f32>,
	@builtin( front_facing ) isFront : bool ) -> OutputStruct {

	// flow
	// code

	nodeVar0 = textureSample( nodeUniform1, nodeUniform1_sampler, ( object.nodeUniform2 * vec3<f32>( nodeVarying6, 1.0 ) ).xy );
	DiffuseColor = ( vec4<f32>( object.nodeUniform0, 1.0 ) * nodeVar0 );
	DiffuseColor.w = ( DiffuseColor.w * object.nodeUniform3 );
	DiffuseColor.w = 1.0;
	Metalness = object.nodeUniform4;
	normalViewGeometry = normalize( v_normalViewGeometry );
	nodeVar1 = max( abs( dpdx( normalViewGeometry ) ), abs( - dpdy( normalViewGeometry ) ) );
	Roughness = min( ( max( object.nodeUniform5, 0.0525 ) + max( max( nodeVar1.x, nodeVar1.y ), nodeVar1.z ) ), 1.0 );
	SpecularColor = vec3<f32>( 0.04, 0.04, 0.04 );
	SpecularColorBlended = mix( vec3<f32>( 0.04, 0.04, 0.04 ), DiffuseColor.xyz, Metalness );
	SpecularF90 = 1.0;
	DiffuseContribution = ( DiffuseColor.xyz * vec3<f32>( ( 1.0 - object.nodeUniform4 ) ) );
	EmissiveColor = ( object.nodeUniform8 * vec3<f32>( object.nodeUniform9 ) );
	nodeVar2 = normalize( dpdx( v_positionView ) );
	NORMAL_normalView = normalViewGeometry;
	nodeVar3 = cross( normalize( - dpdy( v_positionView ) ), NORMAL_normalView );
	nodeVar4 = ( ( f32( isFront ) * 2.0 ) - 1.0 );
	nodeVar5 = ( dot( nodeVar2, nodeVar3 ) * nodeVar4 );
	nodeVar6 = textureSample( nodeUniform12, nodeUniform12_sampler, ( object.nodeUniform13 * vec3<f32>( ( nodeVarying6 + dpdx( nodeVarying6 ) ), 1.0 ) ).xy );
	nodeVar7 = textureSample( nodeUniform12, nodeUniform12_sampler, ( object.nodeUniform13 * vec3<f32>( nodeVarying6, 1.0 ) ).xy );
	nodeVar8 = textureSample( nodeUniform12, nodeUniform12_sampler, ( object.nodeUniform13 * vec3<f32>( ( nodeVarying6 + - dpdy( nodeVarying6 ) ), 1.0 ) ).xy );
	nodeVar9 = ( vec2<f32>( ( nodeVar6.x - nodeVar7.x ), ( nodeVar8.x - nodeVar7.x ) ) * vec2<f32>( object.nodeUniform14 ) );
	normalView = normalize( ( ( vec3<f32>( abs( nodeVar5 ) ) * NORMAL_normalView ) - ( vec3<f32>( sign( nodeVar5 ) ) * ( ( vec3<f32>( nodeVar9.x ) * nodeVar3 ) + ( vec3<f32>( nodeVar9.y ) * cross( NORMAL_normalView, nodeVar2 ) ) ) ) ) );
	positionViewDirection = normalize( v_positionViewDirection );
	nodeVar10 = dot( normalView, positionViewDirection );
	nodeVar11 = textureSample( nodeUniform10, nodeUniform10_sampler, vec2<f32>( Roughness, clamp( nodeVar10, 0.0, 1.0 ) ) ).xy;
	let dfg = nodeVar11;
	nodeVar12 = ( dfg.x + dfg.y );
	nodeVar13 = ( 1.0 / nodeVar12 );
	nodeVar14 = nodeVar13;
	nodeVar15 = ( nodeVar14 - 1.0 );
	nodeVar16 = ( SpecularColorBlended * vec3<f32>( nodeVar15 ) );
	nodeVar17 = ( nodeVar16 + vec3<f32>( 1.0 ) );
	let multiScatteringCompensation = nodeVar17;
	nodeVar18 = ( render.nodeUniform15 - v_positionView );
	nodeVar19 = normalize( nodeVar18 );
	nodeVar20 = dot( normalView, nodeVar19 );

	if ( ( render.nodeUniform17 > 0.0 ) ) {

		nodeVar22 = length( nodeVar18 );
		nodeVar23 = ( nodeVar22 / render.nodeUniform17 );
		nodeVar24 = clamp( ( 1.0 - ( ( ( nodeVar23 * nodeVar23 ) * nodeVar23 ) * nodeVar23 ) ), 0.0, 1.0 );
		nodeVar21 = ( ( 1.0 / max( pow( nodeVar22, render.nodeUniform18 ), 0.01 ) ) * ( nodeVar24 * nodeVar24 ) );

	} else {

		nodeVar21 = ( 1.0 / max( pow( length( nodeVar18 ), render.nodeUniform18 ), 0.01 ) );

	}

	nodeVar25 = ( render.nodeUniform16 * vec3<f32>( nodeVar21 ) );
	nodeVar26 = ( vec3<f32>( clamp( nodeVar20, 0.0, 1.0 ) ) * nodeVar25 );
	nodeVar27 = nodeVar26;
	directDiffuse = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar28 = ( DiffuseContribution * vec3<f32>( 0.3183098861837907 ) );
	nodeVar29 = ( nodeVar27 * nodeVar28 );
	nodeVar30 = ( nodeVar19 + positionViewDirection );
	nodeVar31 = normalize( nodeVar30 );
	nodeVar32 = dot( positionViewDirection, nodeVar31 );
	nodeVar33 = clamp( nodeVar32, 0.0, 1.0 );
	nodeVar34 = exp2( ( ( ( nodeVar33 * -5.55473 ) - 6.98316 ) * nodeVar33 ) );
	nodeVar35 = ( ( SpecularColor * vec3<f32>( ( 1.0 - nodeVar34 ) ) ) + vec3<f32>( ( SpecularF90 * nodeVar34 ) ) );
	nodeVar36 = ( vec3<f32>( 1.0 ) - nodeVar35 );
	nodeVar37 = nodeVar36;
	nodeVar38 = ( nodeVar29 * nodeVar37 );
	nodeVar39 = ( directDiffuse + nodeVar38 );
	directDiffuse = nodeVar39;
	directSpecular = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar40 = normalize( ( nodeVar19 + positionViewDirection ) );
	nodeVar41 = clamp( dot( positionViewDirection, nodeVar40 ), 0.0, 1.0 );
	nodeVar42 = exp2( ( ( ( nodeVar41 * -5.55473 ) - 6.98316 ) * nodeVar41 ) );
	nodeVar43 = ( Roughness * Roughness );
	nodeVar44 = ( ( ( ( SpecularColorBlended * vec3<f32>( ( 1.0 - nodeVar42 ) ) ) + vec3<f32>( ( 1.0 * nodeVar42 ) ) ) * vec3<f32>( V_GGX_SmithCorrelated( nodeVar43, clamp( dot( normalView, nodeVar19 ), 0.0, 1.0 ), clamp( dot( normalView, positionViewDirection ), 0.0, 1.0 ) ) ) ) * vec3<f32>( D_GGX( nodeVar43, clamp( dot( normalView, nodeVar40 ), 0.0, 1.0 ) ) ) );
	nodeVar45 = ( nodeVar27 * nodeVar44 );
	nodeVar46 = ( nodeVar45 * multiScatteringCompensation );
	nodeVar47 = ( directSpecular + nodeVar46 );
	directSpecular = nodeVar47;
	irradiance = vec3<f32>( 0.0, 0.0, 0.0 );
	normalWorld = normalize( ( vec4<f32>( normalView, 0.0 ) * render.cameraViewMatrix ).xyz );
	nodeVar48 = dot( normalWorld, normalize( render.nodeUniform22 ) );
	nodeVar49 = ( nodeVar48 * 0.5 );
	nodeVar50 = ( nodeVar49 + 0.5 );
	nodeVar51 = mix( render.nodeUniform19, render.nodeUniform20, nodeVar50 );
	nodeVar52 = ( irradiance + nodeVar51 );
	irradiance = nodeVar52;
	nodeVar53 = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar54 = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar55 = ( SpecularColor * vec3<f32>( dfg.x ) );
	nodeVar56 = ( SpecularF90 * dfg.y );
	nodeVar57 = ( nodeVar55 + vec3<f32>( nodeVar56 ) );
	nodeVar58 = ( nodeVar53 + nodeVar57 );
	nodeVar53 = nodeVar58;
	nodeVar59 = ( vec3<f32>( 1.0 ) - SpecularColor );
	nodeVar60 = nodeVar59;
	nodeVar61 = ( nodeVar60 * vec3<f32>( 0.047619 ) );
	nodeVar62 = ( SpecularColor + nodeVar61 );
	nodeVar63 = ( nodeVar57 * nodeVar62 );
	nodeVar64 = ( dfg.x + dfg.y );
	nodeVar65 = ( 1.0 - nodeVar64 );
	nodeVar66 = nodeVar65;
	nodeVar67 = ( vec3<f32>( nodeVar66 ) * nodeVar62 );
	nodeVar68 = ( vec3<f32>( 1.0 ) - nodeVar67 );
	nodeVar69 = nodeVar68;
	nodeVar70 = ( nodeVar63 / nodeVar69 );
	nodeVar71 = ( nodeVar70 * vec3<f32>( nodeVar66 ) );
	nodeVar72 = ( nodeVar54 + nodeVar71 );
	nodeVar54 = nodeVar72;
	nodeVar73 = ( DiffuseContribution * vec3<f32>( 0.3183098861837907 ) );
	nodeVar74 = ( irradiance * nodeVar73 );
	nodeVar75 = ( nodeVar53 + nodeVar54 );
	nodeVar76 = ( vec3<f32>( 1.0 ) - nodeVar75 );
	nodeVar77 = nodeVar76;
	nodeVar78 = ( nodeVar74 * nodeVar77 );
	nodeVar79 = nodeVar78;
	indirectDiffuse = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar80 = ( indirectDiffuse + nodeVar79 );
	indirectDiffuse = nodeVar80;
	singleScatteringDielectric = vec3<f32>( 0.0, 0.0, 0.0 );
	multiScatteringDielectric = vec3<f32>( 0.0, 0.0, 0.0 );
	singleScatteringMetallic = vec3<f32>( 0.0, 0.0, 0.0 );
	multiScatteringMetallic = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar81 = ( SpecularColor * vec3<f32>( dfg.x ) );
	nodeVar82 = ( SpecularF90 * dfg.y );
	nodeVar83 = ( nodeVar81 + vec3<f32>( nodeVar82 ) );
	nodeVar84 = ( singleScatteringDielectric + nodeVar83 );
	singleScatteringDielectric = nodeVar84;
	nodeVar85 = ( vec3<f32>( 1.0 ) - SpecularColor );
	nodeVar86 = nodeVar85;
	nodeVar87 = ( nodeVar86 * vec3<f32>( 0.047619 ) );
	nodeVar88 = ( SpecularColor + nodeVar87 );
	nodeVar89 = ( nodeVar83 * nodeVar88 );
	nodeVar90 = ( dfg.x + dfg.y );
	nodeVar91 = ( 1.0 - nodeVar90 );
	nodeVar92 = nodeVar91;
	nodeVar93 = ( vec3<f32>( nodeVar92 ) * nodeVar88 );
	nodeVar94 = ( vec3<f32>( 1.0 ) - nodeVar93 );
	nodeVar95 = nodeVar94;
	nodeVar96 = ( nodeVar89 / nodeVar95 );
	nodeVar97 = ( nodeVar96 * vec3<f32>( nodeVar92 ) );
	nodeVar98 = ( multiScatteringDielectric + nodeVar97 );
	multiScatteringDielectric = nodeVar98;
	nodeVar99 = ( DiffuseColor.xyz * vec3<f32>( dfg.x ) );
	nodeVar100 = ( SpecularF90 * dfg.y );
	nodeVar101 = ( nodeVar99 + vec3<f32>( nodeVar100 ) );
	nodeVar102 = ( singleScatteringMetallic + nodeVar101 );
	singleScatteringMetallic = nodeVar102;
	nodeVar103 = ( vec3<f32>( 1.0 ) - DiffuseColor.xyz );
	nodeVar104 = nodeVar103;
	nodeVar105 = ( nodeVar104 * vec3<f32>( 0.047619 ) );
	nodeVar106 = ( DiffuseColor.xyz + nodeVar105 );
	nodeVar107 = ( nodeVar101 * nodeVar106 );
	nodeVar108 = ( dfg.x + dfg.y );
	nodeVar109 = ( 1.0 - nodeVar108 );
	nodeVar110 = nodeVar109;
	nodeVar111 = ( vec3<f32>( nodeVar110 ) * nodeVar106 );
	nodeVar112 = ( vec3<f32>( 1.0 ) - nodeVar111 );
	nodeVar113 = nodeVar112;
	nodeVar114 = ( nodeVar107 / nodeVar113 );
	nodeVar115 = ( nodeVar114 * vec3<f32>( nodeVar110 ) );
	nodeVar116 = ( multiScatteringMetallic + nodeVar115 );
	multiScatteringMetallic = nodeVar116;
	radiance = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar117 = mix( singleScatteringDielectric, singleScatteringMetallic, Metalness );
	nodeVar118 = ( radiance * nodeVar117 );
	nodeVar119 = mix( multiScatteringDielectric, multiScatteringMetallic, Metalness );
	iblIrradiance = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar120 = ( iblIrradiance * vec3<f32>( 0.3183098861837907 ) );
	nodeVar121 = ( nodeVar119 * nodeVar120 );
	nodeVar122 = ( nodeVar118 + nodeVar121 );
	nodeVar123 = nodeVar122;
	nodeVar124 = ( singleScatteringDielectric + multiScatteringDielectric );
	nodeVar125 = ( vec3<f32>( 1.0 ) - nodeVar124 );
	nodeVar126 = nodeVar125;
	nodeVar127 = ( DiffuseContribution * nodeVar126 );
	nodeVar128 = ( nodeVar127 * nodeVar120 );
	nodeVar129 = nodeVar128;
	indirectSpecular = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar130 = ( indirectSpecular + nodeVar123 );
	indirectSpecular = nodeVar130;
	nodeVar131 = ( indirectDiffuse + nodeVar129 );
	indirectDiffuse = nodeVar131;
	ambientOcclusion = 1.0;
	nodeVar132 = ( indirectDiffuse * vec3<f32>( ambientOcclusion ) );
	indirectDiffuse = nodeVar132;
	nodeVar133 = dot( normalView, positionViewDirection );
	nodeVar134 = ( clamp( nodeVar133, 0.0, 1.0 ) + ambientOcclusion );
	nodeVar135 = ( Roughness * -16.0 );
	nodeVar136 = ( 1.0 - nodeVar135 );
	nodeVar137 = nodeVar136;
	nodeVar138 = ( - nodeVar137 );
	nodeVar139 = exp2( nodeVar138 );
	nodeVar140 = pow( nodeVar134, nodeVar139 );
	nodeVar141 = ( 1.0 - nodeVar140 );
	nodeVar142 = nodeVar141;
	nodeVar143 = ( ambientOcclusion - nodeVar142 );
	nodeVar144 = ( indirectSpecular * vec3<f32>( clamp( nodeVar143, 0.0, 1.0 ) ) );
	indirectSpecular = nodeVar144;
	nodeVar145 = ( directDiffuse + indirectDiffuse );
	totalDiffuse = nodeVar145;
	nodeVar146 = ( directSpecular + indirectSpecular );
	totalSpecular = nodeVar146;
	nodeVar147 = ( totalDiffuse + totalSpecular );
	outgoingLight = nodeVar147;
	nodeVar148 = max( vec4<f32>( ( outgoingLight + EmissiveColor ), DiffuseColor.w ), vec4<f32>( 0.0 ) );
	Output = nodeVar148;

	// result

	output.color = nodeVar148;

	return output;

}
