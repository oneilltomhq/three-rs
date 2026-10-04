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

	let nodeConst0 = ( nodeVarying4 - vec2<f32>( 0.5 ) );

	// result

	output.color = vec4<f32>( vec3<f32>( nodeConst0, sqrt( clamp( ( 1.0 - dot( nodeConst0, nodeConst0 ) ), 0.0, 1.0 ) ) ), 1.0 );

	return output;

}
