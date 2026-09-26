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

struct renderStruct {
	nodeUniform0 : f32,
	cameraProjectionMatrix : mat4x4<f32>,
	cameraViewMatrix : mat4x4<f32>
};
@binding( 0 ) @group( 0 )
var<uniform> render : renderStruct;

struct objectStruct {
	nodeUniform1 : f32,
	nodeUniform2 : f32,
	nodeUniform5 : mat4x4<f32>
};
@binding( 0 ) @group( 1 )
var<uniform> object : objectStruct;

// vars
var<private> DiffuseColor : vec4<f32>;
var<private> Output : vec4<f32>;

// codes


@fragment
fn main(  ) -> OutputStruct {

	// flow
	// code

	let nodeConst0 = ( cos( render.nodeUniform0 ) * 3.0 );
	DiffuseColor = vec4<f32>( vec3<f32>( ( ( sin( render.nodeUniform0 ) * 2.0 ) + ( sin( render.nodeUniform0 ) * 2.0 ) ), ( nodeConst0 + ( nodeConst0 * 0.5 ) ), smoothstep( 1.0, 3.0, ( - object.nodeUniform1 ) ) ), 1.0 );
	DiffuseColor.w = ( DiffuseColor.w * object.nodeUniform2 );
	DiffuseColor.w = 1.0;
	let nodeConst1 = max( vec4<f32>( DiffuseColor.xyz, DiffuseColor.w ), vec4<f32>( 0.0 ) );
	Output = nodeConst1;

	// result

	output.color = nodeConst1;

	return output;

}
