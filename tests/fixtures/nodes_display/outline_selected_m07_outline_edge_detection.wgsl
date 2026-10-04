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
@binding( 0 ) @group( 0 ) var nodeUniform0_sampler : sampler;
@binding( 1 ) @group( 0 ) var nodeUniform0 : texture_2d<f32>;

struct objectStruct {
	nodeUniform1 : mat3x3<f32>
};
@binding( 2 ) @group( 0 )
var<uniform> object : objectStruct;

// vars
var<private> nodeVar0 : vec2<f32>;
var<private> nodeVar1 : vec4<f32>;
var<private> nodeVar2 : vec4<f32>;
var<private> nodeVar3 : vec4<f32>;
var<private> nodeVar4 : vec4<f32>;
var<private> nodeVar5 : vec4<f32>;
var<private> nodeVar6 : vec4<f32>;
var<private> nodeVar7 : vec4<f32>;
var<private> nodeVar8 : vec4<f32>;
var<private> nodeVar9 : vec3<f32>;

// codes


@fragment
fn main( @location( 0 ) nodeVarying0 : vec2<f32> ) -> OutputStruct {

	// flow
	// code

	nodeVar0 = ( vec2<f32>( 1.0, 1.0 ) / vec2<f32>( textureDimensions( nodeUniform0, 0 ) ) );
	let nodeConst0 = ( vec4<f32>( 1.0, 0.0, 0.0, 1.0 ) * vec4<f32>( nodeVar0, nodeVar0 ) );
	nodeVar1 = textureSample( nodeUniform0, nodeUniform0_sampler, ( object.nodeUniform1 * vec3<f32>( ( nodeVarying0 + nodeConst0.xy ), 1.0 ) ).xy );
	nodeVar2 = nodeVar1;
	nodeVar3 = textureSample( nodeUniform0, nodeUniform0_sampler, ( object.nodeUniform1 * vec3<f32>( ( nodeVarying0 - nodeConst0.xy ), 1.0 ) ).xy );
	nodeVar4 = nodeVar3;
	nodeVar5 = textureSample( nodeUniform0, nodeUniform0_sampler, ( object.nodeUniform1 * vec3<f32>( ( nodeVarying0 + nodeConst0.yw ), 1.0 ) ).xy );
	nodeVar6 = nodeVar5;
	nodeVar7 = textureSample( nodeUniform0, nodeUniform0_sampler, ( object.nodeUniform1 * vec3<f32>( ( nodeVarying0 - nodeConst0.yw ), 1.0 ) ).xy );
	nodeVar8 = nodeVar7;

	if ( ( ( 1.0 - min( min( nodeVar2.y, nodeVar4.y ), min( nodeVar6.y, nodeVar8.y ) ) ) > 0.001 ) ) {

		nodeVar9 = vec3<f32>( 1.0, 0.0, 0.0 );

	} else {

		nodeVar9 = vec3<f32>( 0.0, 1.0, 0.0 );

	}


	// result

	output.color = ( vec4<f32>( nodeVar9, 1.0 ) * vec4<f32>( length( vec2<f32>( ( ( nodeVar2.x - nodeVar4.x ) * 0.5 ), ( ( nodeVar6.x - nodeVar8.x ) * 0.5 ) ) ) ) );

	return output;

}
