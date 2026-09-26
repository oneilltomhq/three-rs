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
fn tsl_mod_vec2( x : vec2f, y : vec2f ) -> vec2f { return x - y * floor( x / y ); }
fn tsl_mod_float( x : f32, y : f32 ) -> f32 { return x - y * floor( x / y ); }
fn tsl_mod_vec3( x : vec3f, y : vec3f ) -> vec3f { return x - y * floor( x / y ); }
fn tsl_mod_vec4( x : vec4f, y : vec4f ) -> vec4f { return x - y * floor( x / y ); }


@fragment
fn main( @location( 0 ) nodeVarying4 : vec2<f32> ) -> OutputStruct {

	// flow
	// code


	// result

	output.color = ( ( vec4<f32>( tsl_mod_vec2( nodeVarying4, vec2<f32>( 0.5 ) ), tsl_mod_float( nodeVarying4.x, 0.25 ), f32( ( i32( ( nodeVarying4.x * 10.0 ) ) % 3 ) ) ) + vec4<f32>( tsl_mod_vec3( vec3<f32>( nodeVarying4, 1.0 ), vec3<f32>( 0.5, 0.5, 0.5 ) ), 1.0 ) ) + tsl_mod_vec4( vec4<f32>( nodeVarying4, 0.0, 1.0 ), vec4<f32>( 1.0 ) ) );

	return output;

}
