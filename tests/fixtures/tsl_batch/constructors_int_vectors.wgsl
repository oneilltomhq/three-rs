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

	output.color = vec4<f32>( ( ( vec3<f32>( vec3<u32>( 1u, 2u, 3u ) ) + vec3<f32>( vec3<i32>( -1, 2, -3 ) ) ) + vec3<f32>( vec3<u32>( u32( ( nodeVarying4.x * 10.0 ) ) ) ) ), f32( vec4<i32>( 1, 2, 3, 4 ).w ) );

	return output;

}
