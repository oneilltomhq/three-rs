// Three.js r186 - Node System

// global
diagnostic( off, derivative_uniformity );


// structs

struct OutputStruct {
	@location( 0 ) color: vec4<f32>
};
var<private> output : OutputStruct;

// uniforms
@binding( 0 ) @group( 0 ) var nodeUniform0_sampler : sampler;
@binding( 1 ) @group( 0 ) var nodeUniform0 : texture_2d<f32>;

struct objectStruct {
	nodeUniform1 : f32,
	nodeUniform2 : vec2<f32>,
	nodeUniform3 : f32
};
@binding( 2 ) @group( 0 )
var<uniform> object : objectStruct;

// vars


// codes
fn ChromaticAberrationShader ( uv : vec2<f32>, strength : f32, center : vec2<f32>, scale : f32 ) -> vec4<f32> {

	var nodeVar0 : vec2<f32>;
	var nodeVar1 : f32;
	var nodeVar2 : vec4<f32>;
	var nodeVar3 : vec4<f32>;
	var nodeVar4 : vec4<f32>;
	var nodeVar5 : vec4<f32>;

	nodeVar0 = ( uv - center );
	nodeVar1 = ( strength * length( nodeVar0 ) );
	nodeVar2 = textureSample( nodeUniform0, nodeUniform0_sampler, ( ( center + ( nodeVar0 * vec2<f32>( ( 1.0 + ( ( scale * 0.02 ) * strength ) ) ) ) ) + ( ( nodeVar0 * vec2<f32>( nodeVar1 ) ) * vec2<f32>( 0.01 ) ) ) );
	nodeVar3 = textureSample( nodeUniform0, nodeUniform0_sampler, ( ( center + ( nodeVar0 * vec2<f32>( 1.0 ) ) ) + ( ( nodeVar0 * vec2<f32>( nodeVar1 ) ) * vec2<f32>( 0.0 ) ) ) );
	nodeVar4 = textureSample( nodeUniform0, nodeUniform0_sampler, ( ( center + ( nodeVar0 * vec2<f32>( ( 1.0 - ( ( scale * 0.02 ) * strength ) ) ) ) ) + ( ( nodeVar0 * vec2<f32>( nodeVar1 ) ) * vec2<f32>( -0.01 ) ) ) );
	nodeVar5 = textureSample( nodeUniform0, nodeUniform0_sampler, uv );

	return vec4<f32>( nodeVar2.x, nodeVar3.y, nodeVar4.z, nodeVar5.w );

}




@fragment
fn main( @location( 0 ) nodeVarying0 : vec2<f32> ) -> OutputStruct {

	// flow
	// code


	// result

	output.color = ChromaticAberrationShader( nodeVarying0, object.nodeUniform1, object.nodeUniform2, object.nodeUniform3 );

	return output;

}
