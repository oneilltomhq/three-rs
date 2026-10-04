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
@binding( 3 ) @group( 0 ) var nodeUniform3_sampler : sampler;
@binding( 4 ) @group( 0 ) var nodeUniform3 : texture_2d<f32>;

struct objectStruct {
	nodeUniform1 : mat3x3<f32>,
	nodeUniform2 : vec2<f32>,
	nodeUniform4 : vec2<f32>
};
@binding( 2 ) @group( 0 )
var<uniform> object : objectStruct;

// vars
var<private> nodeVar0 : vec4<f32>;
var<private> nodeVar1 : vec4<f32>;
var<private> nodeVar2 : vec4<f32>;
var<private> nodeVar3 : vec2<f32>;
var<private> nodeVar4 : vec4<f32>;
var<private> nodeVar5 : vec4<f32>;
var<private> nodeVar6 : vec4<f32>;
var<private> nodeVar7 : vec2<f32>;
var<private> nodeVar8 : f32;
var<private> nodeVar9 : f32;
var<private> nodeVar10 : vec4<f32>;
var<private> nodeVar11 : vec4<f32>;
var<private> nodeVar12 : vec2<f32>;
var<private> nodeVar13 : vec4<f32>;
var<private> nodeVar14 : vec4<f32>;
var<private> nodeVar15 : f32;
var<private> nodeVar16 : f32;

// codes


@fragment
fn main( @location( 0 ) nodeVarying0 : vec4<f32>,
	@location( 1 ) nodeVarying1 : vec2<f32> ) -> OutputStruct {

	// flow
	// code

	nodeVar0 = vec4<f32>( 0.0, 0.0, 0.0, 1.0 );
	nodeVar1 = vec4<f32>( 0.0, 0.0, 0.0, 1.0 );
	nodeVar2 = textureSample( nodeUniform0, nodeUniform0_sampler, ( object.nodeUniform1 * vec3<f32>( nodeVarying1, 1.0 ) ).xy );
	nodeVar3 = nodeVar2.xz;
	nodeVar1.x = nodeVar3[ 0 ];
	nodeVar1.z = nodeVar3[ 1 ];
	nodeVar4 = textureSample( nodeUniform0, nodeUniform0_sampler, ( object.nodeUniform1 * vec3<f32>( nodeVarying0.zw, 1.0 ) ).xy );
	nodeVar1.y = nodeVar4.y;
	nodeVar5 = textureSample( nodeUniform0, nodeUniform0_sampler, ( object.nodeUniform1 * vec3<f32>( nodeVarying0.xy, 1.0 ) ).xy );
	nodeVar1.w = nodeVar5.w;

	if ( ( dot( nodeVar1, vec4<f32>( 1.0, 1.0, 1.0, 1.0 ) ) < 0.00001 ) ) {

		nodeVar6 = textureSample( nodeUniform3, nodeUniform3_sampler, nodeVarying1 );
		nodeVar0 = nodeVar6;
		

	} else {

		nodeVar7 = vec2<f32>( 0.0, 0.0 );

		if ( ( nodeVar1.w > nodeVar1.z ) ) {

			nodeVar8 = nodeVar1.w;

		} else {

			nodeVar8 = ( - nodeVar1.z );

		}

		nodeVar7.x = nodeVar8;

		if ( ( nodeVar1.y > nodeVar1.x ) ) {

			nodeVar9 = nodeVar1.y;

		} else {

			nodeVar9 = ( - nodeVar1.x );

		}

		nodeVar7.y = nodeVar9;

		if ( ( abs( nodeVar7.x ) > abs( nodeVar7.y ) ) ) {

			nodeVar7.y = 0.0;
			

		} else {

			nodeVar7.x = 0.0;
			

		}

		nodeVar10 = textureSample( nodeUniform3, nodeUniform3_sampler, nodeVarying1 );
		nodeVar11 = nodeVar10;
		nodeVar12 = nodeVarying1;
		nodeVar12 = ( nodeVar12 + ( sign( nodeVar7 ) * object.nodeUniform4 ) );
		nodeVar13 = textureSample( nodeUniform3, nodeUniform3_sampler, nodeVar12 );
		nodeVar14 = nodeVar13;

		if ( ( abs( nodeVar7.x ) > abs( nodeVar7.y ) ) ) {

			nodeVar15 = abs( nodeVar7.x );

		} else {

			nodeVar15 = abs( nodeVar7.y );

		}

		nodeVar16 = nodeVar15;
		nodeVar0 = mix( nodeVar11, nodeVar14, nodeVar16 );
		

	}


	// result

	output.color = nodeVar0;

	return output;

}
