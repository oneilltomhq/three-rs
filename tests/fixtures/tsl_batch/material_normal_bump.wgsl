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
	nodeUniform5 : f32
};
@binding( 2 ) @group( 1 )
var<uniform> object : objectStruct;

// vars
var<private> normalViewGeometry : vec3<f32>;
var<private> normalView : vec3<f32>;
var<private> nodeVar0 : vec4<f32>;
var<private> nodeVar1 : vec4<f32>;
var<private> nodeVar2 : vec4<f32>;

// codes


@fragment
fn main( @location( 0 ) v_positionView : vec3<f32>,
	@location( 1 ) v_normalViewGeometry : vec3<f32>,
	@location( 2 ) nodeVarying5 : vec2<f32>,
	@builtin( front_facing ) isFront : bool ) -> OutputStruct {

	// flow
	// code

	let nodeConst0 = normalize( dpdx( v_positionView ) );
	normalViewGeometry = normalize( v_normalViewGeometry );
	normalView = normalViewGeometry;
	let nodeConst1 = cross( normalize( - dpdy( v_positionView ) ), normalView );
	let nodeConst2 = ( dot( nodeConst0, nodeConst1 ) * ( ( f32( isFront ) * 2.0 ) - 1.0 ) );
	nodeVar0 = textureSample( nodeUniform3, nodeUniform3_sampler, ( object.nodeUniform4 * vec3<f32>( ( nodeVarying5 + dpdx( nodeVarying5 ) ), 1.0 ) ).xy );
	nodeVar1 = textureSample( nodeUniform3, nodeUniform3_sampler, ( object.nodeUniform4 * vec3<f32>( nodeVarying5, 1.0 ) ).xy );
	let nodeConst3 = nodeVar1.x;
	nodeVar2 = textureSample( nodeUniform3, nodeUniform3_sampler, ( object.nodeUniform4 * vec3<f32>( ( nodeVarying5 + - dpdy( nodeVarying5 ) ), 1.0 ) ).xy );
	let nodeConst4 = ( vec2<f32>( ( nodeVar0.x - nodeConst3 ), ( nodeVar2.x - nodeConst3 ) ) * vec2<f32>( object.nodeUniform5 ) );

	// result

	output.color = vec4<f32>( normalize( ( ( vec3<f32>( abs( nodeConst2 ) ) * normalView ) - ( vec3<f32>( sign( nodeConst2 ) ) * ( ( vec3<f32>( nodeConst4.x ) * nodeConst1 ) + ( vec3<f32>( nodeConst4.y ) * cross( normalView, nodeConst0 ) ) ) ) ) ), 1.0 );

	return output;

}
