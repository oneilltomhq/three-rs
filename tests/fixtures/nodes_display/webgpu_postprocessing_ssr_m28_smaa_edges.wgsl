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

// vars
var<private> nodeVar0 : vec4<f32>;
var<private> nodeVar1 : vec4<f32>;
var<private> nodeVar2 : vec3<f32>;
var<private> nodeVar3 : vec4<f32>;
var<private> nodeVar4 : vec3<f32>;
var<private> nodeVar5 : vec4<f32>;
var<private> nodeVar6 : vec3<f32>;
var<private> nodeVar7 : vec2<f32>;
var<private> nodeVar8 : vec4<f32>;
var<private> nodeVar9 : vec3<f32>;
var<private> nodeVar10 : vec4<f32>;
var<private> nodeVar11 : vec3<f32>;
var<private> nodeVar12 : f32;
var<private> nodeVar13 : vec4<f32>;
var<private> nodeVar14 : vec3<f32>;
var<private> nodeVar15 : vec4<f32>;
var<private> nodeVar16 : vec3<f32>;

// codes


@fragment
fn main( @location( 0 ) nodeVarying0 : vec4<f32>,
	@location( 1 ) nodeVarying1 : vec4<f32>,
	@location( 2 ) nodeVarying2 : vec4<f32>,
	@location( 3 ) nodeVarying3 : vec2<f32> ) -> OutputStruct {

	// flow
	// code

	nodeVar0 = vec4<f32>( 0.0, 0.0, 0.0, 1.0 );
	nodeVar1 = textureSample( nodeUniform0, nodeUniform0_sampler, nodeVarying3 );
	nodeVar2 = nodeVar1.xyz;
	nodeVar3 = textureSample( nodeUniform0, nodeUniform0_sampler, nodeVarying0.xy );
	nodeVar4 = nodeVar3.xyz;
	let nodeConst1 = abs( ( nodeVar2 - nodeVar4 ) );
	nodeVar0.x = max( max( nodeConst1.x, nodeConst1.y ), nodeConst1.z );
	nodeVar5 = textureSample( nodeUniform0, nodeUniform0_sampler, nodeVarying0.zw );
	nodeVar6 = nodeVar5.xyz;
	let nodeConst2 = abs( ( nodeVar2 - nodeVar6 ) );
	nodeVar0.y = max( max( nodeConst2.x, nodeConst2.y ), nodeConst2.z );
	nodeVar7 = step( vec2<f32>( 0.1, 0.1 ), nodeVar0.xy );

	if ( ( dot( nodeVar7, vec2<f32>( 1.0, 1.0 ) ) == 0.0 ) ) {

		discard;
		

	}

	nodeVar8 = textureSample( nodeUniform0, nodeUniform0_sampler, nodeVarying1.xy );
	nodeVar9 = nodeVar8.xyz;
	let nodeConst4 = abs( ( nodeVar2 - nodeVar9 ) );
	nodeVar0.z = max( max( nodeConst4.x, nodeConst4.y ), nodeConst4.z );
	nodeVar10 = textureSample( nodeUniform0, nodeUniform0_sampler, nodeVarying1.zw );
	nodeVar11 = nodeVar10.xyz;
	let nodeConst5 = abs( ( nodeVar2 - nodeVar11 ) );
	nodeVar0.w = max( max( nodeConst5.x, nodeConst5.y ), nodeConst5.z );
	nodeVar12 = max( max( max( nodeVar0.x, nodeVar0.y ), nodeVar0.z ), nodeVar0.w );
	nodeVar13 = textureSample( nodeUniform0, nodeUniform0_sampler, nodeVarying2.xy );
	nodeVar14 = nodeVar13.xyz;
	let nodeConst7 = abs( ( nodeVar2 - nodeVar14 ) );
	nodeVar0.z = max( max( nodeConst7.x, nodeConst7.y ), nodeConst7.z );
	nodeVar15 = textureSample( nodeUniform0, nodeUniform0_sampler, nodeVarying2.zw );
	nodeVar16 = nodeVar15.xyz;
	let nodeConst8 = abs( ( nodeVar2 - nodeVar16 ) );
	nodeVar0.w = max( max( nodeConst8.x, nodeConst8.y ), nodeConst8.z );
	nodeVar7 = ( nodeVar7 * step( vec2<f32>( ( 0.5 * max( max( nodeVar12, nodeVar0.z ), nodeVar0.w ) ) ), nodeVar0.xy ) );

	// result

	output.color = vec4<f32>( nodeVar7, 0.0, 0.0 );

	return output;

}
