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

struct objectStruct {
	nodeUniform1 : f32,
	nodeUniform3 : mat3x3<f32>,
	nodeUniform4 : vec2<f32>
};
@binding( 2 ) @group( 0 )
var<uniform> object : objectStruct;

// vars
var<private> nodeVar0 : vec2<f32>;
var<private> nodeVar1 : f32;
var<private> nodeVar2 : f32;
var<private> nodeVar3 : vec4<f32>;
var<private> nodeVar4 : vec4<f32>;
var<private> nodeVar5 : vec2<f32>;
var<private> nodeVar6 : vec2<f32>;
var<private> nodeVar7 : vec4<f32>;
var<private> nodeVar8 : vec4<f32>;

// codes


@fragment
fn main( @location( 0 ) nodeVarying0 : vec2<f32> ) -> OutputStruct {

	// flow
	// code

	nodeVar0 = ( vec2<f32>( 1.0, 1.0 ) / vec2<f32>( textureDimensions( nodeUniform0, 0 ) ) );
	nodeVar1 = ( object.nodeUniform1 / 2.0 );
	nodeVar2 = ( 0.39894 * ( exp( ( ( ( -0.5 * 0.0 ) * 0.0 ) / ( nodeVar1 * nodeVar1 ) ) ) / nodeVar1 ) );
	nodeVar3 = textureSample( nodeUniform2, nodeUniform2_sampler, ( object.nodeUniform3 * vec3<f32>( nodeVarying0, 1.0 ) ).xy );
	nodeVar4 = ( nodeVar3 * vec4<f32>( nodeVar2 ) );
	nodeVar5 = ( ( ( object.nodeUniform4 * nodeVar0 ) * vec2<f32>( object.nodeUniform1 ) ) / vec2<f32>( 4.0 ) );
	nodeVar6 = nodeVar5;

	for ( var i : i32 = 1; i <= 4; i ++ ) {

		nodeVar7 = textureSample( nodeUniform2, nodeUniform2_sampler, ( object.nodeUniform3 * vec3<f32>( ( nodeVarying0 + nodeVar6 ), 1.0 ) ).xy );
		nodeVar8 = textureSample( nodeUniform2, nodeUniform2_sampler, ( object.nodeUniform3 * vec3<f32>( ( nodeVarying0 - nodeVar6 ), 1.0 ) ).xy );
		let nodeConst0 = ( ( object.nodeUniform1 * f32( i ) ) / 4.0 );
		let nodeConst1 = ( 0.39894 * ( exp( ( ( ( -0.5 * nodeConst0 ) * nodeConst0 ) / ( nodeVar1 * nodeVar1 ) ) ) / nodeVar1 ) );
		nodeVar4 = ( nodeVar4 + ( ( nodeVar7 + nodeVar8 ) * vec4<f32>( nodeConst1 ) ) );
		nodeVar2 = ( nodeVar2 + ( nodeConst1 * 2.0 ) );
		nodeVar6 = ( nodeVar6 + nodeVar5 );

	}


	// result

	output.color = ( nodeVar4 / vec4<f32>( nodeVar2 ) );

	return output;

}
