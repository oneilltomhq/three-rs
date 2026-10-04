// Three.js r187dev - Node System

// global
diagnostic( off, derivative_uniformity );


// directives


// structs

struct OutputType {
	@location( 0 ) m0 : f32,
	@location( 1 ) m1 : f32,
	
};
var<private> output : OutputType;

// uniforms
@binding( 1 ) @group( 0 ) var nodeUniform2 : texture_depth_2d;

struct objectStruct {
	nodeUniform0 : f32,
	nodeUniform1 : f32,
	nodeUniform3 : f32,
	nodeUniform4 : f32,
	nodeUniform5 : f32
};
@binding( 0 ) @group( 0 )
var<uniform> object : objectStruct;

// vars
var<private> DiffuseColor : vec4<f32>;
var<private> nodeVar0 : f32;
var<private> nodeVar1 : f32;
var<private> nodeVar2 : vec2<u32>;
var<private> nodeVar3 : f32;
var<private> Output : f32;

// codes
fn tsl_clampWrapping_float( coord: f32 ) -> f32 { return clamp( coord, 0.0, 1.0 ); }
fn tsl_coord_clampS_clampT_2d( coord : vec2f ) -> vec2f {

	return vec2f(
		tsl_clampWrapping_float( coord.x ),
		tsl_clampWrapping_float( coord.y )
	);

}



@fragment
fn main( @location( 0 ) nodeVarying0 : vec2<f32> ) -> OutputType {

	// flow
	// code

	nodeVar2 = textureDimensions( nodeUniform2, u32( 0 ) );
	nodeVar1 = textureLoad( nodeUniform2, vec2<u32>( clamp( floor( tsl_coord_clampS_clampT_2d( nodeVarying0 ) * vec2<f32>( nodeVar2 ) ), vec2<f32>( 0 ), vec2<f32>( nodeVar2 - vec2<u32>( 1, 1 ) ) ) ), u32( 0 ) );
	let nodeConst0 = ( ( - ( ( object.nodeUniform0 * object.nodeUniform1 ) / ( ( ( object.nodeUniform1 - object.nodeUniform0 ) * nodeVar1 ) - object.nodeUniform1 ) ) ) - object.nodeUniform3 );
	let nodeConst1 = smoothstep( 0.0, object.nodeUniform4, abs( nodeConst0 ) );
	nodeVar0 = ( step( nodeConst0, 0.0 ) * nodeConst1 );
	nodeVar3 = ( step( 0.0, nodeConst0 ) * nodeConst1 );
	DiffuseColor = vec4<f32>( 0.0 );
	DiffuseColor.w = ( DiffuseColor.w * object.nodeUniform5 );
	DiffuseColor.w = 1.0;
	Output = max( vec4<f32>( DiffuseColor.xyz, DiffuseColor.w ), vec4<f32>( 0.0 ) ).x;
	output.m0 = nodeVar0;
	output.m1 = nodeVar3;

	// result

	return output;

}
