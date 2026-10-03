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
@binding( 1 ) @group( 1 ) var nodeUniform2 : texture_2d<f32>;
@binding( 2 ) @group( 1 ) var nodeUniform5 : texture_depth_2d;
@binding( 3 ) @group( 1 ) var nodeUniform7_sampler : sampler;
@binding( 4 ) @group( 1 ) var nodeUniform7 : texture_2d<f32>;

struct objectStruct {
	nodeUniform0 : vec3<f32>,
	nodeUniform1 : f32,
	nodeUniform9 : mat4x4<f32>
};
@binding( 0 ) @group( 1 )
var<uniform> object : objectStruct;

struct renderStruct {
	cameraNear : f32,
	cameraFar : f32,
	cameraProjectionMatrix : mat4x4<f32>,
	cameraViewMatrix : mat4x4<f32>,
	nodeUniform6 : vec2<f32>
};
@binding( 0 ) @group( 0 )
var<uniform> render : renderStruct;

// vars
var<private> DiffuseColor : vec4<f32>;
var<private> Output : vec4<f32>;
var<private> indirectDiffuse : vec3<f32>;
var<private> ambientOcclusion : f32;
var<private> totalDiffuse : vec3<f32>;
var<private> nodeVar0 : vec2<f32>;
var<private> nodeVar1 : vec4<f32>;
var<private> nodeVar2 : f32;
var<private> nodeVar3 : vec2<u32>;
var<private> nodeVar4 : vec4<f32>;
var<private> nodeVar5 : vec2<u32>;
var<private> totalSpecular : vec3<f32>;
var<private> directSpecular : vec3<f32>;
var<private> indirectSpecular : vec3<f32>;
var<private> outgoingLight : vec3<f32>;

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
	@location( 1 ) nodeVarying4 : vec2<f32>,
	@builtin( position ) fragCoord : vec4<f32> ) -> OutputStruct {

	// flow
	// code

	DiffuseColor = vec4<f32>( object.nodeUniform0, 1.0 );
	DiffuseColor.w = ( DiffuseColor.w * object.nodeUniform1 );
	indirectDiffuse = vec3<f32>( 0.0, 0.0, 0.0 );
	indirectDiffuse = vec4<f32>( 0.0, 0.0, 0.0, 0.0 ).xyz;
	indirectDiffuse = ( vec4<f32>( indirectDiffuse, 1.0 ) + vec4<f32>( 1.0, 1.0, 1.0, 0.0 ) ).xyz;
	ambientOcclusion = 1.0;
	indirectDiffuse = ( indirectDiffuse * vec3<f32>( ambientOcclusion ) );
	indirectDiffuse = ( indirectDiffuse * DiffuseColor.xyz );
	nodeVar1 = textureSample( nodeUniform7, nodeUniform7_sampler, ( nodeVarying4 * vec2<f32>( 5.0 ) ) );
	let nodeConst0 = ( ( fragCoord.xy / render.nodeUniform6 ) + ( ( ( nodeVar1.xy * vec2<f32>( 2.0 ) ) - vec2<f32>( 1.0 ) ) * vec2<f32>( 0.1 ) ) );
	nodeVar3 = textureDimensions( nodeUniform5, u32( 0 ) );
	nodeVar2 = textureLoad( nodeUniform5, vec2<u32>( clamp( floor( tsl_coord_clampS_clampT_2d( nodeConst0 ) * vec2<f32>( nodeVar3 ) ), vec2<f32>( 0 ), vec2<f32>( nodeVar3 - vec2<u32>( 1, 1 ) ) ) ), u32( 0 ) );

	if ( ( ( ( ( ( ( render.cameraNear * render.cameraFar ) / ( ( ( render.cameraFar - render.cameraNear ) * nodeVar2 ) - render.cameraFar ) ) + render.cameraNear ) / ( render.cameraNear - render.cameraFar ) ) - ( ( v_positionView.z + render.cameraNear ) / ( render.cameraNear - render.cameraFar ) ) ) < 0.0 ) ) {

		nodeVar0 = ( fragCoord.xy / render.nodeUniform6 );

	} else {

		nodeVar0 = nodeConst0;

	}

	nodeVar5 = textureDimensions( nodeUniform2, u32( 0 ) );
	nodeVar4 = textureLoad( nodeUniform2, vec2<u32>( clamp( floor( tsl_coord_clampS_clampT_2d( nodeVar0 ) * vec2<f32>( nodeVar5 ) ), vec2<f32>( 0 ), vec2<f32>( nodeVar5 - vec2<u32>( 1, 1 ) ) ) ), u32( 0 ) );
	totalDiffuse = nodeVar4.xyz;
	directSpecular = vec3<f32>( 0.0, 0.0, 0.0 );
	indirectSpecular = vec3<f32>( 0.0, 0.0, 0.0 );
	totalSpecular = ( directSpecular + indirectSpecular );
	outgoingLight = ( totalDiffuse + totalSpecular );
	let nodeConst1 = max( vec4<f32>( outgoingLight, DiffuseColor.w ), vec4<f32>( 0.0 ) );
	Output = nodeConst1;

	// result

	output.color = nodeConst1;

	return output;

}
