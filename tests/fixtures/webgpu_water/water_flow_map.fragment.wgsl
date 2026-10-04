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
@binding( 0 ) @group( 1 ) var nodeUniform0_sampler : sampler;
@binding( 1 ) @group( 1 ) var nodeUniform0 : texture_2d<f32>;
@binding( 3 ) @group( 1 ) var nodeUniform2_sampler : sampler;
@binding( 4 ) @group( 1 ) var nodeUniform2 : texture_2d<f32>;
@binding( 5 ) @group( 1 ) var nodeUniform6_sampler : sampler;
@binding( 6 ) @group( 1 ) var nodeUniform6 : texture_2d<f32>;
@binding( 7 ) @group( 1 ) var nodeUniform9 : texture_2d<f32>;
@binding( 8 ) @group( 1 ) var nodeUniform12 : texture_depth_2d;
@binding( 9 ) @group( 1 ) var nodeUniform16_sampler : sampler;
@binding( 10 ) @group( 1 ) var nodeUniform16 : texture_2d<f32>;

struct objectStruct {
	nodeUniform1 : mat3x3<f32>,
	nodeUniform3 : mat3x3<f32>,
	nodeUniform4 : f32,
	nodeUniform5 : vec3<f32>,
	nodeUniform7 : mat3x3<f32>,
	nodeUniform8 : vec3<f32>,
	nodeUniform15 : mat4x4<f32>,
	nodeUniform18 : f32,
	nodeUniform19 : f32
};
@binding( 2 ) @group( 1 )
var<uniform> object : objectStruct;

struct renderStruct {
	cameraNear : f32,
	cameraFar : f32,
	cameraProjectionMatrix : mat4x4<f32>,
	cameraViewMatrix : mat4x4<f32>,
	nodeUniform13 : vec2<f32>,
	cameraPosition : vec3<f32>
};
@binding( 0 ) @group( 0 )
var<uniform> render : renderStruct;

// vars
var<private> DiffuseColor : vec4<f32>;
var<private> nodeVar0 : vec4<f32>;
var<private> nodeVar2 : vec4<f32>;
var<private> nodeVar3 : vec4<f32>;
var<private> nodeVar4 : vec2<f32>;
var<private> nodeVar5 : vec2<f32>;
var<private> nodeVar6 : f32;
var<private> nodeVar7 : vec2<u32>;
var<private> nodeVar8 : vec4<f32>;
var<private> nodeVar9 : vec2<u32>;
var<private> nodeVar10 : vec2<f32>;
var<private> nodeVar11 : vec4<f32>;
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
	@location( 1 ) v_positionWorld : vec3<f32>,
	@location( 2 ) nodeVarying5 : vec2<f32>,
	@builtin( position ) fragCoord : vec4<f32> ) -> OutputStruct {

	// flow
	// code

	nodeVar0 = textureSample( nodeUniform0, nodeUniform0_sampler, ( object.nodeUniform1 * vec3<f32>( nodeVarying5, 1.0 ) ).xy );
	var nodeVar1 : vec2<f32> = ( ( nodeVar0.xy * vec2<f32>( 2.0 ) ) - vec2<f32>( 1.0 ) );
	nodeVar1.x = ( nodeVar1.x * -1.0 );
	nodeVar2 = textureSample( nodeUniform2, nodeUniform2_sampler, ( object.nodeUniform3 * vec3<f32>( ( ( nodeVarying5 * vec2<f32>( object.nodeUniform4 ) ) + ( nodeVar1 * vec2<f32>( object.nodeUniform5.x ) ) ), 1.0 ) ).xy );
	nodeVar3 = textureSample( nodeUniform6, nodeUniform6_sampler, ( object.nodeUniform7 * vec3<f32>( ( ( nodeVarying5 * vec2<f32>( object.nodeUniform4 ) ) + ( nodeVar1 * vec2<f32>( object.nodeUniform5.y ) ) ), 1.0 ) ).xy );
	let nodeConst0 = mix( nodeVar2, nodeVar3, ( abs( ( object.nodeUniform5.z - object.nodeUniform5.x ) ) / object.nodeUniform5.z ) );
	let nodeConst1 = normalize( vec3<f32>( ( ( nodeConst0.x * 2.0 ) - 1.0 ), nodeConst0.z, ( ( nodeConst0.y * 2.0 ) - 1.0 ) ) );
	nodeVar4 = ( nodeConst1.xz * vec2<f32>( 0.05 ) );
	let nodeConst2 = ( ( fragCoord.xy / render.nodeUniform13 ) + nodeVar4 );
	nodeVar7 = textureDimensions( nodeUniform12, u32( 0 ) );
	nodeVar6 = textureLoad( nodeUniform12, vec2<u32>( clamp( floor( tsl_coord_clampS_clampT_2d( nodeConst2 ) * vec2<f32>( nodeVar7 ) ), vec2<f32>( 0 ), vec2<f32>( nodeVar7 - vec2<u32>( 1, 1 ) ) ) ), u32( 0 ) );

	if ( ( ( ( ( ( ( render.cameraNear * render.cameraFar ) / ( ( ( render.cameraFar - render.cameraNear ) * nodeVar6 ) - render.cameraFar ) ) + render.cameraNear ) / ( render.cameraNear - render.cameraFar ) ) - ( ( v_positionView.z + render.cameraNear ) / ( render.cameraNear - render.cameraFar ) ) ) < 0.0 ) ) {

		nodeVar5 = ( fragCoord.xy / render.nodeUniform13 );

	} else {

		nodeVar5 = nodeConst2;

	}

	nodeVar9 = textureDimensions( nodeUniform9, u32( 0 ) );
	nodeVar8 = textureLoad( nodeUniform9, vec2<u32>( clamp( floor( tsl_coord_clampS_clampT_2d( nodeVar5 ) * vec2<f32>( nodeVar9 ) ), vec2<f32>( 0 ), vec2<f32>( nodeVar9 - vec2<u32>( 1, 1 ) ) ) ), u32( 0 ) );
	nodeVar10 = ( fragCoord.xy / render.nodeUniform13 );
	nodeVar11 = textureSample( nodeUniform16, nodeUniform16_sampler, ( vec2<f32>( 1.0 - nodeVar10.x, nodeVar10.y ) + nodeVar4 ) );
	DiffuseColor = ( vec4<f32>( object.nodeUniform8, 1.0 ) * mix( nodeVar8, nodeVar11, ( ( pow( ( 1.0 - max( dot( normalize( ( render.cameraPosition - v_positionWorld ) ), nodeConst1 ), 0.0 ) ), 5.0 ) * ( 1.0 - object.nodeUniform18 ) ) + object.nodeUniform18 ) ) );
	DiffuseColor.w = ( DiffuseColor.w * object.nodeUniform19 );
	let nodeConst3 = max( vec4<f32>( DiffuseColor.xyz, DiffuseColor.w ), vec4<f32>( 0.0 ) );
	Output = nodeConst3;

	// result

	output.color = nodeConst3;

	return output;

}
