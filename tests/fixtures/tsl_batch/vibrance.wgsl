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


@fragment
fn main( @location( 0 ) nodeVarying4 : vec2<f32> ) -> OutputStruct {

	// flow
	// code

	let nodeConst0 = vec3<f32>( nodeVarying4, 0.5 );
	let nodeConst1 = max( nodeConst0.x, max( nodeConst0.y, nodeConst0.z ) );
	let nodeConst2 = vec3<f32>( nodeVarying4.y, nodeVarying4.x, 0.25 );
	let nodeConst3 = max( nodeConst2.x, max( nodeConst2.y, nodeConst2.z ) );

	// result

	output.color = vec4<f32>( ( max( mix( nodeConst0, vec3<f32>( nodeConst1 ), ( ( ( nodeConst1 - ( ( ( nodeConst0.x + nodeConst0.y ) + nodeConst0.z ) / 3.0 ) ) * nodeVarying4.x ) * -3.0 ) ), vec3<f32>( 0.0 ) ) + max( mix( nodeConst2, vec3<f32>( nodeConst3 ), ( ( ( nodeConst3 - ( ( ( nodeConst2.x + nodeConst2.y ) + nodeConst2.z ) / 3.0 ) ) * 0.0 ) * -3.0 ) ), vec3<f32>( 0.0 ) ) ), 1.0 );

	return output;

}
