// Three.js r187dev - Node System

// global
diagnostic( off, derivative_uniformity );


// directives


// structs

struct OutputType {
	@location( 0 ) m0 : f32,
	@location( 1 ) m1 : vec3<f32>,
	
};
var<private> output : OutputType;

// uniforms
@binding( 0 ) @group( 0 ) var nodeUniform0 : texture_depth_2d;
@binding( 2 ) @group( 0 ) var nodeUniform2_sampler : sampler;
@binding( 3 ) @group( 0 ) var nodeUniform2 : texture_2d<f32>;
@binding( 4 ) @group( 0 ) var nodeUniform18_sampler : sampler;
@binding( 5 ) @group( 0 ) var nodeUniform18 : texture_2d<f32>;

struct objectStruct {
	nodeUniform1 : mat4x4<f32>,
	nodeUniform3 : u32,
	nodeUniform4 : u32,
	nodeUniform5 : f32,
	nodeUniform6 : f32,
	nodeUniform7 : f32,
	nodeUniform8 : u32,
	nodeUniform9 : vec2<f32>,
	nodeUniform10 : f32,
	nodeUniform11 : f32,
	nodeUniform12 : f32,
	nodeUniform13 : f32,
	nodeUniform14 : f32,
	nodeUniform15 : f32,
	nodeUniform16 : u32,
	nodeUniform17 : f32,
	nodeUniform19 : f32
};
@binding( 1 ) @group( 0 )
var<uniform> object : objectStruct;

// vars
var<private> DiffuseColor : vec4<f32>;
var<private> nodeVar0 : f32;
var<private> nodeVar1 : vec2<u32>;
var<private> nodeVar2 : f32;
var<private> nodeVar3 : vec3<f32>;
var<private> nodeVar4 : vec4<f32>;
var<private> nodeVar5 : vec3<f32>;
var<private> nodeVar6 : vec3<f32>;
var<private> nodeVar9 : f32;
var<private> nodeVar10 : bool;
var<private> nodeVar13 : vec2<f32>;
var<private> nodeVar14 : f32;
var<private> nodeVar15 : vec2<f32>;
var<private> nodeVar16 : f32;
var<private> nodeVar17 : f32;
var<private> nodeVar18 : bool;
var<private> nodeVar19 : f32;
var<private> nodeVar20 : f32;
var<private> nodeVar21 : bool;
var<private> nodeVar22 : u32;
var<private> nodeVar23 : vec4<f32>;
var<private> nodeVar24 : f32;
var<private> nodeVar25 : vec4<f32>;
var<private> nodeVar26 : f32;
var<private> nodeVar28 : vec2<f32>;
var<private> nodeVar29 : f32;
var<private> nodeVar30 : vec2<f32>;
var<private> nodeVar31 : f32;
var<private> nodeVar32 : f32;
var<private> nodeVar33 : bool;
var<private> nodeVar34 : f32;
var<private> nodeVar35 : f32;
var<private> nodeVar36 : bool;
var<private> nodeVar37 : u32;
var<private> nodeVar38 : vec4<f32>;
var<private> nodeVar39 : f32;
var<private> nodeVar40 : vec4<f32>;
var<private> nodeVar41 : f32;
var<private> nodeVar42 : f32;
var<private> nodeVar43 : f32;
var<private> nodeVar44 : vec3<f32>;
var<private> Output : f32;

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


fn spatialOffsets ( position : vec2<f32> ) -> f32 {

	


	return ( 0.25 * f32( ( i32( ( position.y - position.x ) ) & 3 ) ) );

}


fn tsl_mod_float( x : f32, y : f32 ) -> f32 { return x - y * floor( x / y ); }
fn GTAOFastAcos ( value : vec2<f32> ) -> vec2<f32> {

	var nodeVar1 : f32;
	var nodeVar2 : f32;

	var nodeVar0 : vec2<f32> = ( ( abs( value ) * vec2<f32>( -0.156583 ) ) + vec2<f32>( 1.5707963267948966 ) );
	nodeVar0 = ( nodeVar0 * sqrt( ( vec2<f32>( 1.0 ) - abs( value ) ) ) );

	if ( ( value.x >= 0.0 ) ) {

		nodeVar1 = nodeVar0.x;

	} else {

		nodeVar1 = ( 3.141592653589793 - nodeVar0.x );

	}


	if ( ( value.y >= 0.0 ) ) {

		nodeVar2 = nodeVar0.y;

	} else {

		nodeVar2 = ( 3.141592653589793 - nodeVar0.y );

	}


	return vec2<f32>( nodeVar1, nodeVar2 );

}




@fragment
fn main( @location( 0 ) nodeVarying0 : vec2<f32>,
	@builtin( position ) fragCoord : vec4<f32> ) -> OutputType {

	// flow
	// code

	nodeVar1 = textureDimensions( nodeUniform0, u32( 0 ) );
	nodeVar0 = textureLoad( nodeUniform0, vec2<u32>( clamp( floor( tsl_coord_clampS_clampT_2d( nodeVarying0 ) * vec2<f32>( nodeVar1 ) ), vec2<f32>( 0 ), vec2<f32>( nodeVar1 - vec2<u32>( 1, 1 ) ) ) ), u32( 0 ) );
	nodeVar2 = nodeVar0;

	if ( ( nodeVar2 >= 1.0 ) ) {

		discard;
		

	}

	let nodeConst0 = ( object.nodeUniform1 * vec4<f32>( vec3<f32>( ( ( vec2<f32>( nodeVarying0.x, ( 1.0 - nodeVarying0.y ) ) * vec2<f32>( 2.0 ) ) - vec2<f32>( 1.0 ) ), nodeVar2 ), 1.0 ) );
	nodeVar3 = ( nodeConst0.xyz / vec3<f32>( nodeConst0.w ) );
	nodeVar4 = textureSample( nodeUniform2, nodeUniform2_sampler, nodeVarying0 );
	nodeVar5 = normalize( ( ( nodeVar4 * vec4<f32>( 2.0 ) ) - vec4<f32>( 1.0 ) ).xyz );
	nodeVar6 = normalize( ( - nodeVar3 ) );
	var nodeVar7 : f32 = 0.0;
	var nodeVar8 : vec3<f32> = vec3<f32>( 0.0, 0.0, 0.0 );
	let nodeConst1 = object.nodeUniform3;
	let nodeConst2 = object.nodeUniform4;
	let nodeConst3 = object.nodeUniform5;
	let nodeConst4 = object.nodeUniform6;
	let nodeConst5 = object.nodeUniform7;
	nodeVar9 = 0.0;
	nodeVar10 = bool( object.nodeUniform8 );

	if ( nodeVar10 ) {

		nodeVar9 = ( ( nodeConst5 * ( object.nodeUniform9.x / 2.0 ) ) / 16.0 );
		

	} else {

		nodeVar9 = max( ( ( nodeConst5 * object.nodeUniform10 ) / ( - nodeVar3.z ) ), f32( nodeConst2 ) );
		

	}

	nodeVar9 = ( nodeVar9 / ( f32( nodeConst2 ) + 1.0 ) );
	let nodeConst6 = ( max( 1.0, f32( ( nodeConst2 - 1u ) ) ) * nodeVar9 );

	for ( var i : u32 = 0u; i < nodeConst1; i ++ ) {

		let nodeConst7 = ( ( ( f32( i ) + interleavedGradientNoise( fragCoord.xy ) ) + object.nodeUniform11 ) * ( 3.141592653589793 / f32( nodeConst1 ) ) );
		let nodeConst8 = vec3<f32>( vec2<f32>( cos( nodeConst7 ), sin( nodeConst7 ) ), 0.0 );
		let nodeConst9 = ( nodeConst8.xy * ( vec2<f32>( 1.0 ) / object.nodeUniform9 ) );
		let nodeConst10 = normalize( cross( nodeConst8, nodeVar6 ) );
		let nodeConst11 = cross( nodeVar6, nodeConst10 );
		let nodeConst12 = ( nodeVar5 - ( nodeConst10 * vec3<f32>( dot( nodeVar5, nodeConst10 ) ) ) );
		let nodeConst13 = normalize( nodeConst12 );
		let nodeConst14 = clamp( dot( nodeConst13, nodeVar6 ), -1.0, 1.0 );
		let nodeConst15 = ( ( - sign( dot( nodeConst12, nodeConst11 ) ) ) * acos( nodeConst14 ) );
		var nodeVar11 : u32 = 0u;
		nodeVar11 = 0u;
		let nodeConst16 = object.nodeUniform12;
		let nodeConst17 = object.nodeUniform13;
		let nodeConst18 = object.nodeUniform14;
		var nodeVar12 : vec3<f32> = vec3<f32>( 0.0, 0.0, 0.0 );
		let nodeConst19 = object.nodeUniform4;

		for ( var i : u32 = 0u; i < nodeConst19; i ++ ) {

			let nodeConst20 = ( pow( abs( ( ( nodeVar9 * ( f32( i ) + ( fract( ( spatialOffsets( fragCoord.xy ) + object.nodeUniform15 ) ) + fract( ( sin( tsl_mod_float( dot( ( ( ( nodeVarying0 + vec2<f32>( ( object.nodeUniform11 * 0.02 ) ) ) * vec2<f32>( 2.0 ) ) - vec2<f32>( 1.0 ) ), vec2<f32>( 12.9898, 78.233 ) ), 3.141592653589793 ) ) * 43758.5453 ) ) ) ) ) / nodeConst6 ) ), nodeConst16 ) * nodeConst6 );
			let nodeConst21 = ( nodeConst9 * vec2<f32>( max( nodeConst20, ( f32( i ) + 1.0 ) ) ) );

			if ( true ) {

				nodeVar13 = vec2<f32>( 1.0, -1.0 );

			} else {

				nodeVar13 = vec2<f32>( -1.0, 1.0 );

			}

			let nodeConst22 = ( nodeVarying0 + ( nodeConst21 * nodeVar13 ) );

			if ( ( ( ( ( nodeConst22.x <= 0.0 ) || ( nodeConst22.y <= 0.0 ) ) || ( nodeConst22.x >= 1.0 ) ) || ( nodeConst22.y >= 1.0 ) ) ) {

				break;
				

			}

			nodeVar14 = textureLoad( nodeUniform0, vec2<u32>( clamp( floor( tsl_coord_clampS_clampT_2d( nodeConst22 ) * vec2<f32>( nodeVar1 ) ), vec2<f32>( 0 ), vec2<f32>( nodeVar1 - vec2<u32>( 1, 1 ) ) ) ), u32( 0 ) );
			let nodeConst23 = ( object.nodeUniform1 * vec4<f32>( vec3<f32>( ( ( vec2<f32>( nodeConst22.x, ( 1.0 - nodeConst22.y ) ) * vec2<f32>( 2.0 ) ) - vec2<f32>( 1.0 ) ), nodeVar14 ), 1.0 ) );
			let nodeConst24 = ( nodeConst23.xyz / vec3<f32>( nodeConst23.w ) );
			let nodeConst25 = normalize( ( nodeConst24 - nodeVar3 ) );

			if ( true ) {


				if ( true ) {

					nodeVar16 = 1.0;

				} else {

					nodeVar16 = -1.0;

				}

				nodeVar18 = bool( object.nodeUniform16 );

				if ( nodeVar18 ) {

					nodeVar17 = ( clamp( ( ( - nodeConst24.z ) / object.nodeUniform17 ), 0.0, 1.0 ) * 100.0 );

				} else {

					nodeVar17 = 1.0;

				}

				nodeVar15 = clamp( ( ( ( vec2<f32>( nodeVar16 ) * ( - GTAOFastAcos( clamp( vec2<f32>( dot( nodeConst25, nodeVar6 ), dot( normalize( ( ( nodeConst24 - ( ( vec3<f32>( nodeVar17 ) * nodeVar6 ) * vec3<f32>( nodeConst17 ) ) ) - nodeVar3 ) ), nodeVar6 ) ), vec2<f32>( -1.0 ), vec2<f32>( 1.0 ) ) ) ) ) - vec2<f32>( ( nodeConst15 - 1.5707963267948966 ) ) ) / vec2<f32>( 3.141592653589793 ) ), vec2<f32>( 0.0 ), vec2<f32>( 1.0 ) ).yx;

			} else {


				if ( true ) {

					nodeVar19 = 1.0;

				} else {

					nodeVar19 = -1.0;

				}

				nodeVar21 = bool( object.nodeUniform16 );

				if ( nodeVar21 ) {

					nodeVar20 = ( clamp( ( ( - nodeConst24.z ) / object.nodeUniform17 ), 0.0, 1.0 ) * 100.0 );

				} else {

					nodeVar20 = 1.0;

				}

				nodeVar15 = clamp( ( ( ( vec2<f32>( nodeVar19 ) * ( - GTAOFastAcos( clamp( vec2<f32>( dot( nodeConst25, nodeVar6 ), dot( normalize( ( ( nodeConst24 - ( ( vec3<f32>( nodeVar20 ) * nodeVar6 ) * vec3<f32>( nodeConst17 ) ) ) - nodeVar3 ) ), nodeVar6 ) ), vec2<f32>( -1.0 ), vec2<f32>( 1.0 ) ) ) ) ) - vec2<f32>( ( nodeConst15 - 1.5707963267948966 ) ) ) / vec2<f32>( 3.141592653589793 ) ), vec2<f32>( 0.0 ), vec2<f32>( 1.0 ) );

			}

			let nodeConst26 = nodeVar15.x;
			let nodeConst27 = nodeVar15.y;
			let nodeConst28 = u32( ( nodeVar15 * vec2<f32>( 32.0 ) ).x );
			let nodeConst29 = u32( ceil( ( ( nodeConst27 - nodeConst26 ) * 32.0 ) ) );

			if ( ( nodeConst29 > 0u ) ) {

				nodeVar22 = ( 4294967295u >> ( ( 32u - 32u ) + ( 32u - nodeConst29 ) ) );

			} else {

				nodeVar22 = 0u;

			}

			let nodeConst30 = nodeVar22;
			let nodeConst31 = ( ( nodeConst30 << nodeConst28 ) & ( ~ nodeVar11 ) );
			nodeVar11 = ( nodeVar11 | nodeConst31 );
			let nodeConst32 = countOneBits( nodeConst31 );

			if ( ( f32( nodeConst32 ) > 0.0 ) ) {

				nodeVar23 = textureSample( nodeUniform18, nodeUniform18_sampler, nodeConst22 );
				let nodeVar23 = nodeVar23;

				if ( ( dot( nodeVar23, vec4<f32>( vec3<f32>( 0.2126, 0.7152, 0.0722 ), 1.0 ) ) > 0.001 ) ) {

					let nodeConst33 = normalize( nodeConst25 );
					let nodeConst34 = clamp( dot( nodeVar5, nodeConst33 ), 0.0, 1.0 );

					if ( ( nodeConst34 > 0.001 ) ) {

						nodeVar25 = textureSample( nodeUniform2, nodeUniform2_sampler, nodeConst22 );
						let nodeConst35 = normalize( ( ( nodeVar25 * vec4<f32>( 2.0 ) ) - vec4<f32>( 1.0 ) ).xyz );

						if ( ( ( nodeConst18 > 0.0 ) && ( dot( nodeConst35, nodeVar6 ) > 0.0 ) ) ) {

							let nodeConst36 = dot( nodeConst35, ( - nodeConst33 ) );

							if ( ( sign( nodeConst36 ) < 0.0 ) ) {

								nodeVar26 = ( abs( nodeConst36 ) * nodeConst18 );

							} else {

								nodeVar26 = abs( nodeConst36 );

							}

							nodeVar24 = nodeVar26;

						} else {

							nodeVar24 = clamp( dot( nodeConst35, ( - nodeConst33 ) ), 0.0, 1.0 );

						}

						nodeVar12 = ( vec4<f32>( nodeVar12, 1.0 ) + ( ( ( vec4<f32>( ( f32( nodeConst32 ) / 32.0 ) ) * nodeVar23 ) * vec4<f32>( nodeConst34 ) ) * vec4<f32>( nodeVar24 ) ) ).xyz;
						

					}

					

				}

				

			}


		}

		nodeVar8 = ( nodeVar8 + nodeVar12 );
		let nodeConst37 = object.nodeUniform12;
		let nodeConst38 = object.nodeUniform13;
		let nodeConst39 = object.nodeUniform14;
		var nodeVar27 : vec3<f32> = vec3<f32>( 0.0, 0.0, 0.0 );
		let nodeConst40 = object.nodeUniform4;

		for ( var i : u32 = 0u; i < nodeConst40; i ++ ) {

			let nodeConst41 = ( pow( abs( ( ( nodeVar9 * ( f32( i ) + ( fract( ( spatialOffsets( fragCoord.xy ) + object.nodeUniform15 ) ) + fract( ( sin( tsl_mod_float( dot( ( ( ( nodeVarying0 + vec2<f32>( ( object.nodeUniform11 * 0.02 ) ) ) * vec2<f32>( 2.0 ) ) - vec2<f32>( 1.0 ) ), vec2<f32>( 12.9898, 78.233 ) ), 3.141592653589793 ) ) * 43758.5453 ) ) ) ) ) / nodeConst6 ) ), nodeConst37 ) * nodeConst6 );
			let nodeConst42 = ( nodeConst9 * vec2<f32>( max( nodeConst41, ( f32( i ) + 1.0 ) ) ) );

			if ( false ) {

				nodeVar28 = vec2<f32>( 1.0, -1.0 );

			} else {

				nodeVar28 = vec2<f32>( -1.0, 1.0 );

			}

			let nodeConst43 = ( nodeVarying0 + ( nodeConst42 * nodeVar28 ) );

			if ( ( ( ( ( nodeConst43.x <= 0.0 ) || ( nodeConst43.y <= 0.0 ) ) || ( nodeConst43.x >= 1.0 ) ) || ( nodeConst43.y >= 1.0 ) ) ) {

				break;
				

			}

			nodeVar29 = textureLoad( nodeUniform0, vec2<u32>( clamp( floor( tsl_coord_clampS_clampT_2d( nodeConst43 ) * vec2<f32>( nodeVar1 ) ), vec2<f32>( 0 ), vec2<f32>( nodeVar1 - vec2<u32>( 1, 1 ) ) ) ), u32( 0 ) );
			let nodeConst44 = ( object.nodeUniform1 * vec4<f32>( vec3<f32>( ( ( vec2<f32>( nodeConst43.x, ( 1.0 - nodeConst43.y ) ) * vec2<f32>( 2.0 ) ) - vec2<f32>( 1.0 ) ), nodeVar29 ), 1.0 ) );
			let nodeConst45 = ( nodeConst44.xyz / vec3<f32>( nodeConst44.w ) );
			let nodeConst46 = normalize( ( nodeConst45 - nodeVar3 ) );

			if ( false ) {


				if ( false ) {

					nodeVar31 = 1.0;

				} else {

					nodeVar31 = -1.0;

				}

				nodeVar33 = bool( object.nodeUniform16 );

				if ( nodeVar33 ) {

					nodeVar32 = ( clamp( ( ( - nodeConst45.z ) / object.nodeUniform17 ), 0.0, 1.0 ) * 100.0 );

				} else {

					nodeVar32 = 1.0;

				}

				nodeVar30 = clamp( ( ( ( vec2<f32>( nodeVar31 ) * ( - GTAOFastAcos( clamp( vec2<f32>( dot( nodeConst46, nodeVar6 ), dot( normalize( ( ( nodeConst45 - ( ( vec3<f32>( nodeVar32 ) * nodeVar6 ) * vec3<f32>( nodeConst38 ) ) ) - nodeVar3 ) ), nodeVar6 ) ), vec2<f32>( -1.0 ), vec2<f32>( 1.0 ) ) ) ) ) - vec2<f32>( ( nodeConst15 - 1.5707963267948966 ) ) ) / vec2<f32>( 3.141592653589793 ) ), vec2<f32>( 0.0 ), vec2<f32>( 1.0 ) ).yx;

			} else {


				if ( false ) {

					nodeVar34 = 1.0;

				} else {

					nodeVar34 = -1.0;

				}

				nodeVar36 = bool( object.nodeUniform16 );

				if ( nodeVar36 ) {

					nodeVar35 = ( clamp( ( ( - nodeConst45.z ) / object.nodeUniform17 ), 0.0, 1.0 ) * 100.0 );

				} else {

					nodeVar35 = 1.0;

				}

				nodeVar30 = clamp( ( ( ( vec2<f32>( nodeVar34 ) * ( - GTAOFastAcos( clamp( vec2<f32>( dot( nodeConst46, nodeVar6 ), dot( normalize( ( ( nodeConst45 - ( ( vec3<f32>( nodeVar35 ) * nodeVar6 ) * vec3<f32>( nodeConst38 ) ) ) - nodeVar3 ) ), nodeVar6 ) ), vec2<f32>( -1.0 ), vec2<f32>( 1.0 ) ) ) ) ) - vec2<f32>( ( nodeConst15 - 1.5707963267948966 ) ) ) / vec2<f32>( 3.141592653589793 ) ), vec2<f32>( 0.0 ), vec2<f32>( 1.0 ) );

			}

			let nodeConst47 = nodeVar30.x;
			let nodeConst48 = nodeVar30.y;
			let nodeConst49 = u32( ( nodeVar30 * vec2<f32>( 32.0 ) ).x );
			let nodeConst50 = u32( ceil( ( ( nodeConst48 - nodeConst47 ) * 32.0 ) ) );

			if ( ( nodeConst50 > 0u ) ) {

				nodeVar37 = ( 4294967295u >> ( ( 32u - 32u ) + ( 32u - nodeConst50 ) ) );

			} else {

				nodeVar37 = 0u;

			}

			let nodeConst51 = nodeVar37;
			let nodeConst52 = ( ( nodeConst51 << nodeConst49 ) & ( ~ nodeVar11 ) );
			nodeVar11 = ( nodeVar11 | nodeConst52 );
			let nodeConst53 = countOneBits( nodeConst52 );

			if ( ( f32( nodeConst53 ) > 0.0 ) ) {

				nodeVar38 = textureSample( nodeUniform18, nodeUniform18_sampler, nodeConst43 );
				let nodeVar38 = nodeVar38;

				if ( ( dot( nodeVar38, vec4<f32>( vec3<f32>( 0.2126, 0.7152, 0.0722 ), 1.0 ) ) > 0.001 ) ) {

					let nodeConst54 = normalize( nodeConst46 );
					let nodeConst55 = clamp( dot( nodeVar5, nodeConst54 ), 0.0, 1.0 );

					if ( ( nodeConst55 > 0.001 ) ) {

						nodeVar40 = textureSample( nodeUniform2, nodeUniform2_sampler, nodeConst43 );
						let nodeConst56 = normalize( ( ( nodeVar40 * vec4<f32>( 2.0 ) ) - vec4<f32>( 1.0 ) ).xyz );

						if ( ( ( nodeConst39 > 0.0 ) && ( dot( nodeConst56, nodeVar6 ) > 0.0 ) ) ) {

							let nodeConst57 = dot( nodeConst56, ( - nodeConst54 ) );

							if ( ( sign( nodeConst57 ) < 0.0 ) ) {

								nodeVar41 = ( abs( nodeConst57 ) * nodeConst39 );

							} else {

								nodeVar41 = abs( nodeConst57 );

							}

							nodeVar39 = nodeVar41;

						} else {

							nodeVar39 = clamp( dot( nodeConst56, ( - nodeConst54 ) ), 0.0, 1.0 );

						}

						nodeVar27 = ( vec4<f32>( nodeVar27, 1.0 ) + ( ( ( vec4<f32>( ( f32( nodeConst53 ) / 32.0 ) ) * nodeVar38 ) * vec4<f32>( nodeConst55 ) ) * vec4<f32>( nodeVar39 ) ) ).xyz;
						

					}

					

				}

				

			}


		}

		nodeVar8 = ( nodeVar8 + nodeVar27 );
		nodeVar7 = ( nodeVar7 + ( f32( countOneBits( nodeVar11 ) ) / 32.0 ) );

	}

	nodeVar7 = ( nodeVar7 / f32( nodeConst1 ) );
	nodeVar7 = clamp( pow( ( 1.0 - clamp( nodeVar7, 0.0, 1.0 ) ), nodeConst3 ), 0.0, 1.0 );
	nodeVar8 = ( nodeVar8 / vec3<f32>( f32( nodeConst1 ) ) );
	nodeVar8 = ( nodeVar8 * vec3<f32>( nodeConst4 ) );
	const nodeConst58 = 7.0;
	let nodeConst59 = dot( nodeVar8, vec3<f32>( 0.2126, 0.7152, 0.0722 ) );

	if ( ( nodeConst59 > nodeConst58 ) ) {

		nodeVar42 = ( nodeConst58 / nodeConst59 );

	} else {

		nodeVar42 = 1.0;

	}

	nodeVar8 = ( nodeVar8 * vec3<f32>( nodeVar42 ) );
	nodeVar43 = nodeVar7;
	nodeVar44 = nodeVar8;
	DiffuseColor = vec4<f32>( 0.0, 0.0, 0.0, 0.0 );
	DiffuseColor.w = ( DiffuseColor.w * object.nodeUniform19 );
	DiffuseColor.w = 1.0;
	Output = max( vec4<f32>( DiffuseColor.xyz, DiffuseColor.w ), vec4<f32>( 0.0 ) ).x;
	output.m0 = nodeVar43;
	output.m1 = nodeVar44;

	// result

	return output;

}
