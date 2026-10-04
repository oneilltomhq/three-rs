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

// vars
var<private> nodeVar0 : vec4<f32>;

// codes


@fragment
fn main( @location( 0 ) nodeVarying0 : vec2<f32> ) -> OutputStruct {

	// flow
	// code

	nodeVar0 = textureSample( nodeUniform0, nodeUniform0_sampler, nodeVarying0 );
	let nodeConst0 = nodeVar0.xyz;

	// result

	output.color = vec4<f32>( dot( nodeConst0, vec3<f32>( 0.393, 0.769, 0.189 ) ), dot( nodeConst0, vec3<f32>( 0.349, 0.686, 0.168 ) ), dot( nodeConst0, vec3<f32>( 0.272, 0.534, 0.131 ) ), nodeVar0.w );

	return output;

}
