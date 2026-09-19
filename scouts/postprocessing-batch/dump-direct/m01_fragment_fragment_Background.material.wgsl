// Three.js r186 - Node System

// global
diagnostic( off, derivative_uniformity );


// structs

struct OutputStruct {
	@location( 0 ) color: vec4<f32>
};
var<private> output : OutputStruct;

// uniforms

struct objectStruct {
	nodeUniform0 : vec3<f32>,
	nodeUniform2 : f32,
	nodeUniform3 : f32,
	nodeUniform7 : mat4x4<f32>
};
@binding( 0 ) @group( 1 )
var<uniform> object : objectStruct;

struct renderStruct {
	nodeUniform1 : f32,
	cameraProjectionMatrix : mat4x4<f32>,
	cameraViewMatrix : mat4x4<f32>,
	nodeUniform4 : f32
};
@binding( 0 ) @group( 0 )
var<uniform> render : renderStruct;

// vars
var<private> DiffuseColor : vec4<f32>;
var<private> Output : vec4<f32>;
var<private> nodeVar0 : vec4<f32>;
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


fn neutralToneMapping ( color : vec3<f32>, exposure : f32 ) -> vec3<f32> {

	var nodeVar0 : vec3<f32>;
	var nodeVar1 : f32;
	var nodeVar2 : f32;
	var nodeVar3 : f32;
	var nodeVar4 : f32;
	var nodeVar5 : f32;

	nodeVar0 = ( color * vec3<f32>( exposure ) );
	nodeVar2 = min( nodeVar0.x, min( nodeVar0.y, nodeVar0.z ) );

	if ( ( nodeVar2 < 0.08 ) ) {

		nodeVar1 = ( nodeVar2 - ( 6.25 * ( nodeVar2 * nodeVar2 ) ) );

	} else {

		nodeVar1 = 0.04;

	}

	nodeVar0 = ( nodeVar0 - vec3<f32>( nodeVar1 ) );
	nodeVar3 = max( nodeVar0.x, max( nodeVar0.y, nodeVar0.z ) );

	if ( ( nodeVar3 < 0.76 ) ) {

		return nodeVar0;

	}

	nodeVar4 = ( 1.0 - 0.76 );
	nodeVar5 = ( 1.0 - ( ( nodeVar4 * nodeVar4 ) / ( nodeVar3 + ( nodeVar4 - 0.76 ) ) ) );
	nodeVar0 = ( nodeVar0 * vec3<f32>( ( nodeVar5 / nodeVar3 ) ) );

	return mix( nodeVar0, vec3<f32>( nodeVar5 ), ( 1.0 - ( 1.0 / ( ( 0.15 * ( nodeVar3 - nodeVar5 ) ) + 1.0 ) ) ) );

}


fn sRGBTransferOETF ( color : vec3<f32> ) -> vec3<f32> {

	


	return mix( ( ( pow( color, vec3<f32>( 0.41666 ) ) * vec3<f32>( 1.055 ) ) - vec3<f32>( 0.055 ) ), ( color * vec3<f32>( 12.92 ) ), vec3<f32>( ( color <= vec3<f32>( 0.0031308 ) ) ) );

}


fn fn0 ( color : vec4<f32> ) -> vec4<f32> {

	


	return vec4<f32>( ( color.xyz * vec3<f32>( color.w ) ), color.w );

}




@fragment
fn main(  ) -> OutputStruct {

	// flow
	// code

	DiffuseColor = ( vec4<f32>( object.nodeUniform0, 1.0 ) * vec4<f32>( render.nodeUniform1 ) );
	DiffuseColor.w = ( DiffuseColor.w * object.nodeUniform2 );
	DiffuseColor.w = 1.0;
	nodeVar0 = max( vec4<f32>( DiffuseColor.xyz, DiffuseColor.w ), vec4<f32>( 0.0 ) );
	Output = nodeVar0;
	Output = nodeVar0;
	nodeVar1 = vec4<f32>( max( mix( vec3<f32>( dot( Output.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) ) ), Output.xyz, object.nodeUniform3 ), vec3<f32>( 0.0 ) ), Output.w );
	nodeVar2 = fn1( vec4<f32>( nodeVar1.xyz, clamp( nodeVar1.w, 0.0, 1.0 ) ) );
	nodeVar3 = vec4<f32>( neutralToneMapping( nodeVar2.xyz, render.nodeUniform4 ), nodeVar2.w );

	// result

	output.color = fn0( vec4<f32>( sRGBTransferOETF( nodeVar3.xyz ), nodeVar3.w ) );

	return output;

}
