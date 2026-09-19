// Three.js r186 - Node System

// global
diagnostic( off, derivative_uniformity );


// structs

struct OutputStruct {
	@location( 0 ) color: vec4<f32>
};
var<private> output : OutputStruct;

// uniforms

struct objectStruct {
	nodeUniform1 : mat4x4<f32>,
	nodeUniform5 : mat4x4<f32>,
	nodeUniform7 : f32,
	nodeUniform9 : vec3<f32>,
	nodeUniform10 : f32
};
@binding( 0 ) @group( 1 )
var<uniform> object : objectStruct;

// vars
var<private> DiffuseColor : vec4<f32>;
var<private> alpha : f32;
var<private> nodeVar8 : f32;
var<private> nodeVar9 : vec3<f32>;
var<private> nodeVar10 : vec3<f32>;
var<private> Output : vec4<f32>;
var<private> nodeVar11 : vec4<f32>;

// codes


@fragment
fn main( @location( 0 ) nodeVarying4 : vec2<f32>,
	@location( 1 ) nodeVarying5 : vec3<f32>,
	@location( 2 ) nodeVarying6 : vec3<f32>,
	@location( 3 ) nodeVarying7 : vec3<f32> ) -> OutputStruct {

	// flow
	// code

	DiffuseColor = vec4<f32>( object.nodeUniform9, 1.0 );
	DiffuseColor.w = ( DiffuseColor.w * object.nodeUniform10 );
	alpha = 1.0;

	if ( ( abs( nodeVarying4.y ) > 1.0 ) ) {


		if ( ( nodeVarying4.y > 0.0 ) ) {

			nodeVar8 = ( nodeVarying4.y - 1.0 );

		} else {

			nodeVar8 = ( nodeVarying4.y + 1.0 );

		}


		if ( ( ( ( nodeVarying4.x * nodeVarying4.x ) + ( nodeVar8 * nodeVar8 ) ) > 1.0 ) ) {

			discard;
			

		}

		

	}

	DiffuseColor.w = ( DiffuseColor.w * alpha );

	if ( ( nodeVarying5.y < 0.5 ) ) {

		nodeVar9 = nodeVarying6;

	} else {

		nodeVar9 = nodeVarying7;

	}

	nodeVar10 = ( DiffuseColor.xyz * nodeVar9 );
	DiffuseColor.x = nodeVar10[ 0 ];
	DiffuseColor.y = nodeVar10[ 1 ];
	DiffuseColor.z = nodeVar10[ 2 ];
	nodeVar11 = max( vec4<f32>( DiffuseColor.xyz, DiffuseColor.w ), vec4<f32>( 0.0 ) );
	Output = nodeVar11;

	// result

	output.color = nodeVar11;

	return output;

}
