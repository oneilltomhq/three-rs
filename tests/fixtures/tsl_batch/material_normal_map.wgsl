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
@binding( 0 ) @group( 1 ) var nodeUniform3_sampler : sampler;
@binding( 1 ) @group( 1 ) var nodeUniform3 : texture_2d<f32>;

struct objectStruct {
	nodeUniform1 : mat4x4<f32>,
	nodeUniform2 : mat3x3<f32>,
	nodeUniform4 : mat3x3<f32>,
	nodeUniform5 : vec2<f32>
};
@binding( 2 ) @group( 1 )
var<uniform> object : objectStruct;

// vars
var<private> normalViewGeometry : vec3<f32>;
var<private> normalView : vec3<f32>;
var<private> nodeVar0 : f32;
var<private> tangentViewFrame : vec3<f32>;
var<private> tangentView : vec3<f32>;
var<private> bitangentViewFrame : vec3<f32>;
var<private> bitangentView : vec3<f32>;
var<private> TBNViewMatrix : mat3x3<f32>;
var<private> nodeVar1 : vec4<f32>;

// codes


@fragment
fn main( @location( 0 ) v_positionView : vec3<f32>,
	@location( 1 ) v_normalViewGeometry : vec3<f32>,
	@location( 2 ) nodeVarying5 : vec2<f32> ) -> OutputStruct {

	// flow
	// code

	normalViewGeometry = normalize( v_normalViewGeometry );
	normalView = normalViewGeometry;
	let nodeConst0 = cross( - dpdy( v_positionView ), normalView );
	let nodeConst1 = dpdx( nodeVarying5 );
	let nodeConst2 = cross( normalView, dpdx( v_positionView ) );
	let nodeConst3 = - dpdy( nodeVarying5 );
	let nodeConst4 = ( ( nodeConst0 * vec3<f32>( nodeConst1.x ) ) + ( nodeConst2 * vec3<f32>( nodeConst3.x ) ) );
	let nodeConst5 = ( ( nodeConst0 * vec3<f32>( nodeConst1.y ) ) + ( nodeConst2 * vec3<f32>( nodeConst3.y ) ) );
	let nodeConst6 = max( dot( nodeConst4, nodeConst4 ), dot( nodeConst5, nodeConst5 ) );

	if ( ( nodeConst6 == 0.0 ) ) {

		nodeVar0 = 0.0;

	} else {

		nodeVar0 = inverseSqrt( nodeConst6 );

	}

	tangentViewFrame = ( nodeConst4 * vec3<f32>( nodeVar0 ) );
	tangentView = tangentViewFrame;
	bitangentViewFrame = ( nodeConst5 * nodeVar0 );
	bitangentView = bitangentViewFrame;
	TBNViewMatrix = mat3x3<f32>( tangentView, bitangentView, normalView );
	nodeVar1 = textureSample( nodeUniform3, nodeUniform3_sampler, ( object.nodeUniform4 * vec3<f32>( nodeVarying5, 1.0 ) ).xy );
	let nodeConst7 = ( ( nodeVar1 * vec4<f32>( 2.0 ) ) - vec4<f32>( 1.0 ) );

	// result

	output.color = vec4<f32>( normalize( ( TBNViewMatrix * vec3<f32>( ( nodeConst7.xy * object.nodeUniform5 ), nodeConst7.z ) ) ), 1.0 );

	return output;

}
