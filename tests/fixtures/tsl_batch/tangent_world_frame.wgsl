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
var<private> tangentWorld : vec3<f32>;

// codes


@fragment
fn main( @location( 0 ) v_tangentWorld : vec3<f32> ) -> OutputStruct {

	// flow
	// code

	tangentWorld = normalize( v_tangentWorld );

	// result

	output.color = vec4<f32>( tangentWorld, 1.0 );

	return output;

}
