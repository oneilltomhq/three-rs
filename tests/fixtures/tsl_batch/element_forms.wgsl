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

	output.color = vec4<f32>( smoothstep( 0.2, 0.8, nodeVarying4.x ), step( 0.5, nodeVarying4.y ), smoothstep( vec2<f32>( 0.0 ), vec2<f32>( 1.0 ), nodeVarying4 ) );

	return output;

}
