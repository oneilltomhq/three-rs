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
fn main( @location( 0 ) nodeVarying3 : vec3<f32> ) -> OutputStruct {

	// flow
	// code


	// result

	output.color = vec4<f32>( nodeVarying3, 1.0 );

	return output;

}
