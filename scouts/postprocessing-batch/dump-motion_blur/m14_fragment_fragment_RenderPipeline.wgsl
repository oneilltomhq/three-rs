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
@binding( 2 ) @group( 1 ) var nodeUniform1_sampler : sampler;
@binding( 3 ) @group( 1 ) var nodeUniform1 : texture_2d<f32>;

struct objectStruct {
	nodeUniform2 : f32
};
@binding( 4 ) @group( 1 )
var<uniform> object : objectStruct;

struct renderStruct {
	nodeUniform3 : vec2<f32>
};
@binding( 0 ) @group( 0 )
var<uniform> render : renderStruct;

// vars
var<private> nodeVar0 : vec4<f32>;
var<private> nodeVar1 : vec4<f32>;
var<private> nodeVar2 : vec4<f32>;
var<private> nodeVar3 : vec4<f32>;
var<private> nodeVar4 : vec4<f32>;
var<private> nodeVar5 : vec4<f32>;

// codes
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

	nodeVar0 = textureSample( nodeUniform0, nodeUniform0_sampler, nodeVarying0 );
	nodeVar1 = nodeVar0;

	for ( var i : i32 = 1; i <= 16; i ++ ) {

		nodeVar2 = textureSample( nodeUniform1, nodeUniform1_sampler, nodeVarying0 );
		nodeVar3 = textureSample( nodeUniform0, nodeUniform0_sampler, ( vec4<f32>( vec3<f32>( nodeVarying0, 0.0 ), 1.0 ) + ( ( nodeVar2 * vec4<f32>( object.nodeUniform2 ) ) * vec4<f32>( ( ( f32( i ) / ( 16.0 - 1.0 ) ) - 0.5 ) ) ) ).xy );
		nodeVar1 = ( nodeVar1 + nodeVar3 );

	}

	nodeVar1 = ( nodeVar1 / vec4<f32>( 16.0 ) );
	nodeVar4 = vec4<f32>( ( nodeVar1 * vec4<f32>( ( 1.0 - clamp( ( ( ( ( ( distance( ( fragCoord.xy / render.nodeUniform3 ), vec2<f32>( 0.5 ) ) - 0.6 ) / ( 1.0 - 0.6 ) ) * ( 1.0 - 0.0 ) ) + 0.0 ) * 2.0 ), 0.0, 1.0 ) ) ) ).xyz, nodeVar1.w );
	nodeVar5 = fn1( vec4<f32>( nodeVar4.xyz, clamp( nodeVar4.w, 0.0, 1.0 ) ) );

	// result

	output.color = fn0( vec4<f32>( sRGBTransferOETF( nodeVar5.xyz ), nodeVar5.w ) );

	return output;

}
