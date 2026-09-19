// Three.js r186 - Node System

// global
diagnostic( off, derivative_uniformity );


// structs

struct OutputType {
	@location( 0 ) m0 : vec4<f32>,
	@location( 1 ) m1 : vec4<f32>,
	
};
var<private> output : OutputType;

// uniforms
@binding( 1 ) @group( 1 ) var nodeUniform12_sampler : sampler;
@binding( 2 ) @group( 1 ) var nodeUniform12 : texture_2d<f32>;
@binding( 3 ) @group( 1 ) var nodeUniform21_sampler : sampler_comparison;
@binding( 4 ) @group( 1 ) var nodeUniform21 : texture_depth_2d;

struct objectStruct {
	nodeUniform0 : mat4x4<f32>,
	nodeUniform2 : mat4x4<f32>,
	nodeUniform4 : vec3<f32>,
	nodeUniform5 : f32,
	nodeUniform6 : f32,
	nodeUniform7 : f32,
	nodeUniform9 : mat3x3<f32>,
	nodeUniform10 : vec3<f32>,
	nodeUniform11 : f32,
	nodeUniform13 : mat4x4<f32>,
	nodeUniform40 : mat4x4<f32>,
	nodeUniform42 : mat4x4<f32>,
	nodeUniform43 : mat4x4<f32>
};
@binding( 0 ) @group( 1 )
var<uniform> object : objectStruct;

struct renderStruct {
	nodeUniform41 : mat4x4<f32>,
	cameraProjectionMatrix : mat4x4<f32>,
	cameraViewMatrix : mat4x4<f32>,
	nodeUniform16 : vec3<f32>,
	nodeUniform31 : vec3<f32>,
	nodeUniform32 : vec3<f32>,
	nodeUniform30 : vec3<f32>,
	nodeUniform34 : vec3<f32>,
	nodeUniform35 : vec3<f32>,
	nodeUniform33 : vec3<f32>,
	nodeUniform15 : vec3<f32>,
	nodeUniform36 : vec3<f32>,
	nodeUniform37 : f32,
	nodeUniform38 : f32,
	nodeUniform19 : mat4x4<f32>,
	nodeUniform18 : vec4<f32>,
	nodeUniform25 : mat4x4<f32>,
	nodeUniform24 : vec4<f32>,
	nodeUniform17 : f32,
	nodeUniform20 : f32,
	nodeUniform22 : f32,
	nodeUniform23 : vec2<f32>,
	nodeUniform26 : f32,
	nodeUniform27 : f32,
	nodeUniform28 : vec2<f32>,
	nodeUniform29 : f32
};
@binding( 0 ) @group( 0 )
var<uniform> render : renderStruct;

// vars
var<private> DiffuseColor : vec4<f32>;
var<private> Metalness : f32;
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
var<private> nodeVar11 : vec4<f32>;
var<private> nodeVar12 : vec4<f32>;
var<private> nodeVar13 : vec3<f32>;
var<private> nodeVar14 : vec3<f32>;
var<private> nodeVar15 : f32;
var<private> shadowPositionWorld : vec3<f32>;
var<private> normalWorld : vec3<f32>;
var<private> nodeVar16 : vec4<f32>;
var<private> nodeVar17 : f32;
var<private> shadowValue : f32;
var<private> nodeVar18 : f32;
var<private> nodeVar19 : vec4<f32>;
var<private> nodeVar20 : vec3<f32>;
var<private> nodeVar21 : vec3<f32>;
var<private> nodeVar22 : f32;
var<private> nodeVar23 : f32;
var<private> nodeVar24 : vec2<f32>;
var<private> nodeVar25 : f32;
var<private> nodeVar26 : vec2<f32>;
var<private> nodeVar27 : f32;
var<private> nodeVar28 : vec2<f32>;
var<private> nodeVar29 : f32;
var<private> nodeVar30 : vec2<f32>;
var<private> nodeVar31 : f32;
var<private> nodeVar32 : vec2<f32>;
var<private> nodeVar33 : f32;
var<private> nodeVar34 : f32;
var<private> nodeVar35 : vec4<f32>;
var<private> nodeVar36 : vec3<f32>;
var<private> nodeVar37 : vec3<f32>;
var<private> nodeVar38 : f32;
var<private> nodeVar39 : f32;
var<private> nodeVar40 : vec2<f32>;
var<private> nodeVar41 : f32;
var<private> nodeVar42 : vec2<f32>;
var<private> nodeVar43 : f32;
var<private> nodeVar44 : vec2<f32>;
var<private> nodeVar45 : f32;
var<private> nodeVar46 : vec2<f32>;
var<private> nodeVar47 : f32;
var<private> nodeVar48 : vec2<f32>;
var<private> nodeVar49 : f32;
var<private> nodeVar50 : f32;
var<private> nodeVar51 : vec3<f32>;
var<private> nodeVar52 : vec3<f32>;
var<private> directDiffuse : vec3<f32>;
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
var<private> directSpecular : vec3<f32>;
var<private> nodeVar65 : vec3<f32>;
var<private> nodeVar66 : f32;
var<private> nodeVar67 : f32;
var<private> nodeVar68 : f32;
var<private> nodeVar69 : vec3<f32>;
var<private> nodeVar70 : vec3<f32>;
var<private> nodeVar71 : vec3<f32>;
var<private> nodeVar72 : vec3<f32>;
var<private> irradiance : vec3<f32>;
var<private> nodeVar73 : f32;
var<private> nodeVar74 : f32;
var<private> nodeVar75 : f32;
var<private> nodeVar76 : vec3<f32>;
var<private> nodeVar77 : vec3<f32>;
var<private> nodeVar78 : f32;
var<private> nodeVar79 : f32;
var<private> nodeVar80 : f32;
var<private> nodeVar81 : vec3<f32>;
var<private> nodeVar82 : vec3<f32>;
var<private> nodeVar83 : vec3<f32>;
var<private> nodeVar84 : vec3<f32>;
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
var<private> nodeVar104 : vec3<f32>;
var<private> nodeVar105 : vec3<f32>;
var<private> nodeVar106 : vec3<f32>;
var<private> nodeVar107 : vec3<f32>;
var<private> nodeVar108 : vec3<f32>;
var<private> nodeVar109 : vec3<f32>;
var<private> indirectDiffuse : vec3<f32>;
var<private> nodeVar110 : vec3<f32>;
var<private> singleScatteringDielectric : vec3<f32>;
var<private> multiScatteringDielectric : vec3<f32>;
var<private> singleScatteringMetallic : vec3<f32>;
var<private> multiScatteringMetallic : vec3<f32>;
var<private> nodeVar111 : vec3<f32>;
var<private> nodeVar112 : f32;
var<private> nodeVar113 : vec3<f32>;
var<private> nodeVar114 : vec3<f32>;
var<private> nodeVar115 : vec3<f32>;
var<private> nodeVar116 : vec3<f32>;
var<private> nodeVar117 : vec3<f32>;
var<private> nodeVar118 : vec3<f32>;
var<private> nodeVar119 : vec3<f32>;
var<private> nodeVar120 : f32;
var<private> nodeVar121 : f32;
var<private> nodeVar122 : f32;
var<private> nodeVar123 : vec3<f32>;
var<private> nodeVar124 : vec3<f32>;
var<private> nodeVar125 : vec3<f32>;
var<private> nodeVar126 : vec3<f32>;
var<private> nodeVar127 : vec3<f32>;
var<private> nodeVar128 : vec3<f32>;
var<private> nodeVar129 : vec3<f32>;
var<private> nodeVar130 : f32;
var<private> nodeVar131 : vec3<f32>;
var<private> nodeVar132 : vec3<f32>;
var<private> nodeVar133 : vec3<f32>;
var<private> nodeVar134 : vec3<f32>;
var<private> nodeVar135 : vec3<f32>;
var<private> nodeVar136 : vec3<f32>;
var<private> nodeVar137 : vec3<f32>;
var<private> nodeVar138 : f32;
var<private> nodeVar139 : f32;
var<private> nodeVar140 : f32;
var<private> nodeVar141 : vec3<f32>;
var<private> nodeVar142 : vec3<f32>;
var<private> nodeVar143 : vec3<f32>;
var<private> nodeVar144 : vec3<f32>;
var<private> nodeVar145 : vec3<f32>;
var<private> nodeVar146 : vec3<f32>;
var<private> radiance : vec3<f32>;
var<private> nodeVar147 : vec3<f32>;
var<private> nodeVar148 : vec3<f32>;
var<private> nodeVar149 : vec3<f32>;
var<private> iblIrradiance : vec3<f32>;
var<private> nodeVar150 : vec3<f32>;
var<private> nodeVar151 : vec3<f32>;
var<private> nodeVar152 : vec3<f32>;
var<private> nodeVar153 : vec3<f32>;
var<private> nodeVar154 : vec3<f32>;
var<private> nodeVar155 : vec3<f32>;
var<private> nodeVar156 : vec3<f32>;
var<private> nodeVar157 : vec3<f32>;
var<private> nodeVar158 : vec3<f32>;
var<private> nodeVar159 : vec3<f32>;
var<private> indirectSpecular : vec3<f32>;
var<private> nodeVar160 : vec3<f32>;
var<private> nodeVar161 : vec3<f32>;
var<private> ambientOcclusion : f32;
var<private> nodeVar162 : vec3<f32>;
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
var<private> nodeVar173 : f32;
var<private> nodeVar174 : vec3<f32>;
var<private> totalDiffuse : vec3<f32>;
var<private> nodeVar175 : vec3<f32>;
var<private> totalSpecular : vec3<f32>;
var<private> nodeVar176 : vec3<f32>;
var<private> outgoingLight : vec3<f32>;
var<private> nodeVar177 : vec3<f32>;
var<private> nodeVar178 : vec4<f32>;
var<private> modelViewMatrix : mat4x4<f32>;
var<private> nodeVar179 : vec4<f32>;
var<private> nodeVar180 : vec4<f32>;
var<private> nodeVar181 : vec2<f32>;

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
fn main( @location( 0 ) positionPrevious : vec3<f32>,
	@location( 1 ) positionLocal : vec3<f32>,
	@location( 2 ) v_positionView : vec3<f32>,
	@location( 3 ) v_normalViewGeometry : vec3<f32>,
	@location( 4 ) v_positionViewDirection : vec3<f32>,
	@location( 5 ) v_positionWorld : vec3<f32>,
	@builtin( front_facing ) isFront : bool,
	@builtin( position ) fragCoord : vec4<f32> ) -> OutputType {

	// flow
	// code

	DiffuseColor = vec4<f32>( object.nodeUniform4, 1.0 );
	DiffuseColor.w = ( DiffuseColor.w * object.nodeUniform5 );
	DiffuseColor.w = 1.0;
	Metalness = object.nodeUniform6;
	normalViewGeometry = normalize( v_normalViewGeometry );
	nodeVar2 = max( abs( dpdx( normalViewGeometry ) ), abs( - dpdy( normalViewGeometry ) ) );
	Roughness = min( ( max( object.nodeUniform7, 0.0525 ) + max( max( nodeVar2.x, nodeVar2.y ), nodeVar2.z ) ), 1.0 );
	SpecularColor = vec3<f32>( 0.04, 0.04, 0.04 );
	SpecularColorBlended = mix( vec3<f32>( 0.04, 0.04, 0.04 ), DiffuseColor.xyz, Metalness );
	SpecularF90 = 1.0;
	DiffuseContribution = ( DiffuseColor.xyz * vec3<f32>( ( 1.0 - object.nodeUniform6 ) ) );
	EmissiveColor = ( object.nodeUniform10 * vec3<f32>( object.nodeUniform11 ) );
	NORMAL_normalView = ( normalViewGeometry * vec3<f32>( ( ( f32( isFront ) * 2.0 ) - 1.0 ) ) );
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
	nodeVar11 = vec4<f32>( render.nodeUniform15, 0.0 );
	nodeVar12 = ( render.cameraViewMatrix * nodeVar11 );
	nodeVar13 = normalize( nodeVar12.xyz );
	nodeVar14 = nodeVar13;
	nodeVar15 = dot( normalView, nodeVar14 );
	shadowPositionWorld = v_positionWorld;
	normalWorld = normalize( ( vec4<f32>( normalView, 0.0 ) * render.cameraViewMatrix ).xyz );
	nodeVar16 = vec4<f32>( ( shadowPositionWorld + ( normalWorld * vec3<f32>( render.nodeUniform17 ) ) ), 1.0 );
	nodeVar17 = ( - v_positionView.z );
	shadowValue = 1.0;

	if ( ( ( nodeVar17 >= render.nodeUniform18.x ) && ( nodeVar17 < render.nodeUniform18.y ) ) ) {

		nodeVar19 = ( render.nodeUniform19 * nodeVar16 );
		nodeVar20 = ( nodeVar19.xyz / vec3<f32>( nodeVar19.w ) );
		nodeVar21 = vec3<f32>( nodeVar20.x, ( 1.0 - nodeVar20.y ), ( nodeVar20.z + render.nodeUniform20 ) );

		if ( ( ( ( ( ( nodeVar21.x >= 0.0 ) && ( nodeVar21.x <= 1.0 ) ) && ( nodeVar21.y >= 0.0 ) ) && ( nodeVar21.y <= 1.0 ) ) && ( nodeVar21.z <= 1.0 ) ) ) {

			nodeVar22 = ( interleavedGradientNoise( fragCoord.xy ) * 6.28318530718 );
			nodeVar23 = ( render.nodeUniform22 * ( vec2<f32>( 1.0, 1.0 ) / render.nodeUniform23 ).x );
			nodeVar24 = ( nodeVar21.xy + ( vogelDiskSample( 0, 5, nodeVar22 ) * vec2<f32>( nodeVar23 ) ) );
			nodeVar25 = textureSampleCompare( nodeUniform21, nodeUniform21_sampler, nodeVar24, nodeVar21.z );
			nodeVar26 = ( nodeVar21.xy + ( vogelDiskSample( 1, 5, nodeVar22 ) * vec2<f32>( nodeVar23 ) ) );
			nodeVar27 = textureSampleCompare( nodeUniform21, nodeUniform21_sampler, nodeVar26, nodeVar21.z );
			nodeVar28 = ( nodeVar21.xy + ( vogelDiskSample( 2, 5, nodeVar22 ) * vec2<f32>( nodeVar23 ) ) );
			nodeVar29 = textureSampleCompare( nodeUniform21, nodeUniform21_sampler, nodeVar28, nodeVar21.z );
			nodeVar30 = ( nodeVar21.xy + ( vogelDiskSample( 3, 5, nodeVar22 ) * vec2<f32>( nodeVar23 ) ) );
			nodeVar31 = textureSampleCompare( nodeUniform21, nodeUniform21_sampler, nodeVar30, nodeVar21.z );
			nodeVar32 = ( nodeVar21.xy + ( vogelDiskSample( 4, 5, nodeVar22 ) * vec2<f32>( nodeVar23 ) ) );
			nodeVar33 = textureSampleCompare( nodeUniform21, nodeUniform21_sampler, nodeVar32, nodeVar21.z );
			nodeVar18 = ( ( ( ( ( nodeVar25 + nodeVar27 ) + nodeVar29 ) + nodeVar31 ) + nodeVar33 ) * 0.2 );

		} else {

			nodeVar18 = 1.0;

		}

		shadowValue = mix( nodeVar18, shadowValue, smoothstep( render.nodeUniform18.z, render.nodeUniform18.y, nodeVar17 ) );
		

	}


	if ( ( ( nodeVar17 >= render.nodeUniform24.x ) && ( nodeVar17 < render.nodeUniform24.y ) ) ) {

		nodeVar35 = ( render.nodeUniform25 * nodeVar16 );
		nodeVar36 = ( nodeVar35.xyz / vec3<f32>( nodeVar35.w ) );
		nodeVar37 = vec3<f32>( nodeVar36.x, ( 1.0 - nodeVar36.y ), ( nodeVar36.z + render.nodeUniform26 ) );

		if ( ( ( ( ( ( nodeVar37.x >= 0.0 ) && ( nodeVar37.x <= 1.0 ) ) && ( nodeVar37.y >= 0.0 ) ) && ( nodeVar37.y <= 1.0 ) ) && ( nodeVar37.z <= 1.0 ) ) ) {

			nodeVar38 = ( interleavedGradientNoise( fragCoord.xy ) * 6.28318530718 );
			nodeVar39 = ( render.nodeUniform27 * ( vec2<f32>( 1.0, 1.0 ) / render.nodeUniform28 ).x );
			nodeVar40 = ( nodeVar37.xy + ( vogelDiskSample( 0, 5, nodeVar38 ) * vec2<f32>( nodeVar39 ) ) );
			nodeVar41 = textureSampleCompare( nodeUniform21, nodeUniform21_sampler, nodeVar40, nodeVar37.z );
			nodeVar42 = ( nodeVar37.xy + ( vogelDiskSample( 1, 5, nodeVar38 ) * vec2<f32>( nodeVar39 ) ) );
			nodeVar43 = textureSampleCompare( nodeUniform21, nodeUniform21_sampler, nodeVar42, nodeVar37.z );
			nodeVar44 = ( nodeVar37.xy + ( vogelDiskSample( 2, 5, nodeVar38 ) * vec2<f32>( nodeVar39 ) ) );
			nodeVar45 = textureSampleCompare( nodeUniform21, nodeUniform21_sampler, nodeVar44, nodeVar37.z );
			nodeVar46 = ( nodeVar37.xy + ( vogelDiskSample( 3, 5, nodeVar38 ) * vec2<f32>( nodeVar39 ) ) );
			nodeVar47 = textureSampleCompare( nodeUniform21, nodeUniform21_sampler, nodeVar46, nodeVar37.z );
			nodeVar48 = ( nodeVar37.xy + ( vogelDiskSample( 4, 5, nodeVar38 ) * vec2<f32>( nodeVar39 ) ) );
			nodeVar49 = textureSampleCompare( nodeUniform21, nodeUniform21_sampler, nodeVar48, nodeVar37.z );
			nodeVar34 = ( ( ( ( ( nodeVar41 + nodeVar43 ) + nodeVar45 ) + nodeVar47 ) + nodeVar49 ) * 0.2 );

		} else {

			nodeVar34 = 1.0;

		}

		shadowValue = mix( nodeVar34, shadowValue, smoothstep( render.nodeUniform24.z, render.nodeUniform24.y, nodeVar17 ) );
		

	}

	nodeVar50 = mix( 1.0, shadowValue, render.nodeUniform29 );
	nodeVar51 = ( vec3<f32>( clamp( nodeVar15, 0.0, 1.0 ) ) * ( render.nodeUniform16 * vec3<f32>( nodeVar50 ) ) );
	nodeVar52 = nodeVar51;
	directDiffuse = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar53 = ( DiffuseContribution * vec3<f32>( 0.3183098861837907 ) );
	nodeVar54 = ( nodeVar52 * nodeVar53 );
	nodeVar55 = ( nodeVar14 + positionViewDirection );
	nodeVar56 = normalize( nodeVar55 );
	nodeVar57 = dot( positionViewDirection, nodeVar56 );
	nodeVar58 = clamp( nodeVar57, 0.0, 1.0 );
	nodeVar59 = exp2( ( ( ( nodeVar58 * -5.55473 ) - 6.98316 ) * nodeVar58 ) );
	nodeVar60 = ( ( SpecularColor * vec3<f32>( ( 1.0 - nodeVar59 ) ) ) + vec3<f32>( ( SpecularF90 * nodeVar59 ) ) );
	nodeVar61 = ( vec3<f32>( 1.0 ) - nodeVar60 );
	nodeVar62 = nodeVar61;
	nodeVar63 = ( nodeVar54 * nodeVar62 );
	nodeVar64 = ( directDiffuse + nodeVar63 );
	directDiffuse = nodeVar64;
	directSpecular = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar65 = normalize( ( nodeVar14 + positionViewDirection ) );
	nodeVar66 = clamp( dot( positionViewDirection, nodeVar65 ), 0.0, 1.0 );
	nodeVar67 = exp2( ( ( ( nodeVar66 * -5.55473 ) - 6.98316 ) * nodeVar66 ) );
	nodeVar68 = ( Roughness * Roughness );
	nodeVar69 = ( ( ( ( SpecularColorBlended * vec3<f32>( ( 1.0 - nodeVar67 ) ) ) + vec3<f32>( ( 1.0 * nodeVar67 ) ) ) * vec3<f32>( V_GGX_SmithCorrelated( nodeVar68, clamp( dot( normalView, nodeVar14 ), 0.0, 1.0 ), clamp( dot( normalView, positionViewDirection ), 0.0, 1.0 ) ) ) ) * vec3<f32>( D_GGX( nodeVar68, clamp( dot( normalView, nodeVar65 ), 0.0, 1.0 ) ) ) );
	nodeVar70 = ( nodeVar52 * nodeVar69 );
	nodeVar71 = ( nodeVar70 * multiScatteringCompensation );
	nodeVar72 = ( directSpecular + nodeVar71 );
	directSpecular = nodeVar72;
	irradiance = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar73 = dot( normalWorld, normalize( render.nodeUniform32 ) );
	nodeVar74 = ( nodeVar73 * 0.5 );
	nodeVar75 = ( nodeVar74 + 0.5 );
	nodeVar76 = mix( render.nodeUniform30, render.nodeUniform31, nodeVar75 );
	nodeVar77 = ( irradiance + nodeVar76 );
	irradiance = nodeVar77;
	nodeVar78 = dot( normalWorld, normalize( render.nodeUniform35 ) );
	nodeVar79 = ( nodeVar78 * 0.5 );
	nodeVar80 = ( nodeVar79 + 0.5 );
	nodeVar81 = mix( render.nodeUniform33, render.nodeUniform34, nodeVar80 );
	nodeVar82 = ( irradiance + nodeVar81 );
	irradiance = nodeVar82;
	nodeVar83 = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar84 = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar85 = ( SpecularColor * vec3<f32>( dfg.x ) );
	nodeVar86 = ( SpecularF90 * dfg.y );
	nodeVar87 = ( nodeVar85 + vec3<f32>( nodeVar86 ) );
	nodeVar88 = ( nodeVar83 + nodeVar87 );
	nodeVar83 = nodeVar88;
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
	nodeVar102 = ( nodeVar84 + nodeVar101 );
	nodeVar84 = nodeVar102;
	nodeVar103 = ( DiffuseContribution * vec3<f32>( 0.3183098861837907 ) );
	nodeVar104 = ( irradiance * nodeVar103 );
	nodeVar105 = ( nodeVar83 + nodeVar84 );
	nodeVar106 = ( vec3<f32>( 1.0 ) - nodeVar105 );
	nodeVar107 = nodeVar106;
	nodeVar108 = ( nodeVar104 * nodeVar107 );
	nodeVar109 = nodeVar108;
	indirectDiffuse = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar110 = ( indirectDiffuse + nodeVar109 );
	indirectDiffuse = nodeVar110;
	singleScatteringDielectric = vec3<f32>( 0.0, 0.0, 0.0 );
	multiScatteringDielectric = vec3<f32>( 0.0, 0.0, 0.0 );
	singleScatteringMetallic = vec3<f32>( 0.0, 0.0, 0.0 );
	multiScatteringMetallic = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar111 = ( SpecularColor * vec3<f32>( dfg.x ) );
	nodeVar112 = ( SpecularF90 * dfg.y );
	nodeVar113 = ( nodeVar111 + vec3<f32>( nodeVar112 ) );
	nodeVar114 = ( singleScatteringDielectric + nodeVar113 );
	singleScatteringDielectric = nodeVar114;
	nodeVar115 = ( vec3<f32>( 1.0 ) - SpecularColor );
	nodeVar116 = nodeVar115;
	nodeVar117 = ( nodeVar116 * vec3<f32>( 0.047619 ) );
	nodeVar118 = ( SpecularColor + nodeVar117 );
	nodeVar119 = ( nodeVar113 * nodeVar118 );
	nodeVar120 = ( dfg.x + dfg.y );
	nodeVar121 = ( 1.0 - nodeVar120 );
	nodeVar122 = nodeVar121;
	nodeVar123 = ( vec3<f32>( nodeVar122 ) * nodeVar118 );
	nodeVar124 = ( vec3<f32>( 1.0 ) - nodeVar123 );
	nodeVar125 = nodeVar124;
	nodeVar126 = ( nodeVar119 / nodeVar125 );
	nodeVar127 = ( nodeVar126 * vec3<f32>( nodeVar122 ) );
	nodeVar128 = ( multiScatteringDielectric + nodeVar127 );
	multiScatteringDielectric = nodeVar128;
	nodeVar129 = ( DiffuseColor.xyz * vec3<f32>( dfg.x ) );
	nodeVar130 = ( SpecularF90 * dfg.y );
	nodeVar131 = ( nodeVar129 + vec3<f32>( nodeVar130 ) );
	nodeVar132 = ( singleScatteringMetallic + nodeVar131 );
	singleScatteringMetallic = nodeVar132;
	nodeVar133 = ( vec3<f32>( 1.0 ) - DiffuseColor.xyz );
	nodeVar134 = nodeVar133;
	nodeVar135 = ( nodeVar134 * vec3<f32>( 0.047619 ) );
	nodeVar136 = ( DiffuseColor.xyz + nodeVar135 );
	nodeVar137 = ( nodeVar131 * nodeVar136 );
	nodeVar138 = ( dfg.x + dfg.y );
	nodeVar139 = ( 1.0 - nodeVar138 );
	nodeVar140 = nodeVar139;
	nodeVar141 = ( vec3<f32>( nodeVar140 ) * nodeVar136 );
	nodeVar142 = ( vec3<f32>( 1.0 ) - nodeVar141 );
	nodeVar143 = nodeVar142;
	nodeVar144 = ( nodeVar137 / nodeVar143 );
	nodeVar145 = ( nodeVar144 * vec3<f32>( nodeVar140 ) );
	nodeVar146 = ( multiScatteringMetallic + nodeVar145 );
	multiScatteringMetallic = nodeVar146;
	radiance = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar147 = mix( singleScatteringDielectric, singleScatteringMetallic, Metalness );
	nodeVar148 = ( radiance * nodeVar147 );
	nodeVar149 = mix( multiScatteringDielectric, multiScatteringMetallic, Metalness );
	iblIrradiance = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar150 = ( iblIrradiance * vec3<f32>( 0.3183098861837907 ) );
	nodeVar151 = ( nodeVar149 * nodeVar150 );
	nodeVar152 = ( nodeVar148 + nodeVar151 );
	nodeVar153 = nodeVar152;
	nodeVar154 = ( singleScatteringDielectric + multiScatteringDielectric );
	nodeVar155 = ( vec3<f32>( 1.0 ) - nodeVar154 );
	nodeVar156 = nodeVar155;
	nodeVar157 = ( DiffuseContribution * nodeVar156 );
	nodeVar158 = ( nodeVar157 * nodeVar150 );
	nodeVar159 = nodeVar158;
	indirectSpecular = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar160 = ( indirectSpecular + nodeVar153 );
	indirectSpecular = nodeVar160;
	nodeVar161 = ( indirectDiffuse + nodeVar159 );
	indirectDiffuse = nodeVar161;
	ambientOcclusion = 1.0;
	nodeVar162 = ( indirectDiffuse * vec3<f32>( ambientOcclusion ) );
	indirectDiffuse = nodeVar162;
	nodeVar163 = dot( normalView, positionViewDirection );
	nodeVar164 = ( clamp( nodeVar163, 0.0, 1.0 ) + ambientOcclusion );
	nodeVar165 = ( Roughness * -16.0 );
	nodeVar166 = ( 1.0 - nodeVar165 );
	nodeVar167 = nodeVar166;
	nodeVar168 = ( - nodeVar167 );
	nodeVar169 = exp2( nodeVar168 );
	nodeVar170 = pow( nodeVar164, nodeVar169 );
	nodeVar171 = ( 1.0 - nodeVar170 );
	nodeVar172 = nodeVar171;
	nodeVar173 = ( ambientOcclusion - nodeVar172 );
	nodeVar174 = ( indirectSpecular * vec3<f32>( clamp( nodeVar173, 0.0, 1.0 ) ) );
	indirectSpecular = nodeVar174;
	nodeVar175 = ( directDiffuse + indirectDiffuse );
	totalDiffuse = nodeVar175;
	nodeVar176 = ( directSpecular + indirectSpecular );
	totalSpecular = nodeVar176;
	nodeVar177 = ( totalDiffuse + totalSpecular );
	outgoingLight = nodeVar177;
	Output = max( vec4<f32>( ( outgoingLight + EmissiveColor ), DiffuseColor.w ), vec4<f32>( 0.0 ) );
	nodeVar178 = vec4<f32>( mix( Output.xyz, render.nodeUniform36, smoothstep( render.nodeUniform37, render.nodeUniform38, ( - v_positionView.z ) ) ), Output.w );
	Output = nodeVar178;
	output.m0 = Output;
	modelViewMatrix = ( render.cameraViewMatrix * object.nodeUniform40 );
	nodeVar179 = ( ( render.cameraProjectionMatrix * modelViewMatrix ) * vec4<f32>( positionLocal, 1.0 ) );
	nodeVar180 = ( ( render.nodeUniform41 * ( object.nodeUniform42 * object.nodeUniform43 ) ) * vec4<f32>( positionPrevious, 1.0 ) );
	nodeVar181 = ( ( nodeVar179.xy / vec2<f32>( nodeVar179.w ) ) - ( nodeVar180.xy / vec2<f32>( nodeVar180.w ) ) );
	output.m1 = vec4<f32>( vec3<f32>( nodeVar181, 0.0 ), 1.0 );

	// result

	return output;

}
