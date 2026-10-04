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
var<private> normalLocal : vec3<f32>;
var<private> tangentLocal : vec3<f32>;
var<private> bitangentLocal : vec3<f32>;

// codes


@fragment
fn main( @location( 0 ) nodeVarying4 : vec3<f32>,
	@location( 1 ) nodeVarying5 : vec4<f32> ) -> OutputStruct {

	// flow
	// code

	normalLocal = nodeVarying4;
	tangentLocal = nodeVarying5.xyz;
	bitangentLocal = normalize( ( cross( normalLocal, tangentLocal ) * vec3<f32>( nodeVarying5.w ) ) );

	// result

	output.color = vec4<f32>( bitangentLocal, 1.0 );

	return output;

}
