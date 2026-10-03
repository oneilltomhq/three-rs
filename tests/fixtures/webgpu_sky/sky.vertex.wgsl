// Three.js r187dev - Node System

// directives


// structs


// uniforms

struct objectStruct {
	nodeUniform0 : mat4x4<f32>,
	nodeUniform2 : f32,
	nodeUniform3 : u32,
	nodeUniform4 : f32,
	nodeUniform5 : f32,
	nodeUniform6 : f32,
	nodeUniform8 : f32,
	nodeUniform9 : f32,
	nodeUniform10 : f32,
	nodeUniform11 : vec3<f32>,
	nodeUniform12 : f32,
	nodeUniform13 : f32,
	nodeUniform14 : f32
};
@binding( 0 ) @group( 1 )
var<uniform> object : objectStruct;

struct renderStruct {
	nodeUniform7 : f32,
	cameraProjectionMatrix : mat4x4<f32>,
	cameraViewMatrix : mat4x4<f32>,
	cameraPosition : vec3<f32>
};
@binding( 0 ) @group( 0 )
var<uniform> render : renderStruct;

// varyings

struct VaryingsStruct {
	@location( 0 ) v_positionWorld : vec3<f32>,
	@location( 1 ) nodeVarying5 : f32,
	@location( 2 ) nodeVarying6 : vec3<f32>,
	@location( 3 ) nodeVarying7 : vec3<f32>,
	@location( 4 ) nodeVarying8 : vec3<f32>,
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
	varyings.v_positionWorld = ( object.nodeUniform0 * vec4<f32>( positionLocal, 1.0 ) ).xyz;
	let nodeConst19 = normalize( object.nodeUniform11 );
	varyings.nodeVarying7 = nodeConst19;
	varyings.nodeVarying5 = ( 1000.0 * max( 0.0, ( 1.0 - pow( 2.718281828459045, ( - ( ( 1.6110731556870734 - acos( clamp( nodeConst19.y, -1.0, 1.0 ) ) ) / 1.5 ) ) ) ) ) );
	varyings.nodeVarying6 = ( vec3<f32>( 0.000005804542996261093, 0.000013562911419845635, 0.000030265902468824876 ) * vec3<f32>( ( object.nodeUniform12 - ( 1.0 * ( 1.0 - ( 1.0 - clamp( ( 1.0 - exp( ( object.nodeUniform11.y / 450000.0 ) ) ), 0.0, 1.0 ) ) ) ) ) ) );
	varyings.nodeVarying8 = ( ( vec3<f32>( ( 0.434 * ( ( 0.2 * object.nodeUniform13 ) * 1e-17 ) ) ) * vec3<f32>( 183999185144339.78, 277980239196605.28, 407904795438610.94 ) ) * vec3<f32>( object.nodeUniform14 ) );
	modelViewMatrix = ( render.cameraViewMatrix * object.nodeUniform0 );
	v_positionView = ( modelViewMatrix * vec4<f32>( positionLocal, 1.0 ) ).xyz;
	let VERTEX_nodeConst20 = ( render.cameraProjectionMatrix * vec4<f32>( v_positionView, 1.0 ) );
	VERTEX_v_modelViewProjection = VERTEX_nodeConst20;
	VERTEX_v_modelViewProjection.z = VERTEX_v_modelViewProjection.w;

	// result

	varyings.builtinClipSpace = VERTEX_v_modelViewProjection;

	return varyings;

}
