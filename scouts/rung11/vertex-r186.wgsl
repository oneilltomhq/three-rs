// Three.js r186 - Node System

// directives


// structs


// uniforms
@binding( 1 ) @group( 1 ) var nodeUniform0 : texture_2d<u32>;
@binding( 2 ) @group( 1 ) var nodeUniform1 : texture_2d<f32>;
@binding( 3 ) @group( 1 ) var nodeUniform2 : texture_2d<f32>;

struct renderStruct {
	cameraProjectionMatrix : mat4x4<f32>,
	cameraViewMatrix : mat4x4<f32>
};
@binding( 0 ) @group( 0 )
var<uniform> render : renderStruct;

struct objectStruct {
	nodeUniform3 : vec3<f32>,
	nodeUniform4 : f32,
	nodeUniform6 : mat3x3<f32>,
	nodeUniform8 : mat4x4<f32>
};
@binding( 0 ) @group( 1 )
var<uniform> object : objectStruct;

// varyings

struct VaryingsStruct {
	@location( 0 ) v_normalViewGeometry : vec3<f32>,
	@location( 1 ) @interpolate(flat, either) vBatchIndirectId : u32,
	@location( 2 ) vBatchColor : vec4<f32>,
	@builtin( position ) builtinClipSpace : vec4<f32>
};
var<private> varyings : VaryingsStruct;

// vars
var<private> nodeVar0 : u32;
var<private> nodeVar1 : vec4<f32>;
var<private> nodeVar2 : vec4<f32>;
var<private> nodeVar3 : vec4<f32>;
var<private> nodeVar4 : vec4<f32>;
var<private> nodeVar5 : vec4<f32>;
var<private> nodeVar6 : mat4x4<f32>;
var<private> normalLocal : vec3<f32>;
var<private> modelViewMatrix : mat4x4<f32>;
var<private> VERTEX_nodeVar7 : vec4<f32>;
var<private> positionLocal : vec3<f32>;
var<private> v_modelViewProjection : vec4<f32>;
var<private> v_positionView : vec3<f32>;
var<private> VERTEX_v_modelViewProjection : vec4<f32>;

// codes


@vertex
fn main( @builtin( instance_index ) instanceIndex : u32,
	@location( 0 ) position : vec3<f32>,
	@location( 1 ) normal : vec3<f32> ) -> VaryingsStruct {

	// flow
	// code

	let nodeConst0 = i32( textureDimensions( nodeUniform0, 0 ).x );
	let nodeConst1 = ( i32( instanceIndex ) % nodeConst0 );
	let nodeConst2 = ( i32( instanceIndex ) / nodeConst0 );
	nodeVar0 = textureLoad( nodeUniform0, vec2<i32>( nodeConst1, nodeConst2 ), u32( 0u ) ).x;
	varyings.vBatchIndirectId = nodeVar0;
	let nodeConst3 = i32( textureDimensions( nodeUniform1, 0 ).x );
	let nodeConst4 = i32( ( f32( nodeVar0 ) * 4.0 ) );
	let nodeConst5 = ( nodeConst4 % nodeConst3 );
	let nodeConst6 = ( nodeConst4 / nodeConst3 );
	let nodeConst7 = i32( textureDimensions( nodeUniform2, 0 ).x );
	let nodeConst8 = ( i32( nodeVar0 ) % nodeConst7 );
	let nodeConst9 = ( i32( nodeVar0 ) / nodeConst7 );
	nodeVar1 = textureLoad( nodeUniform2, vec2<i32>( nodeConst8, nodeConst9 ), u32( 0u ) );
	varyings.vBatchColor = nodeVar1;
	positionLocal = position;
	nodeVar2 = textureLoad( nodeUniform1, vec2<i32>( nodeConst5, nodeConst6 ), u32( 0u ) );
	nodeVar3 = textureLoad( nodeUniform1, vec2<i32>( ( nodeConst5 + 1 ), nodeConst6 ), u32( 0u ) );
	nodeVar4 = textureLoad( nodeUniform1, vec2<i32>( ( nodeConst5 + 2 ), nodeConst6 ), u32( 0u ) );
	nodeVar5 = textureLoad( nodeUniform1, vec2<i32>( ( nodeConst5 + 3 ), nodeConst6 ), u32( 0u ) );
	nodeVar6 = mat4x4<f32>( nodeVar2, nodeVar3, nodeVar4, nodeVar5 );
	positionLocal = ( nodeVar6 * vec4<f32>( positionLocal, 1.0 ) ).xyz;
	normalLocal = normal;
	normalLocal = ( mat3x3<f32>( nodeVar6[ 0 ].xyz, nodeVar6[ 1 ].xyz, nodeVar6[ 2 ].xyz ) * ( normalLocal / vec3<f32>( dot( mat3x3<f32>( nodeVar6[ 0 ].xyz, nodeVar6[ 1 ].xyz, nodeVar6[ 2 ].xyz )[ 0u ], mat3x3<f32>( nodeVar6[ 0 ].xyz, nodeVar6[ 1 ].xyz, nodeVar6[ 2 ].xyz )[ 0u ] ), dot( mat3x3<f32>( nodeVar6[ 0 ].xyz, nodeVar6[ 1 ].xyz, nodeVar6[ 2 ].xyz )[ 1u ], mat3x3<f32>( nodeVar6[ 0 ].xyz, nodeVar6[ 1 ].xyz, nodeVar6[ 2 ].xyz )[ 1u ] ), dot( mat3x3<f32>( nodeVar6[ 0 ].xyz, nodeVar6[ 1 ].xyz, nodeVar6[ 2 ].xyz )[ 2u ], mat3x3<f32>( nodeVar6[ 0 ].xyz, nodeVar6[ 1 ].xyz, nodeVar6[ 2 ].xyz )[ 2u ] ) ) ) );
	varyings.v_normalViewGeometry = normalize( ( render.cameraViewMatrix * vec4<f32>( ( object.nodeUniform6 * normalLocal ), 0.0 ) ).xyz );
	modelViewMatrix = ( render.cameraViewMatrix * object.nodeUniform8 );
	v_positionView = ( modelViewMatrix * vec4<f32>( positionLocal, 1.0 ) ).xyz;
	VERTEX_nodeVar7 = ( render.cameraProjectionMatrix * vec4<f32>( v_positionView, 1.0 ) );
	VERTEX_v_modelViewProjection = VERTEX_nodeVar7;

	// result

	varyings.builtinClipSpace = VERTEX_v_modelViewProjection;

	return varyings;

}
