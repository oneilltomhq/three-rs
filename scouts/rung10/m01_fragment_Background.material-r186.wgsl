// Three.js r186 - Node System

// global
diagnostic( off, derivative_uniformity );


// structs

struct OutputStruct {
	@location( 0 ) color: vec4<f32>
};
var<private> output : OutputStruct;

// uniforms

struct renderStruct {
	nodeUniform1 : f32,
	cameraProjectionMatrix : mat4x4<f32>,
	cameraViewMatrix : mat4x4<f32>,
	nodeUniform0 : vec2<f32>
};
@binding( 0 ) @group( 0 )
var<uniform> render : renderStruct;

struct objectStruct {
	nodeUniform2 : f32,
	nodeUniform5 : mat4x4<f32>
};
@binding( 0 ) @group( 1 )
var<uniform> object : objectStruct;

// vars
var<private> DiffuseColor : vec4<f32>;
var<private> Output : vec4<f32>;
var<private> nodeVar0 : vec4<f32>;

// codes


@fragment
fn main( @builtin( position ) fragCoord : vec4<f32> ) -> OutputStruct {

	// flow
	// code

	DiffuseColor = ( vec4<f32>( mix( vec3<f32>( 0.13286832154414627, 0.4969329950515914, 1.0 ), vec3<f32>( 0.057805430183792694, 0.13286832154414627, 1.0 ), ( fragCoord.xy / render.nodeUniform0 ).y ), 1.0 ) * vec4<f32>( render.nodeUniform1 ) );
	DiffuseColor.w = ( DiffuseColor.w * object.nodeUniform2 );
	DiffuseColor.w = 1.0;
	nodeVar0 = max( vec4<f32>( DiffuseColor.xyz, DiffuseColor.w ), vec4<f32>( 0.0 ) );
	Output = nodeVar0;

	// result

	output.color = nodeVar0;

	return output;

}
