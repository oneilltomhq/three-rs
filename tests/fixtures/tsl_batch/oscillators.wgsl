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
	nodeUniform0 : f32,
	cameraProjectionMatrix : mat4x4<f32>,
	cameraViewMatrix : mat4x4<f32>
};
@binding( 0 ) @group( 0 )
var<uniform> render : renderStruct;

// vars


// codes


@fragment
fn main( @location( 0 ) nodeVarying4 : vec2<f32> ) -> OutputStruct {

	// flow
	// code


	// result

	output.color = vec4<f32>( abs( ( ( fract( ( render.nodeUniform0 + 0.5 ) ) * 2.0 ) - 1.0 ) ), round( fract( render.nodeUniform0 ) ), fract( nodeVarying4.x ), ( ( sin( ( ( render.nodeUniform0 + 0.75 ) * 6.283185307179586 ) ) * 0.5 ) + 0.5 ) );

	return output;

}
