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

struct objectStruct {
	nodeUniform1 : mat3x3<f32>,
	nodeUniform2 : f32
};
@binding( 2 ) @group( 0 )
var<uniform> object : objectStruct;

// vars
var<private> nodeVar2 : vec4<f32>;

// codes


@fragment
fn main( @location( 0 ) nodeVarying0 : vec2<f32> ) -> OutputStruct {

	// flow
	// code

	var nodeVar0 : vec4<f32> = vec4<f32>( 0.0, 0.0, 0.0, 0.0 );
	var nodeVar1 : i32 = 0;

	for ( var i : i32 = i32( ( - 1.0 ) ); i <= 1; i ++ ) {


		for ( var j : i32 = i32( ( - 1.0 ) ); j <= 1; j ++ ) {

			nodeVar2 = textureSample( nodeUniform0, nodeUniform0_sampler, ( object.nodeUniform1 * vec3<f32>( ( nodeVarying0 + ( ( vec2<f32>( f32( i ), f32( j ) ) * ( vec2<f32>( 1.0, 1.0 ) / vec2<f32>( textureDimensions( nodeUniform0, 0 ) ) ) ) * vec2<f32>( max( object.nodeUniform2, 1.0 ) ) ) ), 1.0 ) ).xy );
			nodeVar0 = ( nodeVar0 + nodeVar2 );
			nodeVar1 = ( nodeVar1 + 1 );

		}


	}

	nodeVar0 = ( nodeVar0 / vec4<f32>( f32( nodeVar1 ) ) );

	// result

	output.color = nodeVar0;

	return output;

}
