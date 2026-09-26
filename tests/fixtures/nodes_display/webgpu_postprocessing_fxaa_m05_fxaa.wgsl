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

struct NodeBuffer_922Struct {
	value : array< vec4<f32>, 6 >
};
@binding( 2 ) @group( 0 )
var<uniform> NodeBuffer_922 : NodeBuffer_922Struct;

struct objectStruct {
	nodeUniform2 : vec2<f32>
};
@binding( 3 ) @group( 0 )
var<uniform> object : objectStruct;

// vars


// codes
fn FxaaPixelShader ( uv : vec2<f32>, texSize : vec2<f32> ) -> vec4<f32> {

	var nodeVar0 : vec2<f32>;
	var nodeVar1 : vec4<f32>;
	var nodeVar2 : vec4<f32>;
	var nodeVar3 : vec4<f32>;
	var nodeVar4 : vec4<f32>;
	var nodeVar5 : vec4<f32>;
	var nodeVar6 : f32;
	var nodeVar7 : vec4<f32>;
	var nodeVar8 : vec4<f32>;
	var nodeVar9 : vec4<f32>;
	var nodeVar10 : vec4<f32>;
	var nodeVar11 : f32;
	var nodeVar12 : f32;
	var nodeVar13 : f32;
	var nodeVar14 : f32;
	var nodeVar15 : f32;
	var nodeVar16 : vec2<f32>;
	var nodeVar17 : vec2<f32>;
	var nodeVar18 : vec2<f32>;
	var nodeVar19 : vec4<f32>;
	var nodeVar20 : f32;
	var nodeVar21 : bool;
	var nodeVar22 : vec4<f32>;
	var nodeVar23 : vec2<f32>;
	var nodeVar24 : vec4<f32>;
	var nodeVar25 : f32;
	var nodeVar26 : bool;
	var nodeVar27 : vec4<f32>;
	var nodeVar28 : f32;
	var nodeVar29 : f32;
	var nodeVar30 : f32;
	var nodeVar31 : bool;
	var nodeVar32 : f32;
	var nodeVar33 : vec4<f32>;

	nodeVar0 = uv;
	nodeVar1 = textureSampleBias( nodeUniform0, nodeUniform0_sampler, ( uv + ( texSize * vec2<f32>( 0.0, 1.0 ) ) ), -100.0 );
	let nodeConst0 = dot( nodeVar1.xyz, vec3<f32>( 0.3, 0.59, 0.11 ) );
	nodeVar2 = textureSampleBias( nodeUniform0, nodeUniform0_sampler, ( uv + ( texSize * vec2<f32>( 1.0, 0.0 ) ) ), -100.0 );
	let nodeConst1 = dot( nodeVar2.xyz, vec3<f32>( 0.3, 0.59, 0.11 ) );
	nodeVar3 = textureSampleBias( nodeUniform0, nodeUniform0_sampler, ( uv + ( texSize * vec2<f32>( 0.0, -1.0 ) ) ), -100.0 );
	let nodeConst2 = dot( nodeVar3.xyz, vec3<f32>( 0.3, 0.59, 0.11 ) );
	nodeVar4 = textureSampleBias( nodeUniform0, nodeUniform0_sampler, ( uv + ( texSize * vec2<f32>( -1.0, 0.0 ) ) ), -100.0 );
	let nodeConst3 = dot( nodeVar4.xyz, vec3<f32>( 0.3, 0.59, 0.11 ) );
	nodeVar5 = textureSampleBias( nodeUniform0, nodeUniform0_sampler, uv, -100.0 );
	let nodeConst4 = dot( nodeVar5.xyz, vec3<f32>( 0.3, 0.59, 0.11 ) );
	let nodeConst5 = max( max( max( max( nodeConst0, nodeConst1 ), nodeConst2 ), nodeConst3 ), nodeConst4 );
	let nodeConst6 = ( nodeConst5 - min( min( min( min( nodeConst0, nodeConst1 ), nodeConst2 ), nodeConst3 ), nodeConst4 ) );

	if ( ( ! ( nodeConst6 < max( 0.0312, ( 0.063 * nodeConst5 ) ) ) ) ) {

		nodeVar7 = textureSampleBias( nodeUniform0, nodeUniform0_sampler, ( uv + ( texSize * vec2<f32>( 1.0, 1.0 ) ) ), -100.0 );
		let nodeConst7 = dot( nodeVar7.xyz, vec3<f32>( 0.3, 0.59, 0.11 ) );
		nodeVar8 = textureSampleBias( nodeUniform0, nodeUniform0_sampler, ( uv + ( texSize * vec2<f32>( 1.0, -1.0 ) ) ), -100.0 );
		let nodeConst8 = dot( nodeVar8.xyz, vec3<f32>( 0.3, 0.59, 0.11 ) );
		nodeVar9 = textureSampleBias( nodeUniform0, nodeUniform0_sampler, ( uv + ( texSize * vec2<f32>( -1.0, 1.0 ) ) ), -100.0 );
		let nodeConst9 = dot( nodeVar9.xyz, vec3<f32>( 0.3, 0.59, 0.11 ) );
		nodeVar10 = textureSampleBias( nodeUniform0, nodeUniform0_sampler, ( uv + ( texSize * vec2<f32>( -1.0, -1.0 ) ) ), -100.0 );
		let nodeConst10 = dot( nodeVar10.xyz, vec3<f32>( 0.3, 0.59, 0.11 ) );
		let nodeConst11 = ( ( ( abs( ( ( nodeConst0 + nodeConst2 ) - ( nodeConst4 * 2.0 ) ) ) * 2.0 ) + ( abs( ( ( nodeConst7 + nodeConst8 ) - ( nodeConst1 * 2.0 ) ) ) + abs( ( ( nodeConst9 + nodeConst10 ) - ( nodeConst3 * 2.0 ) ) ) ) ) >= ( ( abs( ( ( nodeConst1 + nodeConst3 ) - ( nodeConst4 * 2.0 ) ) ) * 2.0 ) + ( abs( ( ( nodeConst7 + nodeConst9 ) - ( nodeConst0 * 2.0 ) ) ) + abs( ( ( nodeConst8 + nodeConst10 ) - ( nodeConst2 * 2.0 ) ) ) ) ) );

		if ( nodeConst11 ) {

			nodeVar6 = texSize.y;

		} else {

			nodeVar6 = texSize.x;

		}

		nodeVar11 = nodeVar6;
		nodeVar12 = 0.0;
		nodeVar13 = 0.0;

		if ( nodeConst11 ) {

			nodeVar14 = nodeConst0;

		} else {

			nodeVar14 = nodeConst1;

		}

		let nodeConst12 = abs( ( nodeVar14 - nodeConst4 ) );

		if ( nodeConst11 ) {

			nodeVar15 = nodeConst2;

		} else {

			nodeVar15 = nodeConst3;

		}

		let nodeConst13 = abs( ( nodeVar15 - nodeConst4 ) );

		if ( ( nodeConst12 < nodeConst13 ) ) {

			nodeVar11 = ( - nodeVar11 );
			nodeVar12 = nodeVar15;
			nodeVar13 = nodeConst13;
			

		} else {

			nodeVar12 = nodeVar14;
			nodeVar13 = nodeConst12;
			

		}

		nodeVar16 = uv;
		nodeVar17 = vec2<f32>( 0.0, 0.0 );

		if ( nodeConst11 ) {

			nodeVar16.y = ( nodeVar16.y + ( nodeVar11 * 0.5 ) );
			nodeVar17 = vec2<f32>( texSize.x, 0.0 );
			

		} else {

			nodeVar16.x = ( nodeVar16.x + ( nodeVar11 * 0.5 ) );
			nodeVar17 = vec2<f32>( 0.0, texSize.y );
			

		}

		nodeVar18 = ( nodeVar16 + ( nodeVar17 * vec2<f32>( NodeBuffer_922.value[ 0u ].x ) ) );
		nodeVar19 = textureSampleBias( nodeUniform0, nodeUniform0_sampler, nodeVar18, -100.0 );
		let nodeConst14 = ( ( nodeConst4 + nodeVar12 ) * 0.5 );
		nodeVar20 = ( dot( nodeVar19.xyz, vec3<f32>( 0.3, 0.59, 0.11 ) ) - nodeConst14 );
		let nodeConst15 = ( nodeVar13 * 0.25 );
		nodeVar21 = ( abs( nodeVar20 ) >= nodeConst15 );

		for ( var i : i32 = 1; i < 6; i ++ ) {


			if ( nodeVar21 ) {

				break;
				

			}

			nodeVar18 = ( nodeVar18 + ( nodeVar17 * vec2<f32>( NodeBuffer_922.value[ i ].x ) ) );
			nodeVar22 = textureSampleBias( nodeUniform0, nodeUniform0_sampler, nodeVar18, -100.0 );
			nodeVar20 = ( dot( nodeVar22.xyz, vec3<f32>( 0.3, 0.59, 0.11 ) ) - nodeConst14 );
			nodeVar21 = ( abs( nodeVar20 ) >= nodeConst15 );

		}


		if ( ( ! nodeVar21 ) ) {

			nodeVar18 = ( nodeVar18 + ( nodeVar17 * vec2<f32>( 8.0 ) ) );
			

		}

		nodeVar23 = ( nodeVar16 - ( nodeVar17 * vec2<f32>( NodeBuffer_922.value[ 0u ].x ) ) );
		nodeVar24 = textureSampleBias( nodeUniform0, nodeUniform0_sampler, nodeVar23, -100.0 );
		nodeVar25 = ( dot( nodeVar24.xyz, vec3<f32>( 0.3, 0.59, 0.11 ) ) - nodeConst14 );
		nodeVar26 = ( abs( nodeVar25 ) >= nodeConst15 );

		for ( var i : i32 = 1; i < 6; i ++ ) {


			if ( nodeVar26 ) {

				break;
				

			}

			nodeVar23 = ( nodeVar23 - ( nodeVar17 * vec2<f32>( NodeBuffer_922.value[ i ].x ) ) );
			nodeVar27 = textureSampleBias( nodeUniform0, nodeUniform0_sampler, nodeVar23, -100.0 );
			nodeVar25 = ( dot( nodeVar27.xyz, vec3<f32>( 0.3, 0.59, 0.11 ) ) - nodeConst14 );
			nodeVar26 = ( abs( nodeVar25 ) >= nodeConst15 );

		}


		if ( ( ! nodeVar26 ) ) {

			nodeVar23 = ( nodeVar23 - ( nodeVar17 * vec2<f32>( 8.0 ) ) );
			

		}

		nodeVar28 = 0.0;
		nodeVar29 = 0.0;

		if ( nodeConst11 ) {

			nodeVar28 = ( nodeVar18.x - uv.x );
			nodeVar29 = ( uv.x - nodeVar23.x );
			

		} else {

			nodeVar28 = ( nodeVar18.y - uv.y );
			nodeVar29 = ( uv.y - nodeVar23.y );
			

		}

		nodeVar30 = 0.0;
		nodeVar31 = false;

		if ( ( nodeVar28 <= nodeVar29 ) ) {

			nodeVar30 = nodeVar28;
			nodeVar31 = ( nodeVar20 >= 0.0 );
			

		} else {

			nodeVar30 = nodeVar29;
			nodeVar31 = ( nodeVar25 >= 0.0 );
			

		}

		nodeVar32 = 0.0;

		if ( ( nodeVar31 == ( ( nodeConst4 - nodeConst14 ) >= 0.0 ) ) ) {

			nodeVar32 = 0.0;
			

		} else {

			nodeVar32 = ( 0.5 - ( nodeVar30 / ( nodeVar28 + nodeVar29 ) ) );
			

		}


		if ( nodeConst11 ) {

			let nodeConst16 = smoothstep( 0.0, 1.0, clamp( ( abs( ( ( ( ( 2.0 * ( ( ( nodeConst0 + nodeConst1 ) + nodeConst2 ) + nodeConst3 ) ) + ( ( ( nodeConst7 + nodeConst9 ) + nodeConst8 ) + nodeConst10 ) ) * 0.08333333333333333 ) - nodeConst4 ) ) / max( nodeConst6, 0.0 ) ), 0.0, 1.0 ) );
			nodeVar0.y = ( nodeVar0.y + ( nodeVar11 * max( ( ( nodeConst16 * nodeConst16 ) * 1.0 ), nodeVar32 ) ) );
			

		} else {

			let nodeConst17 = smoothstep( 0.0, 1.0, clamp( ( abs( ( ( ( ( 2.0 * ( ( ( nodeConst0 + nodeConst1 ) + nodeConst2 ) + nodeConst3 ) ) + ( ( ( nodeConst7 + nodeConst9 ) + nodeConst8 ) + nodeConst10 ) ) * 0.08333333333333333 ) - nodeConst4 ) ) / max( nodeConst6, 0.0 ) ), 0.0, 1.0 ) );
			nodeVar0.x = ( nodeVar0.x + ( nodeVar11 * max( ( ( nodeConst17 * nodeConst17 ) * 1.0 ), nodeVar32 ) ) );
			

		}

		

	}

	nodeVar33 = textureSampleBias( nodeUniform0, nodeUniform0_sampler, nodeVar0, -100.0 );

	return nodeVar33;

}




@fragment
fn main( @location( 0 ) nodeVarying0 : vec2<f32> ) -> OutputStruct {

	// flow
	// code


	// result

	output.color = FxaaPixelShader( nodeVarying0, object.nodeUniform2 );

	return output;

}
