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
@binding( 0 ) @group( 0 ) var nodeUniform0_sampler : sampler;
@binding( 1 ) @group( 0 ) var nodeUniform0 : texture_2d<f32>;
@binding( 2 ) @group( 0 ) var nodeUniform1_sampler : sampler;
@binding( 3 ) @group( 0 ) var nodeUniform1 : texture_2d<f32>;
@binding( 4 ) @group( 0 ) var nodeUniform2_sampler : sampler;
@binding( 5 ) @group( 0 ) var nodeUniform2 : texture_2d<f32>;
@binding( 6 ) @group( 0 ) var nodeUniform3_sampler : sampler;
@binding( 7 ) @group( 0 ) var nodeUniform3 : texture_2d<f32>;

// vars
var<private> nodeVar0 : vec4<f32>;
var<private> nodeVar1 : f32;
var<private> nodeVar2 : vec4<f32>;
var<private> nodeVar3 : vec3<f32>;

// codes


@fragment
fn main( @location( 0 ) nodeVarying0 : vec2<f32> ) -> OutputStruct {

	// flow
	// code

	nodeVar0 = textureSample( nodeUniform0, nodeUniform0_sampler, nodeVarying0 );
	let nodeVar0 = nodeVar0;
	nodeVar1 = textureSample( nodeUniform1, nodeUniform1_sampler, nodeVarying0 ).x;
	let nodeVar1 = nodeVar1;
	nodeVar2 = textureSample( nodeUniform2, nodeUniform2_sampler, nodeVarying0 );
	let nodeVar2 = nodeVar2;
	nodeVar3 = textureSample( nodeUniform3, nodeUniform3_sampler, nodeVarying0 ).xyz;
	let nodeVar3 = nodeVar3;

	// result

	output.color = vec4<f32>( ( ( nodeVar0.xyz * vec3<f32>( nodeVar1 ) ) + ( nodeVar2.xyz * nodeVar3 ) ), nodeVar0.w );

	return output;

}
