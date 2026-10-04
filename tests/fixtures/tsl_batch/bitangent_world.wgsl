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
	cameraProjectionMatrix : mat4x4<f32>,
	cameraViewMatrix : mat4x4<f32>,
	cameraWorldMatrix : mat4x4<f32>
};
@binding( 0 ) @group( 0 )
var<uniform> render : renderStruct;

// vars
var<private> normalViewGeometry : vec3<f32>;
var<private> normalView : vec3<f32>;
var<private> normalWorld : vec3<f32>;
var<private> tangentWorld : vec3<f32>;
var<private> bitangentWorld : vec3<f32>;

// codes


@fragment
fn main( @location( 0 ) v_normalViewGeometry : vec3<f32>,
	@location( 1 ) v_tangentWorld : vec3<f32>,
	@location( 2 ) nodeVarying7 : vec4<f32> ) -> OutputStruct {

	// flow
	// code

	normalViewGeometry = normalize( v_normalViewGeometry );
	normalView = normalViewGeometry;
	normalWorld = normalize( ( vec4<f32>( normalView, 0.0 ) * render.cameraViewMatrix ).xyz );
	tangentWorld = normalize( v_tangentWorld );
	bitangentWorld = normalize( ( cross( normalWorld, tangentWorld ) * vec3<f32>( nodeVarying7.w ) ) );

	// result

	output.color = vec4<f32>( ( bitangentWorld + tangentWorld ), 1.0 );

	return output;

}
