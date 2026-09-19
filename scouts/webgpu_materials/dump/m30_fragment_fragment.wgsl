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
	nodeUniform1 : mat3x3<f32>,
	nodeUniform2 : f32,
	nodeUniform4 : mat4x4<f32>
};
@binding( 0 ) @group( 1 )
var<uniform> object : objectStruct;

// vars
var<private> DiffuseColor : vec4<f32>;
var<private> normalViewGeometry : vec3<f32>;
var<private> NORMAL_normalView : vec3<f32>;
var<private> normalView : vec3<f32>;
var<private> nodeVar0 : vec4<f32>;
var<private> Output : vec4<f32>;
var<private> nodeVar1 : vec4<f32>;

// codes
fn sRGBTransferEOTF ( color : vec3<f32> ) -> vec3<f32> {

	


	return mix( pow( ( ( color * vec3<f32>( 0.9478672986 ) ) + vec3<f32>( 0.0521327014 ) ), vec3<f32>( 2.4 ) ), ( color * vec3<f32>( 0.0773993808 ) ), vec3<f32>( ( color <= vec3<f32>( 0.04045 ) ) ) );

}




@fragment
fn main( @location( 0 ) v_normalViewGeometry : vec3<f32> ) -> OutputStruct {

	// flow
	// code

	normalViewGeometry = normalize( v_normalViewGeometry );
	NORMAL_normalView = normalViewGeometry;
	normalView = NORMAL_normalView;
	nodeVar0 = vec4<f32>( ( ( normalView * vec3<f32>( 0.5 ) ) + vec3<f32>( 0.5 ) ), object.nodeUniform2 );
	DiffuseColor = vec4<f32>( sRGBTransferEOTF( nodeVar0.xyz ), nodeVar0.w );
	nodeVar1 = max( vec4<f32>( DiffuseColor.xyz, DiffuseColor.w ), vec4<f32>( 0.0 ) );
	Output = nodeVar1;

	// result

	output.color = nodeVar1;

	return output;

}
