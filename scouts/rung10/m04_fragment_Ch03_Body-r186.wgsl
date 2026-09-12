// Three.js r186 - Node System

// global
diagnostic( off, derivative_uniformity );


// structs

struct OutputStruct {
	@location( 0 ) color: vec4<f32>
};
var<private> output : OutputStruct;

// uniforms
@binding( 1 ) @group( 1 ) var nodeUniform4_sampler : sampler;
@binding( 2 ) @group( 1 ) var nodeUniform4 : texture_2d<f32>;
@binding( 3 ) @group( 1 ) var nodeUniform8_sampler : sampler;
@binding( 4 ) @group( 1 ) var nodeUniform8 : texture_2d<f32>;
@binding( 5 ) @group( 1 ) var nodeUniform16_sampler : sampler;
@binding( 6 ) @group( 1 ) var nodeUniform16 : texture_2d<f32>;
@binding( 7 ) @group( 1 ) var nodeUniform21_sampler : sampler;
@binding( 8 ) @group( 1 ) var nodeUniform21 : texture_2d<f32>;
@binding( 9 ) @group( 1 ) var nodeUniform23_sampler : sampler;
@binding( 10 ) @group( 1 ) var nodeUniform23 : texture_2d<f32>;

struct objectStruct {
	nodeUniform0 : mat4x4<f32>,
	nodeUniform2 : mat4x4<f32>,
	nodeUniform3 : vec3<f32>,
	nodeUniform5 : mat3x3<f32>,
	nodeUniform6 : f32,
	nodeUniform7 : f32,
	nodeUniform9 : mat3x3<f32>,
	nodeUniform10 : f32,
	nodeUniform11 : mat3x3<f32>,
	nodeUniform13 : mat3x3<f32>,
	nodeUniform14 : f32,
	nodeUniform15 : vec3<f32>,
	nodeUniform17 : mat3x3<f32>,
	nodeUniform18 : f32,
	nodeUniform19 : vec3<f32>,
	nodeUniform20 : f32,
	nodeUniform22 : mat4x4<f32>,
	nodeUniform24 : mat3x3<f32>,
	nodeUniform25 : vec2<f32>
};
@binding( 0 ) @group( 1 )
var<uniform> object : objectStruct;

struct renderStruct {
	cameraProjectionMatrix : mat4x4<f32>,
	cameraViewMatrix : mat4x4<f32>,
	nodeUniform27 : vec3<f32>,
	nodeUniform28 : f32,
	nodeUniform29 : f32,
	nodeUniform30 : vec3<f32>,
	nodeUniform26 : vec3<f32>
};
@binding( 0 ) @group( 0 )
var<uniform> render : renderStruct;

// vars
var<private> DiffuseColor : vec4<f32>;
var<private> nodeVar1 : vec4<f32>;
var<private> Metalness : f32;
var<private> nodeVar2 : vec4<f32>;
var<private> Roughness : f32;
var<private> nodeVar3 : vec4<f32>;
var<private> normalViewGeometry : vec3<f32>;
var<private> nodeVar4 : vec3<f32>;
var<private> IOR : f32;
var<private> SpecularColor : vec3<f32>;
var<private> nodeVar5 : f32;
var<private> nodeVar6 : vec4<f32>;
var<private> SpecularColorBlended : vec3<f32>;
var<private> SpecularF90 : f32;
var<private> DiffuseContribution : vec3<f32>;
var<private> EmissiveColor : vec3<f32>;
var<private> Output : vec4<f32>;
var<private> nodeVar7 : f32;
var<private> NORMAL_normalView : vec3<f32>;
var<private> nodeVar8 : vec3<f32>;
var<private> nodeVar9 : vec2<f32>;
var<private> nodeVar10 : vec3<f32>;
var<private> nodeVar11 : vec2<f32>;
var<private> nodeVar12 : vec3<f32>;
var<private> nodeVar13 : f32;
var<private> nodeVar14 : vec3<f32>;
var<private> nodeVar15 : f32;
var<private> tangentViewFrame : vec3<f32>;
var<private> NORMAL_tangentView : vec3<f32>;
var<private> bitangentViewFrame : vec3<f32>;
var<private> NORMAL_bitangentView : vec3<f32>;
var<private> NORMAL_TBNViewMatrix : mat3x3<f32>;
var<private> nodeVar16 : vec4<f32>;
var<private> nodeVar17 : vec4<f32>;
var<private> normalView : vec3<f32>;
var<private> positionViewDirection : vec3<f32>;
var<private> nodeVar18 : f32;
var<private> nodeVar19 : vec2<f32>;
var<private> nodeVar20 : f32;
var<private> nodeVar21 : f32;
var<private> nodeVar22 : f32;
var<private> nodeVar23 : f32;
var<private> nodeVar24 : vec3<f32>;
var<private> nodeVar25 : vec3<f32>;
var<private> nodeVar26 : vec3<f32>;
var<private> nodeVar27 : vec3<f32>;
var<private> nodeVar28 : f32;
var<private> nodeVar29 : f32;
var<private> nodeVar30 : f32;
var<private> nodeVar31 : f32;
var<private> nodeVar32 : f32;
var<private> nodeVar33 : vec3<f32>;
var<private> nodeVar34 : vec3<f32>;
var<private> nodeVar35 : vec3<f32>;
var<private> directDiffuse : vec3<f32>;
var<private> nodeVar36 : vec3<f32>;
var<private> nodeVar37 : vec3<f32>;
var<private> nodeVar38 : vec3<f32>;
var<private> nodeVar39 : vec3<f32>;
var<private> nodeVar40 : f32;
var<private> nodeVar41 : f32;
var<private> nodeVar42 : f32;
var<private> nodeVar43 : vec3<f32>;
var<private> nodeVar44 : vec3<f32>;
var<private> nodeVar45 : vec3<f32>;
var<private> nodeVar46 : vec3<f32>;
var<private> nodeVar47 : vec3<f32>;
var<private> directSpecular : vec3<f32>;
var<private> nodeVar48 : vec3<f32>;
var<private> nodeVar49 : f32;
var<private> nodeVar50 : f32;
var<private> nodeVar51 : f32;
var<private> nodeVar52 : vec3<f32>;
var<private> nodeVar53 : vec3<f32>;
var<private> nodeVar54 : vec3<f32>;
var<private> nodeVar55 : vec3<f32>;
var<private> irradiance : vec3<f32>;
var<private> nodeVar56 : vec3<f32>;
var<private> nodeVar57 : vec3<f32>;
var<private> nodeVar58 : vec3<f32>;
var<private> nodeVar59 : vec3<f32>;
var<private> nodeVar60 : f32;
var<private> nodeVar61 : vec3<f32>;
var<private> nodeVar62 : vec3<f32>;
var<private> nodeVar63 : vec3<f32>;
var<private> nodeVar64 : vec3<f32>;
var<private> nodeVar65 : vec3<f32>;
var<private> nodeVar66 : vec3<f32>;
var<private> nodeVar67 : vec3<f32>;
var<private> nodeVar68 : f32;
var<private> nodeVar69 : f32;
var<private> nodeVar70 : f32;
var<private> nodeVar71 : vec3<f32>;
var<private> nodeVar72 : vec3<f32>;
var<private> nodeVar73 : vec3<f32>;
var<private> nodeVar74 : vec3<f32>;
var<private> nodeVar75 : vec3<f32>;
var<private> nodeVar76 : vec3<f32>;
var<private> nodeVar77 : vec3<f32>;
var<private> nodeVar78 : vec3<f32>;
var<private> nodeVar79 : vec3<f32>;
var<private> nodeVar80 : vec3<f32>;
var<private> nodeVar81 : vec3<f32>;
var<private> nodeVar82 : vec3<f32>;
var<private> nodeVar83 : vec3<f32>;
var<private> indirectDiffuse : vec3<f32>;
var<private> nodeVar84 : vec3<f32>;
var<private> singleScatteringDielectric : vec3<f32>;
var<private> multiScatteringDielectric : vec3<f32>;
var<private> singleScatteringMetallic : vec3<f32>;
var<private> multiScatteringMetallic : vec3<f32>;
var<private> nodeVar85 : vec3<f32>;
var<private> nodeVar86 : f32;
var<private> nodeVar87 : vec3<f32>;
var<private> nodeVar88 : vec3<f32>;
var<private> nodeVar89 : vec3<f32>;
var<private> nodeVar90 : vec3<f32>;
var<private> nodeVar91 : vec3<f32>;
var<private> nodeVar92 : vec3<f32>;
var<private> nodeVar93 : vec3<f32>;
var<private> nodeVar94 : f32;
var<private> nodeVar95 : f32;
var<private> nodeVar96 : f32;
var<private> nodeVar97 : vec3<f32>;
var<private> nodeVar98 : vec3<f32>;
var<private> nodeVar99 : vec3<f32>;
var<private> nodeVar100 : vec3<f32>;
var<private> nodeVar101 : vec3<f32>;
var<private> nodeVar102 : vec3<f32>;
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
var<private> radiance : vec3<f32>;
var<private> nodeVar121 : vec3<f32>;
var<private> nodeVar122 : vec3<f32>;
var<private> nodeVar123 : vec3<f32>;
var<private> iblIrradiance : vec3<f32>;
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
var<private> indirectSpecular : vec3<f32>;
var<private> nodeVar134 : vec3<f32>;
var<private> nodeVar135 : vec3<f32>;
var<private> ambientOcclusion : f32;
var<private> nodeVar136 : vec3<f32>;
var<private> nodeVar137 : f32;
var<private> nodeVar138 : f32;
var<private> nodeVar139 : f32;
var<private> nodeVar140 : f32;
var<private> nodeVar141 : f32;
var<private> nodeVar142 : f32;
var<private> nodeVar143 : f32;
var<private> nodeVar144 : f32;
var<private> nodeVar145 : f32;
var<private> nodeVar146 : f32;
var<private> nodeVar147 : f32;
var<private> nodeVar148 : vec3<f32>;
var<private> totalDiffuse : vec3<f32>;
var<private> nodeVar149 : vec3<f32>;
var<private> totalSpecular : vec3<f32>;
var<private> nodeVar150 : vec3<f32>;
var<private> outgoingLight : vec3<f32>;
var<private> nodeVar151 : vec3<f32>;
var<private> nodeVar152 : vec4<f32>;

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

	nodeVar1 = textureSample( nodeUniform4, nodeUniform4_sampler, ( object.nodeUniform5 * vec3<f32>( nodeVarying6, 1.0 ) ).xy );
	DiffuseColor = ( vec4<f32>( object.nodeUniform3, 1.0 ) * nodeVar1 );
	DiffuseColor.w = ( DiffuseColor.w * object.nodeUniform6 );
	DiffuseColor.w = 1.0;
	nodeVar2 = textureSample( nodeUniform8, nodeUniform8_sampler, ( object.nodeUniform9 * vec3<f32>( nodeVarying6, 1.0 ) ).xy );
	Metalness = ( object.nodeUniform7 * nodeVar2.z );
	nodeVar3 = textureSample( nodeUniform8, nodeUniform8_sampler, ( object.nodeUniform11 * vec3<f32>( nodeVarying6, 1.0 ) ).xy );
	normalViewGeometry = normalize( v_normalViewGeometry );
	nodeVar4 = max( abs( dpdx( normalViewGeometry ) ), abs( - dpdy( normalViewGeometry ) ) );
	Roughness = min( ( max( ( object.nodeUniform10 * nodeVar3.y ), 0.0525 ) + max( max( nodeVar4.x, nodeVar4.y ), nodeVar4.z ) ), 1.0 );
	IOR = object.nodeUniform14;
	nodeVar5 = ( ( IOR - 1.0 ) / ( IOR + 1.0 ) );
	nodeVar6 = textureSample( nodeUniform16, nodeUniform16_sampler, ( object.nodeUniform17 * vec3<f32>( nodeVarying6, 1.0 ) ).xy );
	SpecularColor = ( min( ( vec3<f32>( ( nodeVar5 * nodeVar5 ) ) * ( object.nodeUniform15 * nodeVar6.xyz ) ), vec3<f32>( 1.0, 1.0, 1.0 ) ) * vec3<f32>( object.nodeUniform18 ) );
	SpecularColorBlended = mix( SpecularColor, DiffuseColor.xyz, Metalness );
	SpecularF90 = mix( object.nodeUniform18, 1.0, Metalness );
	DiffuseContribution = ( DiffuseColor.xyz * vec3<f32>( ( 1.0 - ( object.nodeUniform7 * nodeVar2.z ) ) ) );
	EmissiveColor = ( object.nodeUniform19 * vec3<f32>( object.nodeUniform20 ) );
	nodeVar7 = ( ( f32( isFront ) * 2.0 ) - 1.0 );
	NORMAL_normalView = ( normalViewGeometry * vec3<f32>( nodeVar7 ) );
	nodeVar8 = cross( - dpdy( v_positionView ), NORMAL_normalView );
	nodeVar9 = dpdx( nodeVarying6 );
	nodeVar10 = cross( NORMAL_normalView, dpdx( v_positionView ) );
	nodeVar11 = - dpdy( nodeVarying6 );
	nodeVar12 = ( ( nodeVar8 * vec3<f32>( nodeVar9.x ) ) + ( nodeVar10 * vec3<f32>( nodeVar11.x ) ) );
	nodeVar14 = ( ( nodeVar8 * vec3<f32>( nodeVar9.y ) ) + ( nodeVar10 * vec3<f32>( nodeVar11.y ) ) );
	nodeVar15 = max( dot( nodeVar12, nodeVar12 ), dot( nodeVar14, nodeVar14 ) );

	if ( ( nodeVar15 == 0.0 ) ) {

		nodeVar13 = 0.0;

	} else {

		nodeVar13 = inverseSqrt( nodeVar15 );

	}

	tangentViewFrame = ( nodeVar12 * vec3<f32>( nodeVar13 ) );
	NORMAL_tangentView = ( tangentViewFrame * vec3<f32>( nodeVar7 ) );
	bitangentViewFrame = ( nodeVar14 * nodeVar13 );
	NORMAL_bitangentView = ( bitangentViewFrame * vec3<f32>( nodeVar7 ) );
	NORMAL_TBNViewMatrix = mat3x3<f32>( NORMAL_tangentView, NORMAL_bitangentView, NORMAL_normalView );
	nodeVar16 = textureSample( nodeUniform23, nodeUniform23_sampler, ( object.nodeUniform24 * vec3<f32>( nodeVarying6, 1.0 ) ).xy );
	nodeVar17 = ( ( nodeVar16 * vec4<f32>( 2.0 ) ) - vec4<f32>( 1.0 ) );
	normalView = normalize( ( NORMAL_TBNViewMatrix * vec3<f32>( ( nodeVar17.xy * object.nodeUniform25 ), nodeVar17.z ) ) );
	positionViewDirection = normalize( v_positionViewDirection );
	nodeVar18 = dot( normalView, positionViewDirection );
	nodeVar19 = textureSample( nodeUniform21, nodeUniform21_sampler, vec2<f32>( Roughness, clamp( nodeVar18, 0.0, 1.0 ) ) ).xy;
	let dfg = nodeVar19;
	nodeVar20 = ( dfg.x + dfg.y );
	nodeVar21 = ( 1.0 / nodeVar20 );
	nodeVar22 = nodeVar21;
	nodeVar23 = ( nodeVar22 - 1.0 );
	nodeVar24 = ( SpecularColorBlended * vec3<f32>( nodeVar23 ) );
	nodeVar25 = ( nodeVar24 + vec3<f32>( 1.0 ) );
	let multiScatteringCompensation = nodeVar25;
	nodeVar26 = ( render.nodeUniform26 - v_positionView );
	nodeVar27 = normalize( nodeVar26 );
	nodeVar28 = dot( normalView, nodeVar27 );

	if ( ( render.nodeUniform28 > 0.0 ) ) {

		nodeVar30 = length( nodeVar26 );
		nodeVar31 = ( nodeVar30 / render.nodeUniform28 );
		nodeVar32 = clamp( ( 1.0 - ( ( ( nodeVar31 * nodeVar31 ) * nodeVar31 ) * nodeVar31 ) ), 0.0, 1.0 );
		nodeVar29 = ( ( 1.0 / max( pow( nodeVar30, render.nodeUniform29 ), 0.01 ) ) * ( nodeVar32 * nodeVar32 ) );

	} else {

		nodeVar29 = ( 1.0 / max( pow( length( nodeVar26 ), render.nodeUniform29 ), 0.01 ) );

	}

	nodeVar33 = ( render.nodeUniform27 * vec3<f32>( nodeVar29 ) );
	nodeVar34 = ( vec3<f32>( clamp( nodeVar28, 0.0, 1.0 ) ) * nodeVar33 );
	nodeVar35 = nodeVar34;
	directDiffuse = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar36 = ( DiffuseContribution * vec3<f32>( 0.3183098861837907 ) );
	nodeVar37 = ( nodeVar35 * nodeVar36 );
	nodeVar38 = ( nodeVar27 + positionViewDirection );
	nodeVar39 = normalize( nodeVar38 );
	nodeVar40 = dot( positionViewDirection, nodeVar39 );
	nodeVar41 = clamp( nodeVar40, 0.0, 1.0 );
	nodeVar42 = exp2( ( ( ( nodeVar41 * -5.55473 ) - 6.98316 ) * nodeVar41 ) );
	nodeVar43 = ( ( SpecularColor * vec3<f32>( ( 1.0 - nodeVar42 ) ) ) + vec3<f32>( ( SpecularF90 * nodeVar42 ) ) );
	nodeVar44 = ( vec3<f32>( 1.0 ) - nodeVar43 );
	nodeVar45 = nodeVar44;
	nodeVar46 = ( nodeVar37 * nodeVar45 );
	nodeVar47 = ( directDiffuse + nodeVar46 );
	directDiffuse = nodeVar47;
	directSpecular = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar48 = normalize( ( nodeVar27 + positionViewDirection ) );
	nodeVar49 = clamp( dot( positionViewDirection, nodeVar48 ), 0.0, 1.0 );
	nodeVar50 = exp2( ( ( ( nodeVar49 * -5.55473 ) - 6.98316 ) * nodeVar49 ) );
	nodeVar51 = ( Roughness * Roughness );
	nodeVar52 = ( ( ( ( SpecularColorBlended * vec3<f32>( ( 1.0 - nodeVar50 ) ) ) + vec3<f32>( ( 1.0 * nodeVar50 ) ) ) * vec3<f32>( V_GGX_SmithCorrelated( nodeVar51, clamp( dot( normalView, nodeVar27 ), 0.0, 1.0 ), clamp( dot( normalView, positionViewDirection ), 0.0, 1.0 ) ) ) ) * vec3<f32>( D_GGX( nodeVar51, clamp( dot( normalView, nodeVar48 ), 0.0, 1.0 ) ) ) );
	nodeVar53 = ( nodeVar35 * nodeVar52 );
	nodeVar54 = ( nodeVar53 * multiScatteringCompensation );
	nodeVar55 = ( directSpecular + nodeVar54 );
	directSpecular = nodeVar55;
	irradiance = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar56 = ( irradiance + render.nodeUniform30 );
	irradiance = nodeVar56;
	nodeVar57 = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar58 = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar59 = ( SpecularColor * vec3<f32>( dfg.x ) );
	nodeVar60 = ( SpecularF90 * dfg.y );
	nodeVar61 = ( nodeVar59 + vec3<f32>( nodeVar60 ) );
	nodeVar62 = ( nodeVar57 + nodeVar61 );
	nodeVar57 = nodeVar62;
	nodeVar63 = ( vec3<f32>( 1.0 ) - SpecularColor );
	nodeVar64 = nodeVar63;
	nodeVar65 = ( nodeVar64 * vec3<f32>( 0.047619 ) );
	nodeVar66 = ( SpecularColor + nodeVar65 );
	nodeVar67 = ( nodeVar61 * nodeVar66 );
	nodeVar68 = ( dfg.x + dfg.y );
	nodeVar69 = ( 1.0 - nodeVar68 );
	nodeVar70 = nodeVar69;
	nodeVar71 = ( vec3<f32>( nodeVar70 ) * nodeVar66 );
	nodeVar72 = ( vec3<f32>( 1.0 ) - nodeVar71 );
	nodeVar73 = nodeVar72;
	nodeVar74 = ( nodeVar67 / nodeVar73 );
	nodeVar75 = ( nodeVar74 * vec3<f32>( nodeVar70 ) );
	nodeVar76 = ( nodeVar58 + nodeVar75 );
	nodeVar58 = nodeVar76;
	nodeVar77 = ( DiffuseContribution * vec3<f32>( 0.3183098861837907 ) );
	nodeVar78 = ( irradiance * nodeVar77 );
	nodeVar79 = ( nodeVar57 + nodeVar58 );
	nodeVar80 = ( vec3<f32>( 1.0 ) - nodeVar79 );
	nodeVar81 = nodeVar80;
	nodeVar82 = ( nodeVar78 * nodeVar81 );
	nodeVar83 = nodeVar82;
	indirectDiffuse = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar84 = ( indirectDiffuse + nodeVar83 );
	indirectDiffuse = nodeVar84;
	singleScatteringDielectric = vec3<f32>( 0.0, 0.0, 0.0 );
	multiScatteringDielectric = vec3<f32>( 0.0, 0.0, 0.0 );
	singleScatteringMetallic = vec3<f32>( 0.0, 0.0, 0.0 );
	multiScatteringMetallic = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar85 = ( SpecularColor * vec3<f32>( dfg.x ) );
	nodeVar86 = ( SpecularF90 * dfg.y );
	nodeVar87 = ( nodeVar85 + vec3<f32>( nodeVar86 ) );
	nodeVar88 = ( singleScatteringDielectric + nodeVar87 );
	singleScatteringDielectric = nodeVar88;
	nodeVar89 = ( vec3<f32>( 1.0 ) - SpecularColor );
	nodeVar90 = nodeVar89;
	nodeVar91 = ( nodeVar90 * vec3<f32>( 0.047619 ) );
	nodeVar92 = ( SpecularColor + nodeVar91 );
	nodeVar93 = ( nodeVar87 * nodeVar92 );
	nodeVar94 = ( dfg.x + dfg.y );
	nodeVar95 = ( 1.0 - nodeVar94 );
	nodeVar96 = nodeVar95;
	nodeVar97 = ( vec3<f32>( nodeVar96 ) * nodeVar92 );
	nodeVar98 = ( vec3<f32>( 1.0 ) - nodeVar97 );
	nodeVar99 = nodeVar98;
	nodeVar100 = ( nodeVar93 / nodeVar99 );
	nodeVar101 = ( nodeVar100 * vec3<f32>( nodeVar96 ) );
	nodeVar102 = ( multiScatteringDielectric + nodeVar101 );
	multiScatteringDielectric = nodeVar102;
	nodeVar103 = ( DiffuseColor.xyz * vec3<f32>( dfg.x ) );
	nodeVar104 = ( SpecularF90 * dfg.y );
	nodeVar105 = ( nodeVar103 + vec3<f32>( nodeVar104 ) );
	nodeVar106 = ( singleScatteringMetallic + nodeVar105 );
	singleScatteringMetallic = nodeVar106;
	nodeVar107 = ( vec3<f32>( 1.0 ) - DiffuseColor.xyz );
	nodeVar108 = nodeVar107;
	nodeVar109 = ( nodeVar108 * vec3<f32>( 0.047619 ) );
	nodeVar110 = ( DiffuseColor.xyz + nodeVar109 );
	nodeVar111 = ( nodeVar105 * nodeVar110 );
	nodeVar112 = ( dfg.x + dfg.y );
	nodeVar113 = ( 1.0 - nodeVar112 );
	nodeVar114 = nodeVar113;
	nodeVar115 = ( vec3<f32>( nodeVar114 ) * nodeVar110 );
	nodeVar116 = ( vec3<f32>( 1.0 ) - nodeVar115 );
	nodeVar117 = nodeVar116;
	nodeVar118 = ( nodeVar111 / nodeVar117 );
	nodeVar119 = ( nodeVar118 * vec3<f32>( nodeVar114 ) );
	nodeVar120 = ( multiScatteringMetallic + nodeVar119 );
	multiScatteringMetallic = nodeVar120;
	radiance = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar121 = mix( singleScatteringDielectric, singleScatteringMetallic, Metalness );
	nodeVar122 = ( radiance * nodeVar121 );
	nodeVar123 = mix( multiScatteringDielectric, multiScatteringMetallic, Metalness );
	iblIrradiance = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar124 = ( iblIrradiance * vec3<f32>( 0.3183098861837907 ) );
	nodeVar125 = ( nodeVar123 * nodeVar124 );
	nodeVar126 = ( nodeVar122 + nodeVar125 );
	nodeVar127 = nodeVar126;
	nodeVar128 = ( singleScatteringDielectric + multiScatteringDielectric );
	nodeVar129 = ( vec3<f32>( 1.0 ) - nodeVar128 );
	nodeVar130 = nodeVar129;
	nodeVar131 = ( DiffuseContribution * nodeVar130 );
	nodeVar132 = ( nodeVar131 * nodeVar124 );
	nodeVar133 = nodeVar132;
	indirectSpecular = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar134 = ( indirectSpecular + nodeVar127 );
	indirectSpecular = nodeVar134;
	nodeVar135 = ( indirectDiffuse + nodeVar133 );
	indirectDiffuse = nodeVar135;
	ambientOcclusion = 1.0;
	nodeVar136 = ( indirectDiffuse * vec3<f32>( ambientOcclusion ) );
	indirectDiffuse = nodeVar136;
	nodeVar137 = dot( normalView, positionViewDirection );
	nodeVar138 = ( clamp( nodeVar137, 0.0, 1.0 ) + ambientOcclusion );
	nodeVar139 = ( Roughness * -16.0 );
	nodeVar140 = ( 1.0 - nodeVar139 );
	nodeVar141 = nodeVar140;
	nodeVar142 = ( - nodeVar141 );
	nodeVar143 = exp2( nodeVar142 );
	nodeVar144 = pow( nodeVar138, nodeVar143 );
	nodeVar145 = ( 1.0 - nodeVar144 );
	nodeVar146 = nodeVar145;
	nodeVar147 = ( ambientOcclusion - nodeVar146 );
	nodeVar148 = ( indirectSpecular * vec3<f32>( clamp( nodeVar147, 0.0, 1.0 ) ) );
	indirectSpecular = nodeVar148;
	nodeVar149 = ( directDiffuse + indirectDiffuse );
	totalDiffuse = nodeVar149;
	nodeVar150 = ( directSpecular + indirectSpecular );
	totalSpecular = nodeVar150;
	nodeVar151 = ( totalDiffuse + totalSpecular );
	outgoingLight = nodeVar151;
	nodeVar152 = max( vec4<f32>( ( outgoingLight + EmissiveColor ), DiffuseColor.w ), vec4<f32>( 0.0 ) );
	Output = nodeVar152;

	// result

	output.color = nodeVar152;

	return output;

}
