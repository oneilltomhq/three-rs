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

// vars


// codes
fn tsl_mod_float( x : f32, y : f32 ) -> f32 { return x - y * floor( x / y ); }


@fragment
fn main( @location( 0 ) nodeVarying4 : vec2<f32> ) -> OutputStruct {

	// flow
	// code

	let nodeConst0 = floor( tsl_mod_float( render.nodeUniform0, ( vec2<f32>( 6.0, 4.0 ).x * vec2<f32>( 6.0, 4.0 ).y ) ) );
	let nodeConst1 = floor( tsl_mod_float( 0.0, ( vec2<f32>( 3.0, 3.0 ).x * vec2<f32>( 3.0, 3.0 ).y ) ) );

	// result

	output.color = vec4<f32>( ( ( nodeVarying4 + vec2<f32>( tsl_mod_float( nodeConst0, vec2<f32>( 6.0, 4.0 ).x ), ( vec2<f32>( 6.0, 4.0 ).y - ceil( ( ( nodeConst0 + 1.0 ) / vec2<f32>( 6.0, 4.0 ).x ) ) ) ) ) * ( vec2<f32>( 1.0 ) / vec2<f32>( 6.0, 4.0 ) ) ), ( ( nodeVarying4 + vec2<f32>( tsl_mod_float( nodeConst1, vec2<f32>( 3.0, 3.0 ).x ), ( vec2<f32>( 3.0, 3.0 ).y - ceil( ( ( nodeConst1 + 1.0 ) / vec2<f32>( 3.0, 3.0 ).x ) ) ) ) ) * ( vec2<f32>( 1.0 ) / vec2<f32>( 3.0, 3.0 ) ) ) );

	return output;

}
