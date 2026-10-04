// Three.js r187dev - Node System

// global
diagnostic( off, derivative_uniformity );


// directives
enable subgroups;

// structs

struct OutputStruct {
	@location( 0 ) color: vec4<f32>
};
var<private> output : OutputStruct;

// uniforms


// vars


// codes


@fragment
fn main( @location( 0 ) nodeVarying4 : vec2<f32> ) -> OutputStruct {

	// flow
	// code


	// result

	output.color = vec4<f32>( subgroupAdd( nodeVarying4.x ), quadSwapX( nodeVarying4.y ), quadSwapDiagonal( nodeVarying4.x ), 1.0 );

	return output;

}
