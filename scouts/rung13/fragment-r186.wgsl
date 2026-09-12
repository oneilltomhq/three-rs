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
	nodeUniform8 : vec3<f32>,
	nodeUniform9 : vec3<f32>,
	nodeUniform10 : f32,
	nodeUniform13 : mat4x4<f32>,
	nodeUniform14 : f32,
	nodeUniform16 : f32
};
@binding( 0 ) @group( 1 )
var<uniform> object : objectStruct;

// vars
var<private> DiffuseColor : vec4<f32>;
var<private> Output : vec4<f32>;
var<private> nodeVar4 : vec4<f32>;

// codes


@fragment
fn main( @location( 0 ) nodeVarying4 : vec4<f32>,
	@location( 1 ) nodeVarying5 : vec2<f32> ) -> OutputStruct {

	// flow
	// code

	DiffuseColor = vec4<f32>( mix( object.nodeUniform8, object.nodeUniform9, ( 1.0 - pow( ( 1.0 - nodeVarying4.x ), 2.0 ) ) ), ( ( 0.1 / length( ( nodeVarying5 - vec2<f32>( 0.5 ) ) ) ) - 0.2 ) );
	DiffuseColor.w = ( DiffuseColor.w * object.nodeUniform10 );
	nodeVar4 = max( vec4<f32>( DiffuseColor.xyz, DiffuseColor.w ), vec4<f32>( 0.0 ) );
	Output = nodeVar4;

	// result

	output.color = nodeVar4;

	return output;

}
