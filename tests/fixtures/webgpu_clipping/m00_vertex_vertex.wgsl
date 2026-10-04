// Three.js r187dev - Node System

// directives
enable clip_distances;

// structs


// uniforms

struct NodeBuffer_934Struct {
	value : array< vec4<f32>, 1 >
};
@binding( 2 ) @group( 0 )
var<uniform> NodeBuffer_934 : NodeBuffer_934Struct;

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

// varyings

struct VaryingsStruct {
	@location( 0 ) v_positionView : vec3<f32>,
	@location( 1 ) v_positionViewDirection : vec3<f32>,
	@location( 2 ) v_normalViewGeometry : vec3<f32>,
	@builtin( clip_distances ) hw_clip_distances : array<f32, 1 >,
	@builtin( position ) builtinClipSpace : vec4<f32>
};
var<private> varyings : VaryingsStruct;

// vars
var<private> modelViewMatrix : mat4x4<f32>;
var<private> normalLocal : vec3<f32>;
var<private> positionLocal : vec3<f32>;
var<private> v_modelViewProjection : vec4<f32>;
var<private> VERTEX_v_modelViewProjection : vec4<f32>;

// codes


@vertex
fn main( @location( 0 ) position : vec3<f32>,
	@location( 1 ) normal : vec3<f32> ) -> VaryingsStruct {

	// flow
	// code

	modelViewMatrix = ( render.cameraViewMatrix * object.nodeUniform1 );
	positionLocal = position;
	varyings.v_positionView = ( modelViewMatrix * vec4<f32>( positionLocal, 1.0 ) ).xyz;
	normalLocal = normal;
	varyings.v_normalViewGeometry = normalize( ( render.cameraViewMatrix * vec4<f32>( ( object.nodeUniform10 * normalLocal ), 0.0 ) ).xyz );
	varyings.v_positionViewDirection = ( - varyings.v_positionView );

	for ( var i : i32 = 0; i < 1; i ++ ) {

		varyings.hw_clip_distances[ i ] = ( - ( dot( varyings.v_positionView, NodeBuffer_934.value[ i ].xyz ) - NodeBuffer_934.value[ i ].w ) );

	}

	let VERTEX_nodeConst15 = ( render.cameraProjectionMatrix * vec4<f32>( varyings.v_positionView, 1.0 ) );
	VERTEX_v_modelViewProjection = VERTEX_nodeConst15;

	// result

	varyings.builtinClipSpace = VERTEX_v_modelViewProjection;

	return varyings;

}
