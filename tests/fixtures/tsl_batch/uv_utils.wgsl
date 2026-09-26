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

	let nodeConst0 = cos( render.nodeUniform0 );
	let nodeConst1 = sin( render.nodeUniform0 );
	let nodeConst2 = ( nodeVarying4 - vec2<f32>( 0.5, 0.5 ) );
	let nodeConst3 = dot( nodeConst2, nodeConst2 );

	// result

	output.color = vec4<f32>( ( ( mat2x2<f32>( nodeConst0, nodeConst1, ( - nodeConst1 ), nodeConst0 ) * ( nodeVarying4 - vec2<f32>( 0.5, 0.5 ) ) ) + vec2<f32>( 0.5, 0.5 ) ), ( nodeVarying4 + ( nodeConst2 * vec2<f32>( ( ( nodeConst3 * nodeConst3 ) * 10.0 ) ) ) ) );

	return output;

}
