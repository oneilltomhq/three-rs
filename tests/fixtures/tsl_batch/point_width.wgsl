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
var<private> pointWidth : f32;

// codes


@fragment
fn main(  ) -> OutputStruct {

	// flow
	// code


	// result

	output.color = vec4<f32>( pointWidth, 0.0, 0.0, 1.0 );

	return output;

}
