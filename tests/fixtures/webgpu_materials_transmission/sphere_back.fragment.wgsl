// Three.js r187dev - Node System

// global
diagnostic( off, derivative_uniformity );


// directives


// structs

struct OutputStruct {
	@location( 0 ) color: vec4<f32>
};
var<private> output : OutputStruct;

// uniforms
@binding( 1 ) @group( 1 ) var nodeUniform2_sampler : sampler;
@binding( 2 ) @group( 1 ) var nodeUniform2 : texture_2d<f32>;
@binding( 3 ) @group( 1 ) var nodeUniform22_sampler : sampler;
@binding( 4 ) @group( 1 ) var nodeUniform22 : texture_2d<f32>;
@binding( 5 ) @group( 1 ) var nodeUniform24_sampler : sampler;
@binding( 6 ) @group( 1 ) var nodeUniform24 : texture_2d<f32>;
@binding( 7 ) @group( 1 ) var nodeUniform25_sampler : sampler;
@binding( 8 ) @group( 1 ) var nodeUniform25 : texture_cube<f32>;

struct objectStruct {
	nodeUniform0 : vec3<f32>,
	nodeUniform1 : f32,
	nodeUniform3 : mat3x3<f32>,
	nodeUniform4 : f32,
	nodeUniform5 : f32,
	nodeUniform7 : mat3x3<f32>,
	nodeUniform8 : f32,
	nodeUniform9 : vec3<f32>,
	nodeUniform10 : f32,
	nodeUniform11 : f32,
	nodeUniform12 : f32,
	nodeUniform13 : f32,
	nodeUniform14 : vec3<f32>,
	nodeUniform15 : vec3<f32>,
	nodeUniform16 : f32,
	nodeUniform19 : mat4x4<f32>,
	nodeUniform21 : mat4x4<f32>,
	nodeUniform26 : mat4x4<f32>,
	nodeUniform28 : f32,
	nodeUniform29 : f32
};
@binding( 0 ) @group( 1 )
var<uniform> object : objectStruct;

struct renderStruct {
	cameraProjectionMatrix : mat4x4<f32>,
	cameraViewMatrix : mat4x4<f32>,
	cameraPosition : vec3<f32>,
	nodeUniform23 : vec2<f32>,
	cameraWorldMatrix : mat4x4<f32>
};
@binding( 0 ) @group( 0 )
var<uniform> render : renderStruct;

// vars
var<private> DiffuseColor : vec4<f32>;
var<private> nodeVar0 : vec4<f32>;
var<private> Metalness : f32;
var<private> Roughness : f32;
var<private> normalViewGeometry : vec3<f32>;
var<private> IOR : f32;
var<private> SpecularColor : vec3<f32>;
var<private> SpecularColorBlended : vec3<f32>;
var<private> SpecularF90 : f32;
var<private> DiffuseContribution : vec3<f32>;
var<private> Transmission : f32;
var<private> Thickness : f32;
var<private> AttenuationDistance : f32;
var<private> AttenuationColor : vec3<f32>;
var<private> EmissiveColor : vec3<f32>;
var<private> Output : vec4<f32>;
var<private> NORMAL_normalView : vec3<f32>;
var<private> normalView : vec3<f32>;
var<private> normalWorld : vec3<f32>;
var<private> nodeVar1 : vec2<f32>;
var<private> nodeVar2 : vec2<f32>;
var<private> nodeVar3 : vec4<f32>;
var<private> nodeVar4 : vec4<f32>;
var<private> nodeVar5 : vec4<f32>;
var<private> nodeVar6 : vec4<f32>;
var<private> nodeVar7 : vec4<f32>;
var<private> nodeVar8 : vec4<f32>;
var<private> nodeVar9 : vec4<f32>;
var<private> nodeVar10 : vec4<f32>;
var<private> positionViewDirection : vec3<f32>;
var<private> nodeVar11 : vec2<f32>;
var<private> singleScatteringDielectric : vec3<f32>;
var<private> multiScatteringDielectric : vec3<f32>;
var<private> radiance : vec3<f32>;
var<private> nodeVar12 : vec4<f32>;
var<private> iblIrradiance : vec3<f32>;
var<private> nodeVar13 : vec4<f32>;
var<private> irradiance : vec3<f32>;
var<private> nodeVar14 : vec3<f32>;
var<private> indirectDiffuse : vec3<f32>;
var<private> singleScatteringMetallic : vec3<f32>;
var<private> multiScatteringMetallic : vec3<f32>;
var<private> nodeVar15 : vec3<f32>;
var<private> nodeVar16 : vec3<f32>;
var<private> indirectSpecular : vec3<f32>;
var<private> ambientOcclusion : f32;
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
fn main( @location( 0 ) v_normalViewGeometry : vec3<f32>,
	@location( 1 ) v_positionWorld : vec3<f32>,
	@location( 2 ) v_positionViewDirection : vec3<f32>,
	@location( 3 ) nodeVarying7 : vec2<f32> ) -> OutputStruct {

	// flow
	// code

	DiffuseColor = vec4<f32>( object.nodeUniform0, 1.0 );
	nodeVar0 = textureSample( nodeUniform2, nodeUniform2_sampler, ( object.nodeUniform3 * vec3<f32>( nodeVarying7, 1.0 ) ).xy );
	DiffuseColor.w = ( vec4<f32>( DiffuseColor.w ) * ( vec4<f32>( object.nodeUniform1 ) * nodeVar0 ) ).x;
	Metalness = object.nodeUniform4;
	normalViewGeometry = normalize( v_normalViewGeometry );
	let nodeConst0 = max( abs( dpdx( normalViewGeometry ) ), abs( - dpdy( normalViewGeometry ) ) );
	Roughness = min( ( max( object.nodeUniform5, 0.045 ) + max( max( nodeConst0.x, nodeConst0.y ), nodeConst0.z ) ), 1.0 );
	IOR = object.nodeUniform8;
	let nodeConst1 = ( ( IOR - 1.0 ) / ( IOR + 1.0 ) );
	SpecularColor = ( min( ( vec3<f32>( ( nodeConst1 * nodeConst1 ) ) * object.nodeUniform9 ), vec3<f32>( 1.0, 1.0, 1.0 ) ) * vec3<f32>( object.nodeUniform10 ) );
	SpecularColorBlended = mix( SpecularColor, DiffuseColor.xyz, Metalness );
	SpecularF90 = mix( object.nodeUniform10, 1.0, Metalness );
	DiffuseContribution = ( DiffuseColor.xyz * vec3<f32>( ( 1.0 - object.nodeUniform4 ) ) );
	Transmission = object.nodeUniform11;
	Thickness = object.nodeUniform12;
	AttenuationDistance = object.nodeUniform13;
	AttenuationColor = object.nodeUniform14;
	EmissiveColor = ( object.nodeUniform15 * vec3<f32>( object.nodeUniform16 ) );
	NORMAL_normalView = ( normalViewGeometry * vec3<f32>( -1.0 ) );
	normalView = NORMAL_normalView;
	normalWorld = normalize( ( vec4<f32>( normalView, 0.0 ) * render.cameraViewMatrix ).xyz );
	let nodeConst2 = normalize( ( render.cameraPosition - v_positionWorld ) );
	let nodeConst3 = getVolumeTransmissionRay( normalWorld, nodeConst2, Thickness, IOR, object.nodeUniform21 );
	let nodeConst4 = ( render.cameraProjectionMatrix * ( render.cameraViewMatrix * vec4<f32>( ( v_positionWorld + nodeConst3 ), 1.0 ) ) );
	nodeVar1 = ( nodeConst4.xy / vec2<f32>( nodeConst4.w ) );
	nodeVar1 = ( nodeVar1 + vec2<f32>( 1.0 ) );
	nodeVar1 = ( nodeVar1 / vec2<f32>( 2.0 ) );
	nodeVar1 = vec2<f32>( nodeVar1.x, ( 1.0 - nodeVar1.y ) );
	nodeVar2 = textureSample( nodeUniform22, nodeUniform22_sampler, vec2<f32>( Roughness, clamp( dot( normalWorld, nodeConst2 ), 0.0, 1.0 ) ) ).xy;
	let nodeConst5 = ( DiffuseContribution * volumeAttenuation( length( nodeConst3 ), AttenuationColor, AttenuationDistance ) );
	let cameraViewport = vec4<f32>( 0.0, 0.0, render.nodeUniform23.x, render.nodeUniform23.y );
	let nodeConst6 = ( ( ( nodeVar1 * cameraViewport.zw ) + cameraViewport.xy ) / render.nodeUniform23 );
	let nodeConst7 = ( log2( cameraViewport.z ) * applyIorToRoughness( Roughness, IOR ) );
	let nodeConst8 = vec4<f32>( vec2<f32>( textureDimensions( nodeUniform24, i32( nodeConst7 ) ) ), vec2<f32>( textureDimensions( nodeUniform24, i32( ( nodeConst7 + 1.0 ) ) ) ) );
	let nodeConst9 = ( ( vec4<f32>( vec3<f32>( nodeConst6, 0.0 ), 1.0 ).xyxy * nodeConst8 ) + vec4<f32>( 0.5 ) );
	let nodeConst10 = fract( nodeConst9 );
	let nodeConst11 = ( vec4<f32>( 0.16666666666666666 ) * ( ( nodeConst10 * ( nodeConst10 * ( ( vec4<f32>( 3.0 ) * nodeConst10 ) - vec4<f32>( 6.0 ) ) ) ) + vec4<f32>( 4.0 ) ) );
	let nodeConst12 = ( ( vec4<f32>( 0.16666666666666666 ) * ( ( nodeConst10 * ( ( nodeConst10 * ( ( - nodeConst10 ) + vec4<f32>( 3.0 ) ) ) - vec4<f32>( 3.0 ) ) ) + vec4<f32>( 1.0 ) ) ) + nodeConst11 );
	let nodeConst13 = floor( nodeConst9 );
	let nodeConst14 = ( vec4<f32>( 1.0 ) / nodeConst8 );
	let nodeConst15 = ( ( ( nodeConst13 + ( vec4<f32>( -1.0 ) + ( nodeConst11 / nodeConst12 ) ) ) - vec4<f32>( 0.5 ) ) * nodeConst14 );
	let nodeConst16 = floor( nodeConst7 );
	nodeVar3 = textureSampleLevel( nodeUniform24, nodeUniform24_sampler, nodeConst15.xy, nodeConst16 );
	let nodeConst17 = ( vec4<f32>( 0.16666666666666666 ) * pow( nodeConst10, vec4<f32>( 3.0 ) ) );
	let nodeConst18 = ( ( vec4<f32>( 0.16666666666666666 ) * ( ( nodeConst10 * ( ( nodeConst10 * ( ( vec4<f32>( -3.0 ) * nodeConst10 ) + vec4<f32>( 3.0 ) ) ) + vec4<f32>( 3.0 ) ) ) + vec4<f32>( 1.0 ) ) ) + nodeConst17 );
	let nodeConst19 = ( ( ( nodeConst13 + ( vec4<f32>( 1.0 ) + ( nodeConst17 / nodeConst18 ) ) ) - vec4<f32>( 0.5 ) ) * nodeConst14 );
	nodeVar4 = textureSampleLevel( nodeUniform24, nodeUniform24_sampler, vec2<f32>( nodeConst19.xy.x, nodeConst15.xy.y ), nodeConst16 );
	nodeVar5 = textureSampleLevel( nodeUniform24, nodeUniform24_sampler, vec2<f32>( nodeConst15.xy.x, nodeConst19.xy.y ), nodeConst16 );
	nodeVar6 = textureSampleLevel( nodeUniform24, nodeUniform24_sampler, nodeConst19.xy, nodeConst16 );
	let nodeConst20 = ceil( nodeConst7 );
	nodeVar7 = textureSampleLevel( nodeUniform24, nodeUniform24_sampler, nodeConst15.zw, nodeConst20 );
	nodeVar8 = textureSampleLevel( nodeUniform24, nodeUniform24_sampler, vec2<f32>( nodeConst19.zw.x, nodeConst15.zw.y ), nodeConst20 );
	nodeVar9 = textureSampleLevel( nodeUniform24, nodeUniform24_sampler, vec2<f32>( nodeConst15.zw.x, nodeConst19.zw.y ), nodeConst20 );
	nodeVar10 = textureSampleLevel( nodeUniform24, nodeUniform24_sampler, nodeConst19.zw, nodeConst20 );
	let nodeConst21 = mix( ( ( vec4<f32>( nodeConst12.xy.y ) * ( ( vec4<f32>( nodeConst12.xy.x ) * nodeVar3 ) + ( vec4<f32>( nodeConst18.xy.x ) * nodeVar4 ) ) ) + ( vec4<f32>( nodeConst18.xy.y ) * ( ( vec4<f32>( nodeConst12.xy.x ) * nodeVar5 ) + ( vec4<f32>( nodeConst18.xy.x ) * nodeVar6 ) ) ) ), ( ( vec4<f32>( nodeConst12.zw.y ) * ( ( vec4<f32>( nodeConst12.zw.x ) * nodeVar7 ) + ( vec4<f32>( nodeConst18.zw.x ) * nodeVar8 ) ) ) + ( vec4<f32>( nodeConst18.zw.y ) * ( ( vec4<f32>( nodeConst12.zw.x ) * nodeVar9 ) + ( vec4<f32>( nodeConst18.zw.x ) * nodeVar10 ) ) ) ), fract( nodeConst7 ) );
	let nodeConst22 = vec4<f32>( ( ( vec3<f32>( 1.0 ) - ( ( SpecularColorBlended * vec3<f32>( nodeVar2.x ) ) + vec3<f32>( ( SpecularF90 * nodeVar2.y ) ) ) ) * ( nodeConst5 * nodeConst21.xyz ) ), ( 1.0 - ( ( 1.0 - nodeConst21.w ) * ( ( ( nodeConst5.x + nodeConst5.y ) + nodeConst5.z ) / 3.0 ) ) ) );
	DiffuseColor.w = ( DiffuseColor.w * mix( 1.0, nodeConst22.w, Transmission ) );
	positionViewDirection = normalize( v_positionViewDirection );
	nodeVar11 = textureSample( nodeUniform22, nodeUniform22_sampler, vec2<f32>( Roughness, clamp( dot( normalView, positionViewDirection ), 0.0, 1.0 ) ) ).xy;
	let dfg = nodeVar11;
	let multiScatteringCompensation = ( ( SpecularColorBlended * vec3<f32>( ( ( 1.0 / ( dfg.x + dfg.y ) ) - 1.0 ) ) ) + vec3<f32>( 1.0 ) );
	singleScatteringDielectric = vec3<f32>( 0.0, 0.0, 0.0 );
	multiScatteringDielectric = vec3<f32>( 0.0, 0.0, 0.0 );
	let nodeConst23 = ( ( SpecularColor * vec3<f32>( dfg.x ) ) + vec3<f32>( ( SpecularF90 * dfg.y ) ) );
	singleScatteringDielectric = ( singleScatteringDielectric + nodeConst23 );
	let nodeConst24 = ( SpecularColor + ( ( vec3<f32>( 1.0 ) - SpecularColor ) * vec3<f32>( 0.047619 ) ) );
	let nodeConst25 = ( 1.0 - ( dfg.x + dfg.y ) );
	multiScatteringDielectric = ( multiScatteringDielectric + ( ( ( nodeConst23 * nodeConst24 ) / ( vec3<f32>( 1.0 ) - ( vec3<f32>( nodeConst25 ) * nodeConst24 ) ) ) * vec3<f32>( nodeConst25 ) ) );
	radiance = vec3<f32>( 0.0, 0.0, 0.0 );
	let nodeConst26 = normalize( ( render.cameraWorldMatrix * vec4<f32>( normalize( mix( reflect( ( - positionViewDirection ), normalView ), normalView, ( ( ( Roughness * Roughness ) * Roughness ) * Roughness ) ) ), 0.0 ) ).xyz );
	let nodeConst27 = ( object.nodeUniform26 * vec4<f32>( nodeConst26, 1.0 ) );
	let nodeConst28 = clamp( Roughness, 0.0, 1.0 );
	nodeVar12 = textureSampleLevel( nodeUniform25, nodeUniform25_sampler, vec3<f32>( ( - nodeConst27.x ), nodeConst27.yz ), ( ( object.nodeUniform28 * nodeConst28 ) * ( 2.0 - nodeConst28 ) ) );
	radiance = ( radiance + ( nodeVar12.xyz * vec3<f32>( object.nodeUniform29 ) ) );
	iblIrradiance = vec3<f32>( 0.0, 0.0, 0.0 );
	let nodeConst29 = ( object.nodeUniform26 * vec4<f32>( normalWorld, 1.0 ) );
	let nodeConst30 = clamp( 1.0, 0.0, 1.0 );
	nodeVar13 = textureSampleLevel( nodeUniform25, nodeUniform25_sampler, vec3<f32>( ( - nodeConst29.x ), nodeConst29.yz ), ( ( object.nodeUniform28 * nodeConst30 ) * ( 2.0 - nodeConst30 ) ) );
	iblIrradiance = ( iblIrradiance + ( ( nodeVar13.xyz * vec3<f32>( 3.141592653589793 ) ) * vec3<f32>( object.nodeUniform29 ) ) );
	irradiance = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar14 = ( ( irradiance * ( DiffuseContribution * vec3<f32>( 0.3183098861837907 ) ) ) * ( vec3<f32>( 1.0 ) - ( singleScatteringDielectric + multiScatteringDielectric ) ) );
	indirectDiffuse = vec3<f32>( 0.0, 0.0, 0.0 );
	indirectDiffuse = ( indirectDiffuse + nodeVar14 );
	singleScatteringMetallic = vec3<f32>( 0.0, 0.0, 0.0 );
	multiScatteringMetallic = vec3<f32>( 0.0, 0.0, 0.0 );
	let nodeConst31 = ( ( DiffuseColor.xyz * vec3<f32>( dfg.x ) ) + vec3<f32>( ( SpecularF90 * dfg.y ) ) );
	singleScatteringMetallic = ( singleScatteringMetallic + nodeConst31 );
	let nodeConst32 = ( DiffuseColor.xyz + ( ( vec3<f32>( 1.0 ) - DiffuseColor.xyz ) * vec3<f32>( 0.047619 ) ) );
	let nodeConst33 = ( 1.0 - ( dfg.x + dfg.y ) );
	multiScatteringMetallic = ( multiScatteringMetallic + ( ( ( nodeConst31 * nodeConst32 ) / ( vec3<f32>( 1.0 ) - ( vec3<f32>( nodeConst33 ) * nodeConst32 ) ) ) * vec3<f32>( nodeConst33 ) ) );
	let nodeConst34 = ( iblIrradiance * vec3<f32>( 0.3183098861837907 ) );
	nodeVar15 = ( ( radiance * mix( singleScatteringDielectric, singleScatteringMetallic, Metalness ) ) + ( mix( multiScatteringDielectric, multiScatteringMetallic, Metalness ) * nodeConst34 ) );
	nodeVar16 = ( ( DiffuseContribution * ( vec3<f32>( 1.0 ) - ( singleScatteringDielectric + multiScatteringDielectric ) ) ) * nodeConst34 );
	indirectSpecular = vec3<f32>( 0.0, 0.0, 0.0 );
	indirectSpecular = ( indirectSpecular + nodeVar15 );
	indirectDiffuse = ( indirectDiffuse + nodeVar16 );
	ambientOcclusion = 1.0;
	indirectDiffuse = ( indirectDiffuse * vec3<f32>( ambientOcclusion ) );
	indirectSpecular = ( indirectSpecular * vec3<f32>( clamp( ( ambientOcclusion - ( 1.0 - pow( ( clamp( dot( normalView, positionViewDirection ), 0.0, 1.0 ) + ambientOcclusion ), exp2( ( - ( 1.0 - ( Roughness * -16.0 ) ) ) ) ) ) ), 0.0, 1.0 ) ) );
	directDiffuse = vec3<f32>( 0.0, 0.0, 0.0 );
	totalDiffuse = mix( vec4<f32>( ( directDiffuse + indirectDiffuse ), 1.0 ), nodeConst22, Transmission ).xyz;
	directSpecular = vec3<f32>( 0.0, 0.0, 0.0 );
	totalSpecular = ( directSpecular + indirectSpecular );
	outgoingLight = ( totalDiffuse + totalSpecular );
	let nodeConst35 = max( vec4<f32>( ( outgoingLight + EmissiveColor ), DiffuseColor.w ), vec4<f32>( 0.0 ) );
	Output = nodeConst35;

	// result

	output.color = nodeConst35;

	return output;

}
