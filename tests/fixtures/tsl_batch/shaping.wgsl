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
var<private> nodeVar0 : f32;

// codes


@fragment
fn main( @location( 0 ) nodeVarying4 : vec2<f32> ) -> OutputStruct {

	// flow
	// code


	if ( ( nodeVarying4.x < 0.5 ) ) {

		nodeVar0 = ( pow( ( 2.0 * nodeVarying4.x ), 2.0 ) * 0.5 );

	} else {

		nodeVar0 = ( 1.0 - ( pow( ( 2.0 * ( 1.0 - nodeVarying4.x ) ), 2.0 ) * 0.5 ) );

	}

	let nodeConst0 = max( abs( ( 3.141592653589793 * ( ( 3.0 * nodeVarying4.x ) - 1.0 ) ) ), 0.000001 );

	// result

	output.color = vec4<f32>( pow( ( pow( nodeVarying4.x, 2.0 ) / ( pow( nodeVarying4.x, 2.0 ) + pow( ( 1.0 - nodeVarying4.x ), 3.0 ) ) ), ( 1.0 / 2.0 ) ), nodeVar0, pow( ( 4.0 * ( nodeVarying4.y * ( 1.0 - nodeVarying4.y ) ) ), 2.0 ), ( sin( nodeConst0 ) / nodeConst0 ) );

	return output;

}
