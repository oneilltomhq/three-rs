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
@binding( 0 ) @group( 1 ) var nodeUniform0 : texture_depth_2d;
@binding( 4 ) @group( 1 ) var nodeUniform8_sampler : sampler_comparison;
@binding( 5 ) @group( 1 ) var nodeUniform8 : texture_depth_cube;

struct NodeBuffer_926Struct {
	value : array< vec4<f32>, 6 >
};
@binding( 2 ) @group( 1 )
var<uniform> NodeBuffer_926 : NodeBuffer_926Struct;

struct NodeBuffer_927Struct {
	value : array< vec4<f32>, 6 >
};
@binding( 3 ) @group( 1 )
var<uniform> NodeBuffer_927 : NodeBuffer_927Struct;

struct objectStruct {
	nodeUniform1 : mat4x4<f32>,
	nodeUniform2 : mat4x4<f32>,
	nodeUniform3 : vec3<f32>,
	nodeUniform6 : f32,
	nodeUniform9 : f32,
	nodeUniform10 : f32,
	nodeUniform11 : f32,
	nodeUniform12 : f32,
	nodeUniform13 : f32
};
@binding( 1 ) @group( 1 )
var<uniform> object : objectStruct;

struct renderStruct {
	nodeUniform7 : vec3<f32>
};
@binding( 0 ) @group( 0 )
var<uniform> render : renderStruct;

// vars
var<private> nodeVar0 : vec4<f32>;
var<private> nodeVar2 : f32;
var<private> nodeVar3 : vec2<u32>;
var<private> nodeVar5 : f32;
var<private> nodeVar6 : vec3<f32>;
var<private> nodeVar7 : f32;
var<private> nodeVar8 : f32;
var<private> nodeVar9 : f32;
var<private> nodeVar10 : f32;
var<private> nodeVar11 : f32;

// codes
fn tsl_clampWrapping_float( coord: f32 ) -> f32 { return clamp( coord, 0.0, 1.0 ); }
fn tsl_coord_clampS_clampT_2d( coord : vec2f ) -> vec2f {

	return vec2f(
		tsl_clampWrapping_float( coord.x ),
		tsl_clampWrapping_float( coord.y )
	);

}

fn interleavedGradientNoise ( position : vec2<f32> ) -> f32 {

	


	return fract( ( 52.9829189 * fract( dot( position, vec2<f32>( 0.06711056, 0.00583715 ) ) ) ) );

}




@fragment
fn main( @location( 0 ) nodeVarying0 : vec2<f32>,
	@builtin( position ) fragCoord : vec4<f32> ) -> OutputStruct {

	// flow
	// code

	nodeVar0 = vec4<f32>( 0.0, 0.0, 0.0, 1.0 );
	var nodeVar1 : bool = false;
	nodeVar3 = textureDimensions( nodeUniform0, u32( 0 ) );
	nodeVar2 = textureLoad( nodeUniform0, vec2<u32>( clamp( floor( tsl_coord_clampS_clampT_2d( nodeVarying0 ) * vec2<f32>( nodeVar3 ) ), vec2<f32>( 0 ), vec2<f32>( nodeVar3 - vec2<u32>( 1, 1 ) ) ) ), u32( 0 ) );
	let nodeConst0 = nodeVar2;
	let nodeConst1 = ( object.nodeUniform1 * vec4<f32>( vec3<f32>( ( ( vec2<f32>( nodeVarying0.x, ( 1.0 - nodeVarying0.y ) ) * vec2<f32>( 2.0 ) ) - vec2<f32>( 1.0 ) ), nodeConst0 ), 1.0 ) );
	let nodeConst2 = ( nodeConst1.xyz / vec3<f32>( nodeConst1.w ) );
	var nodeVar4 : vec4<f32> = ( object.nodeUniform2 * vec4<f32>( nodeConst2, 1.0 ) );
	nodeVar5 = -10000.0;

	for ( var i : i32 = 0; i < 6; i ++ ) {

		nodeVar5 = max( nodeVar5, ( dot( object.nodeUniform3, NodeBuffer_926.value[ i ].xyz ) + NodeBuffer_927.value[ i ].x ) );

	}

	nodeVar6 = object.nodeUniform3;

	if ( ( nodeVar5 < 0.0 ) ) {


		for ( var i : i32 = 0; i < 6; i ++ ) {


			if ( ( ( dot( nodeVar4, vec4<f32>( NodeBuffer_926.value[ i ].xyz, 1.0 ) ) + NodeBuffer_927.value[ i ].x ) > 0.0 ) ) {

				let nodeConst3 = ( nodeVar4 - vec4<f32>( object.nodeUniform3, 1.0 ) );
				nodeVar4 = ( vec4<f32>( object.nodeUniform3, 1.0 ) + ( vec4<f32>( ( - ( ( dot( object.nodeUniform3, NodeBuffer_926.value[ i ].xyz ) + NodeBuffer_927.value[ i ].x ) / dot( vec4<f32>( NodeBuffer_926.value[ i ].xyz, 1.0 ), nodeConst3 ) ) ) ) * nodeConst3 ) );
				

			}


		}

		

	} else {

		let nodeConst4 = ( nodeVar4 - vec4<f32>( object.nodeUniform3, 1.0 ) );
		nodeVar7 = 10000.0;

		for ( var i : i32 = 0; i < 6; i ++ ) {

			let nodeConst5 = ( - ( ( dot( object.nodeUniform3, NodeBuffer_926.value[ i ].xyz ) + NodeBuffer_927.value[ i ].x ) / dot( vec4<f32>( NodeBuffer_926.value[ i ].xyz, 1.0 ), nodeConst4 ) ) );

			if ( ( ( nodeConst5 < nodeVar7 ) && ( nodeConst5 > 0.0 ) ) ) {

				nodeVar7 = nodeConst5;
				

			}


		}


		if ( ( nodeVar7 == 10000.0 ) ) {

			nodeVar1 = true;
			

		} else {

			nodeVar6 = ( vec4<f32>( object.nodeUniform3, 1.0 ) + ( vec4<f32>( ( nodeVar7 + 0.001 ) ) * nodeConst4 ) ).xyz;
			nodeVar8 = -10000.0;

			for ( var i : i32 = 0; i < 6; i ++ ) {

				nodeVar8 = max( nodeVar8, ( dot( nodeVar4, vec4<f32>( NodeBuffer_926.value[ i ].xyz, 1.0 ) ) + NodeBuffer_927.value[ i ].x ) );

			}


			if ( ( nodeVar8 >= 0.0 ) ) {

				nodeVar9 = 10000.0;

				for ( var i : i32 = 0; i < 6; i ++ ) {


					if ( ( ( dot( nodeVar4, vec4<f32>( NodeBuffer_926.value[ i ].xyz, 1.0 ) ) + NodeBuffer_927.value[ i ].x ) > 0.0 ) ) {

						let nodeConst6 = ( - ( ( dot( nodeVar6, NodeBuffer_926.value[ i ].xyz ) + NodeBuffer_927.value[ i ].x ) / dot( vec4<f32>( NodeBuffer_926.value[ i ].xyz, 1.0 ), nodeConst4 ) ) );

						if ( ( ( nodeConst6 < nodeVar9 ) && ( nodeConst6 > 0.0 ) ) ) {

							nodeVar9 = nodeConst6;
							

						}

						

					}


				}


				if ( ( nodeVar9 < distance( nodeVar4, vec4<f32>( nodeVar6, 1.0 ) ) ) ) {

					nodeVar4 = ( vec4<f32>( nodeVar6, 1.0 ) + ( vec4<f32>( nodeVar9 ) * nodeConst4 ) );
					

				}

				

			}

			

		}

		

	}


	if ( ( nodeVar1 == false ) ) {

		nodeVar10 = 0.0;
		let nodeConst7 = interleavedGradientNoise( fragCoord.xy );
		let nodeConst8 = round( ( object.nodeUniform6 + ( ( ( object.nodeUniform6 / 8.0 ) + 2.0 ) * nodeConst7 ) ) );
		let nodeConst9 = u32( nodeConst8 );

		for ( var i : i32 = 0; i < i32( nodeConst9 ); i ++ ) {

			let nodeConst10 = mix( vec4<f32>( nodeVar6, 1.0 ), nodeVar4, ( f32( i ) / nodeConst8 ) );
			let nodeConst11 = ( nodeConst10 - vec4<f32>( render.nodeUniform7, 1.0 ) );
			let nodeConst12 = abs( nodeConst11 );
			let nodeConst13 = ( - max( max( nodeConst12.x, nodeConst12.y ), nodeConst12.z ) );
			nodeVar11 = textureSampleCompare( nodeUniform8, nodeUniform8_sampler, vec3<f32>( nodeConst11.x, ( - nodeConst11.y ), nodeConst11.z ), ( ( ( object.nodeUniform9 + nodeConst13 ) * object.nodeUniform10 ) / ( ( object.nodeUniform10 - object.nodeUniform9 ) * nodeConst13 ) ) );
			let nodeConst14 = vec2<f32>( ( ( 1.0 - nodeVar11 ) + 0.005 ), ( - nodeConst13 ) );
			let nodeConst15 = ( 1.0 - nodeConst14.x );
			nodeVar10 = ( nodeVar10 + ( ( nodeConst15 * ( distance( vec4<f32>( nodeVar6, 1.0 ), nodeVar4 ) * ( object.nodeUniform11 / 100.0 ) ) ) * pow( ( 1.0 - ( nodeConst14.y / object.nodeUniform10 ) ), object.nodeUniform12 ) ) );

		}

		nodeVar10 = ( nodeVar10 / nodeConst8 );
		nodeVar0 = vec4<f32>( vec3<f32>( clamp( ( 1.0 - exp( ( - nodeVar10 ) ) ), 0.0, object.nodeUniform13 ) ), nodeConst0 );
		

	}


	// result

	output.color = nodeVar0;

	return output;

}
