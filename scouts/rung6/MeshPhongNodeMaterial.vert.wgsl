// Three.js r186dev - Node System

// directives


// structs


// uniforms
@binding( 2 ) @group( 1 ) var nodeUniform2 : texture_2d_array<f32>;

struct NodeBuffer_1015Struct {
	value : array< vec4<f32>, 2 >
};
@binding( 1 ) @group( 1 )
var<uniform> NodeBuffer_1015 : NodeBuffer_1015Struct;

struct objectStruct {
	nodeUniform0 : f32,
	nodeUniform3 : vec3<f32>,
	nodeUniform4 : f32,
	nodeUniform5 : f32,
	nodeUniform6 : vec3<f32>,
	nodeUniform7 : vec3<f32>,
	nodeUniform8 : f32,
	nodeUniform11 : mat4x4<f32>
};
@binding( 0 ) @group( 1 )
var<uniform> object : objectStruct;

struct renderStruct {
	cameraProjectionMatrix : mat4x4<f32>,
	cameraViewMatrix : mat4x4<f32>,
	nodeUniform9 : vec3<f32>,
	nodeUniform13 : vec3<f32>,
	nodeUniform14 : f32,
	nodeUniform15 : f32,
	nodeUniform12 : vec3<f32>
};
@binding( 0 ) @group( 0 )
var<uniform> render : renderStruct;

// varyings

struct VaryingsStruct {
	@location( 0 ) v_positionView : vec3<f32>,
	@location( 1 ) v_positionViewDirection : vec3<f32>,
	@builtin( position ) builtinClipSpace : vec4<f32>
};
var<private> varyings : VaryingsStruct;

// vars
var<private> nodeVar0 : f32;
var<private> nodeVar1 : f32;
var<private> nodeVar2 : i32;
var<private> nodeVar3 : i32;
var<private> nodeVar4 : vec2<i32>;
var<private> nodeVar5 : vec4<f32>;
var<private> modelViewMatrix : mat4x4<f32>;
var<private> VERTEX_nodeVar34 : vec4<f32>;
var<private> positionLocal : vec3<f32>;
var<private> v_modelViewProjection : vec4<f32>;
var<private> VERTEX_v_modelViewProjection : vec4<f32>;

// codes


@vertex
fn main( @builtin( vertex_index ) vertexIndex : u32,
	@location( 0 ) position : vec3<f32> ) -> VaryingsStruct {

	// flow
	// code

	positionLocal = position;
	positionLocal = ( positionLocal * vec3<f32>( object.nodeUniform0 ) );

	for ( var i : i32 = 0; i < 2; i ++ ) {

		nodeVar0 = 0.0;
		nodeVar1 = NodeBuffer_1015.value[ i ].x;
		nodeVar0 = nodeVar1;

		if ( ( nodeVar0 != 0.0 ) ) {

			nodeVar2 = ( ( i32( vertexIndex ) * 1 ) + 0 );
			nodeVar3 = ( nodeVar2 / 4096 );
			nodeVar4 = vec2<i32>( ( nodeVar2 - ( nodeVar3 * 4096 ) ), nodeVar3 );
			nodeVar5 = textureLoad( nodeUniform2, nodeVar4, i, u32( 0u ) );
			positionLocal = ( positionLocal + ( nodeVar5.xyz * vec3<f32>( nodeVar0 ) ) );
			

		}


	}

	modelViewMatrix = ( render.cameraViewMatrix * object.nodeUniform11 );
	varyings.v_positionView = ( modelViewMatrix * vec4<f32>( positionLocal, 1.0 ) ).xyz;
	varyings.v_positionViewDirection = ( - varyings.v_positionView );
	VERTEX_nodeVar34 = ( render.cameraProjectionMatrix * vec4<f32>( varyings.v_positionView, 1.0 ) );
	VERTEX_v_modelViewProjection = VERTEX_nodeVar34;

	// result

	varyings.builtinClipSpace = VERTEX_v_modelViewProjection;

	return varyings;

}
