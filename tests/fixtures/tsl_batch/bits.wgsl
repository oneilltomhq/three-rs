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
fn tsl_xor( a : bool, b : bool ) -> bool { return ( a || b ) && !( a && b ); }


@fragment
fn main( @location( 0 ) nodeVarying4 : vec2<f32> ) -> OutputStruct {

	// flow
	// code

	let nodeConst0 = u32( ( nodeVarying4.y * 100.0 ) );

	// result

	output.color = vec4<f32>( f32( tsl_xor( ( nodeVarying4.x > 0.5 ), ( nodeVarying4.y > 0.5 ) ) ), f32( ( ~ i32( ( nodeVarying4.x * 100.0 ) ) ) ), f32( ( ( countOneBits( nodeConst0 ) + countTrailingZeros( nodeConst0 ) ) + countLeadingZeros( nodeConst0 ) ) ), 1.0 );

	return output;

}
