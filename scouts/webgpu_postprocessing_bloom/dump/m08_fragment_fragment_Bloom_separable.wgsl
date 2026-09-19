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
	nodeVar1 = ( nodeVar0.xyz * vec3<f32>( 0.08548714285714286 ) );

	for ( var i : i32 = 0; i < 7; i ++ ) {

		nodeVar2 = ( ( object.nodeUniform3 * object.nodeUniform4 ) * vec2<f32>( array< f32, 7 >( 1.4827874165827566, 3.45990768708058, 5.437195705962934, 7.4147440341828155, 9.392640964546707, 11.370969190043434, 13.0 )[ i ] ) );
		nodeVar3 = textureSample( nodeUniform0, nodeUniform0_sampler, ( object.nodeUniform2 * vec3<f32>( ( nodeVarying0 + nodeVar2 ), 1.0 ) ).xy );
		nodeVar4 = textureSample( nodeUniform0, nodeUniform0_sampler, ( object.nodeUniform5 * vec3<f32>( ( nodeVarying0 - nodeVar2 ), 1.0 ) ).xy );
		nodeVar1 = ( nodeVar1 + ( ( nodeVar3.xyz + nodeVar4.xyz ) * vec3<f32>( array< f32, 7 >( 0.16153278215047456, 0.1287340360249976, 0.08555930154584468, 0.04742132242468507, 0.02191797951110977, 0.008447578271836165, 0.001765199910796836 )[ i ] ) ) );

	}


	// result

	output.color = vec4<f32>( nodeVar1, 1.0 );

	return output;

}
