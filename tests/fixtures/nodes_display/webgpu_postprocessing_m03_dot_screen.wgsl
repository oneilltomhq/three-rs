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
@binding( 1 ) @group( 1 ) var nodeUniform0 : texture_2d<f32>;

struct renderStruct {
	nodeUniform1 : vec2<f32>
};
@binding( 0 ) @group( 0 )
var<uniform> render : renderStruct;

// vars
var<private> nodeVar0 : vec4<f32>;

// codes


@fragment
fn main( @location( 0 ) nodeVarying0 : vec2<f32> ) -> OutputStruct {

	// flow
	// code

	nodeVar0 = textureSample( nodeUniform0, nodeUniform0_sampler, nodeVarying0 );
	let nodeVar0 = nodeVar0;
	let nodeConst0 = cos( 1.57 );
	let nodeConst1 = ( nodeVarying0 * render.nodeUniform1 );
	let nodeConst2 = sin( 1.57 );
	let nodeConst3 = ( vec2<f32>( ( ( nodeConst0 * nodeConst1.x ) - ( nodeConst2 * nodeConst1.y ) ), ( ( nodeConst2 * nodeConst1.x ) + ( nodeConst0 * nodeConst1.y ) ) ) * vec2<f32>( 0.3 ) );

	// result

	output.color = vec4<f32>( vec3<f32>( ( ( ( ( ( ( nodeVar0.x + nodeVar0.y ) + nodeVar0.z ) / 3.0 ) * 10.0 ) - 5.0 ) + ( ( sin( nodeConst3.x ) * sin( nodeConst3.y ) ) * 4.0 ) ) ), nodeVar0.w );

	return output;

}
