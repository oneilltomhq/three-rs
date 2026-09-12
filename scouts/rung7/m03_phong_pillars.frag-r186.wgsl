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
	nodeUniform2 : f32,
	nodeUniform3 : vec3<f32>,
	nodeUniform4 : vec3<f32>,
	nodeUniform5 : f32,
	nodeUniform8 : mat3x3<f32>,
	nodeUniform10 : mat4x4<f32>
};
@binding( 0 ) @group( 1 )
var<uniform> object : objectStruct;

struct renderStruct {
	cameraProjectionMatrix : mat4x4<f32>,
	cameraViewMatrix : mat4x4<f32>,
	nodeUniform6 : vec3<f32>,
	nodeUniform12 : f32,
	nodeUniform13 : f32,
	nodeUniform17 : f32,
	nodeUniform18 : f32,
	nodeUniform11 : vec3<f32>,
	nodeUniform21 : vec3<f32>,
	nodeUniform9 : vec3<f32>,
	nodeUniform15 : vec3<f32>,
	nodeUniform16 : vec3<f32>,
	nodeUniform19 : vec3<f32>,
	nodeUniform20 : vec3<f32>,
	nodeUniform22 : vec3<f32>,
	nodeUniform23 : f32,
	nodeUniform24 : f32
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
var<private> nodeVar0 : vec3<f32>;
var<private> directDiffuse : vec3<f32>;
var<private> normalViewGeometry : vec3<f32>;
var<private> NORMAL_normalView : vec3<f32>;
var<private> normalView : vec3<f32>;
var<private> nodeVar1 : vec3<f32>;
var<private> nodeVar2 : vec3<f32>;
var<private> nodeVar3 : f32;
var<private> nodeVar4 : vec3<f32>;
var<private> nodeVar5 : vec4<f32>;
var<private> nodeVar6 : vec4<f32>;
var<private> nodeVar7 : vec3<f32>;
var<private> nodeVar8 : vec3<f32>;
var<private> nodeVar9 : f32;
var<private> nodeVar10 : f32;
var<private> nodeVar11 : vec3<f32>;
var<private> nodeVar12 : f32;
var<private> nodeVar13 : f32;
var<private> nodeVar14 : f32;
var<private> nodeVar15 : f32;
var<private> nodeVar16 : vec3<f32>;
var<private> nodeVar17 : vec3<f32>;
var<private> nodeVar18 : vec3<f32>;
var<private> nodeVar19 : vec3<f32>;
var<private> nodeVar20 : vec3<f32>;
var<private> directSpecular : vec3<f32>;
var<private> positionViewDirection : vec3<f32>;
var<private> nodeVar21 : vec3<f32>;
var<private> nodeVar22 : f32;
var<private> nodeVar23 : f32;
var<private> nodeVar24 : vec3<f32>;
var<private> nodeVar25 : vec3<f32>;
var<private> nodeVar26 : vec3<f32>;
var<private> nodeVar27 : vec3<f32>;
var<private> nodeVar28 : vec3<f32>;
var<private> nodeVar29 : vec4<f32>;
var<private> nodeVar30 : vec4<f32>;
var<private> nodeVar31 : vec3<f32>;
var<private> nodeVar32 : vec3<f32>;
var<private> nodeVar33 : f32;
var<private> nodeVar34 : vec3<f32>;
var<private> nodeVar35 : vec3<f32>;
var<private> nodeVar36 : vec3<f32>;
var<private> nodeVar37 : vec3<f32>;
var<private> nodeVar38 : vec3<f32>;
var<private> nodeVar39 : f32;
var<private> nodeVar40 : f32;
var<private> nodeVar41 : vec3<f32>;
var<private> nodeVar42 : vec3<f32>;
var<private> nodeVar43 : vec3<f32>;
var<private> nodeVar44 : vec3<f32>;
var<private> indirectDiffuse : vec3<f32>;
var<private> nodeVar45 : vec4<f32>;
var<private> nodeVar46 : vec4<f32>;
var<private> nodeVar47 : vec4<f32>;
var<private> ambientOcclusion : f32;
var<private> nodeVar48 : vec3<f32>;
var<private> totalDiffuse : vec3<f32>;
var<private> nodeVar49 : vec3<f32>;
var<private> totalSpecular : vec3<f32>;
var<private> indirectSpecular : vec3<f32>;
var<private> nodeVar50 : vec3<f32>;
var<private> outgoingLight : vec3<f32>;
var<private> nodeVar51 : vec3<f32>;
var<private> nodeVar52 : vec4<f32>;

// codes


@fragment
fn main( @location( 0 ) v_positionView : vec3<f32>,
	@location( 1 ) v_positionViewDirection : vec3<f32>,
	@location( 2 ) v_normalViewGeometry : vec3<f32> ) -> OutputStruct {

	// flow
	// code

	DiffuseColor = vec4<f32>( object.nodeUniform0, 1.0 );
	DiffuseColor.w = ( DiffuseColor.w * object.nodeUniform1 );
	DiffuseColor.w = 1.0;
	Shininess = max( object.nodeUniform2, 0.0001 );
	SpecularColor = object.nodeUniform3;
	EmissiveColor = ( object.nodeUniform4 * vec3<f32>( object.nodeUniform5 ) );
	irradiance = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar0 = ( irradiance + render.nodeUniform6 );
	irradiance = nodeVar0;
	directDiffuse = vec3<f32>( 0.0, 0.0, 0.0 );
	normalViewGeometry = normalize( v_normalViewGeometry );
	NORMAL_normalView = normalViewGeometry;
	normalView = NORMAL_normalView;
	nodeVar1 = ( render.nodeUniform9 - v_positionView );
	nodeVar2 = normalize( nodeVar1 );
	nodeVar3 = dot( normalView, nodeVar2 );
	nodeVar4 = ( render.nodeUniform15 - render.nodeUniform16 );
	nodeVar5 = vec4<f32>( nodeVar4, 0.0 );
	nodeVar6 = ( render.cameraViewMatrix * nodeVar5 );
	nodeVar7 = normalize( nodeVar6.xyz );
	nodeVar8 = nodeVar7;
	nodeVar9 = dot( nodeVar2, nodeVar8 );
	nodeVar10 = smoothstep( render.nodeUniform12, render.nodeUniform13, nodeVar9 );
	nodeVar11 = ( render.nodeUniform11 * vec3<f32>( nodeVar10 ) );

	if ( ( render.nodeUniform17 > 0.0 ) ) {

		nodeVar13 = length( nodeVar1 );
		nodeVar14 = ( nodeVar13 / render.nodeUniform17 );
		nodeVar15 = clamp( ( 1.0 - ( ( ( nodeVar14 * nodeVar14 ) * nodeVar14 ) * nodeVar14 ) ), 0.0, 1.0 );
		nodeVar12 = ( ( 1.0 / max( pow( nodeVar13, render.nodeUniform18 ), 0.01 ) ) * ( nodeVar15 * nodeVar15 ) );

	} else {

		nodeVar12 = ( 1.0 / max( pow( length( nodeVar1 ), render.nodeUniform18 ), 0.01 ) );

	}

	nodeVar16 = ( nodeVar11 * vec3<f32>( nodeVar12 ) );
	nodeVar17 = ( vec3<f32>( clamp( nodeVar3, 0.0, 1.0 ) ) * nodeVar16 );
	nodeVar18 = ( DiffuseColor.xyz * vec3<f32>( 0.3183098861837907 ) );
	nodeVar19 = ( nodeVar17 * nodeVar18 );
	nodeVar20 = ( directDiffuse + nodeVar19 );
	directDiffuse = nodeVar20;
	directSpecular = vec3<f32>( 0.0, 0.0, 0.0 );
	positionViewDirection = normalize( v_positionViewDirection );
	nodeVar21 = normalize( ( nodeVar2 + positionViewDirection ) );
	nodeVar22 = clamp( dot( positionViewDirection, nodeVar21 ), 0.0, 1.0 );
	nodeVar23 = exp2( ( ( ( nodeVar22 * -5.55473 ) - 6.98316 ) * nodeVar22 ) );
	nodeVar24 = ( ( ( ( SpecularColor * vec3<f32>( ( 1.0 - nodeVar23 ) ) ) + vec3<f32>( ( 1.0 * nodeVar23 ) ) ) * vec3<f32>( 0.25 ) ) * vec3<f32>( ( ( ( ( Shininess * 0.5 ) + 1.0 ) * 0.3183098861837907 ) * pow( clamp( dot( normalView, nodeVar21 ), 0.0, 1.0 ), Shininess ) ) ) );
	nodeVar25 = ( nodeVar17 * nodeVar24 );
	nodeVar26 = ( nodeVar25 * vec3<f32>( 1.0 ) );
	nodeVar27 = ( directSpecular + nodeVar26 );
	directSpecular = nodeVar27;
	nodeVar28 = ( render.nodeUniform19 - render.nodeUniform20 );
	nodeVar29 = vec4<f32>( nodeVar28, 0.0 );
	nodeVar30 = ( render.cameraViewMatrix * nodeVar29 );
	nodeVar31 = normalize( nodeVar30.xyz );
	nodeVar32 = nodeVar31;
	nodeVar33 = dot( normalView, nodeVar32 );
	nodeVar34 = ( vec3<f32>( clamp( nodeVar33, 0.0, 1.0 ) ) * render.nodeUniform21 );
	nodeVar35 = ( DiffuseColor.xyz * vec3<f32>( 0.3183098861837907 ) );
	nodeVar36 = ( nodeVar34 * nodeVar35 );
	nodeVar37 = ( directDiffuse + nodeVar36 );
	directDiffuse = nodeVar37;
	nodeVar38 = normalize( ( nodeVar32 + positionViewDirection ) );
	nodeVar39 = clamp( dot( positionViewDirection, nodeVar38 ), 0.0, 1.0 );
	nodeVar40 = exp2( ( ( ( nodeVar39 * -5.55473 ) - 6.98316 ) * nodeVar39 ) );
	nodeVar41 = ( ( ( ( SpecularColor * vec3<f32>( ( 1.0 - nodeVar40 ) ) ) + vec3<f32>( ( 1.0 * nodeVar40 ) ) ) * vec3<f32>( 0.25 ) ) * vec3<f32>( ( ( ( ( Shininess * 0.5 ) + 1.0 ) * 0.3183098861837907 ) * pow( clamp( dot( normalView, nodeVar38 ), 0.0, 1.0 ), Shininess ) ) ) );
	nodeVar42 = ( nodeVar34 * nodeVar41 );
	nodeVar43 = ( nodeVar42 * vec3<f32>( 1.0 ) );
	nodeVar44 = ( directSpecular + nodeVar43 );
	directSpecular = nodeVar44;
	indirectDiffuse = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar45 = ( DiffuseColor * vec4<f32>( 0.3183098861837907 ) );
	nodeVar46 = ( vec4<f32>( irradiance, 1.0 ) * nodeVar45 );
	nodeVar47 = ( vec4<f32>( indirectDiffuse, 1.0 ) + nodeVar46 );
	indirectDiffuse = nodeVar47.xyz;
	ambientOcclusion = 1.0;
	nodeVar48 = ( indirectDiffuse * vec3<f32>( ambientOcclusion ) );
	indirectDiffuse = nodeVar48;
	nodeVar49 = ( directDiffuse + indirectDiffuse );
	totalDiffuse = nodeVar49;
	indirectSpecular = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar50 = ( directSpecular + indirectSpecular );
	totalSpecular = nodeVar50;
	nodeVar51 = ( totalDiffuse + totalSpecular );
	outgoingLight = nodeVar51;
	Output = max( vec4<f32>( ( outgoingLight + EmissiveColor ), DiffuseColor.w ), vec4<f32>( 0.0 ) );
	nodeVar52 = vec4<f32>( mix( Output.xyz, render.nodeUniform22, smoothstep( render.nodeUniform23, render.nodeUniform24, ( - v_positionView.z ) ) ), Output.w );
	Output = nodeVar52;

	// result

	output.color = nodeVar52;

	return output;

}
