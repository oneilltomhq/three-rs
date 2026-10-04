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
var<private> nodeVar0 : f32;
var<private> tangentViewFrame : vec3<f32>;
var<private> tangentView : vec3<f32>;
var<private> bitangentViewFrame : vec3<f32>;
var<private> bitangentView : vec3<f32>;
var<private> TBNViewMatrix : mat3x3<f32>;

// codes


@fragment
fn main( @location( 0 ) v_positionView : vec3<f32>,
	@location( 1 ) v_positionViewDirection : vec3<f32>,
	@location( 2 ) v_normalViewGeometry : vec3<f32>,
	@location( 3 ) nodeVarying6 : vec2<f32> ) -> OutputStruct {

	// flow
	// code

	positionViewDirection = normalize( v_positionViewDirection );
	normalViewGeometry = normalize( v_normalViewGeometry );
	normalView = normalViewGeometry;
	let nodeConst0 = cross( - dpdy( v_positionView ), normalView );
	let nodeConst1 = dpdx( nodeVarying6 );
	let nodeConst2 = cross( normalView, dpdx( v_positionView ) );
	let nodeConst3 = - dpdy( nodeVarying6 );
	let nodeConst4 = ( ( nodeConst0 * vec3<f32>( nodeConst1.x ) ) + ( nodeConst2 * vec3<f32>( nodeConst3.x ) ) );
	let nodeConst5 = ( ( nodeConst0 * vec3<f32>( nodeConst1.y ) ) + ( nodeConst2 * vec3<f32>( nodeConst3.y ) ) );
	let nodeConst6 = max( dot( nodeConst4, nodeConst4 ), dot( nodeConst5, nodeConst5 ) );

	if ( ( nodeConst6 == 0.0 ) ) {

		nodeVar0 = 0.0;

	} else {

		nodeVar0 = inverseSqrt( nodeConst6 );

	}

	tangentViewFrame = ( nodeConst4 * vec3<f32>( nodeVar0 ) );
	tangentView = tangentViewFrame;
	bitangentViewFrame = ( nodeConst5 * nodeVar0 );
	bitangentView = bitangentViewFrame;
	TBNViewMatrix = mat3x3<f32>( tangentView, bitangentView, normalView );
	let nodeConst7 = ( positionViewDirection * TBNViewMatrix );

	// result

	output.color = vec4<f32>( ( vec3<f32>( nodeVarying6, 0.0 ) - ( nodeConst7 * vec3<f32>( 0.1 ) ) ).xy, nodeConst7.z, 1.0 );

	return output;

}
