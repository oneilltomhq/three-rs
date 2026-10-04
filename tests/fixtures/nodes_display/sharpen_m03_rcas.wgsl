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

// vars
var<private> nodeVar0 : vec4<f32>;
var<private> nodeVar1 : vec4<f32>;
var<private> nodeVar2 : vec4<f32>;
var<private> nodeVar3 : vec4<f32>;
var<private> nodeVar4 : vec4<f32>;
var<private> nodeVar5 : f32;

// codes


@fragment
fn main( @location( 0 ) nodeVarying0 : vec2<f32> ) -> OutputStruct {

	// flow
	// code

	let nodeConst0 = vec2<i32>( i32( floor( ( nodeVarying0.x * f32( textureDimensions( nodeUniform0, 0 ).x ) ) ) ), i32( floor( ( nodeVarying0.y * f32( textureDimensions( nodeUniform0, 0 ).y ) ) ) ) );
	const nodeConst1 = exp2( ( - 0.2 ) );
	nodeVar0 = textureLoad( nodeUniform0, ( nodeConst0 + vec2<i32>( 0, -1 ) ), u32( 0u ) );
	nodeVar1 = textureLoad( nodeUniform0, ( nodeConst0 + vec2<i32>( -1, 0 ) ), u32( 0u ) );
	nodeVar2 = textureLoad( nodeUniform0, ( nodeConst0 + vec2<i32>( 1, 0 ) ), u32( 0u ) );
	nodeVar3 = textureLoad( nodeUniform0, ( nodeConst0 + vec2<i32>( 0, 1 ) ), u32( 0u ) );
	let nodeConst2 = min( min( nodeVar0.xyz, nodeVar1.xyz ), min( nodeVar2.xyz, nodeVar3.xyz ) );
	let nodeConst3 = max( max( nodeVar0.xyz, nodeVar1.xyz ), max( nodeVar2.xyz, nodeVar3.xyz ) );
	const nodeConst4 = 0.1875;
	nodeVar4 = textureLoad( nodeUniform0, nodeConst0, u32( 0u ) );
	let nodeConst5 = ( min( nodeConst2, nodeVar4.xyz ) / ( nodeConst3 * vec3<f32>( 4.0 ) ) );
	let nodeConst6 = ( ( vec3<f32>( 1.0, 1.0, 1.0 ) - max( nodeConst3, nodeVar4.xyz ) ) / ( ( nodeConst2 * vec3<f32>( 4.0 ) ) - vec3<f32>( 4.0 ) ) );
	let nodeConst7 = max( ( - nodeConst5 ), nodeConst6 );
	let nodeConst8 = ( max( ( - nodeConst4 ), min( max( nodeConst7.x, max( nodeConst7.y, nodeConst7.z ) ), 0.0 ) ) * nodeConst1 );
	let nodeConst9 = ( nodeVar0.y + ( ( nodeVar0.z + nodeVar0.x ) * 0.5 ) );
	let nodeConst10 = ( nodeVar1.y + ( ( nodeVar1.z + nodeVar1.x ) * 0.5 ) );
	let nodeConst11 = ( nodeVar2.y + ( ( nodeVar2.z + nodeVar2.x ) * 0.5 ) );
	let nodeConst12 = ( nodeVar3.y + ( ( nodeVar3.z + nodeVar3.x ) * 0.5 ) );
	let nodeConst13 = ( nodeVar4.y + ( ( nodeVar4.z + nodeVar4.x ) * 0.5 ) );
	let nodeConst14 = ( ( ( ( ( nodeConst9 + nodeConst10 ) + nodeConst11 ) + nodeConst12 ) * 0.25 ) - nodeConst13 );
	let nodeConst15 = ( max( max( nodeConst9, nodeConst10 ), max( nodeConst13, max( nodeConst11, nodeConst12 ) ) ) - min( min( nodeConst9, nodeConst10 ), min( nodeConst13, min( nodeConst11, nodeConst12 ) ) ) );
	let nodeConst16 = ( 1.0 - ( clamp( ( abs( nodeConst14 ) / max( nodeConst15, 0.0000152587890625 ) ), 0.0, 1.0 ) * 0.5 ) );

	if ( ( false == true ) ) {

		nodeVar5 = ( nodeConst8 * nodeConst16 );

	} else {

		nodeVar5 = nodeConst8;

	}

	let nodeConst17 = nodeVar5;
	let nodeConst18 = ( ( ( ( ( ( nodeVar0.xyz + nodeVar1.xyz ) + nodeVar2.xyz ) + nodeVar3.xyz ) * vec3<f32>( nodeConst17 ) ) + nodeVar4.xyz ) / vec3<f32>( ( ( nodeConst17 * 4.0 ) + 1.0 ) ) );

	// result

	output.color = vec4<f32>( nodeConst18, nodeVar4.w );

	return output;

}
