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
	cameraProjectionMatrix : mat4x4<f32>,
	cameraViewMatrix : mat4x4<f32>
};
@binding( 0 ) @group( 0 )
var<uniform> render : renderStruct;

// vars


// codes


@fragment
fn main( @location( 0 ) nodeVarying4 : vec2<f32> ) -> OutputStruct {

	// flow
	// code


	// result

	output.color = ( vec4<f32>( normalize( ( render.cameraViewMatrix * vec4<f32>( vec3<f32>( nodeVarying4, 1.0 ), 0.0 ) ).xyz ), 1.0 ) + vec4<f32>( normalize( ( vec4<f32>( vec3<f32>( nodeVarying4.y, nodeVarying4.x, 1.0 ), 0.0 ) * render.cameraViewMatrix ).xyz ), 0.0 ) );

	return output;

}
