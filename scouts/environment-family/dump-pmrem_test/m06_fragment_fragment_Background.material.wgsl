// Three.js r186 - Node System

// global
diagnostic( off, derivative_uniformity );


// structs

struct OutputStruct {
	@location( 0 ) color: vec4<f32>
};
var<private> output : OutputStruct;

// uniforms
@binding( 1 ) @group( 1 ) var nodeUniform8_sampler : sampler;
@binding( 2 ) @group( 1 ) var nodeUniform8 : texture_2d<f32>;

struct renderStruct {
	nodeUniform0 : f32,
	nodeUniform9 : f32,
	nodeUniform3 : mat4x4<f32>,
	cameraProjectionMatrix : mat4x4<f32>,
	cameraViewMatrix : mat4x4<f32>
};
@binding( 0 ) @group( 0 )
var<uniform> render : renderStruct;

struct objectStruct {
	nodeUniform1 : f32,
	nodeUniform2 : mat4x4<f32>,
	nodeUniform5 : mat3x3<f32>,
	nodeUniform6 : f32,
	nodeUniform7 : f32,
	nodeUniform10 : f32,
	nodeUniform12 : mat4x4<f32>
};
@binding( 0 ) @group( 1 )
var<uniform> object : objectStruct;

// vars
var<private> DiffuseColor : vec4<f32>;
var<private> nodeVar0 : f32;
var<private> nodeVar1 : f32;
var<private> nodeVar2 : f32;
var<private> normalWorldGeometry : vec3<f32>;
var<private> nodeVar3 : vec4<f32>;
var<private> nodeVar4 : f32;
var<private> nodeVar5 : f32;
var<private> nodeVar6 : f32;
var<private> nodeVar7 : vec2<f32>;
var<private> nodeVar8 : vec4<f32>;
var<private> nodeVar9 : vec3<f32>;
var<private> nodeVar10 : f32;
var<private> nodeVar11 : f32;
var<private> nodeVar12 : f32;
var<private> nodeVar13 : f32;
var<private> nodeVar14 : f32;
var<private> nodeVar15 : vec2<f32>;
var<private> nodeVar16 : vec4<f32>;
var<private> nodeVar17 : vec3<f32>;
var<private> Output : vec4<f32>;
var<private> nodeVar18 : vec4<f32>;

// codes
fn roughnessToMip ( roughness : f32 ) -> f32 {

	var nodeVar0 : f32;

	nodeVar0 = 0.0;

	if ( ( roughness >= 0.8 ) ) {

		nodeVar0 = ( ( ( ( 1.0 - roughness ) * ( -1.0 - -2.0 ) ) / ( 1.0 - 0.8 ) ) + -2.0 );
		

	} else {


		if ( ( roughness >= 0.4 ) ) {

			nodeVar0 = ( ( ( ( 0.8 - roughness ) * ( 2.0 - -1.0 ) ) / ( 0.8 - 0.4 ) ) + -1.0 );
			

		} else {


			if ( ( roughness >= 0.305 ) ) {

				nodeVar0 = ( ( ( ( 0.4 - roughness ) * ( 3.0 - 2.0 ) ) / ( 0.4 - 0.305 ) ) + 2.0 );
				

			} else {


				if ( ( roughness >= 0.21 ) ) {

					nodeVar0 = ( ( ( ( 0.305 - roughness ) * ( 4.0 - 3.0 ) ) / ( 0.305 - 0.21 ) ) + 3.0 );
					

				} else {

					nodeVar0 = ( -2.0 * log2( ( 1.16 * roughness ) ) );
					

				}

				

			}

			

		}

		

	}


	return nodeVar0;

}


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
fn main( @location( 0 ) v_normalWorldGeometry : vec3<f32> ) -> OutputStruct {

	// flow
	// code

	nodeVar0 = clamp( roughnessToMip( render.nodeUniform0 ), -2.0, object.nodeUniform1 );
	nodeVar1 = floor( nodeVar0 );
	nodeVar2 = nodeVar1;
	normalWorldGeometry = normalize( v_normalWorldGeometry );
	nodeVar3 = ( render.nodeUniform3 * vec4<f32>( normalWorldGeometry, 1.0 ) );
	nodeVar4 = getFace( ( object.nodeUniform2 * vec4<f32>( vec3<f32>( nodeVar3.x, ( - nodeVar3.y ), nodeVar3.z ), 1.0 ) ).xyz );
	nodeVar5 = max( ( 4.0 - nodeVar2 ), 0.0 );
	nodeVar2 = max( nodeVar2, 4.0 );
	nodeVar6 = exp2( nodeVar2 );
	nodeVar7 = ( ( getUV( ( object.nodeUniform2 * vec4<f32>( vec3<f32>( nodeVar3.x, ( - nodeVar3.y ), nodeVar3.z ), 1.0 ) ).xyz, nodeVar4 ) * vec2<f32>( ( nodeVar6 - 2.0 ) ) ) + vec2<f32>( 1.0 ) );

	if ( ( nodeVar4 > 2.0 ) ) {

		nodeVar7.y = ( nodeVar7.y + nodeVar6 );
		nodeVar4 = ( nodeVar4 - 3.0 );
		

	}

	nodeVar7.x = ( nodeVar7.x + ( nodeVar4 * nodeVar6 ) );
	nodeVar7.x = ( nodeVar7.x + ( nodeVar5 * ( 3.0 * 16.0 ) ) );
	nodeVar7.y = ( nodeVar7.y + ( 4.0 * ( exp2( object.nodeUniform1 ) - nodeVar6 ) ) );
	nodeVar7.x = ( nodeVar7.x * object.nodeUniform6 );
	nodeVar7.y = ( nodeVar7.y * object.nodeUniform7 );
	nodeVar8 = textureSampleGrad( nodeUniform8, nodeUniform8_sampler, nodeVar7, vec2<f32>( 0.0, 0.0 ), vec2<f32>( 0.0, 0.0 ) );
	nodeVar9 = nodeVar8.xyz;
	nodeVar10 = fract( nodeVar0 );

	if ( ( nodeVar10 != 0.0 ) ) {

		nodeVar11 = ( nodeVar1 + 1.0 );
		nodeVar12 = getFace( ( object.nodeUniform2 * vec4<f32>( vec3<f32>( nodeVar3.x, ( - nodeVar3.y ), nodeVar3.z ), 1.0 ) ).xyz );
		nodeVar13 = max( ( 4.0 - nodeVar11 ), 0.0 );
		nodeVar11 = max( nodeVar11, 4.0 );
		nodeVar14 = exp2( nodeVar11 );
		nodeVar15 = ( ( getUV( ( object.nodeUniform2 * vec4<f32>( vec3<f32>( nodeVar3.x, ( - nodeVar3.y ), nodeVar3.z ), 1.0 ) ).xyz, nodeVar12 ) * vec2<f32>( ( nodeVar14 - 2.0 ) ) ) + vec2<f32>( 1.0 ) );

		if ( ( nodeVar12 > 2.0 ) ) {

			nodeVar15.y = ( nodeVar15.y + nodeVar14 );
			nodeVar12 = ( nodeVar12 - 3.0 );
			

		}

		nodeVar15.x = ( nodeVar15.x + ( nodeVar12 * nodeVar14 ) );
		nodeVar15.x = ( nodeVar15.x + ( nodeVar13 * ( 3.0 * 16.0 ) ) );
		nodeVar15.y = ( nodeVar15.y + ( 4.0 * ( exp2( object.nodeUniform1 ) - nodeVar14 ) ) );
		nodeVar15.x = ( nodeVar15.x * object.nodeUniform6 );
		nodeVar15.y = ( nodeVar15.y * object.nodeUniform7 );
		nodeVar16 = textureSampleGrad( nodeUniform8, nodeUniform8_sampler, nodeVar15, vec2<f32>( 0.0, 0.0 ), vec2<f32>( 0.0, 0.0 ) );
		nodeVar17 = nodeVar16.xyz;
		nodeVar9 = mix( nodeVar9, nodeVar17, nodeVar10 );
		

	}

	DiffuseColor = ( vec4<f32>( nodeVar9, 1.0 ) * vec4<f32>( render.nodeUniform9 ) );
	DiffuseColor.w = ( DiffuseColor.w * object.nodeUniform10 );
	DiffuseColor.w = 1.0;
	nodeVar18 = max( vec4<f32>( DiffuseColor.xyz, DiffuseColor.w ), vec4<f32>( 0.0 ) );
	Output = nodeVar18;

	// result

	output.color = nodeVar18;

	return output;

}
