// Three.js r187dev - Node System

// global
diagnostic( off, derivative_uniformity );


// directives


// structs

struct OutputType {
	@location( 0 ) m0 : vec4<f32>,
	@location( 1 ) m1 : vec4<f32>,
	@location( 2 ) m2 : vec4<f32>,
	@location( 3 ) m3 : vec4<f32>,
	
};
var<private> output : OutputType;

// uniforms
@binding( 1 ) @group( 1 ) var nodeUniform2_sampler : sampler;
@binding( 2 ) @group( 1 ) var nodeUniform2 : texture_2d<f32>;
@binding( 3 ) @group( 1 ) var nodeUniform13_sampler : sampler;
@binding( 4 ) @group( 1 ) var nodeUniform13 : texture_2d<f32>;
@binding( 5 ) @group( 1 ) var nodeUniform22_sampler : sampler_comparison;
@binding( 6 ) @group( 1 ) var nodeUniform22 : texture_depth_2d;
@binding( 7 ) @group( 1 ) var nodeUniform26_sampler : sampler;
@binding( 8 ) @group( 1 ) var nodeUniform26 : texture_cube<f32>;

struct objectStruct {
	nodeUniform0 : vec3<f32>,
	nodeUniform1 : f32,
	nodeUniform3 : mat3x3<f32>,
	nodeUniform4 : f32,
	nodeUniform5 : f32,
	nodeUniform6 : mat3x3<f32>,
	nodeUniform7 : f32,
	nodeUniform8 : mat3x3<f32>,
	nodeUniform10 : mat3x3<f32>,
	nodeUniform11 : vec3<f32>,
	nodeUniform12 : f32,
	nodeUniform14 : mat4x4<f32>,
	nodeUniform27 : mat4x4<f32>,
	nodeUniform29 : f32,
	nodeUniform30 : f32,
	nodeUniform32 : mat4x4<f32>,
	nodeUniform35 : mat4x4<f32>
};
@binding( 0 ) @group( 1 )
var<uniform> object : objectStruct;

struct renderStruct {
	nodeUniform31 : mat4x4<f32>,
	nodeUniform33 : mat4x4<f32>,
	nodeUniform34 : mat4x4<f32>,
	cameraProjectionMatrix : mat4x4<f32>,
	cameraViewMatrix : mat4x4<f32>,
	nodeUniform18 : vec3<f32>,
	nodeUniform16 : vec3<f32>,
	nodeUniform17 : vec3<f32>,
	nodeUniform19 : mat4x4<f32>,
	nodeUniform20 : f32,
	nodeUniform21 : f32,
	nodeUniform23 : f32,
	nodeUniform24 : vec2<f32>,
	nodeUniform25 : f32,
	cameraWorldMatrix : mat4x4<f32>
};
@binding( 0 ) @group( 0 )
var<uniform> render : renderStruct;

// vars
var<private> DiffuseColor : vec4<f32>;
var<private> AmbientOcclusion : f32;
var<private> nodeVar0 : vec4<f32>;
var<private> Metalness : f32;
var<private> nodeVar1 : vec4<f32>;
var<private> Roughness : f32;
var<private> nodeVar2 : vec4<f32>;
var<private> normalViewGeometry : vec3<f32>;
var<private> SpecularColor : vec3<f32>;
var<private> SpecularColorBlended : vec3<f32>;
var<private> SpecularF90 : f32;
var<private> DiffuseContribution : vec3<f32>;
var<private> EmissiveColor : vec3<f32>;
var<private> Output : vec4<f32>;
var<private> NORMAL_normalView : vec3<f32>;
var<private> normalView : vec3<f32>;
var<private> positionViewDirection : vec3<f32>;
var<private> nodeVar3 : vec2<f32>;
var<private> singleScatteringDielectric : vec3<f32>;
var<private> multiScatteringDielectric : vec3<f32>;
var<private> shadowPositionWorld : vec3<f32>;
var<private> nodeVar4 : f32;
var<private> normalWorld : vec3<f32>;
var<private> nodeVar5 : f32;
var<private> nodeVar6 : f32;
var<private> nodeVar7 : f32;
var<private> nodeVar8 : f32;
var<private> nodeVar9 : f32;
var<private> nodeVar10 : f32;
var<private> nodeVar11 : vec3<f32>;
var<private> directDiffuse : vec3<f32>;
var<private> directSpecular : vec3<f32>;
var<private> radiance : vec3<f32>;
var<private> nodeVar12 : vec4<f32>;
var<private> iblIrradiance : vec3<f32>;
var<private> nodeVar13 : vec4<f32>;
var<private> ambientOcclusion : f32;
var<private> irradiance : vec3<f32>;
var<private> nodeVar14 : vec3<f32>;
var<private> indirectDiffuse : vec3<f32>;
var<private> singleScatteringMetallic : vec3<f32>;
var<private> multiScatteringMetallic : vec3<f32>;
var<private> nodeVar15 : vec3<f32>;
var<private> nodeVar16 : vec3<f32>;
var<private> indirectSpecular : vec3<f32>;
var<private> totalDiffuse : vec3<f32>;
var<private> totalSpecular : vec3<f32>;
var<private> outgoingLight : vec3<f32>;
var<private> modelViewMatrix : mat4x4<f32>;

// codes
fn interleavedGradientNoise ( position : vec2<f32> ) -> f32 {

	


	return fract( ( 52.9829189 * fract( dot( position, vec2<f32>( 0.06711056, 0.00583715 ) ) ) ) );

}


fn vogelDiskSample ( sampleIndex : i32, samplesCount : i32, phi : f32 ) -> vec2<f32> {

	

	let nodeConst0 = ( ( f32( sampleIndex ) * 2.399963229728653 ) + phi );

	return ( vec2<f32>( cos( nodeConst0 ), sin( nodeConst0 ) ) * vec2<f32>( sqrt( ( ( f32( sampleIndex ) + 0.5 ) / f32( samplesCount ) ) ) ) );

}


fn V_GGX_SmithCorrelated ( alpha : f32, dotNL : f32, dotNV : f32 ) -> f32 {

	

	let nodeConst0 = ( alpha * alpha );

	return ( 0.5 / max( ( ( dotNL * sqrt( ( nodeConst0 + ( ( 1.0 - nodeConst0 ) * ( dotNV * dotNV ) ) ) ) ) + ( dotNV * sqrt( ( nodeConst0 + ( ( 1.0 - nodeConst0 ) * ( dotNL * dotNL ) ) ) ) ) ), 0.000001 ) );

}


fn D_GGX ( alpha : f32, dotNH : f32 ) -> f32 {

	

	let nodeConst0 = ( alpha * alpha );
	let nodeConst1 = ( 1.0 - ( ( dotNH * dotNH ) * ( 1.0 - nodeConst0 ) ) );

	return ( ( nodeConst0 / ( nodeConst1 * nodeConst1 ) ) * 0.3183098861837907 );

}




@fragment
fn main( @location( 0 ) positionLocal : vec3<f32>,
	@location( 1 ) v_normalViewGeometry : vec3<f32>,
	@location( 2 ) v_positionViewDirection : vec3<f32>,
	@location( 3 ) v_positionWorld : vec3<f32>,
	@location( 4 ) positionPrevious : vec3<f32>,
	@location( 5 ) nodeVarying8 : vec2<f32>,
	@builtin( front_facing ) isFront : bool,
	@builtin( position ) fragCoord : vec4<f32> ) -> OutputType {

	// flow
	// code

	DiffuseColor = vec4<f32>( object.nodeUniform0, 1.0 );
	DiffuseColor.w = ( DiffuseColor.w * object.nodeUniform1 );
	DiffuseColor.w = 1.0;
	nodeVar0 = textureSample( nodeUniform2, nodeUniform2_sampler, ( object.nodeUniform3 * vec3<f32>( nodeVarying8, 1.0 ) ).xy );
	AmbientOcclusion = ( ( ( nodeVar0.x - 1.0 ) * object.nodeUniform4 ) + 1.0 );
	nodeVar1 = textureSample( nodeUniform2, nodeUniform2_sampler, ( object.nodeUniform6 * vec3<f32>( nodeVarying8, 1.0 ) ).xy );
	Metalness = ( object.nodeUniform5 * nodeVar1.z );
	nodeVar2 = textureSample( nodeUniform2, nodeUniform2_sampler, ( object.nodeUniform8 * vec3<f32>( nodeVarying8, 1.0 ) ).xy );
	normalViewGeometry = normalize( v_normalViewGeometry );
	let nodeConst0 = max( abs( dpdx( normalViewGeometry ) ), abs( - dpdy( normalViewGeometry ) ) );
	Roughness = min( ( max( ( object.nodeUniform7 * nodeVar2.y ), 0.045 ) + max( max( nodeConst0.x, nodeConst0.y ), nodeConst0.z ) ), 1.0 );
	SpecularColor = vec3<f32>( 0.04, 0.04, 0.04 );
	SpecularColorBlended = mix( vec3<f32>( 0.04, 0.04, 0.04 ), DiffuseColor.xyz, Metalness );
	SpecularF90 = 1.0;
	DiffuseContribution = ( DiffuseColor.xyz * vec3<f32>( ( 1.0 - ( object.nodeUniform5 * nodeVar1.z ) ) ) );
	EmissiveColor = ( object.nodeUniform11 * vec3<f32>( object.nodeUniform12 ) );
	NORMAL_normalView = ( normalViewGeometry * vec3<f32>( ( ( f32( isFront ) * 2.0 ) - 1.0 ) ) );
	normalView = NORMAL_normalView;
	positionViewDirection = normalize( v_positionViewDirection );
	nodeVar3 = textureSample( nodeUniform13, nodeUniform13_sampler, vec2<f32>( Roughness, clamp( dot( normalView, positionViewDirection ), 0.0, 1.0 ) ) ).xy;
	let dfg = nodeVar3;
	let multiScatteringCompensation = ( ( SpecularColorBlended * vec3<f32>( ( ( 1.0 / ( dfg.x + dfg.y ) ) - 1.0 ) ) ) + vec3<f32>( 1.0 ) );
	singleScatteringDielectric = vec3<f32>( 0.0, 0.0, 0.0 );
	multiScatteringDielectric = vec3<f32>( 0.0, 0.0, 0.0 );
	let nodeConst1 = ( ( SpecularColor * vec3<f32>( dfg.x ) ) + vec3<f32>( ( SpecularF90 * dfg.y ) ) );
	singleScatteringDielectric = ( singleScatteringDielectric + nodeConst1 );
	let nodeConst2 = ( SpecularColor + ( ( vec3<f32>( 1.0 ) - SpecularColor ) * vec3<f32>( 0.047619 ) ) );
	let nodeConst3 = ( 1.0 - ( dfg.x + dfg.y ) );
	multiScatteringDielectric = ( multiScatteringDielectric + ( ( ( nodeConst1 * nodeConst2 ) / ( vec3<f32>( 1.0 ) - ( vec3<f32>( nodeConst3 ) * nodeConst2 ) ) ) * vec3<f32>( nodeConst3 ) ) );
	let nodeConst4 = normalize( ( render.cameraViewMatrix * vec4<f32>( ( render.nodeUniform16 - render.nodeUniform17 ), 0.0 ) ).xyz );
	shadowPositionWorld = v_positionWorld;
	normalWorld = normalize( ( vec4<f32>( normalView, 0.0 ) * render.cameraViewMatrix ).xyz );
	let nodeConst5 = ( render.nodeUniform19 * vec4<f32>( ( shadowPositionWorld + ( normalWorld * vec3<f32>( render.nodeUniform20 ) ) ), 1.0 ) );
	let nodeConst6 = ( nodeConst5.xyz / vec3<f32>( nodeConst5.w ) );
	let nodeConst7 = vec3<f32>( nodeConst6.x, ( 1.0 - nodeConst6.y ), ( nodeConst6.z + render.nodeUniform21 ) );

	if ( ( ( ( ( ( nodeConst7.x >= 0.0 ) && ( nodeConst7.x <= 1.0 ) ) && ( nodeConst7.y >= 0.0 ) ) && ( nodeConst7.y <= 1.0 ) ) && ( nodeConst7.z <= 1.0 ) ) ) {

		let nodeConst8 = ( interleavedGradientNoise( fragCoord.xy ) * 6.28318530718 );
		let nodeConst9 = ( render.nodeUniform23 * ( vec2<f32>( 1.0, 1.0 ) / render.nodeUniform24 ).x );
		let nodeConst10 = ( nodeConst7.xy + ( vogelDiskSample( 0, 5, nodeConst8 ) * vec2<f32>( nodeConst9 ) ) );
		nodeVar5 = textureSampleCompare( nodeUniform22, nodeUniform22_sampler, nodeConst10, nodeConst7.z );
		let nodeConst11 = ( nodeConst7.xy + ( vogelDiskSample( 1, 5, nodeConst8 ) * vec2<f32>( nodeConst9 ) ) );
		nodeVar6 = textureSampleCompare( nodeUniform22, nodeUniform22_sampler, nodeConst11, nodeConst7.z );
		let nodeConst12 = ( nodeConst7.xy + ( vogelDiskSample( 2, 5, nodeConst8 ) * vec2<f32>( nodeConst9 ) ) );
		nodeVar7 = textureSampleCompare( nodeUniform22, nodeUniform22_sampler, nodeConst12, nodeConst7.z );
		let nodeConst13 = ( nodeConst7.xy + ( vogelDiskSample( 3, 5, nodeConst8 ) * vec2<f32>( nodeConst9 ) ) );
		nodeVar8 = textureSampleCompare( nodeUniform22, nodeUniform22_sampler, nodeConst13, nodeConst7.z );
		let nodeConst14 = ( nodeConst7.xy + ( vogelDiskSample( 4, 5, nodeConst8 ) * vec2<f32>( nodeConst9 ) ) );
		nodeVar9 = textureSampleCompare( nodeUniform22, nodeUniform22_sampler, nodeConst14, nodeConst7.z );
		nodeVar4 = ( ( ( ( ( nodeVar5 + nodeVar6 ) + nodeVar7 ) + nodeVar8 ) + nodeVar9 ) * 0.2 );

	} else {

		nodeVar4 = 1.0;

	}

	nodeVar10 = mix( 1.0, nodeVar4, render.nodeUniform25 );
	nodeVar11 = ( vec3<f32>( clamp( dot( normalView, nodeConst4 ), 0.0, 1.0 ) ) * ( render.nodeUniform18 * vec3<f32>( nodeVar10 ) ) );
	directDiffuse = vec3<f32>( 0.0, 0.0, 0.0 );
	let nodeConst15 = clamp( dot( positionViewDirection, normalize( ( nodeConst4 + positionViewDirection ) ) ), 0.0, 1.0 );
	let nodeConst16 = exp2( ( ( ( nodeConst15 * -5.55473 ) - 6.98316 ) * nodeConst15 ) );
	directDiffuse = ( directDiffuse + ( ( nodeVar11 * ( DiffuseContribution * vec3<f32>( 0.3183098861837907 ) ) ) * ( vec3<f32>( 1.0 ) - ( ( SpecularColor * vec3<f32>( ( 1.0 - nodeConst16 ) ) ) + vec3<f32>( ( SpecularF90 * nodeConst16 ) ) ) ) ) );
	directSpecular = vec3<f32>( 0.0, 0.0, 0.0 );
	let nodeConst17 = normalize( ( nodeConst4 + positionViewDirection ) );
	let nodeConst18 = clamp( dot( positionViewDirection, nodeConst17 ), 0.0, 1.0 );
	let nodeConst19 = exp2( ( ( ( nodeConst18 * -5.55473 ) - 6.98316 ) * nodeConst18 ) );
	let nodeConst20 = max( Roughness, 0.045 );
	let nodeConst21 = ( nodeConst20 * nodeConst20 );
	directSpecular = ( directSpecular + ( ( nodeVar11 * ( ( ( ( SpecularColorBlended * vec3<f32>( ( 1.0 - nodeConst19 ) ) ) + vec3<f32>( ( 1.0 * nodeConst19 ) ) ) * vec3<f32>( V_GGX_SmithCorrelated( nodeConst21, clamp( dot( normalView, nodeConst4 ), 0.0, 1.0 ), clamp( dot( normalView, positionViewDirection ), 0.0, 1.0 ) ) ) ) * vec3<f32>( D_GGX( nodeConst21, clamp( dot( normalView, nodeConst17 ), 0.0, 1.0 ) ) ) ) ) * multiScatteringCompensation ) );
	radiance = vec3<f32>( 0.0, 0.0, 0.0 );
	let nodeConst22 = normalize( ( render.cameraWorldMatrix * vec4<f32>( normalize( mix( reflect( ( - positionViewDirection ), normalView ), normalView, ( ( ( Roughness * Roughness ) * Roughness ) * Roughness ) ) ), 0.0 ) ).xyz );
	let nodeConst23 = ( object.nodeUniform27 * vec4<f32>( nodeConst22, 1.0 ) );
	let nodeConst24 = clamp( Roughness, 0.0, 1.0 );
	nodeVar12 = textureSampleLevel( nodeUniform26, nodeUniform26_sampler, vec3<f32>( ( - nodeConst23.x ), nodeConst23.yz ), ( ( object.nodeUniform29 * nodeConst24 ) * ( 2.0 - nodeConst24 ) ) );
	radiance = ( radiance + ( nodeVar12.xyz * vec3<f32>( object.nodeUniform30 ) ) );
	iblIrradiance = vec3<f32>( 0.0, 0.0, 0.0 );
	let nodeConst25 = ( object.nodeUniform27 * vec4<f32>( normalWorld, 1.0 ) );
	let nodeConst26 = clamp( 1.0, 0.0, 1.0 );
	nodeVar13 = textureSampleLevel( nodeUniform26, nodeUniform26_sampler, vec3<f32>( ( - nodeConst25.x ), nodeConst25.yz ), ( ( object.nodeUniform29 * nodeConst26 ) * ( 2.0 - nodeConst26 ) ) );
	iblIrradiance = ( iblIrradiance + ( ( nodeVar13.xyz * vec3<f32>( 3.141592653589793 ) ) * vec3<f32>( object.nodeUniform30 ) ) );
	ambientOcclusion = 1.0;
	ambientOcclusion = ( ambientOcclusion * AmbientOcclusion );
	irradiance = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar14 = ( ( irradiance * ( DiffuseContribution * vec3<f32>( 0.3183098861837907 ) ) ) * ( vec3<f32>( 1.0 ) - ( singleScatteringDielectric + multiScatteringDielectric ) ) );
	indirectDiffuse = vec3<f32>( 0.0, 0.0, 0.0 );
	indirectDiffuse = ( indirectDiffuse + nodeVar14 );
	singleScatteringMetallic = vec3<f32>( 0.0, 0.0, 0.0 );
	multiScatteringMetallic = vec3<f32>( 0.0, 0.0, 0.0 );
	let nodeConst27 = ( ( DiffuseColor.xyz * vec3<f32>( dfg.x ) ) + vec3<f32>( ( SpecularF90 * dfg.y ) ) );
	singleScatteringMetallic = ( singleScatteringMetallic + nodeConst27 );
	let nodeConst28 = ( DiffuseColor.xyz + ( ( vec3<f32>( 1.0 ) - DiffuseColor.xyz ) * vec3<f32>( 0.047619 ) ) );
	let nodeConst29 = ( 1.0 - ( dfg.x + dfg.y ) );
	multiScatteringMetallic = ( multiScatteringMetallic + ( ( ( nodeConst27 * nodeConst28 ) / ( vec3<f32>( 1.0 ) - ( vec3<f32>( nodeConst29 ) * nodeConst28 ) ) ) * vec3<f32>( nodeConst29 ) ) );
	let nodeConst30 = ( iblIrradiance * vec3<f32>( 0.3183098861837907 ) );
	nodeVar15 = ( ( vec3<f32>( 0.0, 0.0, 0.0 ) * mix( singleScatteringDielectric, singleScatteringMetallic, Metalness ) ) + ( mix( multiScatteringDielectric, multiScatteringMetallic, Metalness ) * nodeConst30 ) );
	nodeVar16 = ( ( DiffuseContribution * ( vec3<f32>( 1.0 ) - ( singleScatteringDielectric + multiScatteringDielectric ) ) ) * nodeConst30 );
	indirectSpecular = vec3<f32>( 0.0, 0.0, 0.0 );
	indirectSpecular = ( indirectSpecular + nodeVar15 );
	indirectDiffuse = ( indirectDiffuse + nodeVar16 );
	indirectDiffuse = ( indirectDiffuse * vec3<f32>( ambientOcclusion ) );
	indirectSpecular = ( indirectSpecular * vec3<f32>( clamp( ( ambientOcclusion - ( 1.0 - pow( ( clamp( dot( normalView, positionViewDirection ), 0.0, 1.0 ) + ambientOcclusion ), exp2( ( - ( 1.0 - ( Roughness * -16.0 ) ) ) ) ) ) ), 0.0, 1.0 ) ) );
	totalDiffuse = ( directDiffuse + indirectDiffuse );
	totalSpecular = ( directSpecular + indirectSpecular );
	outgoingLight = ( totalDiffuse + totalSpecular );
	Output = max( vec4<f32>( ( outgoingLight + EmissiveColor ), DiffuseColor.w ), vec4<f32>( 0.0 ) );
	output.m0 = Output;
	let nodeConst31 = vec4<f32>( ( ( normalView * vec3<f32>( 0.5 ) ) + vec3<f32>( 0.5 ) ), ( object.nodeUniform7 * nodeVar2.y ) );
	output.m1 = nodeConst31;
	modelViewMatrix = ( render.cameraViewMatrix * object.nodeUniform32 );
	let nodeConst32 = ( ( render.nodeUniform31 * modelViewMatrix ) * vec4<f32>( positionLocal, 1.0 ) );
	let nodeConst33 = ( ( render.nodeUniform33 * ( render.nodeUniform34 * object.nodeUniform35 ) ) * vec4<f32>( positionPrevious, 1.0 ) );
	let nodeConst34 = ( ( nodeConst32.xy / vec2<f32>( nodeConst32.w ) ) - ( nodeConst33.xy / vec2<f32>( nodeConst33.w ) ) );
	output.m2 = vec4<f32>( vec3<f32>( nodeConst34, 0.0 ), 1.0 );
	let nodeConst35 = vec4<f32>( DiffuseColor.xyz, ( object.nodeUniform5 * nodeVar1.z ) );
	output.m3 = nodeConst35;

	// result

	return output;

}
