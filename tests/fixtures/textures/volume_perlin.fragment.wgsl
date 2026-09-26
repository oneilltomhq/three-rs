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
@binding( 1 ) @group( 1 ) var nodeUniform3_sampler : sampler;
@binding( 2 ) @group( 1 ) var nodeUniform3 : texture_3d<f32>;

struct objectStruct {
	nodeUniform0 : mat4x4<f32>,
	nodeUniform2 : f32,
	nodeUniform4 : f32,
	nodeUniform5 : u32,
	nodeUniform6 : f32,
	nodeUniform9 : mat4x4<f32>
};
@binding( 0 ) @group( 1 )
var<uniform> object : objectStruct;

// vars
var<private> DiffuseColor : vec4<f32>;
var<private> nodeVar0 : vec4<f32>;
var<private> nodeVar1 : vec3<f32>;
var<private> nodeVar2 : bool;
var<private> nodeVar3 : vec2<f32>;
var<private> nodeVar4 : vec3<f32>;
var<private> nodeVar5 : f32;
var<private> nodeVar6 : vec3<f32>;
var<private> nodeVar7 : f32;
var<private> nodeVar8 : f32;
var<private> nodeVar9 : vec3<f32>;
var<private> nodeVar10 : bool;
var<private> nodeVar11 : vec3<f32>;
var<private> nodeVar12 : vec3<f32>;
var<private> nodeVar13 : f32;
var<private> nodeVar14 : vec3<f32>;
var<private> nodeVar15 : vec3<f32>;
var<private> nodeVar16 : vec3<f32>;
var<private> nodeVar17 : f32;
var<private> nodeVar18 : f32;
var<private> nodeVar19 : f32;
var<private> nodeVar20 : f32;
var<private> nodeVar21 : f32;
var<private> nodeVar22 : f32;
var<private> nodeVar23 : vec3<f32>;
var<private> Output : vec4<f32>;

// codes


@fragment
fn main( @location( 0 ) nodeVarying3 : vec3<f32>,
	@location( 1 ) nodeVarying4 : vec3<f32> ) -> OutputStruct {

	// flow
	// code

	nodeVar0 = vec4<f32>( 0.0, 0.0, 0.0, 0.0 );
	nodeVar1 = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar2 = false;
	let nodeConst1 = normalize( nodeVarying4 );
	let nodeConst2 = ( vec3<f32>( 1.0 ) / nodeConst1 );
	let nodeConst3 = ( ( vec3<f32>( -0.5, -0.5, -0.5 ) - nodeVarying3 ) * nodeConst2 );
	let nodeConst4 = ( ( vec3<f32>( 0.5, 0.5, 0.5 ) - nodeVarying3 ) * nodeConst2 );
	let nodeConst5 = min( nodeConst3, nodeConst4 );
	let nodeConst6 = max( nodeConst3, nodeConst4 );
	nodeVar3 = vec2<f32>( max( nodeConst5.x, max( nodeConst5.y, nodeConst5.z ) ), min( nodeConst6.x, min( nodeConst6.y, nodeConst6.z ) ) );

	if ( ( nodeVar3.x > nodeVar3.y ) ) {

		discard;
		

	}

	nodeVar3 = vec2<f32>( max( nodeVar3.x, 0.0 ), nodeVar3.y );
	nodeVar4 = ( vec3<f32>( 1.0 ) / abs( nodeConst1 ) );
	nodeVar5 = min( nodeVar4.x, min( nodeVar4.y, nodeVar4.z ) );
	nodeVar5 = ( nodeVar5 / object.nodeUniform2 );
	nodeVar6 = ( nodeVarying3 + ( vec3<f32>( nodeVar3.x ) * nodeConst1 ) );

	for ( var i : f32 = nodeVar3.x; i < nodeVar3.y; i += nodeVar5 ) {

		nodeVar7 = textureSampleLevel( nodeUniform3, nodeUniform3_sampler, ( nodeVar6 + vec3<f32>( 0.5 ) ), 0.0 ).x;
		nodeVar8 = nodeVar7;

		if ( ( nodeVar8 > object.nodeUniform4 ) ) {

			nodeVar9 = nodeVar6;
			nodeVar10 = bool( object.nodeUniform5 );

			if ( ( nodeVar10 && nodeVar2 ) ) {

				nodeVar11 = nodeVar1;
				nodeVar12 = nodeVar6;

				for ( var i : i32 = 0; i < 4; i ++ ) {

					let nodeConst7 = ( ( nodeVar11 + nodeVar12 ) * vec3<f32>( 0.5 ) );
					nodeVar13 = textureSampleLevel( nodeUniform3, nodeUniform3_sampler, ( nodeConst7 + vec3<f32>( 0.5 ) ), 0.0 ).x;
					let nodeConst8 = nodeVar13;
					let nodeConst9 = ( nodeConst8 > object.nodeUniform4 );
					nodeVar12 = select( nodeVar12, nodeConst7, nodeConst9 );
					nodeVar11 = select( nodeConst7, nodeVar11, nodeConst9 );

				}

				nodeVar9 = nodeVar12;
				

			}

			nodeVar16 = vec3<f32>( 0.0, 0.0, 0.0 );
			let nodeConst10 = ( nodeVar9 + vec3<f32>( 0.5 ) );

			if ( ( nodeConst10.x < 0.0001 ) ) {

				nodeVar16 = vec3<f32>( 1.0, 0.0, 0.0 );
				

			} else {


				if ( ( nodeConst10.y < 0.0001 ) ) {

					nodeVar16 = vec3<f32>( 0.0, 1.0, 0.0 );
					

				} else {


					if ( ( nodeConst10.z < 0.0001 ) ) {

						nodeVar16 = vec3<f32>( 0.0, 0.0, 1.0 );
						

					} else {


						if ( ( nodeConst10.x > 0.9999 ) ) {

							nodeVar16 = vec3<f32>( -1.0, 0.0, 0.0 );
							

						} else {


							if ( ( nodeConst10.y > 0.9999 ) ) {

								nodeVar16 = vec3<f32>( 0.0, -1.0, 0.0 );
								

							} else {


								if ( ( nodeConst10.z > 0.9999 ) ) {

									nodeVar16 = vec3<f32>( 0.0, 0.0, -1.0 );
									

								} else {

									nodeVar17 = textureSampleLevel( nodeUniform3, nodeUniform3_sampler, ( nodeConst10 + vec3<f32>( -0.01, 0.0, 0.0 ) ), 0.0 ).x;
									nodeVar18 = textureSampleLevel( nodeUniform3, nodeUniform3_sampler, ( nodeConst10 + vec3<f32>( 0.01, 0.0, 0.0 ) ), 0.0 ).x;
									nodeVar19 = textureSampleLevel( nodeUniform3, nodeUniform3_sampler, ( nodeConst10 + vec3<f32>( 0.0, -0.01, 0.0 ) ), 0.0 ).x;
									nodeVar20 = textureSampleLevel( nodeUniform3, nodeUniform3_sampler, ( nodeConst10 + vec3<f32>( 0.0, 0.01, 0.0 ) ), 0.0 ).x;
									nodeVar21 = textureSampleLevel( nodeUniform3, nodeUniform3_sampler, ( nodeConst10 + vec3<f32>( 0.0, 0.0, -0.01 ) ), 0.0 ).x;
									nodeVar22 = textureSampleLevel( nodeUniform3, nodeUniform3_sampler, ( nodeConst10 + vec3<f32>( 0.0, 0.0, 0.01 ) ), 0.0 ).x;
									nodeVar16 = vec3<f32>( ( nodeVar17 - nodeVar18 ), ( nodeVar19 - nodeVar20 ), ( nodeVar21 - nodeVar22 ) );
									

								}

								

							}

							

						}

						

					}

					

				}

				

			}

			nodeVar23 = ( ( normalize( nodeVar16 ) * vec3<f32>( 0.5 ) ) + ( ( nodeVar9 * vec3<f32>( 1.5 ) ) + vec3<f32>( 0.25 ) ) );
			nodeVar0.x = nodeVar23[ 0 ];
			nodeVar0.y = nodeVar23[ 1 ];
			nodeVar0.z = nodeVar23[ 2 ];
			nodeVar0.w = 1.0;
			break;
			

		}

		nodeVar1 = nodeVar6;
		nodeVar2 = true;
		nodeVar6 = ( nodeVar6 + ( nodeConst1 * vec3<f32>( nodeVar5 ) ) );

	}

	DiffuseColor = nodeVar0;
	DiffuseColor.w = ( DiffuseColor.w * object.nodeUniform6 );
	let nodeConst11 = max( vec4<f32>( DiffuseColor.xyz, DiffuseColor.w ), vec4<f32>( 0.0 ) );
	Output = nodeConst11;

	// result

	output.color = nodeConst11;

	return output;

}
