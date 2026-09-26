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
fn main(  ) -> OutputStruct {

	// flow
	// code


	// result

	output.color = vec4<f32>( ( vec3<f32>( 1.0, 0.21586050010324417, 0.0 ) + vec3<f32>( 0.25, 0.5, 0.75 ) ), 1.0 );

	return output;

}
