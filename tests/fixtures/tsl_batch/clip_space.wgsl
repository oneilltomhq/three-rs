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
fn main( @location( 0 ) v_clipSpace : vec4<f32> ) -> OutputStruct {

	// flow
	// code


	// result

	output.color = ( v_clipSpace / vec4<f32>( v_clipSpace.w ) );

	return output;

}
