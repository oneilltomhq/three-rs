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

struct objectStruct {
	nodeUniform0 : f32,
	nodeUniform3 : mat4x4<f32>
};
@binding( 0 ) @group( 1 )
var<uniform> object : objectStruct;

// vars


// codes


@fragment
fn main(  ) -> OutputStruct {

	// flow
	// code


	// result

	output.color = vec4<f32>( object.nodeUniform0, 0.5, 0.25, 1.0 );

	return output;

}
