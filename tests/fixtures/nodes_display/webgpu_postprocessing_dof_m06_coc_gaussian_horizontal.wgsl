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
	nodeUniform1 : mat3x3<f32>,
	nodeUniform2 : vec2<f32>
};
@binding( 2 ) @group( 0 )
var<uniform> object : objectStruct;

// vars
var<private> nodeVar0 : f32;
var<private> nodeVar1 : vec4<f32>;
var<private> nodeVar2 : vec2<f32>;
var<private> nodeVar3 : f32;
var<private> nodeVar4 : f32;
var<private> nodeVar5 : vec2<f32>;
var<private> nodeVar6 : f32;
var<private> nodeVar7 : f32;
var<private> nodeVar8 : vec2<f32>;
var<private> nodeVar9 : f32;
var<private> nodeVar10 : f32;
var<private> nodeVar11 : vec2<f32>;
var<private> nodeVar12 : f32;
var<private> nodeVar13 : f32;
var<private> nodeVar14 : vec2<f32>;
var<private> nodeVar15 : f32;
var<private> nodeVar16 : f32;
var<private> nodeVar17 : vec2<f32>;
var<private> nodeVar18 : f32;
var<private> nodeVar19 : f32;

// codes


@fragment
fn main( @location( 0 ) nodeVarying0 : vec2<f32> ) -> OutputStruct {

	// flow
	// code

	nodeVar0 = textureSample( nodeUniform0, nodeUniform0_sampler, ( object.nodeUniform1 * vec3<f32>( nodeVarying0, 1.0 ) ).xy ).x;
	nodeVar1 = vec4<f32>( ( nodeVar0 * 0.171834215288643 ) );
	let nodeConst0 = ( vec2<f32>( 1.0, 1.0 ) * vec2<f32>( 1.0, 0.0 ) );
	nodeVar2 = ( nodeConst0 * ( object.nodeUniform2 * vec2<f32>( 1.0 ) ) );
	nodeVar3 = textureSample( nodeUniform0, nodeUniform0_sampler, ( object.nodeUniform1 * vec3<f32>( ( nodeVarying0 + nodeVar2 ), 1.0 ) ).xy ).x;
	nodeVar4 = textureSample( nodeUniform0, nodeUniform0_sampler, ( object.nodeUniform1 * vec3<f32>( ( nodeVarying0 - nodeVar2 ), 1.0 ) ).xy ).x;
	nodeVar1 = ( nodeVar1 + vec4<f32>( ( ( nodeVar3 + nodeVar4 ) * 0.15675646343569852 ) ) );
	nodeVar5 = ( nodeConst0 * ( object.nodeUniform2 * vec2<f32>( 2.0 ) ) );
	nodeVar6 = textureSample( nodeUniform0, nodeUniform0_sampler, ( object.nodeUniform1 * vec3<f32>( ( nodeVarying0 + nodeVar5 ), 1.0 ) ).xy ).x;
	nodeVar7 = textureSample( nodeUniform0, nodeUniform0_sampler, ( object.nodeUniform1 * vec3<f32>( ( nodeVarying0 - nodeVar5 ), 1.0 ) ).xy ).x;
	nodeVar1 = ( nodeVar1 + vec4<f32>( ( ( nodeVar6 + nodeVar7 ) * 0.11900710635778596 ) ) );
	nodeVar8 = ( nodeConst0 * ( object.nodeUniform2 * vec2<f32>( 3.0 ) ) );
	nodeVar9 = textureSample( nodeUniform0, nodeUniform0_sampler, ( object.nodeUniform1 * vec3<f32>( ( nodeVarying0 + nodeVar8 ), 1.0 ) ).xy ).x;
	nodeVar10 = textureSample( nodeUniform0, nodeUniform0_sampler, ( object.nodeUniform1 * vec3<f32>( ( nodeVarying0 - nodeVar8 ), 1.0 ) ).xy ).x;
	nodeVar1 = ( nodeVar1 + vec4<f32>( ( ( nodeVar9 + nodeVar10 ) * 0.0751885933407067 ) ) );
	nodeVar11 = ( nodeConst0 * ( object.nodeUniform2 * vec2<f32>( 4.0 ) ) );
	nodeVar12 = textureSample( nodeUniform0, nodeUniform0_sampler, ( object.nodeUniform1 * vec3<f32>( ( nodeVarying0 + nodeVar11 ), 1.0 ) ).xy ).x;
	nodeVar13 = textureSample( nodeUniform0, nodeUniform0_sampler, ( object.nodeUniform1 * vec3<f32>( ( nodeVarying0 - nodeVar11 ), 1.0 ) ).xy ).x;
	nodeVar1 = ( nodeVar1 + vec4<f32>( ( ( nodeVar12 + nodeVar13 ) * 0.039533261951963515 ) ) );
	nodeVar14 = ( nodeConst0 * ( object.nodeUniform2 * vec2<f32>( 5.0 ) ) );
	nodeVar15 = textureSample( nodeUniform0, nodeUniform0_sampler, ( object.nodeUniform1 * vec3<f32>( ( nodeVarying0 + nodeVar14 ), 1.0 ) ).xy ).x;
	nodeVar16 = textureSample( nodeUniform0, nodeUniform0_sampler, ( object.nodeUniform1 * vec3<f32>( ( nodeVarying0 - nodeVar14 ), 1.0 ) ).xy ).x;
	nodeVar1 = ( nodeVar1 + vec4<f32>( ( ( nodeVar15 + nodeVar16 ) * 0.017298361396446957 ) ) );
	nodeVar17 = ( nodeConst0 * ( object.nodeUniform2 * vec2<f32>( 6.0 ) ) );
	nodeVar18 = textureSample( nodeUniform0, nodeUniform0_sampler, ( object.nodeUniform1 * vec3<f32>( ( nodeVarying0 + nodeVar17 ), 1.0 ) ).xy ).x;
	nodeVar19 = textureSample( nodeUniform0, nodeUniform0_sampler, ( object.nodeUniform1 * vec3<f32>( ( nodeVarying0 - nodeVar17 ), 1.0 ) ).xy ).x;
	nodeVar1 = ( nodeVar1 + vec4<f32>( ( ( nodeVar18 + nodeVar19 ) * 0.0062991058730768705 ) ) );

	// result

	output.color = nodeVar1;

	return output;

}
