// Three.js r187dev - Node System

// global
diagnostic( off, derivative_uniformity );


// directives


// structs

struct OutputType {
	@location( 0 ) m0 : vec4<f32>,
	@location( 1 ) m1 : vec4<f32>,
	
};
var<private> output : OutputType;

// uniforms
@binding( 1 ) @group( 1 ) var nodeUniform1_sampler : sampler;
@binding( 2 ) @group( 1 ) var nodeUniform1 : texture_2d<f32>;
@binding( 3 ) @group( 1 ) var nodeUniform5_sampler : sampler;
@binding( 4 ) @group( 1 ) var nodeUniform5 : texture_2d<f32>;
@binding( 5 ) @group( 1 ) var nodeUniform8 : texture_2d<f32>;
@binding( 6 ) @group( 1 ) var nodeUniform11 : texture_depth_2d;
@binding( 7 ) @group( 1 ) var nodeUniform15_sampler : sampler;
@binding( 8 ) @group( 1 ) var nodeUniform15 : texture_2d<f32>;

struct objectStruct {
	nodeUniform0 : vec2<f32>,
	nodeUniform2 : mat3x3<f32>,
	nodeUniform3 : f32,
	nodeUniform4 : vec3<f32>,
	nodeUniform6 : mat3x3<f32>,
	nodeUniform7 : vec3<f32>,
	nodeUniform14 : mat4x4<f32>,
	nodeUniform17 : f32,
	nodeUniform18 : f32
};
@binding( 0 ) @group( 1 )
var<uniform> object : objectStruct;

struct renderStruct {
	cameraNear : f32,
	cameraFar : f32,
	cameraProjectionMatrix : mat4x4<f32>,
	cameraViewMatrix : mat4x4<f32>,
	cameraPosition : vec3<f32>,
	nodeUniform12 : vec2<f32>
};
@binding( 0 ) @group( 0 )
var<uniform> render : renderStruct;

// vars
var<private> DiffuseColor : vec4<f32>;
var<private> nodeVar1 : vec4<f32>;
var<private> nodeVar2 : vec4<f32>;
var<private> nodeVar3 : vec2<f32>;
var<private> nodeVar4 : vec2<f32>;
var<private> nodeVar5 : f32;
var<private> nodeVar6 : vec2<u32>;
var<private> nodeVar7 : vec4<f32>;
var<private> nodeVar8 : vec2<u32>;
var<private> nodeVar9 : vec2<f32>;
var<private> nodeVar10 : vec4<f32>;
var<private> Output : vec4<f32>;
var<private> EmissiveColor : vec3<f32>;

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
	@location( 1 ) v_positionWorld : vec3<f32>,
	@location( 2 ) nodeVarying5 : vec2<f32>,
	@builtin( position ) fragCoord : vec4<f32> ) -> OutputType {

	// flow
	// code

	var nodeVar0 : vec2<f32> = vec2<f32>( object.nodeUniform0.x, object.nodeUniform0.y );
	nodeVar0.x = ( nodeVar0.x * -1.0 );
	nodeVar1 = textureSample( nodeUniform1, nodeUniform1_sampler, ( object.nodeUniform2 * vec3<f32>( ( ( nodeVarying5 * vec2<f32>( object.nodeUniform3 ) ) + ( nodeVar0 * vec2<f32>( object.nodeUniform4.x ) ) ), 1.0 ) ).xy );
	nodeVar2 = textureSample( nodeUniform5, nodeUniform5_sampler, ( object.nodeUniform6 * vec3<f32>( ( ( nodeVarying5 * vec2<f32>( object.nodeUniform3 ) ) + ( nodeVar0 * vec2<f32>( object.nodeUniform4.y ) ) ), 1.0 ) ).xy );
	let nodeConst0 = mix( nodeVar1, nodeVar2, ( abs( ( object.nodeUniform4.z - object.nodeUniform4.x ) ) / object.nodeUniform4.z ) );
	let nodeConst1 = normalize( vec3<f32>( ( ( nodeConst0.x * 2.0 ) - 1.0 ), nodeConst0.z, ( ( nodeConst0.y * 2.0 ) - 1.0 ) ) );
	nodeVar3 = ( nodeConst1.xz * vec2<f32>( 0.05 ) );
	let nodeConst2 = ( ( fragCoord.xy / render.nodeUniform12 ) + nodeVar3 );
	nodeVar6 = textureDimensions( nodeUniform11, u32( 0 ) );
	nodeVar5 = textureLoad( nodeUniform11, vec2<u32>( clamp( floor( tsl_coord_clampS_clampT_2d( nodeConst2 ) * vec2<f32>( nodeVar6 ) ), vec2<f32>( 0 ), vec2<f32>( nodeVar6 - vec2<u32>( 1, 1 ) ) ) ), u32( 0 ) );

	if ( ( ( ( ( ( ( render.cameraNear * render.cameraFar ) / ( ( ( render.cameraFar - render.cameraNear ) * nodeVar5 ) - render.cameraFar ) ) + render.cameraNear ) / ( render.cameraNear - render.cameraFar ) ) - ( ( v_positionView.z + render.cameraNear ) / ( render.cameraNear - render.cameraFar ) ) ) < 0.0 ) ) {

		nodeVar4 = ( fragCoord.xy / render.nodeUniform12 );

	} else {

		nodeVar4 = nodeConst2;

	}

	nodeVar8 = textureDimensions( nodeUniform8, u32( 0 ) );
	nodeVar7 = textureLoad( nodeUniform8, vec2<u32>( clamp( floor( tsl_coord_clampS_clampT_2d( nodeVar4 ) * vec2<f32>( nodeVar8 ) ), vec2<f32>( 0 ), vec2<f32>( nodeVar8 - vec2<u32>( 1, 1 ) ) ) ), u32( 0 ) );
	nodeVar9 = ( fragCoord.xy / render.nodeUniform12 );
	nodeVar10 = textureSample( nodeUniform15, nodeUniform15_sampler, ( vec2<f32>( 1.0 - nodeVar9.x, nodeVar9.y ) + nodeVar3 ) );
	DiffuseColor = ( vec4<f32>( object.nodeUniform7, 1.0 ) * mix( nodeVar7, nodeVar10, ( ( pow( ( 1.0 - max( dot( normalize( ( render.cameraPosition - v_positionWorld ) ), nodeConst1 ), 0.0 ) ), 5.0 ) * ( 1.0 - object.nodeUniform17 ) ) + object.nodeUniform17 ) ) );
	DiffuseColor.w = ( DiffuseColor.w * object.nodeUniform18 );
	Output = max( vec4<f32>( DiffuseColor.xyz, DiffuseColor.w ), vec4<f32>( 0.0 ) );
	output.m0 = Output;
	output.m1 = vec4<f32>( EmissiveColor, 1.0 );

	// result

	return output;

}
