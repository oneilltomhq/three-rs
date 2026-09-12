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
@binding( 3 ) @group( 1 ) var nodeUniform5_sampler : sampler;
@binding( 4 ) @group( 1 ) var nodeUniform5 : texture_2d<f32>;
@binding( 5 ) @group( 1 ) var nodeUniform12_sampler : sampler;
@binding( 6 ) @group( 1 ) var nodeUniform12 : texture_2d<f32>;

struct objectStruct {
	nodeUniform0 : vec3<f32>,
	nodeUniform2 : mat3x3<f32>,
	nodeUniform3 : f32,
	nodeUniform4 : f32,
	nodeUniform6 : mat3x3<f32>,
	nodeUniform7 : f32,
	nodeUniform9 : mat3x3<f32>,
	nodeUniform10 : vec3<f32>,
	nodeUniform11 : f32,
	nodeUniform13 : mat4x4<f32>
};
@binding( 0 ) @group( 1 )
var<uniform> object : objectStruct;

struct renderStruct {
	cameraProjectionMatrix : mat4x4<f32>,
	cameraViewMatrix : mat4x4<f32>,
	nodeUniform15 : vec3<f32>,
	nodeUniform16 : f32,
	nodeUniform17 : f32,
	nodeUniform19 : vec3<f32>,
	nodeUniform21 : vec3<f32>,
	nodeUniform18 : vec3<f32>,
	nodeUniform14 : vec3<f32>
};
@binding( 0 ) @group( 0 )
var<uniform> render : renderStruct;

// vars
var<private> DiffuseColor : vec4<f32>;
var<private> nodeVar0 : vec4<f32>;
var<private> Metalness : f32;
var<private> nodeVar1 : vec4<f32>;
var<private> Roughness : f32;
var<private> normalViewGeometry : vec3<f32>;
var<private> nodeVar2 : vec3<f32>;
var<private> SpecularColor : vec3<f32>;
var<private> SpecularColorBlended : vec3<f32>;
var<private> SpecularF90 : f32;
var<private> DiffuseContribution : vec3<f32>;
var<private> EmissiveColor : vec3<f32>;
var<private> Output : vec4<f32>;
var<private> NORMAL_normalView : vec3<f32>;
var<private> normalView : vec3<f32>;
var<private> positionViewDirection : vec3<f32>;
var<private> nodeVar3 : f32;
var<private> nodeVar4 : vec2<f32>;
var<private> nodeVar5 : f32;
var<private> nodeVar6 : f32;
var<private> nodeVar7 : f32;
var<private> nodeVar8 : f32;
var<private> nodeVar9 : vec3<f32>;
var<private> nodeVar10 : vec3<f32>;
var<private> nodeVar11 : vec3<f32>;
var<private> nodeVar12 : vec3<f32>;
var<private> nodeVar13 : f32;
var<private> nodeVar14 : f32;
var<private> nodeVar15 : f32;
var<private> nodeVar16 : f32;
var<private> nodeVar17 : f32;
var<private> nodeVar18 : vec3<f32>;
var<private> nodeVar19 : vec3<f32>;
var<private> nodeVar20 : vec3<f32>;
var<private> directDiffuse : vec3<f32>;
var<private> nodeVar21 : vec3<f32>;
var<private> nodeVar22 : vec3<f32>;
var<private> nodeVar23 : vec3<f32>;
var<private> nodeVar24 : vec3<f32>;
var<private> nodeVar25 : f32;
var<private> nodeVar26 : f32;
var<private> nodeVar27 : f32;
var<private> nodeVar28 : vec3<f32>;
var<private> nodeVar29 : vec3<f32>;
var<private> nodeVar30 : vec3<f32>;
var<private> nodeVar31 : vec3<f32>;
var<private> nodeVar32 : vec3<f32>;
var<private> directSpecular : vec3<f32>;
var<private> nodeVar33 : vec3<f32>;
var<private> nodeVar34 : f32;
var<private> nodeVar35 : f32;
var<private> nodeVar36 : f32;
var<private> nodeVar37 : vec3<f32>;
var<private> nodeVar38 : vec3<f32>;
var<private> nodeVar39 : vec3<f32>;
var<private> nodeVar40 : vec3<f32>;
var<private> irradiance : vec3<f32>;
var<private> normalWorld : vec3<f32>;
var<private> nodeVar41 : f32;
var<private> nodeVar42 : f32;
var<private> nodeVar43 : f32;
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
var<private> radiance : vec3<f32>;
var<private> nodeVar110 : vec3<f32>;
var<private> nodeVar111 : vec3<f32>;
var<private> nodeVar112 : vec3<f32>;
var<private> iblIrradiance : vec3<f32>;
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
var<private> nodeVar138 : vec3<f32>;
var<private> totalSpecular : vec3<f32>;
var<private> nodeVar139 : vec3<f32>;
var<private> outgoingLight : vec3<f32>;
var<private> nodeVar140 : vec3<f32>;
var<private> nodeVar141 : vec4<f32>;

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
	@location( 3 ) nodeVarying6 : vec2<f32> ) -> OutputStruct {

	// flow
	// code

	nodeVar0 = textureSample( nodeUniform1, nodeUniform1_sampler, ( object.nodeUniform2 * vec3<f32>( nodeVarying6, 1.0 ) ).xy );
	DiffuseColor = ( vec4<f32>( object.nodeUniform0, 1.0 ) * nodeVar0 );
	DiffuseColor.w = ( DiffuseColor.w * object.nodeUniform3 );
	DiffuseColor.w = 1.0;
	nodeVar1 = textureSample( nodeUniform5, nodeUniform5_sampler, ( object.nodeUniform6 * vec3<f32>( nodeVarying6, 1.0 ) ).xy );
	Metalness = ( object.nodeUniform4 * nodeVar1.z );
	normalViewGeometry = normalize( v_normalViewGeometry );
	nodeVar2 = max( abs( dpdx( normalViewGeometry ) ), abs( - dpdy( normalViewGeometry ) ) );
	Roughness = min( ( max( object.nodeUniform7, 0.0525 ) + max( max( nodeVar2.x, nodeVar2.y ), nodeVar2.z ) ), 1.0 );
	SpecularColor = vec3<f32>( 0.04, 0.04, 0.04 );
	SpecularColorBlended = mix( vec3<f32>( 0.04, 0.04, 0.04 ), DiffuseColor.xyz, Metalness );
	SpecularF90 = 1.0;
	DiffuseContribution = ( DiffuseColor.xyz * vec3<f32>( ( 1.0 - ( object.nodeUniform4 * nodeVar1.z ) ) ) );
	EmissiveColor = ( object.nodeUniform10 * vec3<f32>( object.nodeUniform11 ) );
	NORMAL_normalView = normalViewGeometry;
	normalView = NORMAL_normalView;
	positionViewDirection = normalize( v_positionViewDirection );
	nodeVar3 = dot( normalView, positionViewDirection );
	nodeVar4 = textureSample( nodeUniform12, nodeUniform12_sampler, vec2<f32>( Roughness, clamp( nodeVar3, 0.0, 1.0 ) ) ).xy;
	let dfg = nodeVar4;
	nodeVar5 = ( dfg.x + dfg.y );
	nodeVar6 = ( 1.0 / nodeVar5 );
	nodeVar7 = nodeVar6;
	nodeVar8 = ( nodeVar7 - 1.0 );
	nodeVar9 = ( SpecularColorBlended * vec3<f32>( nodeVar8 ) );
	nodeVar10 = ( nodeVar9 + vec3<f32>( 1.0 ) );
	let multiScatteringCompensation = nodeVar10;
	nodeVar11 = ( render.nodeUniform14 - v_positionView );
	nodeVar12 = normalize( nodeVar11 );
	nodeVar13 = dot( normalView, nodeVar12 );

	if ( ( render.nodeUniform16 > 0.0 ) ) {

		nodeVar15 = length( nodeVar11 );
		nodeVar16 = ( nodeVar15 / render.nodeUniform16 );
		nodeVar17 = clamp( ( 1.0 - ( ( ( nodeVar16 * nodeVar16 ) * nodeVar16 ) * nodeVar16 ) ), 0.0, 1.0 );
		nodeVar14 = ( ( 1.0 / max( pow( nodeVar15, render.nodeUniform17 ), 0.01 ) ) * ( nodeVar17 * nodeVar17 ) );

	} else {

		nodeVar14 = ( 1.0 / max( pow( length( nodeVar11 ), render.nodeUniform17 ), 0.01 ) );

	}

	nodeVar18 = ( render.nodeUniform15 * vec3<f32>( nodeVar14 ) );
	nodeVar19 = ( vec3<f32>( clamp( nodeVar13, 0.0, 1.0 ) ) * nodeVar18 );
	nodeVar20 = nodeVar19;
	directDiffuse = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar21 = ( DiffuseContribution * vec3<f32>( 0.3183098861837907 ) );
	nodeVar22 = ( nodeVar20 * nodeVar21 );
	nodeVar23 = ( nodeVar12 + positionViewDirection );
	nodeVar24 = normalize( nodeVar23 );
	nodeVar25 = dot( positionViewDirection, nodeVar24 );
	nodeVar26 = clamp( nodeVar25, 0.0, 1.0 );
	nodeVar27 = exp2( ( ( ( nodeVar26 * -5.55473 ) - 6.98316 ) * nodeVar26 ) );
	nodeVar28 = ( ( SpecularColor * vec3<f32>( ( 1.0 - nodeVar27 ) ) ) + vec3<f32>( ( SpecularF90 * nodeVar27 ) ) );
	nodeVar29 = ( vec3<f32>( 1.0 ) - nodeVar28 );
	nodeVar30 = nodeVar29;
	nodeVar31 = ( nodeVar22 * nodeVar30 );
	nodeVar32 = ( directDiffuse + nodeVar31 );
	directDiffuse = nodeVar32;
	directSpecular = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar33 = normalize( ( nodeVar12 + positionViewDirection ) );
	nodeVar34 = clamp( dot( positionViewDirection, nodeVar33 ), 0.0, 1.0 );
	nodeVar35 = exp2( ( ( ( nodeVar34 * -5.55473 ) - 6.98316 ) * nodeVar34 ) );
	nodeVar36 = ( Roughness * Roughness );
	nodeVar37 = ( ( ( ( SpecularColorBlended * vec3<f32>( ( 1.0 - nodeVar35 ) ) ) + vec3<f32>( ( 1.0 * nodeVar35 ) ) ) * vec3<f32>( V_GGX_SmithCorrelated( nodeVar36, clamp( dot( normalView, nodeVar12 ), 0.0, 1.0 ), clamp( dot( normalView, positionViewDirection ), 0.0, 1.0 ) ) ) ) * vec3<f32>( D_GGX( nodeVar36, clamp( dot( normalView, nodeVar33 ), 0.0, 1.0 ) ) ) );
	nodeVar38 = ( nodeVar20 * nodeVar37 );
	nodeVar39 = ( nodeVar38 * multiScatteringCompensation );
	nodeVar40 = ( directSpecular + nodeVar39 );
	directSpecular = nodeVar40;
	irradiance = vec3<f32>( 0.0, 0.0, 0.0 );
	normalWorld = normalize( ( vec4<f32>( normalView, 0.0 ) * render.cameraViewMatrix ).xyz );
	nodeVar41 = dot( normalWorld, normalize( render.nodeUniform21 ) );
	nodeVar42 = ( nodeVar41 * 0.5 );
	nodeVar43 = ( nodeVar42 + 0.5 );
	nodeVar44 = mix( render.nodeUniform18, render.nodeUniform19, nodeVar43 );
	nodeVar45 = ( irradiance + nodeVar44 );
	irradiance = nodeVar45;
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
	radiance = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar110 = mix( singleScatteringDielectric, singleScatteringMetallic, Metalness );
	nodeVar111 = ( radiance * nodeVar110 );
	nodeVar112 = mix( multiScatteringDielectric, multiScatteringMetallic, Metalness );
	iblIrradiance = vec3<f32>( 0.0, 0.0, 0.0 );
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
	nodeVar138 = ( directDiffuse + indirectDiffuse );
	totalDiffuse = nodeVar138;
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
