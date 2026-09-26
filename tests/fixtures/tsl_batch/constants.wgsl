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

	output.color = vec4<f32>( ( nodeVarying4.x * 3.141592653589793 ), ( nodeVarying4.x * 6.283185307179586 ), ( nodeVarying4.x * 1.5707963267948966 ), min( ( nodeVarying4.x + 0.000001 ), 1000000.0 ) );

	return output;

}
