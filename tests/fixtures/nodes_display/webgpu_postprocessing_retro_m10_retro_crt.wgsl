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

struct renderStruct {
	nodeUniform6 : f32,
	nodeUniform1 : vec2<f32>
};
@binding( 0 ) @group( 0 )
var<uniform> render : renderStruct;

struct objectStruct {
	nodeUniform2 : f32,
	nodeUniform3 : f32,
	nodeUniform4 : f32,
	nodeUniform5 : f32,
	nodeUniform7 : f32,
	nodeUniform8 : f32,
	nodeUniform9 : f32
};
@binding( 2 ) @group( 1 )
var<uniform> object : objectStruct;

// vars
var<private> nodeVar0 : vec4<f32>;
var<private> nodeVar1 : vec4<f32>;
var<private> nodeVar2 : vec4<f32>;
var<private> nodeVar3 : vec4<f32>;

// codes
fn tsl_mod_float( x : f32, y : f32 ) -> f32 { return x - y * floor( x / y ); }
fn fn1 ( color : vec4<f32> ) -> vec4<f32> {

	var nodeVar0 : vec4<f32>;


	if ( ( color.w == 0.0 ) ) {

		nodeVar0 = vec4<f32>( 0.0, 0.0, 0.0, 0.0 );

	} else {

		nodeVar0 = vec4<f32>( ( color.xyz / vec3<f32>( color.w ) ), color.w );

	}


	return nodeVar0;

}


fn sRGBTransferOETF ( color : vec3<f32> ) -> vec3<f32> {

	


	return mix( ( ( pow( color, vec3<f32>( 0.41666 ) ) * vec3<f32>( 1.055 ) ) - vec3<f32>( 0.055 ) ), ( color * vec3<f32>( 12.92 ) ), vec3<f32>( ( color <= vec3<f32>( 0.0031308 ) ) ) );

}


fn fn0 ( color : vec4<f32> ) -> vec4<f32> {

	


	return vec4<f32>( ( color.xyz * vec3<f32>( color.w ) ), color.w );

}




@fragment
fn main( @location( 0 ) nodeVarying0 : vec2<f32>,
	@builtin( position ) fragCoord : vec4<f32> ) -> OutputStruct {

	// flow
	// code

	nodeVar0 = textureSample( nodeUniform0, nodeUniform0_sampler, ( fragCoord.xy / render.nodeUniform1 ) );
	let nodeConst0 = ( ( object.nodeUniform3 + 0.1 ) * 10.0 );
	let nodeConst1 = ( object.nodeUniform2 + ( ( smoothstep( nodeConst0, ( nodeConst0 - ( 1.0 * nodeConst0 ) ), ( length( ( nodeVarying0 - vec2<f32>( 0.5 ) ) ) * 2.0 ) ) * object.nodeUniform3 ) * 0.05 ) );
	nodeVar1 = textureSample( nodeUniform0, nodeUniform0_sampler, ( ( fragCoord.xy / render.nodeUniform1 ) - vec2<f32>( nodeConst1, 0.0 ) ) );
	nodeVar2 = textureSample( nodeUniform0, nodeUniform0_sampler, ( ( fragCoord.xy / render.nodeUniform1 ) - vec2<f32>( ( nodeConst1 * 2.0 ), 0.0 ) ) );
	nodeVar3 = textureSample( nodeUniform0, nodeUniform0_sampler, ( ( fragCoord.xy / render.nodeUniform1 ) - vec2<f32>( ( nodeConst1 * 3.0 ), 0.0 ) ) );
	let nodeConst2 = vec3<f32>( clamp( ( ( ( ( nodeVar0.xyz.x + ( nodeVar1.xyz.x * 0.4 ) ) + ( nodeVar2.xyz.x * 0.2 ) ) + ( nodeVar3.xyz.x * 0.1 ) ) / 1.7 ), 0.0, 1.0 ), clamp( ( ( ( nodeVar0.xyz.y + ( nodeVar1.xyz.y * 0.25 ) ) + ( nodeVar2.xyz.y * 0.1 ) ) / 1.35 ), 0.0, 1.0 ), clamp( ( ( nodeVar0.xyz.z + ( nodeVar1.xyz.z * 0.15 ) ) / 1.15 ), 0.0, 1.0 ) );
	let nodeConst3 = ( ( fragCoord.xy / render.nodeUniform1 ) * render.nodeUniform1 );
	let nodeConst4 = ( ( ( tsl_mod_float( ( ( floor( ( tsl_mod_float( floor( nodeConst3.x ), 4.0 ) + 1.0 ) ) * floor( ( tsl_mod_float( floor( nodeConst3.y ), 4.0 ) + 1.0 ) ) ) * 17.0 ), 16.0 ) / 16.0 ) - 0.5 ) / object.nodeUniform4 );
	let nodeConst5 = ( ( ( floor( ( vec3<f32>( ( nodeConst2.x + nodeConst4 ), ( nodeConst2.y + nodeConst4 ), ( nodeConst2.z + nodeConst4 ) ) * vec3<f32>( object.nodeUniform4 ) ) ) / vec3<f32>( object.nodeUniform4 ) ) * vec3<f32>( mix( ( 1.0 - object.nodeUniform5 ), 1.0, smoothstep( 1.42, ( 1.42 - ( 0.6 * 1.42 ) ), ( length( ( nodeVarying0 - vec2<f32>( 0.5 ) ) ) * 2.0 ) ) ) ) ) * vec3<f32>( ( 1.0 - ( ( ( sin( ( ( nodeVarying0.y - ( render.nodeUniform6 * object.nodeUniform7 ) ) * ( render.nodeUniform1.y * object.nodeUniform8 ) ) ) * 0.5 ) + 0.5 ) * object.nodeUniform9 ) ) ) );
	let nodeConst6 = fn1( vec4<f32>( nodeConst5, clamp( vec4<f32>( nodeConst5, 1.0 ).w, 0.0, 1.0 ) ) );

	// result

	output.color = fn0( vec4<f32>( sRGBTransferOETF( nodeConst6.xyz ), nodeConst6.w ) );

	return output;

}
