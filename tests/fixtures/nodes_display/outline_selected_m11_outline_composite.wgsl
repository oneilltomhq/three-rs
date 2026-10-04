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
@binding( 3 ) @group( 0 ) var nodeUniform2_sampler : sampler;
@binding( 4 ) @group( 0 ) var nodeUniform2 : texture_2d<f32>;
@binding( 5 ) @group( 0 ) var nodeUniform4_sampler : sampler;
@binding( 6 ) @group( 0 ) var nodeUniform4 : texture_2d<f32>;

struct objectStruct {
	nodeUniform1 : mat3x3<f32>,
	nodeUniform3 : mat3x3<f32>,
	nodeUniform5 : mat3x3<f32>,
	nodeUniform6 : f32
};
@binding( 2 ) @group( 0 )
var<uniform> object : objectStruct;

// vars
var<private> nodeVar0 : vec4<f32>;
var<private> nodeVar1 : vec4<f32>;
var<private> nodeVar2 : vec4<f32>;

// codes


@fragment
fn main( @location( 0 ) nodeVarying0 : vec2<f32> ) -> OutputStruct {

	// flow
	// code

	nodeVar0 = textureSample( nodeUniform0, nodeUniform0_sampler, ( object.nodeUniform1 * vec3<f32>( nodeVarying0, 1.0 ) ).xy );
	nodeVar1 = textureSample( nodeUniform2, nodeUniform2_sampler, ( object.nodeUniform3 * vec3<f32>( nodeVarying0, 1.0 ) ).xy );
	nodeVar2 = textureSample( nodeUniform4, nodeUniform4_sampler, ( object.nodeUniform5 * vec3<f32>( nodeVarying0, 1.0 ) ).xy );

	// result

	output.color = ( vec4<f32>( nodeVar0.x ) * ( nodeVar1 + ( nodeVar2 * vec4<f32>( object.nodeUniform6 ) ) ) );

	return output;

}
