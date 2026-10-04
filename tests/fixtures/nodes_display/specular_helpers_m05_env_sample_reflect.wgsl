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
@binding( 0 ) @group( 0 ) var nodeUniform0 : texture_2d<f32>;

struct objectStruct {
	nodeUniform1 : mat4x4<f32>,
	nodeUniform2 : f32
};
@binding( 1 ) @group( 0 )
var<uniform> object : objectStruct;

// vars
var<private> nodeVar0 : vec4<f32>;
var<private> nodeVar1 : vec2<u32>;

// codes
fn tsl_repeatWrapping_float( coord: f32 ) -> f32 { return fract( coord ); }
fn tsl_clampWrapping_float( coord: f32 ) -> f32 { return clamp( coord, 0.0, 1.0 ); }
fn tsl_coord_repeatS_clampT_2d( coord : vec2f ) -> vec2f {

	return vec2f(
		tsl_repeatWrapping_float( coord.x ),
		tsl_clampWrapping_float( coord.y )
	);

}



@fragment
fn main( @location( 0 ) nodeVarying0 : vec2<f32> ) -> OutputStruct {

	// flow
	// code

	let nodeConst0 = normalize( ( object.nodeUniform1 * vec4<f32>( normalize( vec3<f32>( ( nodeVarying0 - vec2<f32>( 0.5 ) ), -1.0 ) ), 0.0 ) ).xyz );
	nodeVar1 = textureDimensions( nodeUniform0, u32( 0.0 ) );
	nodeVar0 = textureLoad( nodeUniform0, vec2<u32>( clamp( floor( tsl_coord_repeatS_clampT_2d( vec2<f32>( ( ( atan2( nodeConst0.z, nodeConst0.x ) * 0.15915494309189535 ) + 0.5 ), ( ( asin( clamp( nodeConst0.y, -1.0, 1.0 ) ) * 0.3183098861837907 ) + 0.5 ) ) ) * vec2<f32>( nodeVar1 ) ), vec2<f32>( 0 ), vec2<f32>( nodeVar1 - vec2<u32>( 1, 1 ) ) ) ), u32( 0.0 ) );

	// result

	output.color = vec4<f32>( ( ( nodeVar0.xyz * vec3<f32>( object.nodeUniform2 ) ) * vec3<f32>( 1.0 ) ), 1.0 );

	return output;

}
