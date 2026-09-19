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

struct objectStruct {
	nodeUniform0 : vec3<f32>,
	nodeUniform1 : f32,
	nodeUniform2 : f32,
	nodeUniform3 : f32,
	nodeUniform5 : mat3x3<f32>,
	nodeUniform6 : vec3<f32>,
	nodeUniform7 : f32,
	nodeUniform9 : mat4x4<f32>
};
@binding( 0 ) @group( 1 )
var<uniform> object : objectStruct;

struct renderStruct {
	cameraProjectionMatrix : mat4x4<f32>,
	cameraViewMatrix : mat4x4<f32>,
	nodeUniform10 : vec3<f32>,
	nodeUniform12 : vec3<f32>,
	nodeUniform13 : f32,
	nodeUniform14 : f32,
	nodeUniform11 : vec3<f32>
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
var<private> irradiance : vec3<f32>;
var<private> nodeVar9 : vec3<f32>;
var<private> nodeVar10 : vec3<f32>;
var<private> nodeVar11 : vec3<f32>;
var<private> nodeVar12 : f32;
var<private> nodeVar13 : f32;
var<private> nodeVar14 : f32;
var<private> nodeVar15 : f32;
var<private> nodeVar16 : f32;
var<private> nodeVar17 : vec3<f32>;
var<private> nodeVar18 : vec3<f32>;
var<private> nodeVar19 : vec3<f32>;
var<private> directDiffuse : vec3<f32>;
var<private> nodeVar20 : vec3<f32>;
var<private> nodeVar21 : vec3<f32>;
var<private> nodeVar22 : vec3<f32>;
var<private> nodeVar23 : vec3<f32>;
var<private> nodeVar24 : f32;
var<private> nodeVar25 : f32;
var<private> nodeVar26 : f32;
var<private> nodeVar27 : vec3<f32>;
var<private> nodeVar28 : vec3<f32>;
var<private> nodeVar29 : vec3<f32>;
var<private> nodeVar30 : vec3<f32>;
var<private> nodeVar31 : vec3<f32>;
var<private> directSpecular : vec3<f32>;
var<private> nodeVar32 : vec3<f32>;
var<private> nodeVar33 : f32;
var<private> nodeVar34 : f32;
var<private> nodeVar35 : f32;
var<private> nodeVar36 : vec3<f32>;
var<private> nodeVar37 : vec3<f32>;
var<private> nodeVar38 : vec3<f32>;
var<private> nodeVar39 : vec3<f32>;
var<private> nodeVar40 : vec3<f32>;
var<private> nodeVar41 : vec3<f32>;
var<private> nodeVar42 : vec3<f32>;
var<private> nodeVar43 : f32;
var<private> nodeVar44 : vec3<f32>;
var<private> nodeVar45 : vec3<f32>;
var<private> nodeVar46 : vec3<f32>;
var<private> nodeVar47 : vec3<f32>;
var<private> nodeVar48 : vec3<f32>;
var<private> nodeVar49 : vec3<f32>;
var<private> nodeVar50 : vec3<f32>;
var<private> nodeVar51 : f32;
var<private> nodeVar52 : f32;
var<private> nodeVar53 : f32;
var<private> nodeVar54 : vec3<f32>;
var<private> nodeVar55 : vec3<f32>;
var<private> nodeVar56 : vec3<f32>;
var<private> nodeVar57 : vec3<f32>;
var<private> nodeVar58 : vec3<f32>;
var<private> nodeVar59 : vec3<f32>;
var<private> nodeVar60 : vec3<f32>;
var<private> nodeVar61 : vec3<f32>;
var<private> nodeVar62 : vec3<f32>;
var<private> nodeVar63 : vec3<f32>;
var<private> nodeVar64 : vec3<f32>;
var<private> nodeVar65 : vec3<f32>;
var<private> nodeVar66 : vec3<f32>;
var<private> indirectDiffuse : vec3<f32>;
var<private> nodeVar67 : vec3<f32>;
var<private> singleScatteringDielectric : vec3<f32>;
var<private> multiScatteringDielectric : vec3<f32>;
var<private> singleScatteringMetallic : vec3<f32>;
var<private> multiScatteringMetallic : vec3<f32>;
var<private> nodeVar68 : vec3<f32>;
var<private> nodeVar69 : f32;
var<private> nodeVar70 : vec3<f32>;
var<private> nodeVar71 : vec3<f32>;
var<private> nodeVar72 : vec3<f32>;
var<private> nodeVar73 : vec3<f32>;
var<private> nodeVar74 : vec3<f32>;
var<private> nodeVar75 : vec3<f32>;
var<private> nodeVar76 : vec3<f32>;
var<private> nodeVar77 : f32;
var<private> nodeVar78 : f32;
var<private> nodeVar79 : f32;
var<private> nodeVar80 : vec3<f32>;
var<private> nodeVar81 : vec3<f32>;
var<private> nodeVar82 : vec3<f32>;
var<private> nodeVar83 : vec3<f32>;
var<private> nodeVar84 : vec3<f32>;
var<private> nodeVar85 : vec3<f32>;
var<private> nodeVar86 : vec3<f32>;
var<private> nodeVar87 : f32;
var<private> nodeVar88 : vec3<f32>;
var<private> nodeVar89 : vec3<f32>;
var<private> nodeVar90 : vec3<f32>;
var<private> nodeVar91 : vec3<f32>;
var<private> nodeVar92 : vec3<f32>;
var<private> nodeVar93 : vec3<f32>;
var<private> nodeVar94 : vec3<f32>;
var<private> nodeVar95 : f32;
var<private> nodeVar96 : f32;
var<private> nodeVar97 : f32;
var<private> nodeVar98 : vec3<f32>;
var<private> nodeVar99 : vec3<f32>;
var<private> nodeVar100 : vec3<f32>;
var<private> nodeVar101 : vec3<f32>;
var<private> nodeVar102 : vec3<f32>;
var<private> nodeVar103 : vec3<f32>;
var<private> radiance : vec3<f32>;
var<private> nodeVar104 : vec3<f32>;
var<private> nodeVar105 : vec3<f32>;
var<private> nodeVar106 : vec3<f32>;
var<private> iblIrradiance : vec3<f32>;
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
var<private> indirectSpecular : vec3<f32>;
var<private> nodeVar117 : vec3<f32>;
var<private> nodeVar118 : vec3<f32>;
var<private> ambientOcclusion : f32;
var<private> nodeVar119 : vec3<f32>;
var<private> nodeVar120 : f32;
var<private> nodeVar121 : f32;
var<private> nodeVar122 : f32;
var<private> nodeVar123 : f32;
var<private> nodeVar124 : f32;
var<private> nodeVar125 : f32;
var<private> nodeVar126 : f32;
var<private> nodeVar127 : f32;
var<private> nodeVar128 : f32;
var<private> nodeVar129 : f32;
var<private> nodeVar130 : f32;
var<private> nodeVar131 : vec3<f32>;
var<private> totalDiffuse : vec3<f32>;
var<private> nodeVar132 : vec3<f32>;
var<private> totalSpecular : vec3<f32>;
var<private> nodeVar133 : vec3<f32>;
var<private> outgoingLight : vec3<f32>;
var<private> nodeVar134 : vec3<f32>;
var<private> nodeVar135 : vec4<f32>;

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
	@location( 3 ) nodeVarying6 : vec4<f32>,
	@builtin( front_facing ) isFront : bool ) -> OutputStruct {

	// flow
	// code

	DiffuseColor = ( vec4<f32>( object.nodeUniform0, 1.0 ) * nodeVarying6 );
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
	NORMAL_normalView = ( normalViewGeometry * vec3<f32>( ( ( f32( isFront ) * 2.0 ) - 1.0 ) ) );
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
	irradiance = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar9 = ( irradiance + render.nodeUniform10 );
	irradiance = nodeVar9;
	nodeVar10 = ( render.nodeUniform11 - v_positionView );
	nodeVar11 = normalize( nodeVar10 );
	nodeVar12 = dot( normalView, nodeVar11 );

	if ( ( render.nodeUniform13 > 0.0 ) ) {

		nodeVar14 = length( nodeVar10 );
		nodeVar15 = ( nodeVar14 / render.nodeUniform13 );
		nodeVar16 = clamp( ( 1.0 - ( ( ( nodeVar15 * nodeVar15 ) * nodeVar15 ) * nodeVar15 ) ), 0.0, 1.0 );
		nodeVar13 = ( ( 1.0 / max( pow( nodeVar14, render.nodeUniform14 ), 0.01 ) ) * ( nodeVar16 * nodeVar16 ) );

	} else {

		nodeVar13 = ( 1.0 / max( pow( length( nodeVar10 ), render.nodeUniform14 ), 0.01 ) );

	}

	nodeVar17 = ( render.nodeUniform12 * vec3<f32>( nodeVar13 ) );
	nodeVar18 = ( vec3<f32>( clamp( nodeVar12, 0.0, 1.0 ) ) * nodeVar17 );
	nodeVar19 = nodeVar18;
	directDiffuse = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar20 = ( DiffuseContribution * vec3<f32>( 0.3183098861837907 ) );
	nodeVar21 = ( nodeVar19 * nodeVar20 );
	nodeVar22 = ( nodeVar11 + positionViewDirection );
	nodeVar23 = normalize( nodeVar22 );
	nodeVar24 = dot( positionViewDirection, nodeVar23 );
	nodeVar25 = clamp( nodeVar24, 0.0, 1.0 );
	nodeVar26 = exp2( ( ( ( nodeVar25 * -5.55473 ) - 6.98316 ) * nodeVar25 ) );
	nodeVar27 = ( ( SpecularColor * vec3<f32>( ( 1.0 - nodeVar26 ) ) ) + vec3<f32>( ( SpecularF90 * nodeVar26 ) ) );
	nodeVar28 = ( vec3<f32>( 1.0 ) - nodeVar27 );
	nodeVar29 = nodeVar28;
	nodeVar30 = ( nodeVar21 * nodeVar29 );
	nodeVar31 = ( directDiffuse + nodeVar30 );
	directDiffuse = nodeVar31;
	directSpecular = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar32 = normalize( ( nodeVar11 + positionViewDirection ) );
	nodeVar33 = clamp( dot( positionViewDirection, nodeVar32 ), 0.0, 1.0 );
	nodeVar34 = exp2( ( ( ( nodeVar33 * -5.55473 ) - 6.98316 ) * nodeVar33 ) );
	nodeVar35 = ( Roughness * Roughness );
	nodeVar36 = ( ( ( ( SpecularColorBlended * vec3<f32>( ( 1.0 - nodeVar34 ) ) ) + vec3<f32>( ( 1.0 * nodeVar34 ) ) ) * vec3<f32>( V_GGX_SmithCorrelated( nodeVar35, clamp( dot( normalView, nodeVar11 ), 0.0, 1.0 ), clamp( dot( normalView, positionViewDirection ), 0.0, 1.0 ) ) ) ) * vec3<f32>( D_GGX( nodeVar35, clamp( dot( normalView, nodeVar32 ), 0.0, 1.0 ) ) ) );
	nodeVar37 = ( nodeVar19 * nodeVar36 );
	nodeVar38 = ( nodeVar37 * multiScatteringCompensation );
	nodeVar39 = ( directSpecular + nodeVar38 );
	directSpecular = nodeVar39;
	nodeVar40 = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar41 = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar42 = ( SpecularColor * vec3<f32>( dfg.x ) );
	nodeVar43 = ( SpecularF90 * dfg.y );
	nodeVar44 = ( nodeVar42 + vec3<f32>( nodeVar43 ) );
	nodeVar45 = ( nodeVar40 + nodeVar44 );
	nodeVar40 = nodeVar45;
	nodeVar46 = ( vec3<f32>( 1.0 ) - SpecularColor );
	nodeVar47 = nodeVar46;
	nodeVar48 = ( nodeVar47 * vec3<f32>( 0.047619 ) );
	nodeVar49 = ( SpecularColor + nodeVar48 );
	nodeVar50 = ( nodeVar44 * nodeVar49 );
	nodeVar51 = ( dfg.x + dfg.y );
	nodeVar52 = ( 1.0 - nodeVar51 );
	nodeVar53 = nodeVar52;
	nodeVar54 = ( vec3<f32>( nodeVar53 ) * nodeVar49 );
	nodeVar55 = ( vec3<f32>( 1.0 ) - nodeVar54 );
	nodeVar56 = nodeVar55;
	nodeVar57 = ( nodeVar50 / nodeVar56 );
	nodeVar58 = ( nodeVar57 * vec3<f32>( nodeVar53 ) );
	nodeVar59 = ( nodeVar41 + nodeVar58 );
	nodeVar41 = nodeVar59;
	nodeVar60 = ( DiffuseContribution * vec3<f32>( 0.3183098861837907 ) );
	nodeVar61 = ( irradiance * nodeVar60 );
	nodeVar62 = ( nodeVar40 + nodeVar41 );
	nodeVar63 = ( vec3<f32>( 1.0 ) - nodeVar62 );
	nodeVar64 = nodeVar63;
	nodeVar65 = ( nodeVar61 * nodeVar64 );
	nodeVar66 = nodeVar65;
	indirectDiffuse = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar67 = ( indirectDiffuse + nodeVar66 );
	indirectDiffuse = nodeVar67;
	singleScatteringDielectric = vec3<f32>( 0.0, 0.0, 0.0 );
	multiScatteringDielectric = vec3<f32>( 0.0, 0.0, 0.0 );
	singleScatteringMetallic = vec3<f32>( 0.0, 0.0, 0.0 );
	multiScatteringMetallic = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar68 = ( SpecularColor * vec3<f32>( dfg.x ) );
	nodeVar69 = ( SpecularF90 * dfg.y );
	nodeVar70 = ( nodeVar68 + vec3<f32>( nodeVar69 ) );
	nodeVar71 = ( singleScatteringDielectric + nodeVar70 );
	singleScatteringDielectric = nodeVar71;
	nodeVar72 = ( vec3<f32>( 1.0 ) - SpecularColor );
	nodeVar73 = nodeVar72;
	nodeVar74 = ( nodeVar73 * vec3<f32>( 0.047619 ) );
	nodeVar75 = ( SpecularColor + nodeVar74 );
	nodeVar76 = ( nodeVar70 * nodeVar75 );
	nodeVar77 = ( dfg.x + dfg.y );
	nodeVar78 = ( 1.0 - nodeVar77 );
	nodeVar79 = nodeVar78;
	nodeVar80 = ( vec3<f32>( nodeVar79 ) * nodeVar75 );
	nodeVar81 = ( vec3<f32>( 1.0 ) - nodeVar80 );
	nodeVar82 = nodeVar81;
	nodeVar83 = ( nodeVar76 / nodeVar82 );
	nodeVar84 = ( nodeVar83 * vec3<f32>( nodeVar79 ) );
	nodeVar85 = ( multiScatteringDielectric + nodeVar84 );
	multiScatteringDielectric = nodeVar85;
	nodeVar86 = ( DiffuseColor.xyz * vec3<f32>( dfg.x ) );
	nodeVar87 = ( SpecularF90 * dfg.y );
	nodeVar88 = ( nodeVar86 + vec3<f32>( nodeVar87 ) );
	nodeVar89 = ( singleScatteringMetallic + nodeVar88 );
	singleScatteringMetallic = nodeVar89;
	nodeVar90 = ( vec3<f32>( 1.0 ) - DiffuseColor.xyz );
	nodeVar91 = nodeVar90;
	nodeVar92 = ( nodeVar91 * vec3<f32>( 0.047619 ) );
	nodeVar93 = ( DiffuseColor.xyz + nodeVar92 );
	nodeVar94 = ( nodeVar88 * nodeVar93 );
	nodeVar95 = ( dfg.x + dfg.y );
	nodeVar96 = ( 1.0 - nodeVar95 );
	nodeVar97 = nodeVar96;
	nodeVar98 = ( vec3<f32>( nodeVar97 ) * nodeVar93 );
	nodeVar99 = ( vec3<f32>( 1.0 ) - nodeVar98 );
	nodeVar100 = nodeVar99;
	nodeVar101 = ( nodeVar94 / nodeVar100 );
	nodeVar102 = ( nodeVar101 * vec3<f32>( nodeVar97 ) );
	nodeVar103 = ( multiScatteringMetallic + nodeVar102 );
	multiScatteringMetallic = nodeVar103;
	radiance = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar104 = mix( singleScatteringDielectric, singleScatteringMetallic, Metalness );
	nodeVar105 = ( radiance * nodeVar104 );
	nodeVar106 = mix( multiScatteringDielectric, multiScatteringMetallic, Metalness );
	iblIrradiance = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar107 = ( iblIrradiance * vec3<f32>( 0.3183098861837907 ) );
	nodeVar108 = ( nodeVar106 * nodeVar107 );
	nodeVar109 = ( nodeVar105 + nodeVar108 );
	nodeVar110 = nodeVar109;
	nodeVar111 = ( singleScatteringDielectric + multiScatteringDielectric );
	nodeVar112 = ( vec3<f32>( 1.0 ) - nodeVar111 );
	nodeVar113 = nodeVar112;
	nodeVar114 = ( DiffuseContribution * nodeVar113 );
	nodeVar115 = ( nodeVar114 * nodeVar107 );
	nodeVar116 = nodeVar115;
	indirectSpecular = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar117 = ( indirectSpecular + nodeVar110 );
	indirectSpecular = nodeVar117;
	nodeVar118 = ( indirectDiffuse + nodeVar116 );
	indirectDiffuse = nodeVar118;
	ambientOcclusion = 1.0;
	nodeVar119 = ( indirectDiffuse * vec3<f32>( ambientOcclusion ) );
	indirectDiffuse = nodeVar119;
	nodeVar120 = dot( normalView, positionViewDirection );
	nodeVar121 = ( clamp( nodeVar120, 0.0, 1.0 ) + ambientOcclusion );
	nodeVar122 = ( Roughness * -16.0 );
	nodeVar123 = ( 1.0 - nodeVar122 );
	nodeVar124 = nodeVar123;
	nodeVar125 = ( - nodeVar124 );
	nodeVar126 = exp2( nodeVar125 );
	nodeVar127 = pow( nodeVar121, nodeVar126 );
	nodeVar128 = ( 1.0 - nodeVar127 );
	nodeVar129 = nodeVar128;
	nodeVar130 = ( ambientOcclusion - nodeVar129 );
	nodeVar131 = ( indirectSpecular * vec3<f32>( clamp( nodeVar130, 0.0, 1.0 ) ) );
	indirectSpecular = nodeVar131;
	nodeVar132 = ( directDiffuse + indirectDiffuse );
	totalDiffuse = nodeVar132;
	nodeVar133 = ( directSpecular + indirectSpecular );
	totalSpecular = nodeVar133;
	nodeVar134 = ( totalDiffuse + totalSpecular );
	outgoingLight = nodeVar134;
	nodeVar135 = max( vec4<f32>( ( outgoingLight + EmissiveColor ), DiffuseColor.w ), vec4<f32>( 0.0 ) );
	Output = nodeVar135;

	// result

	output.color = nodeVar135;

	return output;

}
