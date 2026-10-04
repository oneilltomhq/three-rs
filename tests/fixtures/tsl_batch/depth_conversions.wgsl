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


// codes


@fragment
fn main( @location( 0 ) v_positionView : vec3<f32>,
	@location( 1 ) nodeVarying4 : vec2<f32> ) -> OutputStruct {

	// flow
	// code


	// result

	output.color = vec4<f32>( ( ( v_positionView.z + render.cameraFar ) / ( render.cameraFar - render.cameraNear ) ), ( ( ( render.cameraNear - render.cameraFar ) * nodeVarying4.x ) - render.cameraNear ), ( ( render.cameraNear * ( v_positionView.z + render.cameraFar ) ) / ( v_positionView.z * ( render.cameraNear - render.cameraFar ) ) ), 1.0 );

	return output;

}
