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
	nodeUniform1 : f32
};
@binding( 0 ) @group( 0 )
var<uniform> render : renderStruct;

struct objectStruct {
	nodeUniform2 : f32
};
@binding( 2 ) @group( 1 )
var<uniform> object : objectStruct;

// vars
var<private> nodeVar0 : vec4<f32>;

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
fn main( @location( 0 ) nodeVarying0 : vec2<f32> ) -> OutputStruct {

	// flow
	// code

	nodeVar0 = textureSample( nodeUniform0, nodeUniform0_sampler, nodeVarying0 );
	let nodeConst0 = vec4<f32>( mix( nodeVar0.xyz, ( nodeVar0.xyz + ( nodeVar0.xyz * vec3<f32>( clamp( ( fract( ( sin( tsl_mod_float( dot( fract( ( nodeVarying0 + vec2<f32>( render.nodeUniform1 ) ) ), vec2<f32>( 12.9898, 78.233 ) ), 3.141592653589793 ) ) * 43758.5453 ) ) + 0.1 ), 0.0, 1.0 ) ) ) ), object.nodeUniform2 ), nodeVar0.w );
	let nodeConst1 = fn1( vec4<f32>( nodeConst0.xyz, clamp( nodeConst0.w, 0.0, 1.0 ) ) );

	// result

	output.color = fn0( vec4<f32>( sRGBTransferOETF( nodeConst1.xyz ), nodeConst1.w ) );

	return output;

}
