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

struct objectStruct {
	nodeUniform1 : f32
};
@binding( 2 ) @group( 0 )
var<uniform> object : objectStruct;

// vars
var<private> nodeVar0 : vec4<f32>;

// codes


@fragment
fn main( @location( 0 ) nodeVarying0 : vec2<f32> ) -> OutputStruct {

	// flow
	// code

	nodeVar0 = textureSample( nodeUniform0, nodeUniform0_sampler, nodeVarying0 );
	let nodeConst0 = ( nodeVar0.w * object.nodeUniform1 );
	let nodeConst1 = dot( nodeVar0.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) );
	let nodeConst2 = vec3<f32>( nodeConst1 );
	var nodeVar1 : vec3<f32> = ( vec3<f32>( nodeConst0 ) * mix( ( ( nodeConst2 * nodeVar0.xyz ) * vec3<f32>( 2.0 ) ), ( vec3<f32>( 1.0 ) - ( ( vec3<f32>( 2.0 ) * ( vec3<f32>( 1.0 ) - nodeConst2 ) ) * ( vec3<f32>( 1.0 ) - nodeVar0.xyz ) ) ), min( 1.0, max( 0.0, ( 10.0 * ( nodeConst1 - 0.45 ) ) ) ) ) );
	nodeVar1 = ( nodeVar1 + ( nodeVar0.xyz * vec3<f32>( ( 1.0 - nodeConst0 ) ) ) );

	// result

	output.color = vec4<f32>( nodeVar1, nodeVar0.w );

	return output;

}
