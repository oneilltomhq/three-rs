// Three.js r186 - Node System

// global
diagnostic( off, derivative_uniformity );


// structs

struct OutputStruct {
	@location( 0 ) color: vec4<f32>
};
var<private> output : OutputStruct;

// uniforms
@binding( 1 ) @group( 1 ) var nodeUniform2_sampler : sampler;
@binding( 2 ) @group( 1 ) var nodeUniform2 : texture_2d<f32>;

struct objectStruct {
	nodeUniform0 : f32,
	nodeUniform1 : f32,
	nodeUniform5 : mat4x4<f32>
};
@binding( 0 ) @group( 1 )
var<uniform> object : objectStruct;

// vars
var<private> nodeVar0 : vec3<f32>;
var<private> nodeVar1 : f32;
var<private> nodeVar2 : f32;
var<private> nodeVar3 : f32;
var<private> nodeVar4 : f32;
var<private> nodeVar5 : vec2<f32>;
var<private> nodeVar6 : vec4<f32>;
var<private> nodeVar7 : vec3<f32>;
var<private> nodeVar8 : vec3<f32>;
var<private> nodeVar9 : vec3<f32>;
var<private> nodeVar10 : vec3<f32>;
var<private> nodeVar11 : f32;
var<private> nodeVar12 : f32;
var<private> nodeVar13 : f32;
var<private> nodeVar14 : f32;
var<private> nodeVar15 : f32;
var<private> nodeVar16 : f32;
var<private> nodeVar17 : f32;
var<private> nodeVar18 : f32;
var<private> nodeVar19 : f32;
var<private> nodeVar20 : f32;
var<private> nodeVar21 : vec2<f32>;
var<private> nodeVar22 : vec4<f32>;

// codes
fn getFace ( direction : vec3<f32> ) -> f32 {

	var nodeVar0 : vec3<f32>;
	var nodeVar1 : f32;
	var nodeVar2 : f32;
	var nodeVar3 : f32;
	var nodeVar4 : f32;
	var nodeVar5 : f32;

	nodeVar0 = abs( direction );
	nodeVar1 = -1.0;

	if ( ( nodeVar0.x > nodeVar0.z ) ) {


		if ( ( nodeVar0.x > nodeVar0.y ) ) {


			if ( ( direction.x > 0.0 ) ) {

				nodeVar2 = 0.0;

			} else {

				nodeVar2 = 3.0;

			}

			nodeVar1 = nodeVar2;
			

		} else {


			if ( ( direction.y > 0.0 ) ) {

				nodeVar3 = 1.0;

			} else {

				nodeVar3 = 4.0;

			}

			nodeVar1 = nodeVar3;
			

		}

		

	} else {


		if ( ( nodeVar0.z > nodeVar0.y ) ) {


			if ( ( direction.z > 0.0 ) ) {

				nodeVar4 = 2.0;

			} else {

				nodeVar4 = 5.0;

			}

			nodeVar1 = nodeVar4;
			

		} else {


			if ( ( direction.y > 0.0 ) ) {

				nodeVar5 = 1.0;

			} else {

				nodeVar5 = 4.0;

			}

			nodeVar1 = nodeVar5;
			

		}

		

	}


	return nodeVar1;

}


fn getUV ( direction : vec3<f32>, face : f32 ) -> vec2<f32> {

	var nodeVar0 : vec2<f32>;

	nodeVar0 = vec2<f32>( 0.0, 0.0 );

	if ( ( face == 0.0 ) ) {

		nodeVar0 = ( vec2<f32>( direction.z, direction.y ) / vec2<f32>( abs( direction.x ) ) );
		

	} else {


		if ( ( face == 1.0 ) ) {

			nodeVar0 = ( vec2<f32>( ( - direction.x ), ( - direction.z ) ) / vec2<f32>( abs( direction.y ) ) );
			

		} else {


			if ( ( face == 2.0 ) ) {

				nodeVar0 = ( vec2<f32>( ( - direction.x ), direction.y ) / vec2<f32>( abs( direction.z ) ) );
				

			} else {


				if ( ( face == 3.0 ) ) {

					nodeVar0 = ( vec2<f32>( ( - direction.z ), direction.y ) / vec2<f32>( abs( direction.x ) ) );
					

				} else {


					if ( ( face == 4.0 ) ) {

						nodeVar0 = ( vec2<f32>( ( - direction.x ), direction.z ) / vec2<f32>( abs( direction.y ) ) );
						

					} else {

						nodeVar0 = ( vec2<f32>( direction.x, direction.y ) / vec2<f32>( abs( direction.z ) ) );
						

					}

					

				}

				

			}

			

		}

		

	}


	return ( vec2<f32>( 0.5 ) * ( nodeVar0 + vec2<f32>( 1.0 ) ) );

}




@fragment
fn main( @location( 0 ) nodeVarying4 : vec3<f32> ) -> OutputStruct {

	// flow
	// code

	nodeVar0 = vec3<f32>( 0.0, 0.0, 0.0 );

	if ( ( object.nodeUniform0 == 0.0 ) ) {

		nodeVar1 = object.nodeUniform1;
		nodeVar2 = getFace( normalize( nodeVarying4 ) );
		nodeVar3 = max( ( 4.0 - nodeVar1 ), 0.0 );
		nodeVar1 = max( nodeVar1, 4.0 );
		nodeVar4 = exp2( nodeVar1 );
		nodeVar5 = ( ( getUV( normalize( nodeVarying4 ), nodeVar2 ) * vec2<f32>( ( nodeVar4 - 2.0 ) ) ) + vec2<f32>( 1.0 ) );

		if ( ( nodeVar2 > 2.0 ) ) {

			nodeVar5.y = ( nodeVar5.y + nodeVar4 );
			nodeVar2 = ( nodeVar2 - 3.0 );
			

		}

		nodeVar5.x = ( nodeVar5.x + ( nodeVar2 * nodeVar4 ) );
		nodeVar5.x = ( nodeVar5.x + ( nodeVar3 * ( 3.0 * 16.0 ) ) );
		nodeVar5.y = ( nodeVar5.y + ( 4.0 * ( exp2( 8.0 ) - nodeVar4 ) ) );
		nodeVar5.x = ( nodeVar5.x * 0.0013020833333333333 );
		nodeVar5.y = ( nodeVar5.y * 0.0009765625 );
		nodeVar6 = textureSampleGrad( nodeUniform2, nodeUniform2_sampler, nodeVar5, vec2<f32>( 0.0, 0.0 ), vec2<f32>( 0.0, 0.0 ) );
		nodeVar0 = nodeVar6.xyz;
		

	} else {

		nodeVar8 = normalize( nodeVarying4 );

		if ( ( abs( nodeVar8.z ) < 0.999 ) ) {

			nodeVar7 = vec3<f32>( 0.0, 0.0, 1.0 );

		} else {

			nodeVar7 = vec3<f32>( 1.0, 0.0, 0.0 );

		}

		nodeVar9 = normalize( cross( nodeVar7, nodeVar8 ) );
		nodeVar10 = cross( nodeVar8, nodeVar9 );
		nodeVar11 = min( ( object.nodeUniform0 * 3.0 ), 3.141592653589793 );
		nodeVar12 = ( 1.0 - exp( ( ( ( nodeVar11 * nodeVar11 ) * -0.5 ) / ( object.nodeUniform0 * object.nodeUniform0 ) ) ) );
		nodeVar13 = 0.0;

		for ( var i : i32 = 0; i < 20; i ++ ) {

			nodeVar14 = ( object.nodeUniform0 * sqrt( ( log( ( 1.0 - ( ( ( f32( i ) + 0.5 ) / 20.0 ) * nodeVar12 ) ) ) * -2.0 ) ) );
			nodeVar15 = ( f32( i ) * 2.399963229728653 );
			nodeVar16 = ( sin( nodeVar14 ) / nodeVar14 );
			nodeVar17 = object.nodeUniform1;
			nodeVar18 = getFace( ( ( nodeVar8 * vec3<f32>( cos( nodeVar14 ) ) ) + ( ( ( nodeVar9 * vec3<f32>( cos( nodeVar15 ) ) ) + ( nodeVar10 * vec3<f32>( sin( nodeVar15 ) ) ) ) * vec3<f32>( sin( nodeVar14 ) ) ) ) );
			nodeVar19 = max( ( 4.0 - nodeVar17 ), 0.0 );
			nodeVar17 = max( nodeVar17, 4.0 );
			nodeVar20 = exp2( nodeVar17 );
			nodeVar21 = ( ( getUV( ( ( nodeVar8 * vec3<f32>( cos( nodeVar14 ) ) ) + ( ( ( nodeVar9 * vec3<f32>( cos( nodeVar15 ) ) ) + ( nodeVar10 * vec3<f32>( sin( nodeVar15 ) ) ) ) * vec3<f32>( sin( nodeVar14 ) ) ) ), nodeVar18 ) * vec2<f32>( ( nodeVar20 - 2.0 ) ) ) + vec2<f32>( 1.0 ) );

			if ( ( nodeVar18 > 2.0 ) ) {

				nodeVar21.y = ( nodeVar21.y + nodeVar20 );
				nodeVar18 = ( nodeVar18 - 3.0 );
				

			}

			nodeVar21.x = ( nodeVar21.x + ( nodeVar18 * nodeVar20 ) );
			nodeVar21.x = ( nodeVar21.x + ( nodeVar19 * ( 3.0 * 16.0 ) ) );
			nodeVar21.y = ( nodeVar21.y + ( 4.0 * ( exp2( 8.0 ) - nodeVar20 ) ) );
			nodeVar21.x = ( nodeVar21.x * 0.0013020833333333333 );
			nodeVar21.y = ( nodeVar21.y * 0.0009765625 );
			nodeVar22 = textureSampleGrad( nodeUniform2, nodeUniform2_sampler, nodeVar21, vec2<f32>( 0.0, 0.0 ), vec2<f32>( 0.0, 0.0 ) );
			nodeVar0 = ( vec4<f32>( nodeVar0, 1.0 ) + ( nodeVar22 * vec4<f32>( nodeVar16 ) ) ).xyz;
			nodeVar13 = ( nodeVar13 + nodeVar16 );

		}

		nodeVar0 = ( nodeVar0 / vec3<f32>( nodeVar13 ) );
		

	}


	// result

	output.color = vec4<f32>( nodeVar0, 1.0 );

	return output;

}
