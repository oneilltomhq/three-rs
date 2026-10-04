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

	output.color = ( ( ( vec4<f32>( unpack2x16snorm(u32( ( nodeVarying4.x * 1000.0 ) )), unpack2x16unorm(u32( ( nodeVarying4.y * 1000.0 ) )) ) + vec4<f32>( unpack2x16float(u32( ( nodeVarying4.x * 100.0 ) )), 0.0, 1.0 ) ) + unpack4x8snorm(u32( ( nodeVarying4.y * 100.0 ) )) ) + unpack4x8unorm(u32( ( nodeVarying4.x * 10.0 ) )) );

	return output;

}
