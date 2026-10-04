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
@binding( 1 ) @group( 0 ) var nodeUniform1_sampler : sampler;
@binding( 2 ) @group( 0 ) var nodeUniform1 : texture_2d<f32>;
@binding( 3 ) @group( 0 ) var nodeUniform4_sampler : sampler;
@binding( 4 ) @group( 0 ) var nodeUniform4 : texture_2d<f32>;

struct objectStruct {
	nodeUniform0 : mat3x3<f32>,
	nodeUniform2 : mat3x3<f32>,
	nodeUniform3 : mat3x3<f32>,
	nodeUniform5 : mat3x3<f32>
};
@binding( 0 ) @group( 0 )
var<uniform> object : objectStruct;

// vars
var<private> nodeVar0 : vec4<f32>;
var<private> nodeVar1 : vec4<f32>;

// codes


@fragment
fn main( @location( 0 ) nodeVarying0 : vec2<f32> ) -> OutputStruct {

	// flow
	// code

	nodeVar0 = textureSample( nodeUniform1, nodeUniform1_sampler, ( object.nodeUniform2 * vec3<f32>( nodeVarying0, 1.0 ) ).xy );
	nodeVar1 = textureSample( nodeUniform4, nodeUniform4_sampler, ( object.nodeUniform5 * vec3<f32>( nodeVarying0, 1.0 ) ).xy );

	// result

	output.color = vec4<f32>( clamp( ( ( object.nodeUniform0 * nodeVar0.xyz ) + ( object.nodeUniform3 * nodeVar1.xyz ) ), vec3<f32>( 0.0 ), vec3<f32>( 1.0 ) ), max( nodeVar0.w, nodeVar1.w ) );

	return output;

}
