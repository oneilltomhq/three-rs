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
@binding( 1 ) @group( 1 ) var nodeUniform19_sampler : sampler_comparison;
@binding( 2 ) @group( 1 ) var nodeUniform19 : texture_depth_2d;
@binding( 3 ) @group( 1 ) var nodeUniform23_sampler : sampler;
@binding( 4 ) @group( 1 ) var nodeUniform23 : texture_2d<f32>;

struct objectStruct {
	nodeUniform0 : vec3<f32>,
	nodeUniform1 : f32,
	nodeUniform2 : f32,
	nodeUniform3 : vec3<f32>,
	nodeUniform4 : vec3<f32>,
	nodeUniform5 : f32,
	nodeUniform9 : mat3x3<f32>,
	nodeUniform15 : mat4x4<f32>
};
@binding( 0 ) @group( 1 )
var<uniform> object : objectStruct;

struct renderStruct {
	cameraProjectionMatrix : mat4x4<f32>,
	cameraViewMatrix : mat4x4<f32>,
	nodeUniform7 : vec3<f32>,
	nodeUniform11 : vec3<f32>,
	nodeUniform6 : vec3<f32>,
	nodeUniform14 : vec3<f32>,
	nodeUniform12 : vec3<f32>,
	nodeUniform13 : vec3<f32>,
	nodeUniform16 : mat4x4<f32>,
	nodeUniform17 : f32,
	nodeUniform18 : f32,
	nodeUniform22 : f32,
	nodeUniform25 : vec3<f32>,
	nodeUniform26 : f32,
	nodeUniform27 : f32,
	nodeUniform20 : f32,
	nodeUniform21 : vec2<f32>,
	nodeUniform24 : vec2<f32>
};
@binding( 0 ) @group( 0 )
var<uniform> render : renderStruct;

// vars
var<private> DiffuseColor : vec4<f32>;
var<private> Shininess : f32;
var<private> SpecularColor : vec3<f32>;
var<private> EmissiveColor : vec3<f32>;
var<private> Output : vec4<f32>;
var<private> irradiance : vec3<f32>;
var<private> normalViewGeometry : vec3<f32>;
var<private> NORMAL_normalView : vec3<f32>;
var<private> normalView : vec3<f32>;
var<private> normalWorld : vec3<f32>;
var<private> directDiffuse : vec3<f32>;
var<private> shadowPositionWorld : vec3<f32>;
var<private> nodeVar0 : f32;
var<private> nodeVar1 : f32;
var<private> nodeVar2 : f32;
var<private> nodeVar3 : f32;
var<private> nodeVar4 : f32;
var<private> nodeVar5 : f32;
var<private> nodeVar6 : f32;
var<private> nodeVar7 : f32;
var<private> directSpecular : vec3<f32>;
var<private> positionViewDirection : vec3<f32>;
var<private> indirectDiffuse : vec3<f32>;
var<private> ambientOcclusion : f32;
var<private> totalDiffuse : vec3<f32>;
var<private> totalSpecular : vec3<f32>;
var<private> indirectSpecular : vec3<f32>;
var<private> outgoingLight : vec3<f32>;
var<private> nodeVar8 : vec4<f32>;

// codes
fn interleavedGradientNoise ( position : vec2<f32> ) -> f32 {

	


	return fract( ( 52.9829189 * fract( dot( position, vec2<f32>( 0.06711056, 0.00583715 ) ) ) ) );

}


fn vogelDiskSample ( sampleIndex : i32, samplesCount : i32, phi : f32 ) -> vec2<f32> {

	

	let nodeConst0 = ( ( f32( sampleIndex ) * 2.399963229728653 ) + phi );

	return ( vec2<f32>( cos( nodeConst0 ), sin( nodeConst0 ) ) * vec2<f32>( sqrt( ( ( f32( sampleIndex ) + 0.5 ) / f32( samplesCount ) ) ) ) );

}




@fragment
fn main( @location( 0 ) v_positionView : vec3<f32>,
	@location( 1 ) v_positionWorld : vec3<f32>,
	@location( 2 ) v_positionViewDirection : vec3<f32>,
	@location( 3 ) v_normalViewGeometry : vec3<f32>,
	@builtin( position ) fragCoord : vec4<f32> ) -> OutputStruct {

	// flow
	// code

	DiffuseColor = vec4<f32>( object.nodeUniform0, 1.0 );
	DiffuseColor.w = ( DiffuseColor.w * object.nodeUniform1 );
	DiffuseColor.w = 1.0;
	Shininess = max( object.nodeUniform2, 0.0001 );
	SpecularColor = object.nodeUniform3;
	EmissiveColor = ( object.nodeUniform4 * vec3<f32>( object.nodeUniform5 ) );
	irradiance = vec3<f32>( 0.0, 0.0, 0.0 );
	normalViewGeometry = normalize( v_normalViewGeometry );
	NORMAL_normalView = normalViewGeometry;
	normalView = NORMAL_normalView;
	normalWorld = normalize( ( vec4<f32>( normalView, 0.0 ) * render.cameraViewMatrix ).xyz );
	irradiance = ( irradiance + mix( render.nodeUniform6, render.nodeUniform7, ( ( dot( normalWorld, normalize( render.nodeUniform11 ) ) * 0.5 ) + 0.5 ) ) );
	directDiffuse = vec3<f32>( 0.0, 0.0, 0.0 );
	let nodeConst0 = normalize( ( render.cameraViewMatrix * vec4<f32>( ( render.nodeUniform12 - render.nodeUniform13 ), 0.0 ) ).xyz );
	shadowPositionWorld = v_positionWorld;
	let nodeConst1 = ( render.nodeUniform16 * vec4<f32>( ( shadowPositionWorld + ( normalWorld * vec3<f32>( render.nodeUniform17 ) ) ), 1.0 ) );
	let nodeConst2 = ( nodeConst1.xyz / vec3<f32>( nodeConst1.w ) );
	let nodeConst3 = vec3<f32>( nodeConst2.x, ( 1.0 - nodeConst2.y ), ( nodeConst2.z + render.nodeUniform18 ) );

	if ( ( ( ( ( ( nodeConst3.x >= 0.0 ) && ( nodeConst3.x <= 1.0 ) ) && ( nodeConst3.y >= 0.0 ) ) && ( nodeConst3.y <= 1.0 ) ) && ( nodeConst3.z <= 1.0 ) ) ) {

		let nodeConst4 = ( interleavedGradientNoise( fragCoord.xy ) * 6.28318530718 );
		let nodeConst5 = ( render.nodeUniform20 * ( vec2<f32>( 1.0, 1.0 ) / render.nodeUniform21 ).x );
		let nodeConst6 = ( nodeConst3.xy + ( vogelDiskSample( 0, 5, nodeConst4 ) * vec2<f32>( nodeConst5 ) ) );
		nodeVar1 = textureSampleCompare( nodeUniform19, nodeUniform19_sampler, nodeConst6, nodeConst3.z );
		let nodeConst7 = ( nodeConst3.xy + ( vogelDiskSample( 1, 5, nodeConst4 ) * vec2<f32>( nodeConst5 ) ) );
		nodeVar2 = textureSampleCompare( nodeUniform19, nodeUniform19_sampler, nodeConst7, nodeConst3.z );
		let nodeConst8 = ( nodeConst3.xy + ( vogelDiskSample( 2, 5, nodeConst4 ) * vec2<f32>( nodeConst5 ) ) );
		nodeVar3 = textureSampleCompare( nodeUniform19, nodeUniform19_sampler, nodeConst8, nodeConst3.z );
		let nodeConst9 = ( nodeConst3.xy + ( vogelDiskSample( 3, 5, nodeConst4 ) * vec2<f32>( nodeConst5 ) ) );
		nodeVar4 = textureSampleCompare( nodeUniform19, nodeUniform19_sampler, nodeConst9, nodeConst3.z );
		let nodeConst10 = ( nodeConst3.xy + ( vogelDiskSample( 4, 5, nodeConst4 ) * vec2<f32>( nodeConst5 ) ) );
		nodeVar5 = textureSampleCompare( nodeUniform19, nodeUniform19_sampler, nodeConst10, nodeConst3.z );
		nodeVar0 = ( ( ( ( ( nodeVar1 + nodeVar2 ) + nodeVar3 ) + nodeVar4 ) + nodeVar5 ) * 0.2 );

	} else {

		nodeVar0 = 1.0;

	}

	nodeVar6 = mix( 1.0, nodeVar0, render.nodeUniform22 );
	nodeVar7 = textureSample( nodeUniform23, nodeUniform23_sampler, ( fragCoord.xy / render.nodeUniform24 ) ).x;
	let nodeConst11 = ( vec3<f32>( clamp( dot( normalView, nodeConst0 ), 0.0, 1.0 ) ) * ( ( render.nodeUniform14 * vec3<f32>( nodeVar6 ) ) * vec3<f32>( nodeVar7 ) ) );
	directDiffuse = ( directDiffuse + ( nodeConst11 * ( DiffuseColor.xyz * vec3<f32>( 0.3183098861837907 ) ) ) );
	directSpecular = vec3<f32>( 0.0, 0.0, 0.0 );
	positionViewDirection = normalize( v_positionViewDirection );
	let nodeConst12 = normalize( ( nodeConst0 + positionViewDirection ) );
	let nodeConst13 = clamp( dot( positionViewDirection, nodeConst12 ), 0.0, 1.0 );
	let nodeConst14 = exp2( ( ( ( nodeConst13 * -5.55473 ) - 6.98316 ) * nodeConst13 ) );
	directSpecular = ( directSpecular + ( ( nodeConst11 * ( ( ( ( SpecularColor * vec3<f32>( ( 1.0 - nodeConst14 ) ) ) + vec3<f32>( ( 1.0 * nodeConst14 ) ) ) * vec3<f32>( 0.25 ) ) * vec3<f32>( ( ( ( ( Shininess * 0.5 ) + 1.0 ) * 0.3183098861837907 ) * pow( clamp( dot( normalView, nodeConst12 ), 0.0, 1.0 ), Shininess ) ) ) ) ) * vec3<f32>( 1.0 ) ) );
	indirectDiffuse = vec3<f32>( 0.0, 0.0, 0.0 );
	indirectDiffuse = ( vec4<f32>( indirectDiffuse, 1.0 ) + ( vec4<f32>( irradiance, 1.0 ) * ( DiffuseColor * vec4<f32>( 0.3183098861837907 ) ) ) ).xyz;
	ambientOcclusion = 1.0;
	indirectDiffuse = ( indirectDiffuse * vec3<f32>( ambientOcclusion ) );
	totalDiffuse = ( directDiffuse + indirectDiffuse );
	indirectSpecular = vec3<f32>( 0.0, 0.0, 0.0 );
	totalSpecular = ( directSpecular + indirectSpecular );
	outgoingLight = ( totalDiffuse + totalSpecular );
	Output = max( vec4<f32>( ( outgoingLight + EmissiveColor ), DiffuseColor.w ), vec4<f32>( 0.0 ) );
	nodeVar8 = vec4<f32>( mix( Output.xyz, render.nodeUniform25, smoothstep( render.nodeUniform26, render.nodeUniform27, ( - v_positionView.z ) ) ), Output.w );
	let nodeConst15 = nodeVar8;
	Output = nodeConst15;

	// result

	output.color = nodeConst15;

	return output;

}
