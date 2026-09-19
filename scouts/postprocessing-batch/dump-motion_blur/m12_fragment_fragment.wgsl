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
@binding( 1 ) @group( 1 ) var nodeUniform1_sampler : sampler;
@binding( 2 ) @group( 1 ) var nodeUniform1 : texture_2d<f32>;

struct objectStruct {
	nodeUniform0 : vec3<f32>,
	nodeUniform2 : mat3x3<f32>,
	nodeUniform3 : f32,
	nodeUniform7 : mat3x3<f32>,
	nodeUniform16 : mat4x4<f32>,
	nodeUniform18 : mat4x4<f32>,
	nodeUniform20 : mat4x4<f32>,
	nodeUniform21 : mat4x4<f32>
};
@binding( 0 ) @group( 1 )
var<uniform> object : objectStruct;

struct renderStruct {
	nodeUniform19 : mat4x4<f32>,
	cameraProjectionMatrix : mat4x4<f32>,
	cameraViewMatrix : mat4x4<f32>,
	nodeUniform5 : vec3<f32>,
	nodeUniform9 : vec3<f32>,
	nodeUniform4 : vec3<f32>,
	nodeUniform11 : vec3<f32>,
	nodeUniform12 : vec3<f32>,
	nodeUniform10 : vec3<f32>,
	nodeUniform13 : vec3<f32>,
	nodeUniform14 : f32,
	nodeUniform15 : f32
};
@binding( 0 ) @group( 0 )
var<uniform> render : renderStruct;

// vars
var<private> DiffuseColor : vec4<f32>;
var<private> nodeVar0 : vec4<f32>;
var<private> Output : vec4<f32>;
var<private> irradiance : vec3<f32>;
var<private> normalViewGeometry : vec3<f32>;
var<private> normalView : vec3<f32>;
var<private> normalWorld : vec3<f32>;
var<private> nodeVar1 : f32;
var<private> nodeVar2 : f32;
var<private> nodeVar3 : f32;
var<private> nodeVar4 : vec3<f32>;
var<private> nodeVar5 : vec3<f32>;
var<private> nodeVar6 : f32;
var<private> nodeVar7 : f32;
var<private> nodeVar8 : f32;
var<private> nodeVar9 : vec3<f32>;
var<private> nodeVar10 : vec3<f32>;
var<private> indirectDiffuse : vec3<f32>;
var<private> nodeVar11 : vec4<f32>;
var<private> ambientOcclusion : f32;
var<private> nodeVar12 : vec3<f32>;
var<private> nodeVar13 : vec3<f32>;
var<private> totalDiffuse : vec3<f32>;
var<private> directDiffuse : vec3<f32>;
var<private> nodeVar14 : vec3<f32>;
var<private> totalSpecular : vec3<f32>;
var<private> directSpecular : vec3<f32>;
var<private> indirectSpecular : vec3<f32>;
var<private> nodeVar15 : vec3<f32>;
var<private> outgoingLight : vec3<f32>;
var<private> nodeVar16 : vec3<f32>;
var<private> nodeVar17 : vec4<f32>;
var<private> modelViewMatrix : mat4x4<f32>;
var<private> nodeVar18 : vec4<f32>;
var<private> nodeVar19 : vec4<f32>;
var<private> nodeVar20 : vec2<f32>;

// codes


@fragment
fn main( @location( 0 ) v_positionView : vec3<f32>,
	@location( 1 ) positionLocal : vec3<f32>,
	@location( 2 ) v_normalViewGeometry : vec3<f32>,
	@location( 3 ) positionPrevious : vec3<f32>,
	@location( 4 ) nodeVarying6 : vec2<f32> ) -> OutputType {

	// flow
	// code

	nodeVar0 = textureSample( nodeUniform1, nodeUniform1_sampler, ( object.nodeUniform2 * vec3<f32>( nodeVarying6, 1.0 ) ).xy );
	DiffuseColor = ( vec4<f32>( object.nodeUniform0, 1.0 ) * nodeVar0 );
	DiffuseColor.w = ( DiffuseColor.w * object.nodeUniform3 );
	DiffuseColor.w = 1.0;
	irradiance = vec3<f32>( 0.0, 0.0, 0.0 );
	normalViewGeometry = normalize( v_normalViewGeometry );
	normalView = normalViewGeometry;
	normalWorld = normalize( ( vec4<f32>( normalView, 0.0 ) * render.cameraViewMatrix ).xyz );
	nodeVar1 = dot( normalWorld, normalize( render.nodeUniform9 ) );
	nodeVar2 = ( nodeVar1 * 0.5 );
	nodeVar3 = ( nodeVar2 + 0.5 );
	nodeVar4 = mix( render.nodeUniform4, render.nodeUniform5, nodeVar3 );
	nodeVar5 = ( irradiance + nodeVar4 );
	irradiance = nodeVar5;
	nodeVar6 = dot( normalWorld, normalize( render.nodeUniform12 ) );
	nodeVar7 = ( nodeVar6 * 0.5 );
	nodeVar8 = ( nodeVar7 + 0.5 );
	nodeVar9 = mix( render.nodeUniform10, render.nodeUniform11, nodeVar8 );
	nodeVar10 = ( irradiance + nodeVar9 );
	irradiance = nodeVar10;
	indirectDiffuse = vec3<f32>( 0.0, 0.0, 0.0 );
	indirectDiffuse = vec4<f32>( 0.0, 0.0, 0.0, 0.0 ).xyz;
	nodeVar11 = ( vec4<f32>( indirectDiffuse, 1.0 ) + vec4<f32>( 1.0, 1.0, 1.0, 0.0 ) );
	indirectDiffuse = nodeVar11.xyz;
	ambientOcclusion = 1.0;
	nodeVar12 = ( indirectDiffuse * vec3<f32>( ambientOcclusion ) );
	indirectDiffuse = nodeVar12;
	nodeVar13 = ( indirectDiffuse * DiffuseColor.xyz );
	indirectDiffuse = nodeVar13;
	directDiffuse = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar14 = ( directDiffuse + indirectDiffuse );
	totalDiffuse = nodeVar14;
	directSpecular = vec3<f32>( 0.0, 0.0, 0.0 );
	indirectSpecular = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar15 = ( directSpecular + indirectSpecular );
	totalSpecular = nodeVar15;
	nodeVar16 = ( totalDiffuse + totalSpecular );
	outgoingLight = nodeVar16;
	Output = max( vec4<f32>( outgoingLight, DiffuseColor.w ), vec4<f32>( 0.0 ) );
	nodeVar17 = vec4<f32>( mix( Output.xyz, render.nodeUniform13, smoothstep( render.nodeUniform14, render.nodeUniform15, ( - v_positionView.z ) ) ), Output.w );
	Output = nodeVar17;
	output.m0 = Output;
	modelViewMatrix = ( render.cameraViewMatrix * object.nodeUniform18 );
	nodeVar18 = ( ( render.cameraProjectionMatrix * modelViewMatrix ) * vec4<f32>( positionLocal, 1.0 ) );
	nodeVar19 = ( ( render.nodeUniform19 * ( object.nodeUniform20 * object.nodeUniform21 ) ) * vec4<f32>( positionPrevious, 1.0 ) );
	nodeVar20 = ( ( nodeVar18.xy / vec2<f32>( nodeVar18.w ) ) - ( nodeVar19.xy / vec2<f32>( nodeVar19.w ) ) );
	output.m1 = vec4<f32>( vec3<f32>( nodeVar20, 0.0 ), 1.0 );

	// result

	return output;

}
