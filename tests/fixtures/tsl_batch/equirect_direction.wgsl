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

	let nodeConst0 = ( ( nodeVarying4.y - 0.5 ) * 3.141592653589793 );
	let nodeConst1 = cos( nodeConst0 );
	let nodeConst2 = ( ( nodeVarying4.x - 0.5 ) * 6.283185307179586 );
	let nodeConst3 = ( ( nodeVarying4.y - 0.5 ) * 3.141592653589793 );
	let nodeConst4 = cos( nodeConst3 );
	let nodeConst5 = ( ( nodeVarying4.x - 0.5 ) * 6.283185307179586 );

	// result

	output.color = ( vec4<f32>( vec3<f32>( ( nodeConst1 * cos( nodeConst2 ) ), sin( nodeConst0 ), ( nodeConst1 * sin( nodeConst2 ) ) ), 1.0 ) + vec4<f32>( vec3<f32>( ( nodeConst4 * cos( nodeConst5 ) ), sin( nodeConst3 ), ( nodeConst4 * sin( nodeConst5 ) ) ), 0.0 ) );

	return output;

}
