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


// vars


// codes


@fragment
fn main( @location( 0 ) nodeVarying4 : vec2<f32> ) -> OutputStruct {

	// flow
	// code


	// result

	output.color = vec4<f32>( faceForward( vec3<f32>( nodeVarying4, 1.0 ), vec3<f32>( 0.0, 0.0, 1.0 ), vec3<f32>( nodeVarying4.y, nodeVarying4.x, 0.5 ) ), 1.0 );

	return output;

}
