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
@binding( 2 ) @group( 0 ) var nodeUniform1_sampler : sampler;
@binding( 3 ) @group( 0 ) var nodeUniform1 : texture_2d<f32>;
@binding( 5 ) @group( 0 ) var nodeUniform3_sampler : sampler;
@binding( 6 ) @group( 0 ) var nodeUniform3 : texture_2d<f32>;

struct objectStruct {
	nodeUniform2 : mat3x3<f32>,
	nodeUniform4 : mat3x3<f32>
};
@binding( 4 ) @group( 0 )
var<uniform> object : objectStruct;

// vars
var<private> nodeVar0 : vec4<f32>;
var<private> nodeVar1 : vec4<f32>;
var<private> nodeVar2 : vec4<f32>;
var<private> nodeVar3 : vec3<f32>;
var<private> nodeVar4 : vec4<f32>;
var<private> nodeVar5 : vec3<f32>;

// codes


@fragment
fn main( @location( 0 ) nodeVarying0 : vec2<f32> ) -> OutputStruct {

	// flow
	// code

	nodeVar0 = vec4<f32>( 0.0, 0.0, 0.0, 1.0 );
	nodeVar1 = textureSample( nodeUniform0, nodeUniform0_sampler, nodeVarying0 );
	nodeVar2 = textureSample( nodeUniform1, nodeUniform1_sampler, ( object.nodeUniform2 * vec3<f32>( nodeVarying0, 1.0 ) ).xy );
	nodeVar3 = mix( nodeVar1.xyz, nodeVar2.xyz, ( min( nodeVar2.w, 0.5 ) * 2.0 ) );
	nodeVar0.x = nodeVar3[ 0 ];
	nodeVar0.y = nodeVar3[ 1 ];
	nodeVar0.z = nodeVar3[ 2 ];
	nodeVar4 = textureSample( nodeUniform3, nodeUniform3_sampler, ( object.nodeUniform4 * vec3<f32>( nodeVarying0, 1.0 ) ).xy );
	nodeVar5 = mix( nodeVar0.xyz, nodeVar4.xyz, ( min( nodeVar4.w, 0.5 ) * 2.0 ) );
	nodeVar0.x = nodeVar5[ 0 ];
	nodeVar0.y = nodeVar5[ 1 ];
	nodeVar0.z = nodeVar5[ 2 ];

	// result

	output.color = nodeVar0;

	return output;

}
