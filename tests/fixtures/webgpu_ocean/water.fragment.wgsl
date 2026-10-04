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
@binding( 1 ) @group( 1 ) var nodeUniform5_sampler : sampler;
@binding( 2 ) @group( 1 ) var nodeUniform5 : texture_2d<f32>;
@binding( 3 ) @group( 1 ) var nodeUniform9_sampler : sampler;
@binding( 4 ) @group( 1 ) var nodeUniform9 : texture_2d<f32>;

struct objectStruct {
	nodeUniform0 : vec3<f32>,
	nodeUniform1 : vec3<f32>,
	nodeUniform2 : mat4x4<f32>,
	nodeUniform3 : f32,
	nodeUniform6 : mat3x3<f32>,
	nodeUniform8 : vec3<f32>,
	nodeUniform11 : f32,
	nodeUniform12 : f32
};
@binding( 0 ) @group( 1 )
var<uniform> object : objectStruct;

struct renderStruct {
	nodeUniform4 : f32,
	cameraProjectionMatrix : mat4x4<f32>,
	cameraViewMatrix : mat4x4<f32>,
	cameraPosition : vec3<f32>,
	nodeUniform10 : vec2<f32>
};
@binding( 0 ) @group( 0 )
var<uniform> render : renderStruct;

// vars
var<private> DiffuseColor : vec4<f32>;
var<private> nodeVar0 : vec2<f32>;
var<private> nodeVar1 : vec2<f32>;
var<private> nodeVar2 : vec2<f32>;
var<private> nodeVar3 : vec2<f32>;
var<private> nodeVar4 : vec4<f32>;
var<private> nodeVar5 : vec4<f32>;
var<private> nodeVar6 : vec4<f32>;
var<private> nodeVar7 : vec4<f32>;
var<private> nodeVar8 : vec2<f32>;
var<private> nodeVar9 : vec4<f32>;
var<private> Output : vec4<f32>;

// codes


@fragment
fn main( @location( 0 ) v_positionWorld : vec3<f32>,
	@builtin( position ) fragCoord : vec4<f32> ) -> OutputStruct {

	// flow
	// code

	let nodeConst0 = ( v_positionWorld.xz * vec2<f32>( object.nodeUniform3 ) );
	nodeVar0 = ( ( nodeConst0 / vec2<f32>( 103.0 ) ) + vec2<f32>( ( render.nodeUniform4 / 17.0 ), ( render.nodeUniform4 / 29.0 ) ) );
	nodeVar1 = ( ( nodeConst0 / vec2<f32>( 107.0 ) ) - vec2<f32>( ( render.nodeUniform4 / -19.0 ), ( render.nodeUniform4 / 31.0 ) ) );
	nodeVar2 = ( ( nodeConst0 / vec2<f32>( 8907.0, 9803.0 ) ) + vec2<f32>( ( render.nodeUniform4 / 101.0 ), ( render.nodeUniform4 / 97.0 ) ) );
	nodeVar3 = ( ( nodeConst0 / vec2<f32>( 1091.0, 1027.0 ) ) - vec2<f32>( ( render.nodeUniform4 / 109.0 ), ( render.nodeUniform4 / -113.0 ) ) );
	nodeVar4 = textureSample( nodeUniform5, nodeUniform5_sampler, ( object.nodeUniform6 * vec3<f32>( nodeVar0, 1.0 ) ).xy );
	nodeVar5 = textureSample( nodeUniform5, nodeUniform5_sampler, ( object.nodeUniform6 * vec3<f32>( nodeVar1, 1.0 ) ).xy );
	nodeVar6 = textureSample( nodeUniform5, nodeUniform5_sampler, ( object.nodeUniform6 * vec3<f32>( nodeVar2, 1.0 ) ).xy );
	nodeVar7 = textureSample( nodeUniform5, nodeUniform5_sampler, ( object.nodeUniform6 * vec3<f32>( nodeVar3, 1.0 ) ).xy );
	let nodeConst1 = normalize( ( ( ( ( ( ( ( ( nodeVar4 + nodeVar5 ) + nodeVar6 ) + nodeVar7 ) * vec4<f32>( 0.5 ) ) - vec4<f32>( 1.0 ) ).xzy * vec3<f32>( 1.5 ) ) * vec3<f32>( 1.0 ) ) * vec3<f32>( 1.5 ) ) );
	let nodeConst2 = ( render.cameraPosition - v_positionWorld );
	let nodeConst3 = normalize( nodeConst2 );
	nodeVar8 = ( fragCoord.xy / render.nodeUniform10 );
	nodeVar9 = textureSample( nodeUniform9, nodeUniform9_sampler, ( vec2<f32>( 1.0 - nodeVar8.x, nodeVar8.y ) + ( ( nodeConst1.xz * vec2<f32>( ( 0.001 + ( 1.0 / length( nodeConst2 ) ) ) ) ) * vec2<f32>( object.nodeUniform11 ) ) ) );
	DiffuseColor = vec4<f32>( mix( ( ( ( object.nodeUniform0 * ( ( vec3<f32>( max( dot( object.nodeUniform1, nodeConst1 ), 0.0 ) ) * object.nodeUniform0 ) * vec3<f32>( 0.5 ) ) ) * vec3<f32>( 0.3 ) ) + ( vec3<f32>( max( 0.0, dot( nodeConst1, nodeConst3 ) ) ) * object.nodeUniform8 ) ), ( nodeVar9.xyz + ( ( vec3<f32>( pow( max( 0.0, dot( nodeConst3, normalize( reflect( ( - object.nodeUniform1 ), nodeConst1 ) ) ) ), 100.0 ) ) * object.nodeUniform0 ) * vec3<f32>( 2.0 ) ) ), ( ( pow( ( 1.0 - max( dot( nodeConst3, nodeConst1 ), 0.0 ) ), 5.0 ) * ( 1.0 - 0.02 ) ) + 0.02 ) ), 1.0 );
	DiffuseColor.w = ( DiffuseColor.w * object.nodeUniform12 );
	let nodeConst4 = max( vec4<f32>( DiffuseColor.xyz, DiffuseColor.w ), vec4<f32>( 0.0 ) );
	Output = nodeConst4;

	// result

	output.color = nodeConst4;

	return output;

}
