// Three.js r186 - Node System

// global
diagnostic( off, derivative_uniformity );


// structs

struct OutputStruct {
	@location( 0 ) color: vec4<f32>
};
var<private> output : OutputStruct;

// uniforms
@binding( 2 ) @group( 0 ) var nodeUniform2_sampler : sampler;
@binding( 3 ) @group( 0 ) var nodeUniform2 : texture_2d<f32>;
@binding( 4 ) @group( 0 ) var nodeUniform4_sampler : sampler;
@binding( 5 ) @group( 0 ) var nodeUniform4 : texture_2d<f32>;
@binding( 6 ) @group( 0 ) var nodeUniform6_sampler : sampler;
@binding( 7 ) @group( 0 ) var nodeUniform6 : texture_2d<f32>;
@binding( 8 ) @group( 0 ) var nodeUniform8_sampler : sampler;
@binding( 9 ) @group( 0 ) var nodeUniform8 : texture_2d<f32>;
@binding( 10 ) @group( 0 ) var nodeUniform10_sampler : sampler;
@binding( 11 ) @group( 0 ) var nodeUniform10 : texture_2d<f32>;

struct NodeBuffer_1221Struct {
	value : array< vec4<f32>, 5 >
};
@binding( 1 ) @group( 0 )
var<uniform> NodeBuffer_1221 : NodeBuffer_1221Struct;

struct objectStruct {
	nodeUniform0 : f32,
	nodeUniform3 : mat3x3<f32>,
	nodeUniform5 : mat3x3<f32>,
	nodeUniform7 : mat3x3<f32>,
	nodeUniform9 : mat3x3<f32>,
	nodeUniform11 : mat3x3<f32>,
	nodeUniform12 : f32
};
@binding( 0 ) @group( 0 )
var<uniform> object : objectStruct;

// vars
var<private> nodeVar0 : array< f32, 5 >;
var<private> nodeVar1 : vec4<f32>;
var<private> nodeVar2 : vec4<f32>;
var<private> nodeVar3 : vec4<f32>;
var<private> nodeVar4 : vec4<f32>;
var<private> nodeVar5 : vec4<f32>;

// codes
fn fn4 ( factor : f32, radius : f32 ) -> f32 {

	


	return mix( factor, ( 1.2 - factor ), radius );

}




@fragment
fn main( @location( 0 ) nodeVarying0 : vec2<f32> ) -> OutputStruct {

	// flow
	// code

	nodeVar0 = array< f32, 5 >( 1.0, 0.8, 0.6, 0.4, 0.2 );
	nodeVar1 = textureSample( nodeUniform2, nodeUniform2_sampler, ( object.nodeUniform3 * vec3<f32>( nodeVarying0, 1.0 ) ).xy );
	nodeVar2 = textureSample( nodeUniform4, nodeUniform4_sampler, ( object.nodeUniform5 * vec3<f32>( nodeVarying0, 1.0 ) ).xy );
	nodeVar3 = textureSample( nodeUniform6, nodeUniform6_sampler, ( object.nodeUniform7 * vec3<f32>( nodeVarying0, 1.0 ) ).xy );
	nodeVar4 = textureSample( nodeUniform8, nodeUniform8_sampler, ( object.nodeUniform9 * vec3<f32>( nodeVarying0, 1.0 ) ).xy );
	nodeVar5 = textureSample( nodeUniform10, nodeUniform10_sampler, ( object.nodeUniform11 * vec3<f32>( nodeVarying0, 1.0 ) ).xy );

	// result

	output.color = ( ( ( ( ( ( ( vec4<f32>( fn4( nodeVar0[ 0u ], object.nodeUniform0 ) ) * vec4<f32>( NodeBuffer_1221.value[ 0u ].xyz, 1.0 ) ) * nodeVar1 ) + ( ( vec4<f32>( fn4( nodeVar0[ 1u ], object.nodeUniform0 ) ) * vec4<f32>( NodeBuffer_1221.value[ 1u ].xyz, 1.0 ) ) * nodeVar2 ) ) + ( ( vec4<f32>( fn4( nodeVar0[ 2u ], object.nodeUniform0 ) ) * vec4<f32>( NodeBuffer_1221.value[ 2u ].xyz, 1.0 ) ) * nodeVar3 ) ) + ( ( vec4<f32>( fn4( nodeVar0[ 3u ], object.nodeUniform0 ) ) * vec4<f32>( NodeBuffer_1221.value[ 3u ].xyz, 1.0 ) ) * nodeVar4 ) ) + ( ( vec4<f32>( fn4( nodeVar0[ 4u ], object.nodeUniform0 ) ) * vec4<f32>( NodeBuffer_1221.value[ 4u ].xyz, 1.0 ) ) * nodeVar5 ) ) * vec4<f32>( object.nodeUniform12 ) );

	return output;

}
