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
	nodeUniform0 : f32,
	nodeUniform3 : vec3<f32>,
	nodeUniform4 : f32,
	nodeUniform5 : f32,
	nodeUniform6 : vec3<f32>,
	nodeUniform7 : vec3<f32>,
	nodeUniform8 : f32,
	nodeUniform11 : mat4x4<f32>
};
@binding( 0 ) @group( 1 )
var<uniform> object : objectStruct;

struct renderStruct {
	cameraProjectionMatrix : mat4x4<f32>,
	cameraViewMatrix : mat4x4<f32>,
	nodeUniform9 : vec3<f32>,
	nodeUniform13 : vec3<f32>,
	nodeUniform14 : f32,
	nodeUniform15 : f32,
	nodeUniform12 : vec3<f32>
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
var<private> nodeVar6 : vec3<f32>;
var<private> directDiffuse : vec3<f32>;
var<private> normalFlat : vec3<f32>;
var<private> normalViewGeometry : vec3<f32>;
var<private> NORMAL_normalView : vec3<f32>;
var<private> normalView : vec3<f32>;
var<private> nodeVar7 : vec3<f32>;
var<private> nodeVar8 : vec3<f32>;
var<private> nodeVar9 : f32;
var<private> nodeVar10 : f32;
var<private> nodeVar11 : f32;
var<private> nodeVar12 : f32;
var<private> nodeVar13 : f32;
var<private> nodeVar14 : vec3<f32>;
var<private> nodeVar15 : vec3<f32>;
var<private> nodeVar16 : vec3<f32>;
var<private> nodeVar17 : vec3<f32>;
var<private> nodeVar18 : vec3<f32>;
var<private> directSpecular : vec3<f32>;
var<private> positionViewDirection : vec3<f32>;
var<private> nodeVar19 : vec3<f32>;
var<private> nodeVar20 : f32;
var<private> nodeVar21 : f32;
var<private> nodeVar22 : vec3<f32>;
var<private> nodeVar23 : vec3<f32>;
var<private> nodeVar24 : vec3<f32>;
var<private> nodeVar25 : vec3<f32>;
var<private> indirectDiffuse : vec3<f32>;
var<private> nodeVar26 : vec4<f32>;
var<private> nodeVar27 : vec4<f32>;
var<private> nodeVar28 : vec4<f32>;
var<private> ambientOcclusion : f32;
var<private> nodeVar29 : vec3<f32>;
var<private> totalDiffuse : vec3<f32>;
var<private> nodeVar30 : vec3<f32>;
var<private> totalSpecular : vec3<f32>;
var<private> indirectSpecular : vec3<f32>;
var<private> nodeVar31 : vec3<f32>;
var<private> outgoingLight : vec3<f32>;
var<private> nodeVar32 : vec3<f32>;
var<private> nodeVar33 : vec4<f32>;

// codes


@fragment
fn main( @location( 0 ) v_positionView : vec3<f32>,
	@location( 1 ) v_positionViewDirection : vec3<f32> ) -> OutputStruct {

	// flow
	// code

	DiffuseColor = vec4<f32>( object.nodeUniform3, 1.0 );
	DiffuseColor.w = ( DiffuseColor.w * object.nodeUniform4 );
	DiffuseColor.w = 1.0;
	Shininess = max( object.nodeUniform5, 0.0001 );
	SpecularColor = object.nodeUniform6;
	EmissiveColor = ( object.nodeUniform7 * vec3<f32>( object.nodeUniform8 ) );
	irradiance = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar6 = ( irradiance + render.nodeUniform9 );
	irradiance = nodeVar6;
	directDiffuse = vec3<f32>( 0.0, 0.0, 0.0 );
	normalFlat = normalize( cross( dpdx( v_positionView ), - dpdy( v_positionView ) ) );
	normalViewGeometry = normalFlat;
	NORMAL_normalView = normalViewGeometry;
	normalView = NORMAL_normalView;
	nodeVar7 = ( render.nodeUniform12 - v_positionView );
	nodeVar8 = normalize( nodeVar7 );
	nodeVar9 = dot( normalView, nodeVar8 );

	if ( ( render.nodeUniform14 > 0.0 ) ) {

		nodeVar11 = length( nodeVar7 );
		nodeVar12 = ( nodeVar11 / render.nodeUniform14 );
		nodeVar13 = clamp( ( 1.0 - ( ( ( nodeVar12 * nodeVar12 ) * nodeVar12 ) * nodeVar12 ) ), 0.0, 1.0 );
		nodeVar10 = ( ( 1.0 / max( pow( nodeVar11, render.nodeUniform15 ), 0.01 ) ) * ( nodeVar13 * nodeVar13 ) );

	} else {

		nodeVar10 = ( 1.0 / max( pow( length( nodeVar7 ), render.nodeUniform15 ), 0.01 ) );

	}

	nodeVar14 = ( render.nodeUniform13 * vec3<f32>( nodeVar10 ) );
	nodeVar15 = ( vec3<f32>( clamp( nodeVar9, 0.0, 1.0 ) ) * nodeVar14 );
	nodeVar16 = ( DiffuseColor.xyz * vec3<f32>( 0.3183098861837907 ) );
	nodeVar17 = ( nodeVar15 * nodeVar16 );
	nodeVar18 = ( directDiffuse + nodeVar17 );
	directDiffuse = nodeVar18;
	directSpecular = vec3<f32>( 0.0, 0.0, 0.0 );
	positionViewDirection = normalize( v_positionViewDirection );
	nodeVar19 = normalize( ( nodeVar8 + positionViewDirection ) );
	nodeVar20 = clamp( dot( positionViewDirection, nodeVar19 ), 0.0, 1.0 );
	nodeVar21 = exp2( ( ( ( nodeVar20 * -5.55473 ) - 6.98316 ) * nodeVar20 ) );
	nodeVar22 = ( ( ( ( SpecularColor * vec3<f32>( ( 1.0 - nodeVar21 ) ) ) + vec3<f32>( ( 1.0 * nodeVar21 ) ) ) * vec3<f32>( 0.25 ) ) * vec3<f32>( ( ( ( ( Shininess * 0.5 ) + 1.0 ) * 0.3183098861837907 ) * pow( clamp( dot( normalView, nodeVar19 ), 0.0, 1.0 ), Shininess ) ) ) );
	nodeVar23 = ( nodeVar15 * nodeVar22 );
	nodeVar24 = ( nodeVar23 * vec3<f32>( 1.0 ) );
	nodeVar25 = ( directSpecular + nodeVar24 );
	directSpecular = nodeVar25;
	indirectDiffuse = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar26 = ( DiffuseColor * vec4<f32>( 0.3183098861837907 ) );
	nodeVar27 = ( vec4<f32>( irradiance, 1.0 ) * nodeVar26 );
	nodeVar28 = ( vec4<f32>( indirectDiffuse, 1.0 ) + nodeVar27 );
	indirectDiffuse = nodeVar28.xyz;
	ambientOcclusion = 1.0;
	nodeVar29 = ( indirectDiffuse * vec3<f32>( ambientOcclusion ) );
	indirectDiffuse = nodeVar29;
	nodeVar30 = ( directDiffuse + indirectDiffuse );
	totalDiffuse = nodeVar30;
	indirectSpecular = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar31 = ( directSpecular + indirectSpecular );
	totalSpecular = nodeVar31;
	nodeVar32 = ( totalDiffuse + totalSpecular );
	outgoingLight = nodeVar32;
	nodeVar33 = max( vec4<f32>( ( outgoingLight + EmissiveColor ), DiffuseColor.w ), vec4<f32>( 0.0 ) );
	Output = nodeVar33;

	// result

	output.color = nodeVar33;

	return output;

}
