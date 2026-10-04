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

struct objectStruct {
	nodeUniform1 : mat4x4<f32>,
	nodeUniform2 : mat3x3<f32>,
	nodeUniform3 : f32
};
@binding( 0 ) @group( 1 )
var<uniform> object : objectStruct;

struct renderStruct {
	cameraProjectionMatrix : mat4x4<f32>,
	cameraViewMatrix : mat4x4<f32>,
	cameraWorldMatrix : mat4x4<f32>
};
@binding( 0 ) @group( 0 )
var<uniform> render : renderStruct;

// vars
var<private> positionViewDirection : vec3<f32>;
var<private> normalViewGeometry : vec3<f32>;
var<private> normalView : vec3<f32>;
var<private> refractVector : vec3<f32>;

// codes


@fragment
fn main( @location( 0 ) v_positionViewDirection : vec3<f32>,
	@location( 1 ) v_normalViewGeometry : vec3<f32> ) -> OutputStruct {

	// flow
	// code

	positionViewDirection = normalize( v_positionViewDirection );
	normalViewGeometry = normalize( v_normalViewGeometry );
	normalView = normalViewGeometry;
	let nodeConst0 = refract( ( - positionViewDirection ), normalView, object.nodeUniform3 );
	refractVector = normalize( ( render.cameraWorldMatrix * vec4<f32>( nodeConst0, 0.0 ) ).xyz );

	// result

	output.color = vec4<f32>( ( ( reflect( ( - positionViewDirection ), normalView ) + nodeConst0 ) + refractVector ), 1.0 );

	return output;

}
