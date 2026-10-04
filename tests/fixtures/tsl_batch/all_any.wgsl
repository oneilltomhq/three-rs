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

	output.color = vec4<f32>( f32( all( vec2<bool>( ( nodeVarying4.x > 0.5 ), ( nodeVarying4.y > 0.5 ) ) ) ), f32( any( vec3<bool>( ( nodeVarying4.x < 0.25 ), ( nodeVarying4.y < 0.25 ), false ) ) ), f32( all( ( nodeVarying4 > vec2<f32>( 0.5, 0.5 ) ) ) ), f32( any( ( nodeVarying4 < vec2<f32>( 0.5, 0.5 ) ) ) ) );

	return output;

}
