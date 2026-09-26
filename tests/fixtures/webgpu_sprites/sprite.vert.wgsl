// Three.js r187dev - Node System

// directives


// structs


// uniforms

struct renderStruct {
	cameraProjectionMatrix : mat4x4<f32>,
	cameraViewMatrix : mat4x4<f32>
};
@binding( 0 ) @group( 0 )
var<uniform> render : renderStruct;

struct objectStruct {
	nodeUniform1 : mat3x3<f32>,
	nodeUniform3 : mat4x4<f32>,
	nodeUniform4 : f32,
	nodeUniform5 : vec2<f32>
};
@binding( 2 ) @group( 1 )
var<uniform> object : objectStruct;

// varyings

struct VaryingsStruct {
	@location( 0 ) v_positionView : vec4<f32>,
	@location( 1 ) nodeVarying3 : vec2<f32>,
	@builtin( position ) builtinClipSpace : vec4<f32>
};
var<private> varyings : VaryingsStruct;

// vars
var<private> modelViewMatrix : mat4x4<f32>;
var<private> v_modelViewProjection : vec4<f32>;
var<private> VERTEX_v_modelViewProjection : vec4<f32>;

// codes


@vertex
fn main( @location( 0 ) uv : vec2<f32>,
	@location( 1 ) position : vec3<f32> ) -> VaryingsStruct {

	// flow
	// code

	varyings.nodeVarying3 = uv;
	modelViewMatrix = ( render.cameraViewMatrix * object.nodeUniform3 );
	let nodeConst0 = ( modelViewMatrix * vec4<f32>( vec3<f32>( 0.0, 0.0, 0.0 ), 1.0 ) );
	let nodeConst1 = object.nodeUniform4;
	let nodeConst2 = cos( nodeConst1 );
	let nodeConst3 = sin( nodeConst1 );
	let nodeConst4 = vec4<f32>( ( nodeConst0.xy + ( mat2x2<f32>( nodeConst2, nodeConst3, ( - nodeConst3 ), nodeConst2 ) * ( ( position.xy - ( object.nodeUniform5 - vec2<f32>( 0.5 ) ) ) * vec2<f32>( length( object.nodeUniform3[ 0u ].xyz ), length( object.nodeUniform3[ 1u ].xyz ) ) ) ) ), nodeConst0.zw );
	varyings.v_positionView = nodeConst4;
	let VERTEX_nodeConst6 = ( render.cameraProjectionMatrix * varyings.v_positionView );
	VERTEX_v_modelViewProjection = VERTEX_nodeConst6;

	// result

	varyings.builtinClipSpace = VERTEX_v_modelViewProjection;

	return varyings;

}
