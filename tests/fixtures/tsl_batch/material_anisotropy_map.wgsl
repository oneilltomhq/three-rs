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
@binding( 1 ) @group( 1 ) var nodeUniform1_sampler : sampler;
@binding( 2 ) @group( 1 ) var nodeUniform1 : texture_2d<f32>;

struct objectStruct {
	nodeUniform0 : vec2<f32>,
	nodeUniform2 : mat3x3<f32>,
	nodeUniform5 : mat4x4<f32>
};
@binding( 0 ) @group( 1 )
var<uniform> object : objectStruct;

// vars
var<private> nodeVar0 : vec4<f32>;

// codes


@fragment
fn main( @location( 0 ) nodeVarying4 : vec2<f32> ) -> OutputStruct {

	// flow
	// code

	nodeVar0 = textureSample( nodeUniform1, nodeUniform1_sampler, ( object.nodeUniform2 * vec3<f32>( nodeVarying4, 1.0 ) ).xy );

	// result

	output.color = vec4<f32>( ( mat2x2<f32>( object.nodeUniform0.x, object.nodeUniform0.y, ( - object.nodeUniform0.y ), object.nodeUniform0.x ) * ( normalize( ( ( nodeVar0.xy * vec2<f32>( 2.0 ) ) - vec2<f32>( 1.0, 1.0 ) ) ) * vec2<f32>( nodeVar0.z ) ) ), 0.0, 1.0 );

	return output;

}
