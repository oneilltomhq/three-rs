// Three.js r186 - Node System

// directives


// structs


// uniforms

struct renderStruct {
	nodeUniform5 : f32,
	cameraProjectionMatrix : mat4x4<f32>,
	cameraViewMatrix : mat4x4<f32>
};
@binding( 0 ) @group( 0 )
var<uniform> render : renderStruct;

struct objectStruct {
	nodeUniform8 : vec3<f32>,
	nodeUniform9 : vec3<f32>,
	nodeUniform10 : f32,
	nodeUniform13 : mat4x4<f32>,
	nodeUniform14 : f32,
	nodeUniform16 : f32
};
@binding( 0 ) @group( 1 )
var<uniform> object : objectStruct;

// varyings

struct VaryingsStruct {
	@location( 0 ) nodeVarying4 : vec4<f32>,
	@location( 1 ) nodeVarying5 : vec2<f32>,
	@builtin( position ) builtinClipSpace : vec4<f32>
};
var<private> varyings : VaryingsStruct;

// vars
var<private> nodeVar0 : mat4x4<f32>;
var<private> normalLocal : vec3<f32>;
var<private> nodeVar1 : f32;
var<private> nodeVar2 : f32;
var<private> nodeVar3 : vec3<f32>;
var<private> modelViewMatrix : mat4x4<f32>;
var<private> nodeVar5 : vec4<f32>;
var<private> nodeVar6 : f32;
var<private> nodeVar7 : f32;
var<private> VERTEX_nodeVar8 : vec4<f32>;
var<private> positionLocal : vec3<f32>;
var<private> v_modelViewProjection : vec4<f32>;
var<private> v_positionView : vec4<f32>;
var<private> VERTEX_v_modelViewProjection : vec4<f32>;

// codes

fn tsl_inverse_mat3( m : mat3x3<f32> ) -> mat3x3<f32> {

	let a00 = m[ 0 ][ 0 ]; let a01 = m[ 0 ][ 1 ]; let a02 = m[ 0 ][ 2 ];
	let a10 = m[ 1 ][ 0 ]; let a11 = m[ 1 ][ 1 ]; let a12 = m[ 1 ][ 2 ];
	let a20 = m[ 2 ][ 0 ]; let a21 = m[ 2 ][ 1 ]; let a22 = m[ 2 ][ 2 ];

	let b01 = a22 * a11 - a12 * a21;
	let b11 = - a22 * a10 + a12 * a20;
	let b21 = a21 * a10 - a11 * a20;

	let det = a00 * b01 + a01 * b11 + a02 * b21;

	return mat3x3<f32>(
		b01, ( - a22 * a01 + a02 * a21 ), ( a12 * a01 - a02 * a11 ),
		b11, ( a22 * a00 - a02 * a20 ), ( - a12 * a00 + a02 * a10 ),
		b21, ( - a21 * a00 + a01 * a20 ), ( a11 * a00 - a01 * a10 )
	) * ( 1.0 / det );

}



@vertex
fn main( @location( 0 ) position : vec3<f32>,
	@location( 1 ) normal : vec3<f32>,
	@location( 2 ) uv : vec2<f32>,
	@location( 3 ) nodeAttribute0 : vec4<f32>,
	@location( 4 ) nodeAttribute1 : vec4<f32>,
	@location( 5 ) nodeAttribute2 : vec4<f32>,
	@location( 6 ) nodeAttribute3 : vec4<f32>,
	@location( 7 ) nodeAttribute4 : vec4<f32>,
	@location( 8 ) nodeAttribute6 : vec4<f32>,
	@location( 9 ) nodeAttribute7 : vec4<f32>,
	@location( 10 ) nodeAttribute15 : vec4<f32> ) -> VaryingsStruct {

	// flow
	// code

	positionLocal = position;
	nodeVar0 = mat4x4<f32>( nodeAttribute0, nodeAttribute1, nodeAttribute2, nodeAttribute3 );
	positionLocal = ( nodeVar0 * vec4<f32>( positionLocal, 1.0 ) ).xyz;
	normalLocal = normal;
	normalLocal = normalize( ( transpose( tsl_inverse_mat3( mat3x3<f32>( nodeVar0[ 0 ].xyz, nodeVar0[ 1 ].xyz, nodeVar0[ 2 ].xyz ) ) ) * normalLocal ) );
	nodeVar1 = ( ( floor( nodeAttribute4.x ) * ( 6.283185307179586 / 3.0 ) ) + ( render.nodeUniform5 * ( 1.0 - nodeAttribute6.x ) ) );
	nodeVar2 = ( pow( nodeAttribute6.x, 1.5 ) * 5.0 );
	nodeVar3 = ( ( vec3<f32>( cos( nodeVar1 ), 0.0, sin( nodeVar1 ) ) * vec3<f32>( nodeVar2 ) ) + ( ( ( ( nodeAttribute7.xyz * nodeAttribute7.xyz ) * nodeAttribute7.xyz ) * vec3<f32>( nodeAttribute6.x ) ) + vec3<f32>( 0.2 ) ) );
	positionLocal = nodeVar3;
	varyings.nodeVarying4 = nodeAttribute6;
	varyings.nodeVarying5 = uv;
	modelViewMatrix = ( render.cameraViewMatrix * object.nodeUniform13 );
	nodeVar5 = ( modelViewMatrix * vec4<f32>( nodeVar3, 1.0 ) );
	nodeVar6 = cos( object.nodeUniform14 );
	nodeVar7 = sin( object.nodeUniform14 );
	v_positionView = vec4<f32>( ( nodeVar5.xy + ( mat2x2<f32>( nodeVar6, nodeVar7, ( - nodeVar7 ), nodeVar6 ) * ( position.xy * ( vec2<f32>( length( object.nodeUniform13[ 0u ].xyz ), length( object.nodeUniform13[ 1u ].xyz ) ) * vec2<f32>( ( nodeAttribute15.x * object.nodeUniform16 ) ) ) ) ) ), nodeVar5.zw );
	VERTEX_nodeVar8 = ( render.cameraProjectionMatrix * v_positionView );
	VERTEX_v_modelViewProjection = VERTEX_nodeVar8;

	// result

	varyings.builtinClipSpace = VERTEX_v_modelViewProjection;

	return varyings;

}
