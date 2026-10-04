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

struct renderStruct {
	cameraProjectionMatrix : mat4x4<f32>,
	cameraViewMatrix : mat4x4<f32>,
	nodeUniform1 : vec2<f32>,
	nodeUniform0 : vec4<f32>
};
@binding( 0 ) @group( 0 )
var<uniform> render : renderStruct;

// vars


// codes


@fragment
fn main( @builtin( position ) fragCoord : vec4<f32> ) -> OutputStruct {

	// flow
	// code

	let nodeConst0 = ( fragCoord.xy - render.nodeUniform0.xy );

	// result

	output.color = vec4<f32>( ( nodeConst0 / render.nodeUniform0.zw ), ( nodeConst0 / render.nodeUniform1 ) );

	return output;

}
