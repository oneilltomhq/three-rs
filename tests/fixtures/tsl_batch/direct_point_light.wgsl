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

	let nodeConst0 = vec3<f32>( nodeVarying4, 1.0 );

	if ( ( nodeVarying4.x > 0.0 ) ) {

		let nodeConst1 = length( nodeConst0 );
		let nodeConst2 = ( nodeConst1 / nodeVarying4.x );
		let nodeConst3 = clamp( ( 1.0 - ( ( ( nodeConst2 * nodeConst2 ) * nodeConst2 ) * nodeConst2 ) ), 0.0, 1.0 );
		nodeVar0 = ( ( 1.0 / max( pow( nodeConst1, 2.0 ), 0.01 ) ) * ( nodeConst3 * nodeConst3 ) );

	} else {

		nodeVar0 = ( 1.0 / max( pow( length( nodeConst0 ), 2.0 ), 0.01 ) );

	}


	// result

	output.color = vec4<f32>( ( normalize( nodeConst0 ) + ( vec3<f32>( 1.0, 0.5, 0.25 ) * vec3<f32>( nodeVar0 ) ) ), 1.0 );

	return output;

}
