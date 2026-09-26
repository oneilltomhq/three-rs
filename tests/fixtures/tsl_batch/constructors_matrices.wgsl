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

struct objectStruct {
	nodeUniform0 : mat4x4<f32>,
	nodeUniform3 : mat4x4<f32>
};
@binding( 0 ) @group( 1 )
var<uniform> object : objectStruct;

// vars


// codes


@fragment
fn main( @location( 0 ) nodeVarying4 : vec2<f32> ) -> OutputStruct {

	// flow
	// code


	// result

	output.color = ( vec4<f32>( ( mat3x3<f32>( object.nodeUniform0[ 0 ].xyz, object.nodeUniform0[ 1 ].xyz, object.nodeUniform0[ 2 ].xyz ) * vec3<f32>( nodeVarying4, 1.0 ) ), 1.0 ) + vec4<f32>( ( mat2x2<f32>( 1.0, 3.0, 2.0, 4.0 ) * nodeVarying4 ), 0.0, 0.0 ) );

	return output;

}
