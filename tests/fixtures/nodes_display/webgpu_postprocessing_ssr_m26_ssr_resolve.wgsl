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

// vars
var<private> nodeVar0 : vec4<f32>;
var<private> nodeVar1 : vec4<f32>;
var<private> nodeVar2 : vec4<f32>;

// codes


@fragment
fn main( @location( 0 ) nodeVarying0 : vec2<f32> ) -> OutputStruct {

	// flow
	// code

	nodeVar0 = textureSample( nodeUniform0, nodeUniform0_sampler, nodeVarying0 );
	let nodeVar0 = nodeVar0;
	nodeVar1 = textureSample( nodeUniform2, nodeUniform2_sampler, nodeVarying0 );
	let nodeVar1 = nodeVar1;
	nodeVar2 = textureSampleLevel( nodeUniform1, nodeUniform1_sampler, nodeVarying0, clamp( ( ( nodeVar1.y * nodeVar1.y ) * 4.0 ), 0.0, 4.0 ) );
	let nodeConst0 = nodeVar2;

	// result

	output.color = ( nodeVar0 + vec4<f32>( nodeConst0.xyz, 1.0 ) );

	return output;

}
