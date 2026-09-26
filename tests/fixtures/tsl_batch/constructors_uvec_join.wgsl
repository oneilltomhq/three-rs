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

	output.color = vec4<f32>( vec2<f32>( vec2<u32>( u32( ( nodeVarying4.x * 4.0 ) ), u32( ( nodeVarying4.y * 4.0 ) ) ) ), f32( vec4<u32>( 1u, 2u, 3u, 4u ).z ), 1.0 );

	return output;

}
