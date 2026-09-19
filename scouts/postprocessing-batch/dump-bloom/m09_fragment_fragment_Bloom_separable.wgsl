// Three.js r186 - Node System

// global
diagnostic( off, derivative_uniformity );


// structs

struct OutputStruct {
	@location( 0 ) color: vec4<f32>
};
var<private> output : OutputStruct;

// uniforms
@binding( 0 ) @group( 0 ) var nodeUniform0_sampler : sampler;
@binding( 1 ) @group( 0 ) var nodeUniform0 : texture_2d<f32>;

struct objectStruct {
	nodeUniform1 : mat3x3<f32>,
	nodeUniform2 : mat3x3<f32>,
	nodeUniform3 : vec2<f32>,
	nodeUniform4 : vec2<f32>,
	nodeUniform5 : mat3x3<f32>
};
@binding( 2 ) @group( 0 )
var<uniform> object : objectStruct;

// vars
var<private> nodeVar0 : vec4<f32>;
var<private> nodeVar1 : vec3<f32>;
var<private> nodeVar2 : vec2<f32>;
var<private> nodeVar3 : vec4<f32>;
var<private> nodeVar4 : vec4<f32>;

// codes


@fragment
fn main( @location( 0 ) nodeVarying0 : vec2<f32> ) -> OutputStruct {

	// flow
	// code

	nodeVar0 = textureSample( nodeUniform0, nodeUniform0_sampler, ( object.nodeUniform1 * vec3<f32>( nodeVarying0, 1.0 ) ).xy );
	nodeVar1 = ( nodeVar0.xyz * vec3<f32>( 0.06649000000000001 ) );

	for ( var i : i32 = 0; i < 9; i ++ ) {

		nodeVar2 = ( ( object.nodeUniform3 * object.nodeUniform4 ) * vec2<f32>( array< f32, 9 >( 1.4895848401126355, 3.4757135713665743, 5.4618796740944076, 7.448104232731768, 9.434407974610943, 11.4208111469608, 13.407333400045928, 15.39399367783732, 17.0 )[ i ] ) );
		nodeVar3 = textureSample( nodeUniform0, nodeUniform0_sampler, ( object.nodeUniform2 * vec3<f32>( ( nodeVarying0 + nodeVar2 ), 1.0 ) ).xy );
		nodeVar4 = textureSample( nodeUniform0, nodeUniform0_sampler, ( object.nodeUniform5 * vec3<f32>( ( nodeVarying0 - nodeVar2 ), 1.0 ) ).xy );
		nodeVar1 = ( nodeVar1 + ( ( nodeVar3.xyz + nodeVar4.xyz ) * vec3<f32>( array< f32, 9 >( 0.1284697562799138, 0.11191824897278832, 0.08731326755905255, 0.06100111134675511, 0.0381655709162591, 0.02138356611366251, 0.010729024104628605, 0.004820686863785114, 0.001201009762281257 )[ i ] ) ) );

	}


	// result

	output.color = vec4<f32>( nodeVar1, 1.0 );

	return output;

}
