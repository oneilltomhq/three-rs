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

	output.color = ( ( vec4<f32>( unpack4xI8( u32( ( nodeVarying4.x * 1000.0 ) ) ) ) + vec4<f32>( unpack4xU8( u32( ( nodeVarying4.y * 1000.0 ) ) ) ) ) + vec4<f32>( f32( dot4U8Packed( u32( ( nodeVarying4.x * 100.0 ) ), u32( ( nodeVarying4.y * 100.0 ) ) ) ), f32( dot4I8Packed( u32( ( nodeVarying4.x * 50.0 ) ), u32( ( nodeVarying4.y * 50.0 ) ) ) ), 0.0, 1.0 ) );

	return output;

}
