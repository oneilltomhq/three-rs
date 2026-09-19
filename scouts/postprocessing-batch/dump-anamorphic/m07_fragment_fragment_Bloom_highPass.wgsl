// Three.js r186 - Node System

// global
diagnostic( off, derivative_uniformity );


// structs

struct OutputStruct {
	@location( 0 ) color: vec4<f32>
};
var<private> output : OutputStruct;

// uniforms
@binding( 1 ) @group( 1 ) var nodeUniform1_sampler : sampler;
@binding( 2 ) @group( 1 ) var nodeUniform1 : texture_2d<f32>;

struct objectStruct {
	nodeUniform0 : f32
};
@binding( 0 ) @group( 1 )
var<uniform> object : objectStruct;

struct renderStruct {
	nodeUniform2 : vec4<f32>
};
@binding( 0 ) @group( 0 )
var<uniform> render : renderStruct;

// vars
var<private> nodeVar0 : vec4<f32>;
var<private> nodeVar1 : f32;
var<private> nodeVar2 : vec4<f32>;

// codes


@fragment
fn main( @location( 0 ) nodeVarying0 : vec2<f32> ) -> OutputStruct {

	// flow
	// code

	nodeVar0 = vec4<f32>( 0.0, 0.0, 0.0, 0.0 );
	nodeVar1 = ( object.nodeUniform0 / 2.0 );

	for ( var i : i32 = i32( ( - nodeVar1 ) ); i < i32( nodeVar1 ); i ++ ) {

		nodeVar2 = textureSample( nodeUniform1, nodeUniform1_sampler, vec2<f32>( ( nodeVarying0.x + ( ( ( vec2<f32>( 1.0, 1.0 ) / render.nodeUniform2.zw ).x * f32( i ) ) * 4.0 ) ), nodeVarying0.y ) );
		nodeVar0 = ( nodeVar0 + ( nodeVar2 * vec4<f32>( pow( ( 1.0 - ( abs( f32( i ) ) / nodeVar1 ) ), 2.0 ) ) ) );

	}


	// result

	output.color = ( nodeVar0 / vec4<f32>( ( object.nodeUniform0 / 3.0 ) ) );

	return output;

}
