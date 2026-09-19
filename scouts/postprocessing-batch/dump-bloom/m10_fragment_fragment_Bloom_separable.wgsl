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
	nodeVar1 = ( nodeVar0.xyz * vec3<f32>( 0.0544009090909091 ) );

	for ( var i : i32 = 0; i < 11; i ++ ) {

		nodeVar2 = ( ( object.nodeUniform3 * object.nodeUniform4 ) * vec2<f32>( array< f32, 11 >( 1.4930273115580075, 3.483735079616612, 5.474454081211128, 7.465190698953718, 9.455951266953607, 11.446742053599085, 13.437569244721434, 15.42843892723576, 17.419357073349026, 19.410329525419996, 21.0 )[ i ] ) );
		nodeVar3 = textureSample( nodeUniform0, nodeUniform0_sampler, ( object.nodeUniform2 * vec3<f32>( ( nodeVarying0 + nodeVar2 ), 1.0 ) ).xy );
		nodeVar4 = textureSample( nodeUniform0, nodeUniform0_sampler, ( object.nodeUniform5 * vec3<f32>( ( nodeVarying0 - nodeVar2 ), 1.0 ) ).xy );
		nodeVar1 = ( nodeVar1 + ( ( nodeVar3.xyz + nodeVar4.xyz ) * vec3<f32>( array< f32, 11 >( 0.10631235328115614, 0.09691539802869314, 0.08204439964112456, 0.06449885004133733, 0.04708705604762678, 0.03192252271800665, 0.020097333155365302, 0.011749640609175892, 0.006379034087935798, 0.0032160931712181943, 0.0009013823526604976 )[ i ] ) ) );

	}


	// result

	output.color = vec4<f32>( nodeVar1, 1.0 );

	return output;

}
