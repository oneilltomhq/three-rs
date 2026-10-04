// Three.js r187dev - Node System

// global
diagnostic( off, derivative_uniformity );


// directives


// structs

struct OutputStruct {
	@location( 0 ) color: f32
};
var<private> output : OutputStruct;

// uniforms
@binding( 1 ) @group( 0 ) var nodeUniform2 : texture_depth_2d;
@binding( 2 ) @group( 0 ) var nodeUniform3_sampler : sampler;
@binding( 3 ) @group( 0 ) var nodeUniform3 : texture_2d<f32>;

struct objectStruct {
	nodeUniform0 : f32,
	nodeUniform1 : f32,
	nodeUniform4 : mat3x3<f32>,
	nodeUniform5 : vec2<f32>,
	nodeUniform6 : f32,
	nodeUniform7 : f32
};
@binding( 0 ) @group( 0 )
var<uniform> object : objectStruct;

// vars
var<private> nodeVar0 : f32;
var<private> nodeVar1 : vec2<u32>;
var<private> nodeVar2 : f32;
var<private> nodeVar3 : f32;
var<private> nodeVar4 : f32;
var<private> nodeVar5 : f32;
var<private> nodeVar6 : f32;

// codes
fn tsl_clampWrapping_float( coord: f32 ) -> f32 { return clamp( coord, 0.0, 1.0 ); }
fn tsl_coord_clampS_clampT_2d( coord : vec2f ) -> vec2f {

	return vec2f(
		tsl_clampWrapping_float( coord.x ),
		tsl_clampWrapping_float( coord.y )
	);

}



@fragment
fn main( @location( 0 ) nodeVarying0 : vec2<f32> ) -> OutputStruct {

	// flow
	// code

	nodeVar1 = textureDimensions( nodeUniform2, u32( 0 ) );
	nodeVar0 = textureLoad( nodeUniform2, vec2<u32>( clamp( floor( tsl_coord_clampS_clampT_2d( nodeVarying0 ) * vec2<f32>( nodeVar1 ) ), vec2<f32>( 0 ), vec2<f32>( nodeVar1 - vec2<u32>( 1, 1 ) ) ) ), u32( 0 ) );
	nodeVar2 = ( ( object.nodeUniform0 * object.nodeUniform1 ) / ( ( ( object.nodeUniform1 - object.nodeUniform0 ) * nodeVar0 ) - object.nodeUniform1 ) );
	nodeVar3 = 0.0;
	nodeVar4 = 0.0;

	for ( var i : i32 = -2; i < 3; i ++ ) {

		let nodeConst0 = f32( i );
		let nodeConst1 = ( nodeVarying0 + ( object.nodeUniform5 * vec2<f32>( nodeConst0 ) ) );
		nodeVar5 = textureSample( nodeUniform3, nodeUniform3_sampler, ( object.nodeUniform4 * vec3<f32>( nodeConst1, 1.0 ) ).xy ).x;
		nodeVar6 = textureLoad( nodeUniform2, vec2<u32>( clamp( floor( tsl_coord_clampS_clampT_2d( nodeConst1 ) * vec2<f32>( nodeVar1 ) ), vec2<f32>( 0 ), vec2<f32>( nodeVar1 - vec2<u32>( 1, 1 ) ) ) ), u32( 0 ) );
		let nodeConst2 = ( exp( ( ( nodeConst0 * nodeConst0 ) * -0.5 ) ) * exp( ( - ( ( abs( ( ( ( object.nodeUniform0 * object.nodeUniform1 ) / ( ( ( object.nodeUniform1 - object.nodeUniform0 ) * nodeVar6 ) - object.nodeUniform1 ) ) - nodeVar2 ) ) / object.nodeUniform6 ) * object.nodeUniform7 ) ) ) );
		nodeVar3 = ( nodeVar3 + ( nodeVar5 * nodeConst2 ) );
		nodeVar4 = ( nodeVar4 + nodeConst2 );

	}


	// result

	output.color = ( nodeVar3 / max( nodeVar4, 0.0001 ) );

	return output;

}
