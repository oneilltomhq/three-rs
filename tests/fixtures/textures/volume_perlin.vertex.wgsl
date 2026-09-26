// Three.js r187dev - Node System

// directives


// structs


// uniforms

struct objectStruct {
	nodeUniform0 : mat4x4<f32>,
	nodeUniform2 : f32,
	nodeUniform4 : f32,
	nodeUniform5 : u32,
	nodeUniform6 : f32,
	nodeUniform9 : mat4x4<f32>
};
@binding( 0 ) @group( 1 )
var<uniform> object : objectStruct;

struct renderStruct {
	cameraProjectionMatrix : mat4x4<f32>,
	cameraViewMatrix : mat4x4<f32>,
	cameraPosition : vec3<f32>
};
@binding( 0 ) @group( 0 )
var<uniform> render : renderStruct;

// varyings

struct VaryingsStruct {
	@location( 0 ) nodeVarying3 : vec3<f32>,
	@location( 1 ) nodeVarying4 : vec3<f32>,
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

	let nodeConst0 = ( object.nodeUniform0 * vec4<f32>( render.cameraPosition, 1.0 ) ).xyz;
	varyings.nodeVarying3 = nodeConst0;
	varyings.nodeVarying4 = ( position - varyings.nodeVarying3 );
	modelViewMatrix = ( render.cameraViewMatrix * object.nodeUniform9 );
	positionLocal = position;
	v_positionView = ( modelViewMatrix * vec4<f32>( positionLocal, 1.0 ) ).xyz;
	let VERTEX_nodeConst12 = ( render.cameraProjectionMatrix * vec4<f32>( v_positionView, 1.0 ) );
	VERTEX_v_modelViewProjection = VERTEX_nodeConst12;

	// result

	varyings.builtinClipSpace = VERTEX_v_modelViewProjection;

	return varyings;

}
