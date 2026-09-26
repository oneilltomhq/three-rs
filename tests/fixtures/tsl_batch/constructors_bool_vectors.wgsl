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

	output.color = ( ( vec4<f32>( vec2<f32>( vec2<bool>( ( nodeVarying4.x > 0.5 ), true ) ), vec2<f32>( vec3<bool>( true, false, ( nodeVarying4.y < 0.5 ) ).xy ) ) + vec4<f32>( f32( ( nodeVarying4.x > nodeVarying4.y ) ) ) ) + vec4<f32>( vec4<bool>( true, false, true, false ) ) );

	return output;

}
