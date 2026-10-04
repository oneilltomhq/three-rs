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
var<private> normalFlat : vec3<f32>;
var<private> normalViewGeometry : vec3<f32>;
var<private> normalView : vec3<f32>;

// codes


@fragment
fn main( @location( 0 ) v_positionView : vec3<f32> ) -> OutputStruct {

	// flow
	// code

	normalFlat = normalize( cross( dpdx( v_positionView ), - dpdy( v_positionView ) ) );
	normalViewGeometry = normalFlat;
	normalView = normalViewGeometry;

	// result

	output.color = vec4<f32>( ( normalView + normalView ), 1.0 );

	return output;

}
