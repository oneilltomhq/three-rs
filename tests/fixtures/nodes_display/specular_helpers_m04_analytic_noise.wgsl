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
	nodeUniform0 : vec2<f32>
};
@binding( 0 ) @group( 0 )
var<uniform> object : objectStruct;

// vars
var<private> nodeVar0 : vec4<f32>;

// codes
fn tsl_mod_vec2( x : vec2f, y : vec2f ) -> vec2f { return x - y * floor( x / y ); }


@fragment
fn main( @location( 0 ) nodeVarying0 : vec2<f32> ) -> OutputStruct {

	// flow
	// code

	nodeVar0 = vec4<f32>( 0.0, 0.0, 0.0, 1.0 );
	let nodeConst0 = ( 3 + 47 );
	let nodeConst1 = tsl_mod_vec2( ( floor( ( nodeVarying0 * object.nodeUniform0 ) ) + floor( ( fract( vec2<f32>( ( f32( nodeConst0 ) * 0.7548776662 ), ( f32( nodeConst0 ) * 0.569840291 ) ) ) * vec2<f32>( 32.0 ) ) ) ), vec2<f32>( 32.0 ) );
	let nodeConst2 = ( ( ( nodeConst1.x * 0.7548776662466927 ) + ( nodeConst1.y * 0.5698402909980532 ) ) + 47.0 );
	nodeVar0 = vec4<f32>( fract( ( ( nodeConst2 * 1.324717957244746 ) * 0.7548776662466927 ) ), fract( ( ( nodeConst2 * 2.649435914489492 ) * 0.5698402909980532 ) ), fract( ( ( nodeConst2 * 3.974153871734238 ) * 0.419875421 ) ), fract( ( ( nodeConst2 * 5.298871828978984 ) * 0.43015970900194667 ) ) );

	// result

	output.color = nodeVar0;

	return output;

}
