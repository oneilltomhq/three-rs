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
fn cineonToneMapping ( color : vec3<f32>, exposure : f32 ) -> vec3<f32> {

	

	let nodeConst0 = max( ( ( color * vec3<f32>( exposure ) ) - vec3<f32>( 0.004 ) ), vec3<f32>( 0.0 ) );

	return pow( ( ( nodeConst0 * ( ( nodeConst0 * vec3<f32>( 6.2 ) ) + vec3<f32>( 0.5 ) ) ) / ( ( nodeConst0 * ( ( nodeConst0 * vec3<f32>( 6.2 ) ) + vec3<f32>( 1.7 ) ) ) + vec3<f32>( 0.06 ) ) ), vec3<f32>( 2.2 ) );

}




@fragment
fn main( @location( 0 ) nodeVarying4 : vec2<f32> ) -> OutputStruct {

	// flow
	// code


	// result

	output.color = vec4<f32>( cineonToneMapping( vec3<f32>( nodeVarying4, 0.5 ), 1.2 ), 1.0 );

	return output;

}
