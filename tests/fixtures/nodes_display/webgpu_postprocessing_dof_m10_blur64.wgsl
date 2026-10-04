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
@binding( 3 ) @group( 0 ) var nodeUniform3_sampler : sampler;
@binding( 4 ) @group( 0 ) var nodeUniform3 : texture_2d<f32>;

struct NodeBuffer_978Struct {
	value : array< vec4<f32>, 64 >
};
@binding( 5 ) @group( 0 )
var<uniform> NodeBuffer_978 : NodeBuffer_978Struct;

struct objectStruct {
	nodeUniform1 : vec2<f32>,
	nodeUniform2 : f32,
	nodeUniform4 : mat3x3<f32>
};
@binding( 2 ) @group( 0 )
var<uniform> object : objectStruct;

// vars
var<private> nodeVar1 : f32;
var<private> nodeVar2 : vec4<f32>;
var<private> nodeVar3 : f32;

// codes


@fragment
fn main( @location( 0 ) nodeVarying0 : vec2<f32> ) -> OutputStruct {

	// flow
	// code

	var nodeVar0 : vec3<f32> = vec3<f32>( 0.0, 0.0, 0.0 );

	for ( var i : i32 = 0; i < 64; i ++ ) {

		nodeVar1 = textureSample( nodeUniform3, nodeUniform3_sampler, ( object.nodeUniform4 * vec3<f32>( nodeVarying0, 1.0 ) ).xy ).x;
		nodeVar2 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 + ( ( ( object.nodeUniform1 * vec2<f32>( object.nodeUniform2 ) ) * vec2<f32>( nodeVar1 ) ) * NodeBuffer_978.value[ i ].xy ) ) );
		nodeVar0 = ( nodeVar0 + nodeVar2.xyz );

	}

	nodeVar0 = ( nodeVar0 / vec3<f32>( 64.0 ) );
	nodeVar3 = textureSample( nodeUniform3, nodeUniform3_sampler, ( object.nodeUniform4 * vec3<f32>( nodeVarying0, 1.0 ) ).xy ).x;

	// result

	output.color = vec4<f32>( nodeVar0, nodeVar3 );

	return output;

}
