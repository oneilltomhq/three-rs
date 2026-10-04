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
@binding( 1 ) @group( 0 ) var nodeUniform1_sampler : sampler;
@binding( 2 ) @group( 0 ) var nodeUniform1 : texture_2d<f32>;

struct objectStruct {
	nodeUniform0 : f32,
	nodeUniform2 : f32,
	nodeUniform3 : f32
};
@binding( 0 ) @group( 0 )
var<uniform> object : objectStruct;

// vars
var<private> nodeVar0 : vec2<f32>;
var<private> nodeVar1 : vec2<f32>;
var<private> nodeVar2 : vec4<f32>;
var<private> nodeVar3 : vec2<f32>;
var<private> nodeVar4 : vec4<f32>;

// codes


@fragment
fn main( @location( 0 ) nodeVarying0 : vec2<f32> ) -> OutputStruct {

	// flow
	// code

	nodeVar0 = ( vec2<f32>( 1.0 ) - nodeVarying0 );
	nodeVar1 = ( ( vec2<f32>( 0.5, 0.5 ) - nodeVar0 ) * vec2<f32>( object.nodeUniform0 ) );
	nodeVar2 = vec4<f32>( 0.0, 0.0, 0.0, 1.0 );

	for ( var i : i32 = 0; i < 4; i ++ ) {

		nodeVar3 = fract( ( nodeVar0 + ( nodeVar1 * vec2<f32>( f32( i ) ) ) ) );
		nodeVar4 = textureSample( nodeUniform1, nodeUniform1_sampler, nodeVar3 );
		nodeVar2 = ( nodeVar2 + vec4<f32>( ( ( max( ( nodeVar4.xyz - vec3<f32>( object.nodeUniform2 ) ), vec3<f32>( 0.0, 0.0, 0.0 ) ) * vec3<f32>( 1.0, 1.0, 1.0 ) ) * vec3<f32>( pow( ( 1.0 - distance( nodeVar3, vec2<f32>( 0.5, 0.5 ) ) ), object.nodeUniform3 ) ) ), 1.0 ) );

	}


	// result

	output.color = nodeVar2;

	return output;

}
