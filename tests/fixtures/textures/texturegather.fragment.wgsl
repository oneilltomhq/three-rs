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
@binding( 2 ) @group( 1 ) var nodeUniform1_sampler : sampler_comparison;
@binding( 3 ) @group( 1 ) var nodeUniform1 : texture_depth_2d;

struct objectStruct {
	nodeUniform2 : f32,
	nodeUniform5 : mat4x4<f32>
};
@binding( 4 ) @group( 1 )
var<uniform> object : objectStruct;

// vars
var<private> DiffuseColor : vec4<f32>;
var<private> nodeVar0 : vec4<f32>;
var<private> nodeVar1 : vec2<f32>;
var<private> nodeVar2 : vec4<f32>;
var<private> nodeVar3 : vec4<f32>;
var<private> Output : vec4<f32>;

// codes


@fragment
fn main( @location( 0 ) nodeVarying4 : vec2<f32> ) -> OutputStruct {

	// flow
	// code

	nodeVar0 = vec4<f32>( 1.0, 1.0, 1.0, 1.0 );
	nodeVar1 = nodeVarying4;

	if ( ( nodeVar1.y > 0.5 ) ) {

		nodeVar2 = textureGather( 0, nodeUniform0, nodeUniform0_sampler, ( nodeVar1 * vec2<f32>( 10.0 ) ), vec2<i32>( 0, 7 ) );
		nodeVar0 = nodeVar2;
		

	} else {

		nodeVar3 = vec4<f32>( textureGatherCompare( nodeUniform1, nodeUniform1_sampler, nodeVar1, 1.0, vec2<i32>( 0, 7 ) ) );
		nodeVar0 = nodeVar3;
		

	}

	DiffuseColor = nodeVar0;
	DiffuseColor.w = ( DiffuseColor.w * object.nodeUniform2 );
	DiffuseColor.w = 1.0;
	let nodeConst0 = max( vec4<f32>( DiffuseColor.xyz, DiffuseColor.w ), vec4<f32>( 0.0 ) );
	Output = nodeConst0;

	// result

	output.color = nodeConst0;

	return output;

}
