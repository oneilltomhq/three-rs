// Three.js r187dev - Node System

// global
diagnostic( off, derivative_uniformity );


// directives


// structs

struct OutputType {
	@location( 0 ) m0 : vec4<f32>,
	@location( 1 ) m1 : vec4<f32>,
	
};
var<private> output : OutputType;

// uniforms
@binding( 1 ) @group( 1 ) var nodeUniform1_sampler : sampler;
@binding( 2 ) @group( 1 ) var nodeUniform1 : texture_2d<f32>;
@binding( 3 ) @group( 1 ) var nodeUniform4_sampler : sampler;
@binding( 4 ) @group( 1 ) var nodeUniform4 : texture_2d<f32>;
@binding( 5 ) @group( 1 ) var nodeUniform17_sampler : sampler;
@binding( 6 ) @group( 1 ) var nodeUniform17 : texture_2d<f32>;
@binding( 7 ) @group( 1 ) var nodeUniform24_sampler : sampler;
@binding( 8 ) @group( 1 ) var nodeUniform24 : texture_2d<f32>;
@binding( 9 ) @group( 1 ) var nodeUniform29_sampler : sampler;
@binding( 10 ) @group( 1 ) var nodeUniform29 : texture_2d<f32>;
@binding( 11 ) @group( 1 ) var nodeUniform34_sampler : sampler;
@binding( 12 ) @group( 1 ) var nodeUniform34 : texture_2d<f32>;
@binding( 13 ) @group( 1 ) var nodeUniform36_sampler : sampler;
@binding( 14 ) @group( 1 ) var nodeUniform36 : texture_2d<f32>;
@binding( 15 ) @group( 1 ) var nodeUniform37_sampler : sampler;
@binding( 16 ) @group( 1 ) var nodeUniform37 : texture_cube<f32>;

struct objectStruct {
	nodeUniform0 : vec3<f32>,
	nodeUniform2 : mat3x3<f32>,
	nodeUniform3 : f32,
	nodeUniform5 : mat3x3<f32>,
	nodeUniform6 : f32,
	nodeUniform7 : f32,
	nodeUniform8 : mat3x3<f32>,
	nodeUniform9 : f32,
	nodeUniform10 : mat3x3<f32>,
	nodeUniform12 : mat3x3<f32>,
	nodeUniform13 : f32,
	nodeUniform14 : vec3<f32>,
	nodeUniform15 : f32,
	nodeUniform16 : f32,
	nodeUniform18 : mat3x3<f32>,
	nodeUniform19 : f32,
	nodeUniform20 : f32,
	nodeUniform21 : vec3<f32>,
	nodeUniform22 : vec3<f32>,
	nodeUniform23 : f32,
	nodeUniform25 : mat3x3<f32>,
	nodeUniform28 : mat4x4<f32>,
	nodeUniform30 : mat3x3<f32>,
	nodeUniform31 : vec2<f32>,
	nodeUniform33 : mat4x4<f32>,
	nodeUniform38 : mat4x4<f32>,
	nodeUniform40 : f32,
	nodeUniform41 : f32
};
@binding( 0 ) @group( 1 )
var<uniform> object : objectStruct;

struct renderStruct {
	cameraProjectionMatrix : mat4x4<f32>,
	cameraViewMatrix : mat4x4<f32>,
	cameraWorldMatrix : mat4x4<f32>,
	cameraPosition : vec3<f32>,
	nodeUniform35 : vec2<f32>
};
@binding( 0 ) @group( 0 )
var<uniform> render : renderStruct;

// vars
var<private> DiffuseColor : vec4<f32>;
var<private> nodeVar0 : vec4<f32>;
var<private> AmbientOcclusion : f32;
var<private> nodeVar1 : vec4<f32>;
var<private> Metalness : f32;
var<private> nodeVar2 : vec4<f32>;
var<private> Roughness : f32;
var<private> nodeVar3 : vec4<f32>;
var<private> normalViewGeometry : vec3<f32>;
var<private> IOR : f32;
var<private> SpecularColor : vec3<f32>;
var<private> SpecularColorBlended : vec3<f32>;
var<private> SpecularF90 : f32;
var<private> DiffuseContribution : vec3<f32>;
var<private> Transmission : f32;
var<private> nodeVar4 : vec4<f32>;
var<private> Thickness : f32;
var<private> AttenuationDistance : f32;
var<private> AttenuationColor : vec3<f32>;
var<private> EmissiveColor : vec3<f32>;
var<private> nodeVar5 : vec4<f32>;
var<private> Output : vec4<f32>;
var<private> NORMAL_normalView : vec3<f32>;
var<private> nodeVar6 : f32;
var<private> tangentViewFrame : vec3<f32>;
var<private> NORMAL_tangentView : vec3<f32>;
var<private> bitangentViewFrame : vec3<f32>;
var<private> NORMAL_bitangentView : vec3<f32>;
var<private> NORMAL_TBNViewMatrix : mat3x3<f32>;
var<private> nodeVar7 : vec4<f32>;
var<private> normalView : vec3<f32>;
var<private> normalWorld : vec3<f32>;
var<private> nodeVar8 : vec2<f32>;
var<private> nodeVar9 : vec2<f32>;
var<private> nodeVar10 : vec4<f32>;
var<private> nodeVar11 : vec4<f32>;
var<private> nodeVar12 : vec4<f32>;
var<private> nodeVar13 : vec4<f32>;
var<private> nodeVar14 : vec4<f32>;
var<private> nodeVar15 : vec4<f32>;
var<private> nodeVar16 : vec4<f32>;
var<private> nodeVar17 : vec4<f32>;
var<private> positionViewDirection : vec3<f32>;
var<private> nodeVar18 : vec2<f32>;
var<private> singleScatteringDielectric : vec3<f32>;
var<private> multiScatteringDielectric : vec3<f32>;
var<private> radiance : vec3<f32>;
var<private> nodeVar19 : vec4<f32>;
var<private> iblIrradiance : vec3<f32>;
var<private> nodeVar20 : vec4<f32>;
var<private> ambientOcclusion : f32;
var<private> irradiance : vec3<f32>;
var<private> nodeVar21 : vec3<f32>;
var<private> indirectDiffuse : vec3<f32>;
var<private> singleScatteringMetallic : vec3<f32>;
var<private> multiScatteringMetallic : vec3<f32>;
var<private> nodeVar22 : vec3<f32>;
var<private> nodeVar23 : vec3<f32>;
var<private> indirectSpecular : vec3<f32>;
var<private> totalDiffuse : vec3<f32>;
var<private> directDiffuse : vec3<f32>;
var<private> totalSpecular : vec3<f32>;
var<private> directSpecular : vec3<f32>;
var<private> outgoingLight : vec3<f32>;

// codes
fn getVolumeTransmissionRay ( n : vec3<f32>, v : vec3<f32>, thickness : f32, ior : f32, modelMatrix : mat4x4<f32> ) -> vec3<f32> {

	


	return ( normalize( refract( ( - v ), normalize( n ), ( 1.0 / ior ) ) ) * ( vec3<f32>( thickness ) * vec3<f32>( length( modelMatrix[ 0u ].xyz ), length( modelMatrix[ 1u ].xyz ), length( modelMatrix[ 2u ].xyz ) ) ) );

}


fn volumeAttenuation ( transmissionDistance : f32, attenuationColor : vec3<f32>, attenuationDistance : f32 ) -> vec3<f32> {

	


	if ( ( attenuationDistance != 0.0 ) ) {

		return exp( ( ( - ( ( - log( attenuationColor ) ) / vec3<f32>( attenuationDistance ) ) ) * vec3<f32>( transmissionDistance ) ) );

	}


	return vec3<f32>( 1.0, 1.0, 1.0 );

}


fn applyIorToRoughness ( roughness : f32, ior : f32 ) -> f32 {

	


	return ( roughness * clamp( ( ( ior * 2.0 ) - 2.0 ), 0.0, 1.0 ) );

}




@fragment
fn main( @location( 0 ) v_positionView : vec3<f32>,
	@location( 1 ) v_normalViewGeometry : vec3<f32>,
	@location( 2 ) v_positionWorld : vec3<f32>,
	@location( 3 ) v_positionViewDirection : vec3<f32>,
	@location( 4 ) nodeVarying7 : vec2<f32> ) -> OutputType {

	// flow
	// code

	nodeVar0 = textureSample( nodeUniform1, nodeUniform1_sampler, ( object.nodeUniform2 * vec3<f32>( nodeVarying7, 1.0 ) ).xy );
	DiffuseColor = ( vec4<f32>( object.nodeUniform0, 1.0 ) * nodeVar0 );
	DiffuseColor.w = ( DiffuseColor.w * object.nodeUniform3 );
	DiffuseColor.w = 1.0;
	nodeVar1 = textureSample( nodeUniform4, nodeUniform4_sampler, ( object.nodeUniform5 * vec3<f32>( nodeVarying7, 1.0 ) ).xy );
	AmbientOcclusion = ( ( ( nodeVar1.x - 1.0 ) * object.nodeUniform6 ) + 1.0 );
	nodeVar2 = textureSample( nodeUniform4, nodeUniform4_sampler, ( object.nodeUniform8 * vec3<f32>( nodeVarying7, 1.0 ) ).xy );
	Metalness = ( object.nodeUniform7 * nodeVar2.z );
	nodeVar3 = textureSample( nodeUniform4, nodeUniform4_sampler, ( object.nodeUniform10 * vec3<f32>( nodeVarying7, 1.0 ) ).xy );
	normalViewGeometry = normalize( v_normalViewGeometry );
	let nodeConst0 = max( abs( dpdx( normalViewGeometry ) ), abs( - dpdy( normalViewGeometry ) ) );
	Roughness = min( ( max( ( object.nodeUniform9 * nodeVar3.y ), 0.045 ) + max( max( nodeConst0.x, nodeConst0.y ), nodeConst0.z ) ), 1.0 );
	IOR = object.nodeUniform13;
	let nodeConst1 = ( ( IOR - 1.0 ) / ( IOR + 1.0 ) );
	SpecularColor = ( min( ( vec3<f32>( ( nodeConst1 * nodeConst1 ) ) * object.nodeUniform14 ), vec3<f32>( 1.0, 1.0, 1.0 ) ) * vec3<f32>( object.nodeUniform15 ) );
	SpecularColorBlended = mix( SpecularColor, DiffuseColor.xyz, Metalness );
	SpecularF90 = mix( object.nodeUniform15, 1.0, Metalness );
	DiffuseContribution = ( DiffuseColor.xyz * vec3<f32>( ( 1.0 - ( object.nodeUniform7 * nodeVar2.z ) ) ) );
	nodeVar4 = textureSample( nodeUniform17, nodeUniform17_sampler, ( object.nodeUniform18 * vec3<f32>( nodeVarying7, 1.0 ) ).xy );
	Transmission = ( object.nodeUniform16 * nodeVar4.x );
	Thickness = object.nodeUniform19;
	AttenuationDistance = object.nodeUniform20;
	AttenuationColor = object.nodeUniform21;
	nodeVar5 = textureSample( nodeUniform24, nodeUniform24_sampler, ( object.nodeUniform25 * vec3<f32>( nodeVarying7, 1.0 ) ).xy );
	EmissiveColor = ( vec4<f32>( ( object.nodeUniform22 * vec3<f32>( object.nodeUniform23 ) ), 1.0 ) * nodeVar5 ).xyz;
	NORMAL_normalView = normalViewGeometry;
	let nodeConst2 = cross( - dpdy( v_positionView ), NORMAL_normalView );
	let nodeConst3 = dpdx( nodeVarying7 );
	let nodeConst4 = cross( NORMAL_normalView, dpdx( v_positionView ) );
	let nodeConst5 = - dpdy( nodeVarying7 );
	let nodeConst6 = ( ( nodeConst2 * vec3<f32>( nodeConst3.x ) ) + ( nodeConst4 * vec3<f32>( nodeConst5.x ) ) );
	let nodeConst7 = ( ( nodeConst2 * vec3<f32>( nodeConst3.y ) ) + ( nodeConst4 * vec3<f32>( nodeConst5.y ) ) );
	let nodeConst8 = max( dot( nodeConst6, nodeConst6 ), dot( nodeConst7, nodeConst7 ) );

	if ( ( nodeConst8 == 0.0 ) ) {

		nodeVar6 = 0.0;

	} else {

		nodeVar6 = inverseSqrt( nodeConst8 );

	}

	tangentViewFrame = ( nodeConst6 * vec3<f32>( nodeVar6 ) );
	NORMAL_tangentView = tangentViewFrame;
	bitangentViewFrame = ( nodeConst7 * nodeVar6 );
	NORMAL_bitangentView = bitangentViewFrame;
	NORMAL_TBNViewMatrix = mat3x3<f32>( NORMAL_tangentView, NORMAL_bitangentView, NORMAL_normalView );
	nodeVar7 = textureSample( nodeUniform29, nodeUniform29_sampler, ( object.nodeUniform30 * vec3<f32>( nodeVarying7, 1.0 ) ).xy );
	let nodeConst9 = ( ( nodeVar7 * vec4<f32>( 2.0 ) ) - vec4<f32>( 1.0 ) );
	normalView = normalize( ( NORMAL_TBNViewMatrix * vec3<f32>( ( nodeConst9.xy * object.nodeUniform31 ), nodeConst9.z ) ) );
	normalWorld = normalize( ( vec4<f32>( normalView, 0.0 ) * render.cameraViewMatrix ).xyz );
	let nodeConst10 = normalize( ( render.cameraPosition - v_positionWorld ) );
	let nodeConst11 = getVolumeTransmissionRay( normalWorld, nodeConst10, Thickness, IOR, object.nodeUniform33 );
	let nodeConst12 = ( render.cameraProjectionMatrix * ( render.cameraViewMatrix * vec4<f32>( ( v_positionWorld + nodeConst11 ), 1.0 ) ) );
	nodeVar8 = ( nodeConst12.xy / vec2<f32>( nodeConst12.w ) );
	nodeVar8 = ( nodeVar8 + vec2<f32>( 1.0 ) );
	nodeVar8 = ( nodeVar8 / vec2<f32>( 2.0 ) );
	nodeVar8 = vec2<f32>( nodeVar8.x, ( 1.0 - nodeVar8.y ) );
	nodeVar9 = textureSample( nodeUniform34, nodeUniform34_sampler, vec2<f32>( Roughness, clamp( dot( normalWorld, nodeConst10 ), 0.0, 1.0 ) ) ).xy;
	let nodeConst13 = ( DiffuseContribution * volumeAttenuation( length( nodeConst11 ), AttenuationColor, AttenuationDistance ) );
	let cameraViewport = vec4<f32>( 0.0, 0.0, render.nodeUniform35.x, render.nodeUniform35.y );
	let nodeConst14 = ( ( ( nodeVar8 * cameraViewport.zw ) + cameraViewport.xy ) / render.nodeUniform35 );
	let nodeConst15 = ( log2( cameraViewport.z ) * applyIorToRoughness( Roughness, IOR ) );
	let nodeConst16 = vec4<f32>( vec2<f32>( textureDimensions( nodeUniform36, i32( nodeConst15 ) ) ), vec2<f32>( textureDimensions( nodeUniform36, i32( ( nodeConst15 + 1.0 ) ) ) ) );
	let nodeConst17 = ( ( vec4<f32>( vec3<f32>( nodeConst14, 0.0 ), 1.0 ).xyxy * nodeConst16 ) + vec4<f32>( 0.5 ) );
	let nodeConst18 = fract( nodeConst17 );
	let nodeConst19 = ( vec4<f32>( 0.16666666666666666 ) * ( ( nodeConst18 * ( nodeConst18 * ( ( vec4<f32>( 3.0 ) * nodeConst18 ) - vec4<f32>( 6.0 ) ) ) ) + vec4<f32>( 4.0 ) ) );
	let nodeConst20 = ( ( vec4<f32>( 0.16666666666666666 ) * ( ( nodeConst18 * ( ( nodeConst18 * ( ( - nodeConst18 ) + vec4<f32>( 3.0 ) ) ) - vec4<f32>( 3.0 ) ) ) + vec4<f32>( 1.0 ) ) ) + nodeConst19 );
	let nodeConst21 = floor( nodeConst17 );
	let nodeConst22 = ( vec4<f32>( 1.0 ) / nodeConst16 );
	let nodeConst23 = ( ( ( nodeConst21 + ( vec4<f32>( -1.0 ) + ( nodeConst19 / nodeConst20 ) ) ) - vec4<f32>( 0.5 ) ) * nodeConst22 );
	let nodeConst24 = floor( nodeConst15 );
	nodeVar10 = textureSampleLevel( nodeUniform36, nodeUniform36_sampler, nodeConst23.xy, nodeConst24 );
	let nodeConst25 = ( vec4<f32>( 0.16666666666666666 ) * pow( nodeConst18, vec4<f32>( 3.0 ) ) );
	let nodeConst26 = ( ( vec4<f32>( 0.16666666666666666 ) * ( ( nodeConst18 * ( ( nodeConst18 * ( ( vec4<f32>( -3.0 ) * nodeConst18 ) + vec4<f32>( 3.0 ) ) ) + vec4<f32>( 3.0 ) ) ) + vec4<f32>( 1.0 ) ) ) + nodeConst25 );
	let nodeConst27 = ( ( ( nodeConst21 + ( vec4<f32>( 1.0 ) + ( nodeConst25 / nodeConst26 ) ) ) - vec4<f32>( 0.5 ) ) * nodeConst22 );
	nodeVar11 = textureSampleLevel( nodeUniform36, nodeUniform36_sampler, vec2<f32>( nodeConst27.xy.x, nodeConst23.xy.y ), nodeConst24 );
	nodeVar12 = textureSampleLevel( nodeUniform36, nodeUniform36_sampler, vec2<f32>( nodeConst23.xy.x, nodeConst27.xy.y ), nodeConst24 );
	nodeVar13 = textureSampleLevel( nodeUniform36, nodeUniform36_sampler, nodeConst27.xy, nodeConst24 );
	let nodeConst28 = ceil( nodeConst15 );
	nodeVar14 = textureSampleLevel( nodeUniform36, nodeUniform36_sampler, nodeConst23.zw, nodeConst28 );
	nodeVar15 = textureSampleLevel( nodeUniform36, nodeUniform36_sampler, vec2<f32>( nodeConst27.zw.x, nodeConst23.zw.y ), nodeConst28 );
	nodeVar16 = textureSampleLevel( nodeUniform36, nodeUniform36_sampler, vec2<f32>( nodeConst23.zw.x, nodeConst27.zw.y ), nodeConst28 );
	nodeVar17 = textureSampleLevel( nodeUniform36, nodeUniform36_sampler, nodeConst27.zw, nodeConst28 );
	let nodeConst29 = mix( ( ( vec4<f32>( nodeConst20.xy.y ) * ( ( vec4<f32>( nodeConst20.xy.x ) * nodeVar10 ) + ( vec4<f32>( nodeConst26.xy.x ) * nodeVar11 ) ) ) + ( vec4<f32>( nodeConst26.xy.y ) * ( ( vec4<f32>( nodeConst20.xy.x ) * nodeVar12 ) + ( vec4<f32>( nodeConst26.xy.x ) * nodeVar13 ) ) ) ), ( ( vec4<f32>( nodeConst20.zw.y ) * ( ( vec4<f32>( nodeConst20.zw.x ) * nodeVar14 ) + ( vec4<f32>( nodeConst26.zw.x ) * nodeVar15 ) ) ) + ( vec4<f32>( nodeConst26.zw.y ) * ( ( vec4<f32>( nodeConst20.zw.x ) * nodeVar16 ) + ( vec4<f32>( nodeConst26.zw.x ) * nodeVar17 ) ) ) ), fract( nodeConst15 ) );
	let nodeConst30 = vec4<f32>( ( ( vec3<f32>( 1.0 ) - ( ( SpecularColorBlended * vec3<f32>( nodeVar9.x ) ) + vec3<f32>( ( SpecularF90 * nodeVar9.y ) ) ) ) * ( nodeConst13 * nodeConst29.xyz ) ), ( 1.0 - ( ( 1.0 - nodeConst29.w ) * ( ( ( nodeConst13.x + nodeConst13.y ) + nodeConst13.z ) / 3.0 ) ) ) );
	DiffuseColor.w = ( DiffuseColor.w * mix( 1.0, nodeConst30.w, Transmission ) );
	positionViewDirection = normalize( v_positionViewDirection );
	nodeVar18 = textureSample( nodeUniform34, nodeUniform34_sampler, vec2<f32>( Roughness, clamp( dot( normalView, positionViewDirection ), 0.0, 1.0 ) ) ).xy;
	let dfg = nodeVar18;
	let multiScatteringCompensation = ( ( SpecularColorBlended * vec3<f32>( ( ( 1.0 / ( dfg.x + dfg.y ) ) - 1.0 ) ) ) + vec3<f32>( 1.0 ) );
	singleScatteringDielectric = vec3<f32>( 0.0, 0.0, 0.0 );
	multiScatteringDielectric = vec3<f32>( 0.0, 0.0, 0.0 );
	let nodeConst31 = ( ( SpecularColor * vec3<f32>( dfg.x ) ) + vec3<f32>( ( SpecularF90 * dfg.y ) ) );
	singleScatteringDielectric = ( singleScatteringDielectric + nodeConst31 );
	let nodeConst32 = ( SpecularColor + ( ( vec3<f32>( 1.0 ) - SpecularColor ) * vec3<f32>( 0.047619 ) ) );
	let nodeConst33 = ( 1.0 - ( dfg.x + dfg.y ) );
	multiScatteringDielectric = ( multiScatteringDielectric + ( ( ( nodeConst31 * nodeConst32 ) / ( vec3<f32>( 1.0 ) - ( vec3<f32>( nodeConst33 ) * nodeConst32 ) ) ) * vec3<f32>( nodeConst33 ) ) );
	radiance = vec3<f32>( 0.0, 0.0, 0.0 );
	let nodeConst34 = normalize( ( render.cameraWorldMatrix * vec4<f32>( normalize( mix( reflect( ( - positionViewDirection ), normalView ), normalView, ( ( ( Roughness * Roughness ) * Roughness ) * Roughness ) ) ), 0.0 ) ).xyz );
	let nodeConst35 = ( object.nodeUniform38 * vec4<f32>( nodeConst34, 1.0 ) );
	let nodeConst36 = clamp( Roughness, 0.0, 1.0 );
	nodeVar19 = textureSampleLevel( nodeUniform37, nodeUniform37_sampler, vec3<f32>( ( - nodeConst35.x ), nodeConst35.yz ), ( ( object.nodeUniform40 * nodeConst36 ) * ( 2.0 - nodeConst36 ) ) );
	radiance = ( radiance + ( nodeVar19.xyz * vec3<f32>( object.nodeUniform41 ) ) );
	iblIrradiance = vec3<f32>( 0.0, 0.0, 0.0 );
	let nodeConst37 = ( object.nodeUniform38 * vec4<f32>( normalWorld, 1.0 ) );
	let nodeConst38 = clamp( 1.0, 0.0, 1.0 );
	nodeVar20 = textureSampleLevel( nodeUniform37, nodeUniform37_sampler, vec3<f32>( ( - nodeConst37.x ), nodeConst37.yz ), ( ( object.nodeUniform40 * nodeConst38 ) * ( 2.0 - nodeConst38 ) ) );
	iblIrradiance = ( iblIrradiance + ( ( nodeVar20.xyz * vec3<f32>( 3.141592653589793 ) ) * vec3<f32>( object.nodeUniform41 ) ) );
	ambientOcclusion = 1.0;
	ambientOcclusion = ( ambientOcclusion * AmbientOcclusion );
	irradiance = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar21 = ( ( irradiance * ( DiffuseContribution * vec3<f32>( 0.3183098861837907 ) ) ) * ( vec3<f32>( 1.0 ) - ( singleScatteringDielectric + multiScatteringDielectric ) ) );
	indirectDiffuse = vec3<f32>( 0.0, 0.0, 0.0 );
	indirectDiffuse = ( indirectDiffuse + nodeVar21 );
	singleScatteringMetallic = vec3<f32>( 0.0, 0.0, 0.0 );
	multiScatteringMetallic = vec3<f32>( 0.0, 0.0, 0.0 );
	let nodeConst39 = ( ( DiffuseColor.xyz * vec3<f32>( dfg.x ) ) + vec3<f32>( ( SpecularF90 * dfg.y ) ) );
	singleScatteringMetallic = ( singleScatteringMetallic + nodeConst39 );
	let nodeConst40 = ( DiffuseColor.xyz + ( ( vec3<f32>( 1.0 ) - DiffuseColor.xyz ) * vec3<f32>( 0.047619 ) ) );
	let nodeConst41 = ( 1.0 - ( dfg.x + dfg.y ) );
	multiScatteringMetallic = ( multiScatteringMetallic + ( ( ( nodeConst39 * nodeConst40 ) / ( vec3<f32>( 1.0 ) - ( vec3<f32>( nodeConst41 ) * nodeConst40 ) ) ) * vec3<f32>( nodeConst41 ) ) );
	let nodeConst42 = ( iblIrradiance * vec3<f32>( 0.3183098861837907 ) );
	nodeVar22 = ( ( radiance * mix( singleScatteringDielectric, singleScatteringMetallic, Metalness ) ) + ( mix( multiScatteringDielectric, multiScatteringMetallic, Metalness ) * nodeConst42 ) );
	nodeVar23 = ( ( DiffuseContribution * ( vec3<f32>( 1.0 ) - ( singleScatteringDielectric + multiScatteringDielectric ) ) ) * nodeConst42 );
	indirectSpecular = vec3<f32>( 0.0, 0.0, 0.0 );
	indirectSpecular = ( indirectSpecular + nodeVar22 );
	indirectDiffuse = ( indirectDiffuse + nodeVar23 );
	indirectDiffuse = ( indirectDiffuse * vec3<f32>( ambientOcclusion ) );
	indirectSpecular = ( indirectSpecular * vec3<f32>( clamp( ( ambientOcclusion - ( 1.0 - pow( ( clamp( dot( normalView, positionViewDirection ), 0.0, 1.0 ) + ambientOcclusion ), exp2( ( - ( 1.0 - ( Roughness * -16.0 ) ) ) ) ) ) ), 0.0, 1.0 ) ) );
	directDiffuse = vec3<f32>( 0.0, 0.0, 0.0 );
	totalDiffuse = mix( vec4<f32>( ( directDiffuse + indirectDiffuse ), 1.0 ), nodeConst30, Transmission ).xyz;
	directSpecular = vec3<f32>( 0.0, 0.0, 0.0 );
	totalSpecular = ( directSpecular + indirectSpecular );
	outgoingLight = ( totalDiffuse + totalSpecular );
	Output = max( vec4<f32>( ( outgoingLight + EmissiveColor ), DiffuseColor.w ), vec4<f32>( 0.0 ) );
	output.m0 = Output;
	output.m1 = vec4<f32>( EmissiveColor, 1.0 );

	// result

	return output;

}
