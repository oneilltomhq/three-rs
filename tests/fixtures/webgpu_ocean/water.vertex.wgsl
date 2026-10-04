// Three.js r187dev - Node System

// directives


// structs


// uniforms

struct objectStruct {
	nodeUniform0 : vec3<f32>,
	nodeUniform1 : vec3<f32>,
	nodeUniform2 : mat4x4<f32>,
	nodeUniform3 : f32,
	nodeUniform6 : mat3x3<f32>,
	nodeUniform8 : vec3<f32>,
	nodeUniform11 : f32,
	nodeUniform12 : f32
};
@binding( 0 ) @group( 1 )
var<uniform> object : objectStruct;

struct renderStruct {
	nodeUniform4 : f32,
	cameraProjectionMatrix : mat4x4<f32>,
	cameraViewMatrix : mat4x4<f32>,
	cameraPosition : vec3<f32>,
	nodeUniform10 : vec2<f32>
};
@binding( 0 ) @group( 0 )
var<uniform> render : renderStruct;

// varyings

struct VaryingsStruct {
	@location( 0 ) v_positionWorld : vec3<f32>,
	@builtin( position ) builtinClipSpace : vec4<f32>
};
var<private> varyings : VaryingsStruct;

// vars
var<private> modelViewMatrix : mat4x4<f32>;
var<private> v_modelViewProjection : vec4<f32>;
var<private> v_positionView : vec3<f32>;
var<private> positionLocal : vec3<f32>;
var<private> VERTEX_v_modelViewProjection : vec4<f32>;

// codes


@vertex
fn main( @location( 0 ) position : vec3<f32> ) -> VaryingsStruct {

	// flow
	// code

	positionLocal = position;
	varyings.v_positionWorld = ( object.nodeUniform2 * vec4<f32>( positionLocal, 1.0 ) ).xyz;
	modelViewMatrix = ( render.cameraViewMatrix * object.nodeUniform2 );
	v_positionView = ( modelViewMatrix * vec4<f32>( positionLocal, 1.0 ) ).xyz;
	let VERTEX_nodeConst5 = ( render.cameraProjectionMatrix * vec4<f32>( v_positionView, 1.0 ) );
	VERTEX_v_modelViewProjection = VERTEX_nodeConst5;

	// result

	varyings.builtinClipSpace = VERTEX_v_modelViewProjection;

	return varyings;

}
