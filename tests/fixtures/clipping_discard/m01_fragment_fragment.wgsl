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

struct NodeBuffer_937Struct {
	value : array< vec4<f32>, 1 >
};
@binding( 0 ) @group( 0 )
var<uniform> NodeBuffer_937 : NodeBuffer_937Struct;

struct NodeBuffer_941Struct {
	value : array< vec4<f32>, 2 >
};
@binding( 1 ) @group( 0 )
var<uniform> NodeBuffer_941 : NodeBuffer_941Struct;

struct objectStruct {
	nodeUniform1 : mat4x4<f32>,
	nodeUniform4 : vec3<f32>,
	nodeUniform5 : f32
};
@binding( 0 ) @group( 1 )
var<uniform> object : objectStruct;

// vars
var<private> clipped : bool;
var<private> DiffuseColor : vec4<f32>;
var<private> Output : vec4<f32>;

// codes


@fragment
fn main( @location( 0 ) v_positionView : vec3<f32> ) -> OutputStruct {

	// flow
	// code


	for ( var i : i32 = 0; i < 1; i ++ ) {


		if ( ( dot( v_positionView, NodeBuffer_937.value[ i ].xyz ) > NodeBuffer_937.value[ i ].w ) ) {

			discard;
			

		}


	}

	clipped = true;

	for ( var i : i32 = 0; i < 2; i ++ ) {

		clipped = ( ( dot( v_positionView, NodeBuffer_941.value[ i ].xyz ) > NodeBuffer_941.value[ i ].w ) && clipped );

	}


	if ( clipped ) {

		discard;
		

	}

	DiffuseColor = vec4<f32>( object.nodeUniform4, 1.0 );
	DiffuseColor.w = ( DiffuseColor.w * object.nodeUniform5 );
	DiffuseColor.w = 1.0;
	let nodeConst0 = max( vec4<f32>( DiffuseColor.xyz, DiffuseColor.w ), vec4<f32>( 0.0 ) );
	Output = nodeConst0;

	// result

	output.color = nodeConst0;

	return output;

}
