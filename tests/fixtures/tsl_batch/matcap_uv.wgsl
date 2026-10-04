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
var<private> positionViewDirection : vec3<f32>;
var<private> normalViewGeometry : vec3<f32>;
var<private> normalView : vec3<f32>;
var<private> matcapUV : vec2<f32>;

// codes


@fragment
fn main( @location( 0 ) v_positionViewDirection : vec3<f32>,
	@location( 1 ) v_normalViewGeometry : vec3<f32> ) -> OutputStruct {

	// flow
	// code

	positionViewDirection = normalize( v_positionViewDirection );
	let nodeConst0 = normalize( vec3<f32>( positionViewDirection.z, 0.0, ( - positionViewDirection.x ) ) );
	normalViewGeometry = normalize( v_normalViewGeometry );
	normalView = normalViewGeometry;
	matcapUV = ( ( vec2<f32>( dot( nodeConst0, normalView ), dot( cross( positionViewDirection, nodeConst0 ), normalView ) ) * vec2<f32>( 0.495 ) ) + vec2<f32>( 0.5 ) );

	// result

	output.color = vec4<f32>( matcapUV, 0.0, 1.0 );

	return output;

}
