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
	nodeUniform11 : vec3<f32>,
	nodeUniform12 : f32,
	nodeUniform13 : f32,
	nodeUniform15 : vec3<f32>,
	nodeUniform17 : vec3<f32>,
	nodeUniform14 : vec3<f32>,
	nodeUniform10 : vec3<f32>
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
var<private> nodeVar9 : vec3<f32>;
var<private> nodeVar10 : vec3<f32>;
var<private> nodeVar11 : f32;
var<private> nodeVar12 : f32;
var<private> nodeVar13 : f32;
var<private> nodeVar14 : f32;
var<private> nodeVar15 : f32;
var<private> nodeVar16 : vec3<f32>;
var<private> nodeVar17 : vec3<f32>;
var<private> nodeVar18 : vec3<f32>;
var<private> directDiffuse : vec3<f32>;
var<private> nodeVar19 : vec3<f32>;
var<private> nodeVar20 : vec3<f32>;
var<private> nodeVar21 : vec3<f32>;
var<private> nodeVar22 : vec3<f32>;
var<private> nodeVar23 : f32;
var<private> nodeVar24 : f32;
var<private> nodeVar25 : f32;
var<private> nodeVar26 : vec3<f32>;
var<private> nodeVar27 : vec3<f32>;
var<private> nodeVar28 : vec3<f32>;
var<private> nodeVar29 : vec3<f32>;
var<private> nodeVar30 : vec3<f32>;
var<private> directSpecular : vec3<f32>;
var<private> nodeVar31 : vec3<f32>;
var<private> nodeVar32 : f32;
var<private> nodeVar33 : f32;
var<private> nodeVar34 : f32;
var<private> nodeVar35 : vec3<f32>;
var<private> nodeVar36 : vec3<f32>;
var<private> nodeVar37 : vec3<f32>;
var<private> nodeVar38 : vec3<f32>;
var<private> irradiance : vec3<f32>;
var<private> normalWorld : vec3<f32>;
var<private> nodeVar39 : f32;
var<private> nodeVar40 : f32;
var<private> nodeVar41 : f32;
var<private> nodeVar42 : vec3<f32>;
var<private> nodeVar43 : vec3<f32>;
var<private> nodeVar44 : vec3<f32>;
var<private> nodeVar45 : vec3<f32>;
var<private> nodeVar46 : vec3<f32>;
var<private> nodeVar47 : f32;
var<private> nodeVar48 : vec3<f32>;
var<private> nodeVar49 : vec3<f32>;
var<private> nodeVar50 : vec3<f32>;
var<private> nodeVar51 : vec3<f32>;
var<private> nodeVar52 : vec3<f32>;
var<private> nodeVar53 : vec3<f32>;
var<private> nodeVar54 : vec3<f32>;
var<private> nodeVar55 : f32;
var<private> nodeVar56 : f32;
var<private> nodeVar57 : f32;
var<private> nodeVar58 : vec3<f32>;
var<private> nodeVar59 : vec3<f32>;
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
var<private> indirectDiffuse : vec3<f32>;
var<private> nodeVar71 : vec3<f32>;
var<private> singleScatteringDielectric : vec3<f32>;
var<private> multiScatteringDielectric : vec3<f32>;
var<private> singleScatteringMetallic : vec3<f32>;
var<private> multiScatteringMetallic : vec3<f32>;
var<private> nodeVar72 : vec3<f32>;
var<private> nodeVar73 : f32;
var<private> nodeVar74 : vec3<f32>;
var<private> nodeVar75 : vec3<f32>;
var<private> nodeVar76 : vec3<f32>;
var<private> nodeVar77 : vec3<f32>;
var<private> nodeVar78 : vec3<f32>;
var<private> nodeVar79 : vec3<f32>;
var<private> nodeVar80 : vec3<f32>;
var<private> nodeVar81 : f32;
var<private> nodeVar82 : f32;
var<private> nodeVar83 : f32;
var<private> nodeVar84 : vec3<f32>;
var<private> nodeVar85 : vec3<f32>;
var<private> nodeVar86 : vec3<f32>;
var<private> nodeVar87 : vec3<f32>;
var<private> nodeVar88 : vec3<f32>;
var<private> nodeVar89 : vec3<f32>;
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
var<private> radiance : vec3<f32>;
var<private> nodeVar108 : vec3<f32>;
var<private> nodeVar109 : vec3<f32>;
var<private> nodeVar110 : vec3<f32>;
var<private> iblIrradiance : vec3<f32>;
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
var<private> indirectSpecular : vec3<f32>;
var<private> nodeVar121 : vec3<f32>;
var<private> nodeVar122 : vec3<f32>;
var<private> ambientOcclusion : f32;
var<private> nodeVar123 : vec3<f32>;
var<private> nodeVar124 : f32;
var<private> nodeVar125 : f32;
var<private> nodeVar126 : f32;
var<private> nodeVar127 : f32;
var<private> nodeVar128 : f32;
var<private> nodeVar129 : f32;
var<private> nodeVar130 : f32;
var<private> nodeVar131 : f32;
var<private> nodeVar132 : f32;
var<private> nodeVar133 : f32;
var<private> nodeVar134 : f32;
var<private> nodeVar135 : vec3<f32>;
var<private> totalDiffuse : vec3<f32>;
var<private> nodeVar136 : vec3<f32>;
var<private> totalSpecular : vec3<f32>;
var<private> nodeVar137 : vec3<f32>;
var<private> outgoingLight : vec3<f32>;
var<private> nodeVar138 : vec3<f32>;
var<private> nodeVar139 : vec4<f32>;

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
	@location( 2 ) v_positionViewDirection : vec3<f32> ) -> OutputStruct {

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
	nodeVar9 = ( render.nodeUniform10 - v_positionView );
	nodeVar10 = normalize( nodeVar9 );
	nodeVar11 = dot( normalView, nodeVar10 );

	if ( ( render.nodeUniform12 > 0.0 ) ) {

		nodeVar13 = length( nodeVar9 );
		nodeVar14 = ( nodeVar13 / render.nodeUniform12 );
		nodeVar15 = clamp( ( 1.0 - ( ( ( nodeVar14 * nodeVar14 ) * nodeVar14 ) * nodeVar14 ) ), 0.0, 1.0 );
		nodeVar12 = ( ( 1.0 / max( pow( nodeVar13, render.nodeUniform13 ), 0.01 ) ) * ( nodeVar15 * nodeVar15 ) );

	} else {

		nodeVar12 = ( 1.0 / max( pow( length( nodeVar9 ), render.nodeUniform13 ), 0.01 ) );

	}

	nodeVar16 = ( render.nodeUniform11 * vec3<f32>( nodeVar12 ) );
	nodeVar17 = ( vec3<f32>( clamp( nodeVar11, 0.0, 1.0 ) ) * nodeVar16 );
	nodeVar18 = nodeVar17;
	directDiffuse = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar19 = ( DiffuseContribution * vec3<f32>( 0.3183098861837907 ) );
	nodeVar20 = ( nodeVar18 * nodeVar19 );
	nodeVar21 = ( nodeVar10 + positionViewDirection );
	nodeVar22 = normalize( nodeVar21 );
	nodeVar23 = dot( positionViewDirection, nodeVar22 );
	nodeVar24 = clamp( nodeVar23, 0.0, 1.0 );
	nodeVar25 = exp2( ( ( ( nodeVar24 * -5.55473 ) - 6.98316 ) * nodeVar24 ) );
	nodeVar26 = ( ( SpecularColor * vec3<f32>( ( 1.0 - nodeVar25 ) ) ) + vec3<f32>( ( SpecularF90 * nodeVar25 ) ) );
	nodeVar27 = ( vec3<f32>( 1.0 ) - nodeVar26 );
	nodeVar28 = nodeVar27;
	nodeVar29 = ( nodeVar20 * nodeVar28 );
	nodeVar30 = ( directDiffuse + nodeVar29 );
	directDiffuse = nodeVar30;
	directSpecular = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar31 = normalize( ( nodeVar10 + positionViewDirection ) );
	nodeVar32 = clamp( dot( positionViewDirection, nodeVar31 ), 0.0, 1.0 );
	nodeVar33 = exp2( ( ( ( nodeVar32 * -5.55473 ) - 6.98316 ) * nodeVar32 ) );
	nodeVar34 = ( Roughness * Roughness );
	nodeVar35 = ( ( ( ( SpecularColorBlended * vec3<f32>( ( 1.0 - nodeVar33 ) ) ) + vec3<f32>( ( 1.0 * nodeVar33 ) ) ) * vec3<f32>( V_GGX_SmithCorrelated( nodeVar34, clamp( dot( normalView, nodeVar10 ), 0.0, 1.0 ), clamp( dot( normalView, positionViewDirection ), 0.0, 1.0 ) ) ) ) * vec3<f32>( D_GGX( nodeVar34, clamp( dot( normalView, nodeVar31 ), 0.0, 1.0 ) ) ) );
	nodeVar36 = ( nodeVar18 * nodeVar35 );
	nodeVar37 = ( nodeVar36 * multiScatteringCompensation );
	nodeVar38 = ( directSpecular + nodeVar37 );
	directSpecular = nodeVar38;
	irradiance = vec3<f32>( 0.0, 0.0, 0.0 );
	normalWorld = normalize( ( vec4<f32>( normalView, 0.0 ) * render.cameraViewMatrix ).xyz );
	nodeVar39 = dot( normalWorld, normalize( render.nodeUniform17 ) );
	nodeVar40 = ( nodeVar39 * 0.5 );
	nodeVar41 = ( nodeVar40 + 0.5 );
	nodeVar42 = mix( render.nodeUniform14, render.nodeUniform15, nodeVar41 );
	nodeVar43 = ( irradiance + nodeVar42 );
	irradiance = nodeVar43;
	nodeVar44 = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar45 = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar46 = ( SpecularColor * vec3<f32>( dfg.x ) );
	nodeVar47 = ( SpecularF90 * dfg.y );
	nodeVar48 = ( nodeVar46 + vec3<f32>( nodeVar47 ) );
	nodeVar49 = ( nodeVar44 + nodeVar48 );
	nodeVar44 = nodeVar49;
	nodeVar50 = ( vec3<f32>( 1.0 ) - SpecularColor );
	nodeVar51 = nodeVar50;
	nodeVar52 = ( nodeVar51 * vec3<f32>( 0.047619 ) );
	nodeVar53 = ( SpecularColor + nodeVar52 );
	nodeVar54 = ( nodeVar48 * nodeVar53 );
	nodeVar55 = ( dfg.x + dfg.y );
	nodeVar56 = ( 1.0 - nodeVar55 );
	nodeVar57 = nodeVar56;
	nodeVar58 = ( vec3<f32>( nodeVar57 ) * nodeVar53 );
	nodeVar59 = ( vec3<f32>( 1.0 ) - nodeVar58 );
	nodeVar60 = nodeVar59;
	nodeVar61 = ( nodeVar54 / nodeVar60 );
	nodeVar62 = ( nodeVar61 * vec3<f32>( nodeVar57 ) );
	nodeVar63 = ( nodeVar45 + nodeVar62 );
	nodeVar45 = nodeVar63;
	nodeVar64 = ( DiffuseContribution * vec3<f32>( 0.3183098861837907 ) );
	nodeVar65 = ( irradiance * nodeVar64 );
	nodeVar66 = ( nodeVar44 + nodeVar45 );
	nodeVar67 = ( vec3<f32>( 1.0 ) - nodeVar66 );
	nodeVar68 = nodeVar67;
	nodeVar69 = ( nodeVar65 * nodeVar68 );
	nodeVar70 = nodeVar69;
	indirectDiffuse = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar71 = ( indirectDiffuse + nodeVar70 );
	indirectDiffuse = nodeVar71;
	singleScatteringDielectric = vec3<f32>( 0.0, 0.0, 0.0 );
	multiScatteringDielectric = vec3<f32>( 0.0, 0.0, 0.0 );
	singleScatteringMetallic = vec3<f32>( 0.0, 0.0, 0.0 );
	multiScatteringMetallic = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar72 = ( SpecularColor * vec3<f32>( dfg.x ) );
	nodeVar73 = ( SpecularF90 * dfg.y );
	nodeVar74 = ( nodeVar72 + vec3<f32>( nodeVar73 ) );
	nodeVar75 = ( singleScatteringDielectric + nodeVar74 );
	singleScatteringDielectric = nodeVar75;
	nodeVar76 = ( vec3<f32>( 1.0 ) - SpecularColor );
	nodeVar77 = nodeVar76;
	nodeVar78 = ( nodeVar77 * vec3<f32>( 0.047619 ) );
	nodeVar79 = ( SpecularColor + nodeVar78 );
	nodeVar80 = ( nodeVar74 * nodeVar79 );
	nodeVar81 = ( dfg.x + dfg.y );
	nodeVar82 = ( 1.0 - nodeVar81 );
	nodeVar83 = nodeVar82;
	nodeVar84 = ( vec3<f32>( nodeVar83 ) * nodeVar79 );
	nodeVar85 = ( vec3<f32>( 1.0 ) - nodeVar84 );
	nodeVar86 = nodeVar85;
	nodeVar87 = ( nodeVar80 / nodeVar86 );
	nodeVar88 = ( nodeVar87 * vec3<f32>( nodeVar83 ) );
	nodeVar89 = ( multiScatteringDielectric + nodeVar88 );
	multiScatteringDielectric = nodeVar89;
	nodeVar90 = ( DiffuseColor.xyz * vec3<f32>( dfg.x ) );
	nodeVar91 = ( SpecularF90 * dfg.y );
	nodeVar92 = ( nodeVar90 + vec3<f32>( nodeVar91 ) );
	nodeVar93 = ( singleScatteringMetallic + nodeVar92 );
	singleScatteringMetallic = nodeVar93;
	nodeVar94 = ( vec3<f32>( 1.0 ) - DiffuseColor.xyz );
	nodeVar95 = nodeVar94;
	nodeVar96 = ( nodeVar95 * vec3<f32>( 0.047619 ) );
	nodeVar97 = ( DiffuseColor.xyz + nodeVar96 );
	nodeVar98 = ( nodeVar92 * nodeVar97 );
	nodeVar99 = ( dfg.x + dfg.y );
	nodeVar100 = ( 1.0 - nodeVar99 );
	nodeVar101 = nodeVar100;
	nodeVar102 = ( vec3<f32>( nodeVar101 ) * nodeVar97 );
	nodeVar103 = ( vec3<f32>( 1.0 ) - nodeVar102 );
	nodeVar104 = nodeVar103;
	nodeVar105 = ( nodeVar98 / nodeVar104 );
	nodeVar106 = ( nodeVar105 * vec3<f32>( nodeVar101 ) );
	nodeVar107 = ( multiScatteringMetallic + nodeVar106 );
	multiScatteringMetallic = nodeVar107;
	radiance = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar108 = mix( singleScatteringDielectric, singleScatteringMetallic, Metalness );
	nodeVar109 = ( radiance * nodeVar108 );
	nodeVar110 = mix( multiScatteringDielectric, multiScatteringMetallic, Metalness );
	iblIrradiance = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar111 = ( iblIrradiance * vec3<f32>( 0.3183098861837907 ) );
	nodeVar112 = ( nodeVar110 * nodeVar111 );
	nodeVar113 = ( nodeVar109 + nodeVar112 );
	nodeVar114 = nodeVar113;
	nodeVar115 = ( singleScatteringDielectric + multiScatteringDielectric );
	nodeVar116 = ( vec3<f32>( 1.0 ) - nodeVar115 );
	nodeVar117 = nodeVar116;
	nodeVar118 = ( DiffuseContribution * nodeVar117 );
	nodeVar119 = ( nodeVar118 * nodeVar111 );
	nodeVar120 = nodeVar119;
	indirectSpecular = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar121 = ( indirectSpecular + nodeVar114 );
	indirectSpecular = nodeVar121;
	nodeVar122 = ( indirectDiffuse + nodeVar120 );
	indirectDiffuse = nodeVar122;
	ambientOcclusion = 1.0;
	nodeVar123 = ( indirectDiffuse * vec3<f32>( ambientOcclusion ) );
	indirectDiffuse = nodeVar123;
	nodeVar124 = dot( normalView, positionViewDirection );
	nodeVar125 = ( clamp( nodeVar124, 0.0, 1.0 ) + ambientOcclusion );
	nodeVar126 = ( Roughness * -16.0 );
	nodeVar127 = ( 1.0 - nodeVar126 );
	nodeVar128 = nodeVar127;
	nodeVar129 = ( - nodeVar128 );
	nodeVar130 = exp2( nodeVar129 );
	nodeVar131 = pow( nodeVar125, nodeVar130 );
	nodeVar132 = ( 1.0 - nodeVar131 );
	nodeVar133 = nodeVar132;
	nodeVar134 = ( ambientOcclusion - nodeVar133 );
	nodeVar135 = ( indirectSpecular * vec3<f32>( clamp( nodeVar134, 0.0, 1.0 ) ) );
	indirectSpecular = nodeVar135;
	nodeVar136 = ( directDiffuse + indirectDiffuse );
	totalDiffuse = nodeVar136;
	nodeVar137 = ( directSpecular + indirectSpecular );
	totalSpecular = nodeVar137;
	nodeVar138 = ( totalDiffuse + totalSpecular );
	outgoingLight = nodeVar138;
	nodeVar139 = max( vec4<f32>( ( outgoingLight + EmissiveColor ), DiffuseColor.w ), vec4<f32>( 0.0 ) );
	Output = nodeVar139;

	// result

	output.color = nodeVar139;

	return output;

}
