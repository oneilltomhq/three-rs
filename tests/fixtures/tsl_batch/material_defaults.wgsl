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
var<private> normalViewGeometry : vec3<f32>;
var<private> normalView : vec3<f32>;

// codes


@fragment
fn main( @location( 0 ) v_normalViewGeometry : vec3<f32> ) -> OutputStruct {

	// flow
	// code

	normalViewGeometry = normalize( v_normalViewGeometry );
	normalView = normalViewGeometry;

	// result

	output.color = vec4<f32>( ( ( normalView + normalView ) + vec3<f32>( 0.0, 0.0, 0.0 ) ), ( 1.0 + 1.0 ) );

	return output;

}
