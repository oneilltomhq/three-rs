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
var<private> bitangentGeometry : vec3<f32>;

// codes


@fragment
fn main( @location( 0 ) nodeVarying4 : vec3<f32>,
	@location( 1 ) nodeVarying5 : vec4<f32> ) -> OutputStruct {

	// flow
	// code

	bitangentGeometry = normalize( ( cross( nodeVarying4, nodeVarying5.xyz ) * vec3<f32>( nodeVarying5.w ) ) );

	// result

	output.color = vec4<f32>( bitangentGeometry, 1.0 );

	return output;

}
