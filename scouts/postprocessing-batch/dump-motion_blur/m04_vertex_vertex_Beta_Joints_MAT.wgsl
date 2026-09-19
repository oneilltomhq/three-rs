// Three.js r186 - Node System

// directives


// structs


// uniforms

struct NodeBuffer_1285Struct {
	value : array< mat4x4<f32>, 67 >
};
@binding( 5 ) @group( 1 )
var<uniform> NodeBuffer_1285 : NodeBuffer_1285Struct;

struct NodeBuffer_3603Struct {
	value : array< mat4x4<f32>, 67 >
};
@binding( 6 ) @group( 1 )
var<uniform> NodeBuffer_3603 : NodeBuffer_3603Struct;

struct objectStruct {
	nodeUniform0 : mat4x4<f32>,
	nodeUniform2 : mat4x4<f32>,
	nodeUniform4 : vec3<f32>,
	nodeUniform5 : f32,
	nodeUniform6 : f32,
	nodeUniform7 : f32,
	nodeUniform9 : mat3x3<f32>,
	nodeUniform10 : vec3<f32>,
	nodeUniform11 : f32,
	nodeUniform13 : mat4x4<f32>,
	nodeUniform40 : mat4x4<f32>,
	nodeUniform42 : mat4x4<f32>,
	nodeUniform43 : mat4x4<f32>
};
@binding( 0 ) @group( 1 )
var<uniform> object : objectStruct;

struct renderStruct {
	nodeUniform41 : mat4x4<f32>,
	cameraProjectionMatrix : mat4x4<f32>,
	cameraViewMatrix : mat4x4<f32>,
	nodeUniform16 : vec3<f32>,
	nodeUniform31 : vec3<f32>,
	nodeUniform32 : vec3<f32>,
	nodeUniform30 : vec3<f32>,
	nodeUniform34 : vec3<f32>,
	nodeUniform35 : vec3<f32>,
	nodeUniform33 : vec3<f32>,
	nodeUniform15 : vec3<f32>,
	nodeUniform36 : vec3<f32>,
	nodeUniform37 : f32,
	nodeUniform38 : f32,
	nodeUniform19 : mat4x4<f32>,
	nodeUniform18 : vec4<f32>,
	nodeUniform25 : mat4x4<f32>,
	nodeUniform24 : vec4<f32>,
	nodeUniform17 : f32,
	nodeUniform20 : f32,
	nodeUniform22 : f32,
	nodeUniform23 : vec2<f32>,
	nodeUniform26 : f32,
	nodeUniform27 : f32,
	nodeUniform28 : vec2<f32>,
	nodeUniform29 : f32
};
@binding( 0 ) @group( 0 )
var<uniform> render : renderStruct;

// varyings

struct VaryingsStruct {
	@location( 0 ) positionPrevious : vec3<f32>,
	@location( 1 ) positionLocal : vec3<f32>,
	@location( 2 ) v_positionView : vec3<f32>,
	@location( 3 ) v_normalViewGeometry : vec3<f32>,
	@location( 4 ) v_positionViewDirection : vec3<f32>,
	@location( 5 ) v_positionWorld : vec3<f32>,
	@builtin( position ) builtinClipSpace : vec4<f32>
};
var<private> varyings : VaryingsStruct;

// vars
var<private> nodeVar0 : vec4<f32>;
var<private> nodeVar1 : vec4<f32>;
var<private> normalLocal : vec3<f32>;
var<private> modelViewMatrix : mat4x4<f32>;
var<private> VERTEX_nodeVar182 : vec4<f32>;
var<private> v_modelViewProjection : vec4<f32>;
var<private> VERTEX_v_modelViewProjection : vec4<f32>;

// codes


@vertex
fn main( @location( 0 ) position : vec3<f32>,
	@location( 1 ) skinIndex : vec4<u32>,
	@location( 2 ) skinWeight : vec4<f32>,
	@location( 3 ) normal : vec3<f32> ) -> VaryingsStruct {

	// flow
	// code

	varyings.positionPrevious = position;
	nodeVar0 = ( object.nodeUniform2 * vec4<f32>( varyings.positionPrevious, 1.0 ) );
	varyings.positionPrevious = ( object.nodeUniform0 * ( ( ( ( ( skinWeight.x * NodeBuffer_1285.value[ skinIndex.x ] ) * nodeVar0 ) + ( ( skinWeight.y * NodeBuffer_1285.value[ skinIndex.y ] ) * nodeVar0 ) ) + ( ( skinWeight.z * NodeBuffer_1285.value[ skinIndex.z ] ) * nodeVar0 ) ) + ( ( skinWeight.w * NodeBuffer_1285.value[ skinIndex.w ] ) * nodeVar0 ) ) ).xyz;
	varyings.positionLocal = position;
	nodeVar1 = ( object.nodeUniform2 * vec4<f32>( varyings.positionLocal, 1.0 ) );
	varyings.positionLocal = ( object.nodeUniform0 * ( ( ( ( ( skinWeight.x * NodeBuffer_3603.value[ skinIndex.x ] ) * nodeVar1 ) + ( ( skinWeight.y * NodeBuffer_3603.value[ skinIndex.y ] ) * nodeVar1 ) ) + ( ( skinWeight.z * NodeBuffer_3603.value[ skinIndex.z ] ) * nodeVar1 ) ) + ( ( skinWeight.w * NodeBuffer_3603.value[ skinIndex.w ] ) * nodeVar1 ) ) ).xyz;
	normalLocal = normal;
	normalLocal = ( mat3x3<f32>( ( ( object.nodeUniform0 * ( ( ( skinWeight.x * NodeBuffer_3603.value[ skinIndex.x ] + skinWeight.y * NodeBuffer_3603.value[ skinIndex.y ] ) + skinWeight.z * NodeBuffer_3603.value[ skinIndex.z ] ) + skinWeight.w * NodeBuffer_3603.value[ skinIndex.w ] ) ) * object.nodeUniform2 )[ 0 ].xyz, ( ( object.nodeUniform0 * ( ( ( skinWeight.x * NodeBuffer_3603.value[ skinIndex.x ] + skinWeight.y * NodeBuffer_3603.value[ skinIndex.y ] ) + skinWeight.z * NodeBuffer_3603.value[ skinIndex.z ] ) + skinWeight.w * NodeBuffer_3603.value[ skinIndex.w ] ) ) * object.nodeUniform2 )[ 1 ].xyz, ( ( object.nodeUniform0 * ( ( ( skinWeight.x * NodeBuffer_3603.value[ skinIndex.x ] + skinWeight.y * NodeBuffer_3603.value[ skinIndex.y ] ) + skinWeight.z * NodeBuffer_3603.value[ skinIndex.z ] ) + skinWeight.w * NodeBuffer_3603.value[ skinIndex.w ] ) ) * object.nodeUniform2 )[ 2 ].xyz ) * normalLocal );
	varyings.v_normalViewGeometry = normalize( ( render.cameraViewMatrix * vec4<f32>( ( object.nodeUniform9 * normalLocal ), 0.0 ) ).xyz );
	modelViewMatrix = ( render.cameraViewMatrix * object.nodeUniform13 );
	varyings.v_positionView = ( modelViewMatrix * vec4<f32>( varyings.positionLocal, 1.0 ) ).xyz;
	varyings.v_positionViewDirection = ( - varyings.v_positionView );
	varyings.v_positionWorld = ( object.nodeUniform13 * vec4<f32>( varyings.positionLocal, 1.0 ) ).xyz;
	VERTEX_nodeVar182 = ( render.cameraProjectionMatrix * vec4<f32>( varyings.v_positionView, 1.0 ) );
	VERTEX_v_modelViewProjection = VERTEX_nodeVar182;

	// result

	varyings.builtinClipSpace = VERTEX_v_modelViewProjection;

	return varyings;

}
