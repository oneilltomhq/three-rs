// Three.js r186 - Node System

// global
diagnostic( off, derivative_uniformity );


// structs

struct OutputStruct {
	@location( 0 ) color: vec4<f32>
};
var<private> output : OutputStruct;

// uniforms

struct objectStruct {
	nodeUniform0 : vec3<f32>,
	nodeUniform1 : f32,
	nodeUniform2 : vec3<f32>,
	nodeUniform3 : f32,
	nodeUniform5 : mat3x3<f32>,
	nodeUniform7 : mat4x4<f32>
};
@binding( 0 ) @group( 1 )
var<uniform> object : objectStruct;

struct renderStruct {
	cameraProjectionMatrix : mat4x4<f32>,
	cameraViewMatrix : mat4x4<f32>,
	nodeUniform8 : vec3<f32>,
	nodeUniform9 : f32,
	nodeUniform10 : f32,
	nodeUniform6 : vec3<f32>
};
@binding( 0 ) @group( 0 )
var<uniform> render : renderStruct;

// vars
var<private> DiffuseColor : vec4<f32>;
var<private> EmissiveColor : vec3<f32>;
var<private> Output : vec4<f32>;
var<private> directDiffuse : vec3<f32>;
var<private> normalViewGeometry : vec3<f32>;
var<private> NORMAL_normalView : vec3<f32>;
var<private> normalView : vec3<f32>;
var<private> nodeVar0 : vec3<f32>;
var<private> nodeVar1 : vec3<f32>;
var<private> nodeVar2 : f32;
var<private> nodeVar3 : f32;
var<private> nodeVar4 : f32;
var<private> nodeVar5 : f32;
var<private> nodeVar6 : f32;
var<private> nodeVar7 : vec3<f32>;
var<private> nodeVar8 : vec3<f32>;
var<private> nodeVar9 : vec3<f32>;
var<private> nodeVar10 : vec3<f32>;
var<private> nodeVar11 : vec3<f32>;
var<private> indirectDiffuse : vec3<f32>;
var<private> irradiance : vec3<f32>;
var<private> nodeVar12 : vec4<f32>;
var<private> nodeVar13 : vec4<f32>;
var<private> nodeVar14 : vec4<f32>;
var<private> ambientOcclusion : f32;
var<private> nodeVar15 : vec3<f32>;
var<private> totalDiffuse : vec3<f32>;
var<private> nodeVar16 : vec3<f32>;
var<private> totalSpecular : vec3<f32>;
var<private> directSpecular : vec3<f32>;
var<private> indirectSpecular : vec3<f32>;
var<private> nodeVar17 : vec3<f32>;
var<private> outgoingLight : vec3<f32>;
var<private> nodeVar18 : vec3<f32>;
var<private> nodeVar19 : vec4<f32>;

// codes


@fragment
fn main( @location( 0 ) v_positionView : vec3<f32>,
	@location( 1 ) v_normalViewGeometry : vec3<f32> ) -> OutputStruct {

	// flow
	// code

	DiffuseColor = vec4<f32>( object.nodeUniform0, 1.0 );
	DiffuseColor.w = ( DiffuseColor.w * object.nodeUniform1 );
	DiffuseColor.w = 1.0;
	EmissiveColor = ( object.nodeUniform2 * vec3<f32>( object.nodeUniform3 ) );
	directDiffuse = vec3<f32>( 0.0, 0.0, 0.0 );
	normalViewGeometry = normalize( v_normalViewGeometry );
	NORMAL_normalView = normalViewGeometry;
	normalView = NORMAL_normalView;
	nodeVar0 = ( render.nodeUniform6 - v_positionView );
	nodeVar1 = normalize( nodeVar0 );
	nodeVar2 = dot( normalView, nodeVar1 );

	if ( ( render.nodeUniform9 > 0.0 ) ) {

		nodeVar4 = length( nodeVar0 );
		nodeVar5 = ( nodeVar4 / render.nodeUniform9 );
		nodeVar6 = clamp( ( 1.0 - ( ( ( nodeVar5 * nodeVar5 ) * nodeVar5 ) * nodeVar5 ) ), 0.0, 1.0 );
		nodeVar3 = ( ( 1.0 / max( pow( nodeVar4, render.nodeUniform10 ), 0.01 ) ) * ( nodeVar6 * nodeVar6 ) );

	} else {

		nodeVar3 = ( 1.0 / max( pow( length( nodeVar0 ), render.nodeUniform10 ), 0.01 ) );

	}

	nodeVar7 = ( render.nodeUniform8 * vec3<f32>( nodeVar3 ) );
	nodeVar8 = ( vec3<f32>( clamp( nodeVar2, 0.0, 1.0 ) ) * nodeVar7 );
	nodeVar9 = ( DiffuseColor.xyz * vec3<f32>( 0.3183098861837907 ) );
	nodeVar10 = ( nodeVar8 * nodeVar9 );
	nodeVar11 = ( directDiffuse + nodeVar10 );
	directDiffuse = nodeVar11;
	indirectDiffuse = vec3<f32>( 0.0, 0.0, 0.0 );
	irradiance = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar12 = ( DiffuseColor * vec4<f32>( 0.3183098861837907 ) );
	nodeVar13 = ( vec4<f32>( irradiance, 1.0 ) * nodeVar12 );
	nodeVar14 = ( vec4<f32>( indirectDiffuse, 1.0 ) + nodeVar13 );
	indirectDiffuse = nodeVar14.xyz;
	ambientOcclusion = 1.0;
	nodeVar15 = ( indirectDiffuse * vec3<f32>( ambientOcclusion ) );
	indirectDiffuse = nodeVar15;
	nodeVar16 = ( directDiffuse + indirectDiffuse );
	totalDiffuse = nodeVar16;
	directSpecular = vec3<f32>( 0.0, 0.0, 0.0 );
	indirectSpecular = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar17 = ( directSpecular + indirectSpecular );
	totalSpecular = nodeVar17;
	nodeVar18 = ( totalDiffuse + totalSpecular );
	outgoingLight = nodeVar18;
	nodeVar19 = max( vec4<f32>( ( outgoingLight + EmissiveColor ), DiffuseColor.w ), vec4<f32>( 0.0 ) );
	Output = nodeVar19;

	// result

	output.color = nodeVar19;

	return output;

}
