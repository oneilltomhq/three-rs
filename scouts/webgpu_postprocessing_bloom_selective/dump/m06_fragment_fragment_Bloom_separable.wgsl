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
	nodeVar1 = ( nodeVar0.xyz * vec3<f32>( 0.119682 ) );

	for ( var i : i32 = 0; i < 5; i ++ ) {

		nodeVar2 = ( ( object.nodeUniform3 * object.nodeUniform4 ) * vec2<f32>( array< f32, 5 >( 1.4663011645670991, 3.421894767115691, 5.378716404808932, 7.337378162829917, 9.0 )[ i ] ) );
		nodeVar3 = textureSample( nodeUniform0, nodeUniform0_sampler, ( object.nodeUniform2 * vec3<f32>( ( nodeVarying0 + nodeVar2 ), 1.0 ) ).xy );
		nodeVar4 = textureSample( nodeUniform0, nodeUniform0_sampler, ( object.nodeUniform5 * vec3<f32>( ( nodeVarying0 - nodeVar2 ), 1.0 ) ).xy );
		nodeVar1 = ( nodeVar1 + ( ( nodeVar3.xyz + nodeVar4.xyz ) * vec3<f32>( array< f32, 5 >( 0.21438250006287293, 0.13808060217496526, 0.06253996870210721, 0.019913324055006204, 0.0031262625741366435 )[ i ] ) ) );

	}


	// result

	output.color = vec4<f32>( nodeVar1, 1.0 );

	return output;

}
