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
	cameraNear : f32,
	cameraFar : f32,
	cameraProjectionMatrix : mat4x4<f32>,
	cameraViewMatrix : mat4x4<f32>
};
@binding( 0 ) @group( 0 )
var<uniform> render : renderStruct;

// vars
var<private> nodeVar0 : f32;

// codes


@fragment
fn main( @location( 0 ) v_positionView : vec3<f32>,
	@location( 1 ) nodeVarying4 : vec2<f32> ) -> OutputStruct {

	// flow
	// code

	nodeVar0 = max( render.cameraNear, 0.000001 );

	// result

	output.color = vec4<f32>( ( log2( ( ( - v_positionView.z ) / nodeVar0 ) ) / log2( ( render.cameraFar / nodeVar0 ) ) ), ( - ( pow( 2.718281828459045, ( nodeVarying4.x * log( ( render.cameraFar / render.cameraNear ) ) ) ) * render.cameraNear ) ), 0.0, 1.0 );

	return output;

}
