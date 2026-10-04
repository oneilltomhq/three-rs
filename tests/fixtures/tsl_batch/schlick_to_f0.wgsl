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
fn Schlick_to_F0 ( f : vec3<f32>, f90 : f32, dotVH : f32 ) -> vec3<f32> {

	

	let nodeConst0 = clamp( ( 1.0 - dotVH ), 0.0, 1.0 );
	let nodeConst1 = ( nodeConst0 * nodeConst0 );
	let nodeConst2 = clamp( ( ( nodeConst0 * nodeConst1 ) * nodeConst1 ), 0.0, 0.9999 );

	return ( ( f - ( vec3<f32>( f90 ) * vec3<f32>( nodeConst2 ) ) ) / vec3<f32>( ( 1.0 - nodeConst2 ) ) );

}




@fragment
fn main( @location( 0 ) nodeVarying4 : vec2<f32> ) -> OutputStruct {

	// flow
	// code


	// result

	output.color = vec4<f32>( Schlick_to_F0( vec3<f32>( nodeVarying4, 0.5 ), 1.0, nodeVarying4.x ), 1.0 );

	return output;

}
