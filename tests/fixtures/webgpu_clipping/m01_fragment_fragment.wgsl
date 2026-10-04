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

struct NodeBuffer_1013Struct {
	value : array< vec4<f32>, 2 >
};
@binding( 0 ) @group( 0 )
var<uniform> NodeBuffer_1013 : NodeBuffer_1013Struct;

struct objectStruct {
	nodeUniform1 : mat4x4<f32>,
	nodeUniform2 : vec3<f32>,
	nodeUniform3 : f32,
	nodeUniform4 : f32,
	nodeUniform5 : vec3<f32>,
	nodeUniform6 : vec3<f32>,
	nodeUniform7 : f32,
	nodeUniform10 : mat3x3<f32>
};
@binding( 0 ) @group( 1 )
var<uniform> object : objectStruct;

struct renderStruct {
	cameraViewMatrix : mat4x4<f32>,
	cameraProjectionMatrix : mat4x4<f32>,
	nodeUniform9 : vec3<f32>,
	nodeUniform13 : f32,
	nodeUniform14 : f32,
	nodeUniform18 : f32,
	nodeUniform19 : f32,
	nodeUniform12 : vec3<f32>,
	nodeUniform22 : vec3<f32>,
	nodeUniform11 : vec3<f32>,
	nodeUniform16 : vec3<f32>,
	nodeUniform17 : vec3<f32>,
	nodeUniform20 : vec3<f32>,
	nodeUniform21 : vec3<f32>
};
@binding( 1 ) @group( 0 )
var<uniform> render : renderStruct;

// vars
var<private> DiffuseColor : vec4<f32>;
var<private> Shininess : f32;
var<private> SpecularColor : vec3<f32>;
var<private> EmissiveColor : vec3<f32>;
var<private> distanceToPlane : f32;
var<private> distanceToGradient : f32;
var<private> clipOpacity : f32;
var<private> intersectionClipOpacity : f32;
var<private> Output : vec4<f32>;
var<private> irradiance : vec3<f32>;
var<private> directDiffuse : vec3<f32>;
var<private> normalViewGeometry : vec3<f32>;
var<private> NORMAL_normalView : vec3<f32>;
var<private> normalView : vec3<f32>;
var<private> nodeVar0 : f32;
var<private> directSpecular : vec3<f32>;
var<private> positionViewDirection : vec3<f32>;
var<private> indirectDiffuse : vec3<f32>;
var<private> ambientOcclusion : f32;
var<private> totalDiffuse : vec3<f32>;
var<private> totalSpecular : vec3<f32>;
var<private> indirectSpecular : vec3<f32>;
var<private> outgoingLight : vec3<f32>;

// codes


@fragment
fn main( @location( 0 ) v_positionView : vec3<f32>,
	@location( 1 ) v_positionViewDirection : vec3<f32>,
	@location( 2 ) v_normalViewGeometry : vec3<f32>,
	@builtin( front_facing ) isFront : bool ) -> OutputStruct {

	// flow
	// code

	DiffuseColor = vec4<f32>( object.nodeUniform2, 1.0 );
	DiffuseColor.w = ( DiffuseColor.w * object.nodeUniform3 );
	Shininess = max( object.nodeUniform4, 0.0001 );
	SpecularColor = object.nodeUniform5;
	EmissiveColor = ( object.nodeUniform6 * vec3<f32>( object.nodeUniform7 ) );
	distanceToPlane = 0.0;
	distanceToGradient = 0.0;
	clipOpacity = 1.0;
	intersectionClipOpacity = 1.0;

	for ( var i : i32 = 0; i < 2; i ++ ) {

		distanceToPlane = ( ( - dot( v_positionView, NodeBuffer_1013.value[ i ].xyz ) ) + NodeBuffer_1013.value[ i ].w );
		distanceToGradient = ( fwidth( distanceToPlane ) / 2.0 );
		intersectionClipOpacity = ( intersectionClipOpacity * ( 1.0 - smoothstep( ( - distanceToGradient ), distanceToGradient, distanceToPlane ) ) );

	}

	clipOpacity = ( clipOpacity * ( 1.0 - intersectionClipOpacity ) );
	DiffuseColor.w = ( DiffuseColor.w * clipOpacity );

	if ( ( DiffuseColor.w == 0.0 ) ) {

		discard;
		

	}

	irradiance = vec3<f32>( 0.0, 0.0, 0.0 );
	irradiance = ( irradiance + render.nodeUniform9 );
	directDiffuse = vec3<f32>( 0.0, 0.0, 0.0 );
	normalViewGeometry = normalize( v_normalViewGeometry );
	NORMAL_normalView = ( normalViewGeometry * vec3<f32>( ( ( f32( isFront ) * 2.0 ) - 1.0 ) ) );
	normalView = NORMAL_normalView;
	let nodeConst0 = ( render.nodeUniform11 - v_positionView );
	let nodeConst1 = normalize( nodeConst0 );

	if ( ( render.nodeUniform18 > 0.0 ) ) {

		let nodeConst2 = length( nodeConst0 );
		let nodeConst3 = ( nodeConst2 / render.nodeUniform18 );
		let nodeConst4 = clamp( ( 1.0 - ( ( ( nodeConst3 * nodeConst3 ) * nodeConst3 ) * nodeConst3 ) ), 0.0, 1.0 );
		nodeVar0 = ( ( 1.0 / max( pow( nodeConst2, render.nodeUniform19 ), 0.01 ) ) * ( nodeConst4 * nodeConst4 ) );

	} else {

		nodeVar0 = ( 1.0 / max( pow( length( nodeConst0 ), render.nodeUniform19 ), 0.01 ) );

	}

	let nodeConst5 = ( vec3<f32>( clamp( dot( normalView, nodeConst1 ), 0.0, 1.0 ) ) * ( ( render.nodeUniform12 * vec3<f32>( smoothstep( render.nodeUniform13, render.nodeUniform14, dot( nodeConst1, normalize( ( render.cameraViewMatrix * vec4<f32>( ( render.nodeUniform16 - render.nodeUniform17 ), 0.0 ) ).xyz ) ) ) ) ) * vec3<f32>( nodeVar0 ) ) );
	directDiffuse = ( directDiffuse + ( nodeConst5 * ( DiffuseColor.xyz * vec3<f32>( 0.3183098861837907 ) ) ) );
	directSpecular = vec3<f32>( 0.0, 0.0, 0.0 );
	positionViewDirection = normalize( v_positionViewDirection );
	let nodeConst6 = normalize( ( nodeConst1 + positionViewDirection ) );
	let nodeConst7 = clamp( dot( positionViewDirection, nodeConst6 ), 0.0, 1.0 );
	let nodeConst8 = exp2( ( ( ( nodeConst7 * -5.55473 ) - 6.98316 ) * nodeConst7 ) );
	directSpecular = ( directSpecular + ( ( nodeConst5 * ( ( ( ( SpecularColor * vec3<f32>( ( 1.0 - nodeConst8 ) ) ) + vec3<f32>( ( 1.0 * nodeConst8 ) ) ) * vec3<f32>( 0.25 ) ) * vec3<f32>( ( ( ( ( Shininess * 0.5 ) + 1.0 ) * 0.3183098861837907 ) * pow( clamp( dot( normalView, nodeConst6 ), 0.0, 1.0 ), Shininess ) ) ) ) ) * vec3<f32>( 1.0 ) ) );
	let nodeConst9 = normalize( ( render.cameraViewMatrix * vec4<f32>( ( render.nodeUniform20 - render.nodeUniform21 ), 0.0 ) ).xyz );
	let nodeConst10 = ( vec3<f32>( clamp( dot( normalView, nodeConst9 ), 0.0, 1.0 ) ) * render.nodeUniform22 );
	directDiffuse = ( directDiffuse + ( nodeConst10 * ( DiffuseColor.xyz * vec3<f32>( 0.3183098861837907 ) ) ) );
	let nodeConst11 = normalize( ( nodeConst9 + positionViewDirection ) );
	let nodeConst12 = clamp( dot( positionViewDirection, nodeConst11 ), 0.0, 1.0 );
	let nodeConst13 = exp2( ( ( ( nodeConst12 * -5.55473 ) - 6.98316 ) * nodeConst12 ) );
	directSpecular = ( directSpecular + ( ( nodeConst10 * ( ( ( ( SpecularColor * vec3<f32>( ( 1.0 - nodeConst13 ) ) ) + vec3<f32>( ( 1.0 * nodeConst13 ) ) ) * vec3<f32>( 0.25 ) ) * vec3<f32>( ( ( ( ( Shininess * 0.5 ) + 1.0 ) * 0.3183098861837907 ) * pow( clamp( dot( normalView, nodeConst11 ), 0.0, 1.0 ), Shininess ) ) ) ) ) * vec3<f32>( 1.0 ) ) );
	indirectDiffuse = vec3<f32>( 0.0, 0.0, 0.0 );
	indirectDiffuse = ( vec4<f32>( indirectDiffuse, 1.0 ) + ( vec4<f32>( irradiance, 1.0 ) * ( DiffuseColor * vec4<f32>( 0.3183098861837907 ) ) ) ).xyz;
	ambientOcclusion = 1.0;
	indirectDiffuse = ( indirectDiffuse * vec3<f32>( ambientOcclusion ) );
	totalDiffuse = ( directDiffuse + indirectDiffuse );
	indirectSpecular = vec3<f32>( 0.0, 0.0, 0.0 );
	totalSpecular = ( directSpecular + indirectSpecular );
	outgoingLight = ( totalDiffuse + totalSpecular );
	let nodeConst14 = max( vec4<f32>( ( outgoingLight + EmissiveColor ), DiffuseColor.w ), vec4<f32>( 0.0 ) );
	Output = nodeConst14;

	// result

	output.color = nodeConst14;

	return output;

}
