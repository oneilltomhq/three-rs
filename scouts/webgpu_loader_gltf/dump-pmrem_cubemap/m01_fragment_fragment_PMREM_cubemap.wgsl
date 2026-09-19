// Three.js r186 - Node System

// global
diagnostic( off, derivative_uniformity );


// structs

struct OutputStruct {
	@location( 0 ) color: vec4<f32>
};
var<private> output : OutputStruct;

// uniforms
@binding( 0 ) @group( 1 ) var nodeUniform0_sampler : sampler;
@binding( 1 ) @group( 1 ) var nodeUniform0 : texture_cube<f32>;

struct objectStruct {
	nodeUniform1 : mat4x4<f32>,
	nodeUniform4 : mat4x4<f32>
};
@binding( 2 ) @group( 1 )
var<uniform> object : objectStruct;

// vars
var<private> nodeVar0 : vec4<f32>;
var<private> nodeVar1 : vec4<f32>;

// codes


@fragment
fn main( @location( 0 ) nodeVarying4 : vec3<f32> ) -> OutputStruct {

	// flow
	// code

	nodeVar0 = ( object.nodeUniform1 * vec4<f32>( normalize( nodeVarying4 ), 1.0 ) );
	nodeVar1 = textureSample( nodeUniform0, nodeUniform0_sampler, vec3<f32>( ( - nodeVar0.x ), nodeVar0.yz ) );

	// result

	output.color = nodeVar1;

	return output;

}
