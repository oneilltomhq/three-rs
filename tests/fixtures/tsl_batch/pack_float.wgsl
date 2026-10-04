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

	output.color = vec4<f32>( f32( pack2x16snorm(nodeVarying4) ), f32( pack2x16unorm(nodeVarying4) ), f32( pack2x16float(nodeVarying4) ), ( f32( pack4x8snorm(vec4<f32>( nodeVarying4, 0.0, 1.0 )) ) + f32( pack4x8unorm(vec4<f32>( nodeVarying4, 0.0, 1.0 )) ) ) );

	return output;

}
