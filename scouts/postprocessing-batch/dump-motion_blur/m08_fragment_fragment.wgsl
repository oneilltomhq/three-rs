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
@binding( 0 ) @group( 1 ) var nodeUniform0_sampler : sampler;
@binding( 1 ) @group( 1 ) var nodeUniform0 : texture_2d<f32>;
@binding( 3 ) @group( 1 ) var nodeUniform16_sampler : sampler_comparison;
@binding( 4 ) @group( 1 ) var nodeUniform16 : texture_depth_2d;

struct objectStruct {
	nodeUniform1 : f32,
	nodeUniform2 : f32,
	nodeUniform3 : vec3<f32>,
	nodeUniform4 : vec3<f32>,
	nodeUniform5 : f32,
	nodeUniform7 : mat3x3<f32>,
	nodeUniform11 : mat4x4<f32>,
	nodeUniform35 : mat4x4<f32>,
	nodeUniform37 : mat4x4<f32>,
	nodeUniform38 : mat4x4<f32>
};
@binding( 2 ) @group( 1 )
var<uniform> object : objectStruct;

struct renderStruct {
	nodeUniform36 : mat4x4<f32>,
	cameraProjectionMatrix : mat4x4<f32>,
	cameraViewMatrix : mat4x4<f32>,
	nodeUniform10 : vec3<f32>,
	nodeUniform26 : vec3<f32>,
	nodeUniform27 : vec3<f32>,
	nodeUniform25 : vec3<f32>,
	nodeUniform29 : vec3<f32>,
	nodeUniform30 : vec3<f32>,
	nodeUniform28 : vec3<f32>,
	nodeUniform9 : vec3<f32>,
	nodeUniform31 : vec3<f32>,
	nodeUniform32 : f32,
	nodeUniform33 : f32,
	nodeUniform14 : mat4x4<f32>,
	nodeUniform13 : vec4<f32>,
	nodeUniform20 : mat4x4<f32>,
	nodeUniform19 : vec4<f32>,
	nodeUniform12 : f32,
	nodeUniform15 : f32,
	nodeUniform17 : f32,
	nodeUniform18 : vec2<f32>,
	nodeUniform21 : f32,
	nodeUniform22 : f32,
	nodeUniform23 : vec2<f32>,
	nodeUniform24 : f32
};
@binding( 0 ) @group( 0 )
var<uniform> render : renderStruct;

// vars
var<private> DiffuseColor : vec4<f32>;
var<private> nodeVar0 : vec4<f32>;
var<private> Shininess : f32;
var<private> SpecularColor : vec3<f32>;
var<private> EmissiveColor : vec3<f32>;
var<private> Output : vec4<f32>;
var<private> directDiffuse : vec3<f32>;
var<private> normalViewGeometry : vec3<f32>;
var<private> NORMAL_normalView : vec3<f32>;
var<private> normalView : vec3<f32>;
var<private> nodeVar1 : vec4<f32>;
var<private> nodeVar2 : vec4<f32>;
var<private> nodeVar3 : vec3<f32>;
var<private> nodeVar4 : vec3<f32>;
var<private> nodeVar5 : f32;
var<private> shadowPositionWorld : vec3<f32>;
var<private> normalWorld : vec3<f32>;
var<private> nodeVar6 : vec4<f32>;
var<private> nodeVar7 : f32;
var<private> shadowValue : f32;
var<private> nodeVar8 : f32;
var<private> nodeVar9 : vec4<f32>;
var<private> nodeVar10 : vec3<f32>;
var<private> nodeVar11 : vec3<f32>;
var<private> nodeVar12 : f32;
var<private> nodeVar13 : f32;
var<private> nodeVar14 : vec2<f32>;
var<private> nodeVar15 : f32;
var<private> nodeVar16 : vec2<f32>;
var<private> nodeVar17 : f32;
var<private> nodeVar18 : vec2<f32>;
var<private> nodeVar19 : f32;
var<private> nodeVar20 : vec2<f32>;
var<private> nodeVar21 : f32;
var<private> nodeVar22 : vec2<f32>;
var<private> nodeVar23 : f32;
var<private> nodeVar24 : f32;
var<private> nodeVar25 : vec4<f32>;
var<private> nodeVar26 : vec3<f32>;
var<private> nodeVar27 : vec3<f32>;
var<private> nodeVar28 : f32;
var<private> nodeVar29 : f32;
var<private> nodeVar30 : vec2<f32>;
var<private> nodeVar31 : f32;
var<private> nodeVar32 : vec2<f32>;
var<private> nodeVar33 : f32;
var<private> nodeVar34 : vec2<f32>;
var<private> nodeVar35 : f32;
var<private> nodeVar36 : vec2<f32>;
var<private> nodeVar37 : f32;
var<private> nodeVar38 : vec2<f32>;
var<private> nodeVar39 : f32;
var<private> nodeVar40 : f32;
var<private> nodeVar41 : vec3<f32>;
var<private> nodeVar42 : vec3<f32>;
var<private> nodeVar43 : vec3<f32>;
var<private> nodeVar44 : vec3<f32>;
var<private> directSpecular : vec3<f32>;
var<private> positionViewDirection : vec3<f32>;
var<private> nodeVar45 : vec3<f32>;
var<private> nodeVar46 : f32;
var<private> nodeVar47 : f32;
var<private> nodeVar48 : vec3<f32>;
var<private> nodeVar49 : vec3<f32>;
var<private> nodeVar50 : vec3<f32>;
var<private> nodeVar51 : vec3<f32>;
var<private> irradiance : vec3<f32>;
var<private> nodeVar52 : f32;
var<private> nodeVar53 : f32;
var<private> nodeVar54 : f32;
var<private> nodeVar55 : vec3<f32>;
var<private> nodeVar56 : vec3<f32>;
var<private> nodeVar57 : f32;
var<private> nodeVar58 : f32;
var<private> nodeVar59 : f32;
var<private> nodeVar60 : vec3<f32>;
var<private> nodeVar61 : vec3<f32>;
var<private> indirectDiffuse : vec3<f32>;
var<private> nodeVar62 : vec4<f32>;
var<private> nodeVar63 : vec4<f32>;
var<private> nodeVar64 : vec4<f32>;
var<private> ambientOcclusion : f32;
var<private> nodeVar65 : vec3<f32>;
var<private> totalDiffuse : vec3<f32>;
var<private> nodeVar66 : vec3<f32>;
var<private> totalSpecular : vec3<f32>;
var<private> indirectSpecular : vec3<f32>;
var<private> nodeVar67 : vec3<f32>;
var<private> outgoingLight : vec3<f32>;
var<private> nodeVar68 : vec3<f32>;
var<private> nodeVar69 : vec4<f32>;
var<private> modelViewMatrix : mat4x4<f32>;
var<private> nodeVar70 : vec4<f32>;
var<private> nodeVar71 : vec4<f32>;
var<private> nodeVar72 : vec2<f32>;

// codes
fn interleavedGradientNoise ( position : vec2<f32> ) -> f32 {

	


	return fract( ( 52.9829189 * fract( dot( position, vec2<f32>( 0.06711056, 0.00583715 ) ) ) ) );

}


fn vogelDiskSample ( sampleIndex : i32, samplesCount : i32, phi : f32 ) -> vec2<f32> {

	var nodeVar0 : f32;

	nodeVar0 = ( ( f32( sampleIndex ) * 2.399963229728653 ) + phi );

	return ( vec2<f32>( cos( nodeVar0 ), sin( nodeVar0 ) ) * vec2<f32>( sqrt( ( ( f32( sampleIndex ) + 0.5 ) / f32( samplesCount ) ) ) ) );

}




@fragment
fn main( @location( 0 ) v_positionView : vec3<f32>,
	@location( 1 ) positionLocal : vec3<f32>,
	@location( 2 ) v_positionWorld : vec3<f32>,
	@location( 3 ) v_positionViewDirection : vec3<f32>,
	@location( 4 ) positionPrevious : vec3<f32>,
	@location( 5 ) v_normalViewGeometry : vec3<f32>,
	@location( 6 ) nodeVarying8 : vec2<f32>,
	@builtin( position ) fragCoord : vec4<f32> ) -> OutputType {

	// flow
	// code

	nodeVar0 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying8 * vec2<f32>( 5.0 ) ) );
	DiffuseColor = nodeVar0;
	DiffuseColor.w = ( DiffuseColor.w * object.nodeUniform1 );
	DiffuseColor.w = 1.0;
	Shininess = max( object.nodeUniform2, 0.0001 );
	SpecularColor = object.nodeUniform3;
	EmissiveColor = ( object.nodeUniform4 * vec3<f32>( object.nodeUniform5 ) );
	directDiffuse = vec3<f32>( 0.0, 0.0, 0.0 );
	normalViewGeometry = normalize( v_normalViewGeometry );
	NORMAL_normalView = normalViewGeometry;
	normalView = NORMAL_normalView;
	nodeVar1 = vec4<f32>( render.nodeUniform9, 0.0 );
	nodeVar2 = ( render.cameraViewMatrix * nodeVar1 );
	nodeVar3 = normalize( nodeVar2.xyz );
	nodeVar4 = nodeVar3;
	nodeVar5 = dot( normalView, nodeVar4 );
	shadowPositionWorld = v_positionWorld;
	normalWorld = normalize( ( vec4<f32>( normalView, 0.0 ) * render.cameraViewMatrix ).xyz );
	nodeVar6 = vec4<f32>( ( shadowPositionWorld + ( normalWorld * vec3<f32>( render.nodeUniform12 ) ) ), 1.0 );
	nodeVar7 = ( - v_positionView.z );
	shadowValue = 1.0;

	if ( ( ( nodeVar7 >= render.nodeUniform13.x ) && ( nodeVar7 < render.nodeUniform13.y ) ) ) {

		nodeVar9 = ( render.nodeUniform14 * nodeVar6 );
		nodeVar10 = ( nodeVar9.xyz / vec3<f32>( nodeVar9.w ) );
		nodeVar11 = vec3<f32>( nodeVar10.x, ( 1.0 - nodeVar10.y ), ( nodeVar10.z + render.nodeUniform15 ) );

		if ( ( ( ( ( ( nodeVar11.x >= 0.0 ) && ( nodeVar11.x <= 1.0 ) ) && ( nodeVar11.y >= 0.0 ) ) && ( nodeVar11.y <= 1.0 ) ) && ( nodeVar11.z <= 1.0 ) ) ) {

			nodeVar12 = ( interleavedGradientNoise( fragCoord.xy ) * 6.28318530718 );
			nodeVar13 = ( render.nodeUniform17 * ( vec2<f32>( 1.0, 1.0 ) / render.nodeUniform18 ).x );
			nodeVar14 = ( nodeVar11.xy + ( vogelDiskSample( 0, 5, nodeVar12 ) * vec2<f32>( nodeVar13 ) ) );
			nodeVar15 = textureSampleCompare( nodeUniform16, nodeUniform16_sampler, nodeVar14, nodeVar11.z );
			nodeVar16 = ( nodeVar11.xy + ( vogelDiskSample( 1, 5, nodeVar12 ) * vec2<f32>( nodeVar13 ) ) );
			nodeVar17 = textureSampleCompare( nodeUniform16, nodeUniform16_sampler, nodeVar16, nodeVar11.z );
			nodeVar18 = ( nodeVar11.xy + ( vogelDiskSample( 2, 5, nodeVar12 ) * vec2<f32>( nodeVar13 ) ) );
			nodeVar19 = textureSampleCompare( nodeUniform16, nodeUniform16_sampler, nodeVar18, nodeVar11.z );
			nodeVar20 = ( nodeVar11.xy + ( vogelDiskSample( 3, 5, nodeVar12 ) * vec2<f32>( nodeVar13 ) ) );
			nodeVar21 = textureSampleCompare( nodeUniform16, nodeUniform16_sampler, nodeVar20, nodeVar11.z );
			nodeVar22 = ( nodeVar11.xy + ( vogelDiskSample( 4, 5, nodeVar12 ) * vec2<f32>( nodeVar13 ) ) );
			nodeVar23 = textureSampleCompare( nodeUniform16, nodeUniform16_sampler, nodeVar22, nodeVar11.z );
			nodeVar8 = ( ( ( ( ( nodeVar15 + nodeVar17 ) + nodeVar19 ) + nodeVar21 ) + nodeVar23 ) * 0.2 );

		} else {

			nodeVar8 = 1.0;

		}

		shadowValue = mix( nodeVar8, shadowValue, smoothstep( render.nodeUniform13.z, render.nodeUniform13.y, nodeVar7 ) );
		

	}


	if ( ( ( nodeVar7 >= render.nodeUniform19.x ) && ( nodeVar7 < render.nodeUniform19.y ) ) ) {

		nodeVar25 = ( render.nodeUniform20 * nodeVar6 );
		nodeVar26 = ( nodeVar25.xyz / vec3<f32>( nodeVar25.w ) );
		nodeVar27 = vec3<f32>( nodeVar26.x, ( 1.0 - nodeVar26.y ), ( nodeVar26.z + render.nodeUniform21 ) );

		if ( ( ( ( ( ( nodeVar27.x >= 0.0 ) && ( nodeVar27.x <= 1.0 ) ) && ( nodeVar27.y >= 0.0 ) ) && ( nodeVar27.y <= 1.0 ) ) && ( nodeVar27.z <= 1.0 ) ) ) {

			nodeVar28 = ( interleavedGradientNoise( fragCoord.xy ) * 6.28318530718 );
			nodeVar29 = ( render.nodeUniform22 * ( vec2<f32>( 1.0, 1.0 ) / render.nodeUniform23 ).x );
			nodeVar30 = ( nodeVar27.xy + ( vogelDiskSample( 0, 5, nodeVar28 ) * vec2<f32>( nodeVar29 ) ) );
			nodeVar31 = textureSampleCompare( nodeUniform16, nodeUniform16_sampler, nodeVar30, nodeVar27.z );
			nodeVar32 = ( nodeVar27.xy + ( vogelDiskSample( 1, 5, nodeVar28 ) * vec2<f32>( nodeVar29 ) ) );
			nodeVar33 = textureSampleCompare( nodeUniform16, nodeUniform16_sampler, nodeVar32, nodeVar27.z );
			nodeVar34 = ( nodeVar27.xy + ( vogelDiskSample( 2, 5, nodeVar28 ) * vec2<f32>( nodeVar29 ) ) );
			nodeVar35 = textureSampleCompare( nodeUniform16, nodeUniform16_sampler, nodeVar34, nodeVar27.z );
			nodeVar36 = ( nodeVar27.xy + ( vogelDiskSample( 3, 5, nodeVar28 ) * vec2<f32>( nodeVar29 ) ) );
			nodeVar37 = textureSampleCompare( nodeUniform16, nodeUniform16_sampler, nodeVar36, nodeVar27.z );
			nodeVar38 = ( nodeVar27.xy + ( vogelDiskSample( 4, 5, nodeVar28 ) * vec2<f32>( nodeVar29 ) ) );
			nodeVar39 = textureSampleCompare( nodeUniform16, nodeUniform16_sampler, nodeVar38, nodeVar27.z );
			nodeVar24 = ( ( ( ( ( nodeVar31 + nodeVar33 ) + nodeVar35 ) + nodeVar37 ) + nodeVar39 ) * 0.2 );

		} else {

			nodeVar24 = 1.0;

		}

		shadowValue = mix( nodeVar24, shadowValue, smoothstep( render.nodeUniform19.z, render.nodeUniform19.y, nodeVar7 ) );
		

	}

	nodeVar40 = mix( 1.0, shadowValue, render.nodeUniform24 );
	nodeVar41 = ( vec3<f32>( clamp( nodeVar5, 0.0, 1.0 ) ) * ( render.nodeUniform10 * vec3<f32>( nodeVar40 ) ) );
	nodeVar42 = ( DiffuseColor.xyz * vec3<f32>( 0.3183098861837907 ) );
	nodeVar43 = ( nodeVar41 * nodeVar42 );
	nodeVar44 = ( directDiffuse + nodeVar43 );
	directDiffuse = nodeVar44;
	directSpecular = vec3<f32>( 0.0, 0.0, 0.0 );
	positionViewDirection = normalize( v_positionViewDirection );
	nodeVar45 = normalize( ( nodeVar4 + positionViewDirection ) );
	nodeVar46 = clamp( dot( positionViewDirection, nodeVar45 ), 0.0, 1.0 );
	nodeVar47 = exp2( ( ( ( nodeVar46 * -5.55473 ) - 6.98316 ) * nodeVar46 ) );
	nodeVar48 = ( ( ( ( SpecularColor * vec3<f32>( ( 1.0 - nodeVar47 ) ) ) + vec3<f32>( ( 1.0 * nodeVar47 ) ) ) * vec3<f32>( 0.25 ) ) * vec3<f32>( ( ( ( ( Shininess * 0.5 ) + 1.0 ) * 0.3183098861837907 ) * pow( clamp( dot( normalView, nodeVar45 ), 0.0, 1.0 ), Shininess ) ) ) );
	nodeVar49 = ( nodeVar41 * nodeVar48 );
	nodeVar50 = ( nodeVar49 * vec3<f32>( 1.0 ) );
	nodeVar51 = ( directSpecular + nodeVar50 );
	directSpecular = nodeVar51;
	irradiance = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar52 = dot( normalWorld, normalize( render.nodeUniform27 ) );
	nodeVar53 = ( nodeVar52 * 0.5 );
	nodeVar54 = ( nodeVar53 + 0.5 );
	nodeVar55 = mix( render.nodeUniform25, render.nodeUniform26, nodeVar54 );
	nodeVar56 = ( irradiance + nodeVar55 );
	irradiance = nodeVar56;
	nodeVar57 = dot( normalWorld, normalize( render.nodeUniform30 ) );
	nodeVar58 = ( nodeVar57 * 0.5 );
	nodeVar59 = ( nodeVar58 + 0.5 );
	nodeVar60 = mix( render.nodeUniform28, render.nodeUniform29, nodeVar59 );
	nodeVar61 = ( irradiance + nodeVar60 );
	irradiance = nodeVar61;
	indirectDiffuse = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar62 = ( DiffuseColor * vec4<f32>( 0.3183098861837907 ) );
	nodeVar63 = ( vec4<f32>( irradiance, 1.0 ) * nodeVar62 );
	nodeVar64 = ( vec4<f32>( indirectDiffuse, 1.0 ) + nodeVar63 );
	indirectDiffuse = nodeVar64.xyz;
	ambientOcclusion = 1.0;
	nodeVar65 = ( indirectDiffuse * vec3<f32>( ambientOcclusion ) );
	indirectDiffuse = nodeVar65;
	nodeVar66 = ( directDiffuse + indirectDiffuse );
	totalDiffuse = nodeVar66;
	indirectSpecular = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar67 = ( directSpecular + indirectSpecular );
	totalSpecular = nodeVar67;
	nodeVar68 = ( totalDiffuse + totalSpecular );
	outgoingLight = nodeVar68;
	Output = max( vec4<f32>( ( outgoingLight + EmissiveColor ), DiffuseColor.w ), vec4<f32>( 0.0 ) );
	nodeVar69 = vec4<f32>( mix( Output.xyz, render.nodeUniform31, smoothstep( render.nodeUniform32, render.nodeUniform33, ( - v_positionView.z ) ) ), Output.w );
	Output = nodeVar69;
	output.m0 = Output;
	modelViewMatrix = ( render.cameraViewMatrix * object.nodeUniform35 );
	nodeVar70 = ( ( render.cameraProjectionMatrix * modelViewMatrix ) * vec4<f32>( positionLocal, 1.0 ) );
	nodeVar71 = ( ( render.nodeUniform36 * ( object.nodeUniform37 * object.nodeUniform38 ) ) * vec4<f32>( positionPrevious, 1.0 ) );
	nodeVar72 = ( ( nodeVar70.xy / vec2<f32>( nodeVar70.w ) ) - ( nodeVar71.xy / vec2<f32>( nodeVar71.w ) ) );
	output.m1 = vec4<f32>( vec3<f32>( nodeVar72, 0.0 ), 1.0 );

	// result

	return output;

}
