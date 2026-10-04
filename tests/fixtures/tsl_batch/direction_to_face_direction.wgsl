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
fn main( @location( 0 ) nodeVarying4 : vec2<f32>,
	@builtin( front_facing ) isFront : bool ) -> OutputStruct {

	// flow
	// code


	// result

	output.color = vec4<f32>( ( vec3<f32>( nodeVarying4, 1.0 ) * vec3<f32>( ( ( f32( isFront ) * 2.0 ) - 1.0 ) ) ), 1.0 );

	return output;

}
