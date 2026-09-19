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

struct objectStruct {
	nodeUniform1 : f32,
	nodeUniform2 : f32,
	nodeUniform3 : vec3<f32>,
	nodeUniform4 : vec3<f32>,
	nodeUniform5 : f32,
	nodeUniform7 : mat3x3<f32>,
	nodeUniform11 : mat4x4<f32>,
	nodeUniform22 : mat4x4<f32>,
	nodeUniform24 : mat4x4<f32>,
	nodeUniform25 : mat4x4<f32>
};
@binding( 2 ) @group( 1 )
var<uniform> object : objectStruct;

struct renderStruct {
	nodeUniform23 : mat4x4<f32>,
	cameraProjectionMatrix : mat4x4<f32>,
	cameraViewMatrix : mat4x4<f32>,
	nodeUniform10 : vec3<f32>,
	nodeUniform13 : vec3<f32>,
	nodeUniform14 : vec3<f32>,
	nodeUniform12 : vec3<f32>,
	nodeUniform16 : vec3<f32>,
	nodeUniform17 : vec3<f32>,
	nodeUniform15 : vec3<f32>,
	nodeUniform9 : vec3<f32>,
	nodeUniform18 : vec3<f32>,
	nodeUniform19 : f32,
	nodeUniform20 : f32
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
var<private> nodeVar6 : vec3<f32>;
var<private> nodeVar7 : vec3<f32>;
var<private> nodeVar8 : vec3<f32>;
var<private> nodeVar9 : vec3<f32>;
var<private> directSpecular : vec3<f32>;
var<private> positionViewDirection : vec3<f32>;
var<private> nodeVar10 : vec3<f32>;
var<private> nodeVar11 : f32;
var<private> nodeVar12 : f32;
var<private> nodeVar13 : vec3<f32>;
var<private> nodeVar14 : vec3<f32>;
var<private> nodeVar15 : vec3<f32>;
var<private> nodeVar16 : vec3<f32>;
var<private> irradiance : vec3<f32>;
var<private> normalWorld : vec3<f32>;
var<private> nodeVar17 : f32;
var<private> nodeVar18 : f32;
var<private> nodeVar19 : f32;
var<private> nodeVar20 : vec3<f32>;
var<private> nodeVar21 : vec3<f32>;
var<private> nodeVar22 : f32;
var<private> nodeVar23 : f32;
var<private> nodeVar24 : f32;
var<private> nodeVar25 : vec3<f32>;
var<private> nodeVar26 : vec3<f32>;
var<private> indirectDiffuse : vec3<f32>;
var<private> nodeVar27 : vec4<f32>;
var<private> nodeVar28 : vec4<f32>;
var<private> nodeVar29 : vec4<f32>;
var<private> ambientOcclusion : f32;
var<private> nodeVar30 : vec3<f32>;
var<private> totalDiffuse : vec3<f32>;
var<private> nodeVar31 : vec3<f32>;
var<private> totalSpecular : vec3<f32>;
var<private> indirectSpecular : vec3<f32>;
var<private> nodeVar32 : vec3<f32>;
var<private> outgoingLight : vec3<f32>;
var<private> nodeVar33 : vec3<f32>;
var<private> nodeVar34 : vec4<f32>;
var<private> modelViewMatrix : mat4x4<f32>;
var<private> nodeVar35 : vec4<f32>;
var<private> nodeVar36 : vec4<f32>;
var<private> nodeVar37 : vec2<f32>;

// codes


@fragment
fn main( @location( 0 ) v_positionView : vec3<f32>,
	@location( 1 ) positionLocal : vec3<f32>,
	@location( 2 ) v_positionViewDirection : vec3<f32>,
	@location( 3 ) positionPrevious : vec3<f32>,
	@location( 4 ) v_normalViewGeometry : vec3<f32>,
	@location( 5 ) nodeVarying7 : vec2<f32> ) -> OutputType {

	// flow
	// code

	nodeVar0 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying7 * vec2<f32>( 5.0 ) ) );
	DiffuseColor = nodeVar0;
	DiffuseColor.w = ( DiffuseColor.w * object.nodeUniform1 );
	DiffuseColor.w = 1.0;
	Shininess = max( object.nodeUniform2, 0.0001 );
	SpecularColor = object.nodeUniform3;
	EmissiveColor = ( object.nodeUniform4 * vec3<f32>( object.nodeUniform5 ) );
	directDiffuse = vec3<f32>( 0.0, 0.0, 0.0 );
	normalViewGeometry = normalize( v_normalViewGeometry );
	NORMAL_normalView = ( normalViewGeometry * vec3<f32>( -1.0 ) );
	normalView = NORMAL_normalView;
	nodeVar1 = vec4<f32>( render.nodeUniform9, 0.0 );
	nodeVar2 = ( render.cameraViewMatrix * nodeVar1 );
	nodeVar3 = normalize( nodeVar2.xyz );
	nodeVar4 = nodeVar3;
	nodeVar5 = dot( normalView, nodeVar4 );
	nodeVar6 = ( vec3<f32>( clamp( nodeVar5, 0.0, 1.0 ) ) * render.nodeUniform10 );
	nodeVar7 = ( DiffuseColor.xyz * vec3<f32>( 0.3183098861837907 ) );
	nodeVar8 = ( nodeVar6 * nodeVar7 );
	nodeVar9 = ( directDiffuse + nodeVar8 );
	directDiffuse = nodeVar9;
	directSpecular = vec3<f32>( 0.0, 0.0, 0.0 );
	positionViewDirection = normalize( v_positionViewDirection );
	nodeVar10 = normalize( ( nodeVar4 + positionViewDirection ) );
	nodeVar11 = clamp( dot( positionViewDirection, nodeVar10 ), 0.0, 1.0 );
	nodeVar12 = exp2( ( ( ( nodeVar11 * -5.55473 ) - 6.98316 ) * nodeVar11 ) );
	nodeVar13 = ( ( ( ( SpecularColor * vec3<f32>( ( 1.0 - nodeVar12 ) ) ) + vec3<f32>( ( 1.0 * nodeVar12 ) ) ) * vec3<f32>( 0.25 ) ) * vec3<f32>( ( ( ( ( Shininess * 0.5 ) + 1.0 ) * 0.3183098861837907 ) * pow( clamp( dot( normalView, nodeVar10 ), 0.0, 1.0 ), Shininess ) ) ) );
	nodeVar14 = ( nodeVar6 * nodeVar13 );
	nodeVar15 = ( nodeVar14 * vec3<f32>( 1.0 ) );
	nodeVar16 = ( directSpecular + nodeVar15 );
	directSpecular = nodeVar16;
	irradiance = vec3<f32>( 0.0, 0.0, 0.0 );
	normalWorld = normalize( ( vec4<f32>( normalView, 0.0 ) * render.cameraViewMatrix ).xyz );
	nodeVar17 = dot( normalWorld, normalize( render.nodeUniform14 ) );
	nodeVar18 = ( nodeVar17 * 0.5 );
	nodeVar19 = ( nodeVar18 + 0.5 );
	nodeVar20 = mix( render.nodeUniform12, render.nodeUniform13, nodeVar19 );
	nodeVar21 = ( irradiance + nodeVar20 );
	irradiance = nodeVar21;
	nodeVar22 = dot( normalWorld, normalize( render.nodeUniform17 ) );
	nodeVar23 = ( nodeVar22 * 0.5 );
	nodeVar24 = ( nodeVar23 + 0.5 );
	nodeVar25 = mix( render.nodeUniform15, render.nodeUniform16, nodeVar24 );
	nodeVar26 = ( irradiance + nodeVar25 );
	irradiance = nodeVar26;
	indirectDiffuse = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar27 = ( DiffuseColor * vec4<f32>( 0.3183098861837907 ) );
	nodeVar28 = ( vec4<f32>( irradiance, 1.0 ) * nodeVar27 );
	nodeVar29 = ( vec4<f32>( indirectDiffuse, 1.0 ) + nodeVar28 );
	indirectDiffuse = nodeVar29.xyz;
	ambientOcclusion = 1.0;
	nodeVar30 = ( indirectDiffuse * vec3<f32>( ambientOcclusion ) );
	indirectDiffuse = nodeVar30;
	nodeVar31 = ( directDiffuse + indirectDiffuse );
	totalDiffuse = nodeVar31;
	indirectSpecular = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar32 = ( directSpecular + indirectSpecular );
	totalSpecular = nodeVar32;
	nodeVar33 = ( totalDiffuse + totalSpecular );
	outgoingLight = nodeVar33;
	Output = max( vec4<f32>( ( outgoingLight + EmissiveColor ), DiffuseColor.w ), vec4<f32>( 0.0 ) );
	nodeVar34 = vec4<f32>( mix( Output.xyz, render.nodeUniform18, smoothstep( render.nodeUniform19, render.nodeUniform20, ( - v_positionView.z ) ) ), Output.w );
	Output = nodeVar34;
	output.m0 = Output;
	modelViewMatrix = ( render.cameraViewMatrix * object.nodeUniform22 );
	nodeVar35 = ( ( render.cameraProjectionMatrix * modelViewMatrix ) * vec4<f32>( positionLocal, 1.0 ) );
	nodeVar36 = ( ( render.nodeUniform23 * ( object.nodeUniform24 * object.nodeUniform25 ) ) * vec4<f32>( positionPrevious, 1.0 ) );
	nodeVar37 = ( ( nodeVar35.xy / vec2<f32>( nodeVar35.w ) ) - ( nodeVar36.xy / vec2<f32>( nodeVar36.w ) ) );
	output.m1 = vec4<f32>( vec3<f32>( nodeVar37, 0.0 ), 1.0 );

	// result

	return output;

}
