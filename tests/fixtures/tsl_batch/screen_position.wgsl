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
	cameraViewMatrix : mat4x4<f32>
};
@binding( 0 ) @group( 0 )
var<uniform> render : renderStruct;

// vars
var<private> nodeVar0 : vec2<f32>;

// codes


@fragment
fn main( @location( 0 ) v_positionView : vec3<f32> ) -> OutputStruct {

	// flow
	// code

	let nodeConst0 = ( render.cameraProjectionMatrix * vec4<f32>( v_positionView, 1.0 ) );
	nodeVar0 = ( ( ( nodeConst0.xy / vec2<f32>( nodeConst0.w ) ) * vec2<f32>( 0.5 ) ) + vec2<f32>( 0.5 ) );

	// result

	output.color = vec4<f32>( vec2<f32>( nodeVar0.x, ( 1.0 - nodeVar0.y ) ), 0.0, 1.0 );

	return output;

}
