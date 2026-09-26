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


// vars


// codes
fn tsl_mod_float( x : f32, y : f32 ) -> f32 { return x - y * floor( x / y ); }


@fragment
fn main( @location( 0 ) nodeVarying4 : vec2<f32> ) -> OutputStruct {

	// flow
	// code

	let nodeConst0 = ( ( u32( ( nodeVarying4.x * 100.0 ) ) * 747796405u ) + 2891336453u );
	let nodeConst1 = ( ( ( nodeConst0 >> ( ( nodeConst0 >> 28u ) + 4u ) ) ^ nodeConst0 ) * 277803737u );

	// result

	output.color = vec4<f32>( ( f32( ( ( nodeConst1 >> 22u ) ^ nodeConst1 ) ) * 2.3283064365386963e-10 ), fract( ( sin( tsl_mod_float( dot( nodeVarying4, vec2<f32>( 12.9898, 78.233 ) ), 3.141592653589793 ) ) * 43758.5453 ) ), 0.0, 1.0 );

	return output;

}
