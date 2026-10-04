// Three.js r187dev - Node System

// global
diagnostic( off, derivative_uniformity );


// directives


// structs

struct OutputStruct {
	@location( 0 ) color: vec4<f32>
};
var<private> output : OutputStruct;

// uniforms

struct renderStruct {
	nodeUniform0 : mat4x4<f32>,
	cameraProjectionMatrix : mat4x4<f32>,
	cameraViewMatrix : mat4x4<f32>
};
@binding( 0 ) @group( 0 )
var<uniform> render : renderStruct;

// vars


// codes


@fragment
fn main( @location( 0 ) v_positionView : vec3<f32>,
	@location( 1 ) v_positionWorld : vec3<f32> ) -> OutputStruct {

	// flow
	// code

	let nodeConst0 = ( render.nodeUniform0 * vec4<f32>( v_positionWorld, 1.0 ) );
	let nodeConst1 = ( render.nodeUniform0 * vec4<f32>( v_positionView, 1.0 ) );

	// result

	output.color = vec4<f32>( ( ( nodeConst0.xyz / vec3<f32>( nodeConst0.w ) ) + ( nodeConst1.xyz / vec3<f32>( nodeConst1.w ) ) ), 1.0 );

	return output;

}
