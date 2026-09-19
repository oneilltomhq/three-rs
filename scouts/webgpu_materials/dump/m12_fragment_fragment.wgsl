// Three.js r186 - Node System

// global
diagnostic( off, derivative_uniformity );


// structs

struct OutputStruct {
	@location( 0 ) color: vec4<f32>
};
var<private> output : OutputStruct;

// uniforms
@binding( 0 ) @group( 1 ) var nodeUniform0_sampler : sampler;
@binding( 1 ) @group( 1 ) var nodeUniform0 : texture_2d<f32>;

struct objectStruct {
	nodeUniform1 : f32,
	nodeUniform4 : mat4x4<f32>
};
@binding( 2 ) @group( 1 )
var<uniform> object : objectStruct;

// vars
var<private> DiffuseColor : vec4<f32>;
var<private> nodeVar0 : vec4<f32>;
var<private> normalLocal : vec3<f32>;
var<private> nodeVar1 : vec3<f32>;
var<private> nodeVar2 : vec3<f32>;
var<private> nodeVar3 : vec4<f32>;
var<private> nodeVar4 : vec4<f32>;
var<private> Output : vec4<f32>;
var<private> nodeVar5 : vec4<f32>;

// codes


@fragment
fn main( @location( 0 ) positionLocal : vec3<f32>,
	@location( 1 ) nodeVarying4 : vec3<f32> ) -> OutputStruct {

	// flow
	// code

	nodeVar0 = textureSample( nodeUniform0, nodeUniform0_sampler, ( positionLocal.yz * vec2<f32>( 0.01 ) ) );
	normalLocal = nodeVarying4;
	nodeVar1 = normalize( abs( normalLocal ) );
	nodeVar2 = ( nodeVar1 / vec3<f32>( dot( nodeVar1, vec3<f32>( 1.0, 1.0, 1.0 ) ) ) );
	nodeVar3 = textureSample( nodeUniform0, nodeUniform0_sampler, ( positionLocal.zx * vec2<f32>( 0.01 ) ) );
	nodeVar4 = textureSample( nodeUniform0, nodeUniform0_sampler, ( positionLocal.xy * vec2<f32>( 0.01 ) ) );
	DiffuseColor = ( ( ( nodeVar0 * vec4<f32>( nodeVar2.x ) ) + ( nodeVar3 * vec4<f32>( nodeVar2.y ) ) ) + ( nodeVar4 * vec4<f32>( nodeVar2.z ) ) );
	DiffuseColor.w = ( DiffuseColor.w * object.nodeUniform1 );
	DiffuseColor.w = 1.0;
	nodeVar5 = max( vec4<f32>( DiffuseColor.xyz, DiffuseColor.w ), vec4<f32>( 0.0 ) );
	Output = nodeVar5;

	// result

	output.color = nodeVar5;

	return output;

}
