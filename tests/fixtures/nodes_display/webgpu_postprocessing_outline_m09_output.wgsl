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
@binding( 1 ) @group( 1 ) var nodeUniform1_sampler : sampler;
@binding( 2 ) @group( 1 ) var nodeUniform1 : texture_2d<f32>;
@binding( 3 ) @group( 1 ) var nodeUniform6_sampler : sampler;
@binding( 4 ) @group( 1 ) var nodeUniform6 : texture_2d<f32>;

struct objectStruct {
	nodeUniform0 : f32,
	nodeUniform2 : vec3<f32>,
	nodeUniform3 : vec3<f32>,
	nodeUniform4 : f32
};
@binding( 0 ) @group( 1 )
var<uniform> object : objectStruct;

struct renderStruct {
	nodeUniform5 : f32
};
@binding( 0 ) @group( 0 )
var<uniform> render : renderStruct;

// vars
var<private> nodeVar0 : vec3<f32>;
var<private> nodeVar1 : vec4<f32>;
var<private> nodeVar2 : vec4<f32>;
var<private> nodeVar3 : vec4<f32>;

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
fn main( @location( 0 ) nodeVarying0 : vec2<f32> ) -> OutputStruct {

	// flow
	// code


	if ( ( object.nodeUniform0 > 0.0 ) ) {

		nodeVar1 = textureSample( nodeUniform1, nodeUniform1_sampler, nodeVarying0 );
		let nodeConst0 = nodeVar1;
		nodeVar0 = ( ( ( ( vec3<f32>( nodeConst0.x ) * object.nodeUniform2 ) + ( vec3<f32>( nodeConst0.y ) * object.nodeUniform3 ) ) * vec3<f32>( object.nodeUniform4 ) ) * vec3<f32>( ( ( ( ( sin( ( ( ( ( render.nodeUniform5 / object.nodeUniform0 ) * 2.0 ) + 0.75 ) * 6.283185307179586 ) ) * 0.5 ) + 0.5 ) * 0.5 ) + 0.5 ) ) );

	} else {

		nodeVar2 = textureSample( nodeUniform1, nodeUniform1_sampler, nodeVarying0 );
		let nodeConst1 = nodeVar2;
		nodeVar0 = ( ( ( vec3<f32>( nodeConst1.x ) * object.nodeUniform2 ) + ( vec3<f32>( nodeConst1.y ) * object.nodeUniform3 ) ) * vec3<f32>( object.nodeUniform4 ) );

	}

	nodeVar3 = textureSample( nodeUniform6, nodeUniform6_sampler, nodeVarying0 );
	let nodeConst2 = ( vec4<f32>( nodeVar0, 1.0 ) + nodeVar3 );
	let nodeConst3 = fn1( vec4<f32>( nodeConst2.xyz, clamp( nodeConst2.w, 0.0, 1.0 ) ) );

	// result

	output.color = fn0( vec4<f32>( sRGBTransferOETF( nodeConst3.xyz ), nodeConst3.w ) );

	return output;

}
