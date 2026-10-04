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
	cameraViewMatrix : mat4x4<f32>
};
@binding( 0 ) @group( 0 )
var<uniform> render : renderStruct;

struct objectStruct {
	nodeUniform1 : mat4x4<f32>,
	nodeUniform2 : mat4x4<f32>,
	nodeUniform3 : mat3x3<f32>,
	nodeUniform6 : mat4x4<f32>
};
@binding( 0 ) @group( 1 )
var<uniform> object : objectStruct;

// vars
var<private> highpModelViewMatrix : mat4x4<f32>;
var<private> highpModelNormalViewMatrix : mat3x3<f32>;
var<private> normalLocal : vec3<f32>;

// codes


@fragment
fn main( @location( 0 ) positionLocal : vec3<f32>,
	@location( 1 ) nodeVarying4 : vec3<f32> ) -> OutputStruct {

	// flow
	// code

	highpModelViewMatrix = object.nodeUniform2;
	highpModelNormalViewMatrix = object.nodeUniform3;
	normalLocal = nodeVarying4;

	// result

	output.color = vec4<f32>( ( ( ( ( render.cameraViewMatrix * object.nodeUniform1 ) * vec4<f32>( positionLocal, 1.0 ) ).xyz + ( highpModelViewMatrix * vec4<f32>( positionLocal, 1.0 ) ).xyz ) + ( highpModelNormalViewMatrix * normalLocal ) ), 1.0 );

	return output;

}
