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
@binding( 0 ) @group( 0 ) var nodeUniform0_sampler : sampler;
@binding( 1 ) @group( 0 ) var nodeUniform0 : texture_2d<f32>;

struct objectStruct {
	nodeUniform1 : vec2<f32>
};
@binding( 2 ) @group( 0 )
var<uniform> object : objectStruct;

// vars
var<private> nodeVar0 : vec4<f32>;
var<private> nodeVar1 : vec4<f32>;
var<private> nodeVar2 : vec4<f32>;
var<private> nodeVar3 : vec4<f32>;
var<private> nodeVar4 : vec4<f32>;
var<private> nodeVar5 : vec4<f32>;
var<private> nodeVar6 : vec4<f32>;
var<private> nodeVar7 : vec4<f32>;
var<private> nodeVar8 : vec4<f32>;

// codes


@fragment
fn main( @location( 0 ) nodeVarying0 : vec2<f32> ) -> OutputStruct {

	// flow
	// code

	nodeVar0 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 + ( object.nodeUniform1 * vec2<f32>( -1.0, -1.0 ) ) ) );
	let nodeConst0 = dot( nodeVar0.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) );
	nodeVar1 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 + ( object.nodeUniform1 * vec2<f32>( 0.0, -1.0 ) ) ) );
	let nodeConst1 = dot( nodeVar1.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) );
	nodeVar2 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 + ( object.nodeUniform1 * vec2<f32>( 1.0, -1.0 ) ) ) );
	let nodeConst2 = dot( nodeVar2.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) );
	nodeVar3 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 + ( object.nodeUniform1 * vec2<f32>( -1.0, 0.0 ) ) ) );
	let nodeConst3 = dot( nodeVar3.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) );
	nodeVar4 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 + ( object.nodeUniform1 * vec2<f32>( 0.0, 0.0 ) ) ) );
	let nodeConst4 = dot( nodeVar4.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) );
	nodeVar5 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 + ( object.nodeUniform1 * vec2<f32>( 1.0, 0.0 ) ) ) );
	let nodeConst5 = dot( nodeVar5.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) );
	nodeVar6 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 + ( object.nodeUniform1 * vec2<f32>( -1.0, 1.0 ) ) ) );
	let nodeConst6 = dot( nodeVar6.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) );
	nodeVar7 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 + ( object.nodeUniform1 * vec2<f32>( 0.0, 1.0 ) ) ) );
	let nodeConst7 = dot( nodeVar7.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) );
	nodeVar8 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 + ( object.nodeUniform1 * vec2<f32>( 1.0, 1.0 ) ) ) );
	let nodeConst8 = dot( nodeVar8.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) );
	let nodeConst9 = ( ( ( ( ( ( ( ( ( mat3x3<f32>( -1.0, 0.0, 1.0, -2.0, 0.0, 2.0, -1.0, 0.0, 1.0 )[ 0u ][ 0u ] * nodeConst0 ) + ( mat3x3<f32>( -1.0, 0.0, 1.0, -2.0, 0.0, 2.0, -1.0, 0.0, 1.0 )[ 1u ][ 0u ] * nodeConst1 ) ) + ( mat3x3<f32>( -1.0, 0.0, 1.0, -2.0, 0.0, 2.0, -1.0, 0.0, 1.0 )[ 2u ][ 0u ] * nodeConst2 ) ) + ( mat3x3<f32>( -1.0, 0.0, 1.0, -2.0, 0.0, 2.0, -1.0, 0.0, 1.0 )[ 0u ][ 1u ] * nodeConst3 ) ) + ( mat3x3<f32>( -1.0, 0.0, 1.0, -2.0, 0.0, 2.0, -1.0, 0.0, 1.0 )[ 1u ][ 1u ] * nodeConst4 ) ) + ( mat3x3<f32>( -1.0, 0.0, 1.0, -2.0, 0.0, 2.0, -1.0, 0.0, 1.0 )[ 2u ][ 1u ] * nodeConst5 ) ) + ( mat3x3<f32>( -1.0, 0.0, 1.0, -2.0, 0.0, 2.0, -1.0, 0.0, 1.0 )[ 0u ][ 2u ] * nodeConst6 ) ) + ( mat3x3<f32>( -1.0, 0.0, 1.0, -2.0, 0.0, 2.0, -1.0, 0.0, 1.0 )[ 1u ][ 2u ] * nodeConst7 ) ) + ( mat3x3<f32>( -1.0, 0.0, 1.0, -2.0, 0.0, 2.0, -1.0, 0.0, 1.0 )[ 2u ][ 2u ] * nodeConst8 ) );
	let nodeConst10 = ( ( ( ( ( ( ( ( ( mat3x3<f32>( -1.0, -2.0, -1.0, 0.0, 0.0, 0.0, 1.0, 2.0, 1.0 )[ 0u ][ 0u ] * nodeConst0 ) + ( mat3x3<f32>( -1.0, -2.0, -1.0, 0.0, 0.0, 0.0, 1.0, 2.0, 1.0 )[ 1u ][ 0u ] * nodeConst1 ) ) + ( mat3x3<f32>( -1.0, -2.0, -1.0, 0.0, 0.0, 0.0, 1.0, 2.0, 1.0 )[ 2u ][ 0u ] * nodeConst2 ) ) + ( mat3x3<f32>( -1.0, -2.0, -1.0, 0.0, 0.0, 0.0, 1.0, 2.0, 1.0 )[ 0u ][ 1u ] * nodeConst3 ) ) + ( mat3x3<f32>( -1.0, -2.0, -1.0, 0.0, 0.0, 0.0, 1.0, 2.0, 1.0 )[ 1u ][ 1u ] * nodeConst4 ) ) + ( mat3x3<f32>( -1.0, -2.0, -1.0, 0.0, 0.0, 0.0, 1.0, 2.0, 1.0 )[ 2u ][ 1u ] * nodeConst5 ) ) + ( mat3x3<f32>( -1.0, -2.0, -1.0, 0.0, 0.0, 0.0, 1.0, 2.0, 1.0 )[ 0u ][ 2u ] * nodeConst6 ) ) + ( mat3x3<f32>( -1.0, -2.0, -1.0, 0.0, 0.0, 0.0, 1.0, 2.0, 1.0 )[ 1u ][ 2u ] * nodeConst7 ) ) + ( mat3x3<f32>( -1.0, -2.0, -1.0, 0.0, 0.0, 0.0, 1.0, 2.0, 1.0 )[ 2u ][ 2u ] * nodeConst8 ) );

	// result

	output.color = vec4<f32>( vec3<f32>( sqrt( ( ( nodeConst9 * nodeConst9 ) + ( nodeConst10 * nodeConst10 ) ) ) ), 1.0 );

	return output;

}
