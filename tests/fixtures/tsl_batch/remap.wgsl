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

	output.color = vec4<f32>( ( ( ( ( nodeVarying4.x - 0.2 ) / ( 0.8 - 0.2 ) ) * ( 1.0 - 0.0 ) ) + 0.0 ), ( ( clamp( ( ( nodeVarying4.x - 0.2 ) / ( 0.8 - 0.2 ) ), 0.0, 1.0 ) * ( 2.0 - 1.0 ) ) + 1.0 ), ( ( ( ( nodeVarying4.y - 0.0 ) / ( 1.0 - 0.0 ) ) * ( 3.0 - 2.0 ) ) + 2.0 ), ( ( clamp( ( ( nodeVarying4.y - 0.1 ) / ( 0.9 - 0.1 ) ), 0.0, 1.0 ) * ( 1.0 - 0.0 ) ) + 0.0 ) );

	return output;

}
