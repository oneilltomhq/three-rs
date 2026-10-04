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

	output.color = vec4<f32>( f32( pack4xI8( vec4<i32>( i32( ( nodeVarying4.x * 10.0 ) ), i32( -2.0 ), i32( 3.0 ), i32( -4.0 ) ) ) ), f32( pack4xU8( vec4<u32>( u32( ( nodeVarying4.y * 10.0 ) ), u32( 2.0 ), u32( 3.0 ), u32( 4.0 ) ) ) ), f32( pack4xI8Clamp( vec4<i32>( i32( ( nodeVarying4.x * 300.0 ) ), i32( -200.0 ), i32( 3.0 ), i32( 4.0 ) ) ) ), f32( pack4xU8Clamp( vec4<u32>( u32( ( nodeVarying4.y * 300.0 ) ), u32( 2.0 ), u32( 3.0 ), u32( 4.0 ) ) ) ) );

	return output;

}
