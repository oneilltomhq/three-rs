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
@binding( 1 ) @group( 1 ) var nodeUniform4 : texture_depth_2d;

struct objectStruct {
	nodeUniform1 : mat4x4<f32>,
	nodeUniform2 : f32,
	nodeUniform3 : f32,
	nodeUniform5 : mat3x3<f32>,
	nodeUniform7 : f32
};
@binding( 0 ) @group( 1 )
var<uniform> object : objectStruct;

struct renderStruct {
	cameraProjectionMatrix : mat4x4<f32>,
	cameraViewMatrix : mat4x4<f32>,
	nodeUniform6 : vec2<f32>
};
@binding( 0 ) @group( 0 )
var<uniform> render : renderStruct;

// vars
var<private> DiffuseColor : vec4<f32>;
var<private> nodeVar0 : f32;
var<private> nodeVar1 : f32;
var<private> nodeVar2 : vec2<u32>;
var<private> Output : vec4<f32>;

// codes
fn tsl_clampWrapping_float( coord: f32 ) -> f32 { return clamp( coord, 0.0, 1.0 ); }
fn tsl_coord_clampS_clampT_2d( coord : vec2f ) -> vec2f {

	return vec2f(
		tsl_clampWrapping_float( coord.x ),
		tsl_clampWrapping_float( coord.y )
	);

}



@fragment
fn main( @location( 0 ) v_positionView : vec3<f32>,
	@builtin( position ) fragCoord : vec4<f32> ) -> OutputStruct {

	// flow
	// code

	nodeVar2 = textureDimensions( nodeUniform4, u32( 0 ) );
	nodeVar1 = textureLoad( nodeUniform4, vec2<u32>( clamp( floor( tsl_coord_clampS_clampT_2d( ( object.nodeUniform5 * vec3<f32>( ( fragCoord.xy / render.nodeUniform6 ), 1.0 ) ).xy ) * vec2<f32>( nodeVar2 ) ), vec2<f32>( 0 ), vec2<f32>( nodeVar2 - vec2<u32>( 1, 1 ) ) ) ), u32( 0 ) );

	if ( ( v_positionView.z <= ( ( object.nodeUniform2 * object.nodeUniform3 ) / ( ( ( object.nodeUniform3 - object.nodeUniform2 ) * nodeVar1 ) - object.nodeUniform3 ) ) ) ) {

		nodeVar0 = 1.0;

	} else {

		nodeVar0 = 0.0;

	}

	DiffuseColor = vec4<f32>( vec3<f32>( 0.0, nodeVar0, 1.0 ), 1.0 );
	DiffuseColor.w = ( DiffuseColor.w * object.nodeUniform7 );
	DiffuseColor.w = 1.0;
	let nodeConst0 = max( vec4<f32>( DiffuseColor.xyz, DiffuseColor.w ), vec4<f32>( 0.0 ) );
	Output = nodeConst0;

	// result

	output.color = nodeConst0;

	return output;

}
