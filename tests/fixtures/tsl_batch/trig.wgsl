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

	output.color = vec4<f32>( ( ( ( atan( nodeVarying4.x ) + acos( nodeVarying4.x ) ) + asin( nodeVarying4.x ) ) + tan( nodeVarying4.x ) ), ( ( sinh( nodeVarying4.x ) + cosh( nodeVarying4.x ) ) + tanh( nodeVarying4.x ) ), ( ( asinh( nodeVarying4.x ) + acosh( ( nodeVarying4.x + 1.0 ) ) ) + atanh( ( nodeVarying4.x * 0.5 ) ) ), ( ( ( round( nodeVarying4.x ) + trunc( nodeVarying4.y ) ) + degrees( nodeVarying4.x ) ) + radians( nodeVarying4.y ) ) );

	return output;

}
