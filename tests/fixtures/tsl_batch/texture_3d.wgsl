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
@binding( 0 ) @group( 1 ) var nodeUniform0_sampler : sampler;
@binding( 1 ) @group( 1 ) var nodeUniform0 : texture_3d<f32>;

// vars
var<private> nodeVar0 : f32;
var<private> nodeVar1 : f32;

// codes


@fragment
fn main( @location( 0 ) nodeVarying4 : vec2<f32> ) -> OutputStruct {

	// flow
	// code

	nodeVar0 = textureLoad( nodeUniform0, vec3<i32>( i32( ( nodeVarying4.x * 8.0 ) ), i32( ( nodeVarying4.y * 8.0 ) ), i32( 2.0 ) ), u32( 0u ) ).x;
	let nodeConst0 = vec3<f32>( nodeVarying4, 0.5 );
	nodeVar1 = textureSampleLevel( nodeUniform0, nodeUniform0_sampler, nodeConst0, 1.0 ).x;

	// result

	output.color = vec4<f32>( nodeVar0, nodeVar1, 0.0, 1.0 );

	return output;

}
