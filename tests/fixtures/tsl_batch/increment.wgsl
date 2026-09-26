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
var<private> nodeVar0 : i32;

// codes


@fragment
fn main( @location( 0 ) nodeVarying4 : vec2<f32> ) -> OutputStruct {

	// flow
	// code

	nodeVar0 = i32( ( nodeVarying4.x * 10.0 ) );
	let nodeConst0 = nodeVar0;
	nodeVar0 = ( nodeVar0 + 1 );
	let nodeConst1 = nodeVar0;
	nodeVar0 = ( nodeVar0 - 1 );
	nodeVar0 = ( nodeVar0 + 1 );
	nodeVar0 = ( nodeVar0 - 1 );

	// result

	output.color = vec4<f32>( f32( nodeConst0 ), f32( nodeConst1 ), f32( nodeVar0 ), f32( nodeVar0 ) );

	return output;

}
