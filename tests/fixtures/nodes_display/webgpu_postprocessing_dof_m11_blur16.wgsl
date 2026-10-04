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

struct NodeBuffer_983Struct {
	value : array< vec4<f32>, 16 >
};
@binding( 3 ) @group( 0 )
var<uniform> NodeBuffer_983 : NodeBuffer_983Struct;

struct objectStruct {
	nodeUniform1 : mat3x3<f32>,
	nodeUniform2 : vec2<f32>,
	nodeUniform3 : f32
};
@binding( 2 ) @group( 0 )
var<uniform> object : objectStruct;

// vars
var<private> nodeVar0 : vec4<f32>;
var<private> nodeVar1 : vec4<f32>;
var<private> nodeVar2 : vec4<f32>;
var<private> nodeVar3 : vec3<f32>;

// codes


@fragment
fn main( @location( 0 ) nodeVarying0 : vec2<f32> ) -> OutputStruct {

	// flow
	// code

	nodeVar0 = textureSample( nodeUniform0, nodeUniform0_sampler, ( object.nodeUniform1 * vec3<f32>( nodeVarying0, 1.0 ) ).xy );
	nodeVar1 = nodeVar0;

	for ( var i : i32 = 0; i < 16; i ++ ) {

		nodeVar2 = textureSample( nodeUniform0, nodeUniform0_sampler, ( object.nodeUniform1 * vec3<f32>( ( nodeVarying0 + ( ( ( object.nodeUniform2 * vec2<f32>( object.nodeUniform3 ) ) * vec2<f32>( nodeVar1.w ) ) * NodeBuffer_983.value[ i ].xy ) ), 1.0 ) ).xy );
		nodeVar3 = max( nodeVar2.xyz, nodeVar1.xyz );
		nodeVar1.x = nodeVar3[ 0 ];
		nodeVar1.y = nodeVar3[ 1 ];
		nodeVar1.z = nodeVar3[ 2 ];

	}


	// result

	output.color = vec4<f32>( nodeVar1.xyz, nodeVar1.w );

	return output;

}
