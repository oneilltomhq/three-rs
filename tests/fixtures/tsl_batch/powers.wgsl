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


	// result

	output.color = vec4<f32>( ( nodeVarying4.x * nodeVarying4.x ), ( ( nodeVarying4.y * nodeVarying4.y ) * nodeVarying4.y ), ( ( ( nodeVarying4.x * nodeVarying4.x ) * nodeVarying4.x ) * nodeVarying4.x ), ( ( abs( ( nodeVarying4.x - nodeVarying4.y ) ) + ( sign( nodeVarying4.x ) * pow( abs( nodeVarying4.x ), 0.3333333333333333 ) ) ) + dot( nodeVarying4, nodeVarying4 ) ) );

	return output;

}
