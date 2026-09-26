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

	output.color = vec4<f32>( f32( bitcast<i32>( nodeVarying4.x ) ), f32( bitcast<u32>( nodeVarying4.y ) ), bitcast<f32>( i32( ( nodeVarying4.x * 10.0 ) ) ), bitcast<f32>( u32( ( nodeVarying4.y * 10.0 ) ) ) );

	return output;

}
