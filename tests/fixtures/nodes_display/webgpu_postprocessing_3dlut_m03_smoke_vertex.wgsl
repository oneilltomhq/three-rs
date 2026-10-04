// Three.js r187dev - Node System

// directives


// structs


// uniforms
@binding( 3 ) @group( 1 ) var nodeUniform0_sampler : sampler;
@binding( 4 ) @group( 1 ) var nodeUniform0 : texture_2d<f32>;

struct renderStruct {
	nodeUniform1 : f32,
	nodeUniform3 : f32,
	cameraProjectionMatrix : mat4x4<f32>,
	cameraViewMatrix : mat4x4<f32>
};
@binding( 0 ) @group( 0 )
var<uniform> render : renderStruct;

struct objectStruct {
	nodeUniform4 : f32,
	nodeUniform7 : mat4x4<f32>
};
@binding( 2 ) @group( 1 )
var<uniform> object : objectStruct;

// varyings

struct VaryingsStruct {
	@location( 0 ) nodeVarying4 : vec2<f32>,
	@builtin( position ) builtinClipSpace : vec4<f32>
};
var<private> varyings : VaryingsStruct;

// vars
var<private> nodeVar0 : vec4<f32>;
var<private> nodeVar1 : vec2<f32>;
var<private> nodeVar2 : vec4<f32>;
var<private> nodeVar3 : vec4<f32>;
var<private> modelViewMatrix : mat4x4<f32>;
var<private> positionLocal : vec3<f32>;
var<private> v_modelViewProjection : vec4<f32>;
var<private> v_positionView : vec3<f32>;
var<private> VERTEX_v_modelViewProjection : vec4<f32>;

// codes
fn tsl_mod_float( x : f32, y : f32 ) -> f32 { return x - y * floor( x / y ); }
fn tsl_mod_vec2( x : vec2f, y : vec2f ) -> vec2f { return x - y * floor( x / y ); }


@vertex
fn main( @location( 0 ) position : vec3<f32>,
	@location( 1 ) uv : vec2<f32> ) -> VaryingsStruct {

	// flow
	// code

	positionLocal = position;
	nodeVar0 = textureSampleLevel( nodeUniform0, nodeUniform0_sampler, vec2<f32>( 0.5, tsl_mod_float( ( ( uv.y * 0.2 ) - ( render.nodeUniform1 * 0.005 ) ), 1.0 ) ), 0 );
	let nodeConst0 = ( nodeVar0.x * 10.0 );
	let nodeConst1 = cos( nodeConst0 );
	let nodeConst2 = sin( nodeConst0 );
	nodeVar1 = ( ( mat2x2<f32>( nodeConst1, nodeConst2, ( - nodeConst2 ), nodeConst1 ) * ( positionLocal.xz - vec2<f32>( 0.0, 0.0 ) ) ) + vec2<f32>( 0.0, 0.0 ) );
	positionLocal.x = nodeVar1[ 0 ];
	positionLocal.z = nodeVar1[ 1 ];
	nodeVar2 = textureSampleLevel( nodeUniform0, nodeUniform0_sampler, tsl_mod_vec2( vec2<f32>( 0.25, ( render.nodeUniform1 * 0.01 ) ), vec2<f32>( 1.0 ) ), 0 );
	nodeVar3 = textureSampleLevel( nodeUniform0, nodeUniform0_sampler, tsl_mod_vec2( vec2<f32>( 0.75, ( render.nodeUniform1 * 0.01 ) ), vec2<f32>( 1.0 ) ), 0 );
	positionLocal = ( positionLocal + vec3<f32>( ( vec2<f32>( ( nodeVar2.x - 0.5 ), ( nodeVar3.x - 0.5 ) ) * vec2<f32>( ( pow( uv.y, 2.0 ) * 10.0 ) ) ), 0.0 ) );
	positionLocal = positionLocal;
	varyings.nodeVarying4 = uv;
	modelViewMatrix = ( render.cameraViewMatrix * object.nodeUniform7 );
	v_positionView = ( modelViewMatrix * vec4<f32>( positionLocal, 1.0 ) ).xyz;
	let VERTEX_nodeConst5 = ( render.cameraProjectionMatrix * vec4<f32>( v_positionView, 1.0 ) );
	VERTEX_v_modelViewProjection = VERTEX_nodeConst5;

	// result

	varyings.builtinClipSpace = VERTEX_v_modelViewProjection;

	return varyings;

}
