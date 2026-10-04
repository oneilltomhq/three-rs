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
@binding( 1 ) @group( 0 ) var nodeUniform1_sampler : sampler;
@binding( 2 ) @group( 0 ) var nodeUniform1 : texture_2d<f32>;
@binding( 3 ) @group( 0 ) var nodeUniform3 : texture_depth_2d;
@binding( 4 ) @group( 0 ) var nodeUniform4_sampler : sampler;
@binding( 5 ) @group( 0 ) var nodeUniform4 : texture_2d<f32>;

struct objectStruct {
	nodeUniform0 : f32,
	nodeUniform2 : vec2<f32>,
	nodeUniform5 : mat4x4<f32>,
	nodeUniform6 : mat4x4<f32>,
	nodeUniform7 : f32,
	nodeUniform8 : f32,
	nodeUniform9 : f32,
	nodeUniform10 : f32,
	nodeUniform11 : f32,
	nodeUniform12 : mat4x4<f32>,
	nodeUniform13 : f32,
	nodeUniform14 : f32,
	nodeUniform15 : f32,
	nodeUniform16 : u32,
	nodeUniform17 : f32
};
@binding( 0 ) @group( 0 )
var<uniform> object : objectStruct;

// vars
var<private> nodeVar0 : f32;
var<private> nodeVar1 : vec2<u32>;
var<private> nodeVar2 : vec4<f32>;
var<private> nodeVar3 : vec4<f32>;
var<private> nodeVar4 : vec4<f32>;
var<private> nodeVar5 : f32;
var<private> nodeVar6 : f32;
var<private> nodeVar7 : f32;
var<private> nodeVar8 : bool;
var<private> nodeVar9 : vec3<f32>;
var<private> nodeVar10 : f32;
var<private> nodeVar11 : f32;
var<private> nodeVar12 : f32;
var<private> nodeVar13 : vec3<f32>;
var<private> nodeVar14 : f32;
var<private> nodeVar15 : f32;
var<private> nodeVar16 : vec3<f32>;
var<private> nodeVar17 : vec3<f32>;
var<private> nodeVar18 : vec3<f32>;
var<private> nodeVar19 : f32;
var<private> nodeVar20 : vec2<f32>;
var<private> nodeVar21 : vec2<f32>;
var<private> nodeVar22 : vec4<f32>;
var<private> nodeVar23 : f32;
var<private> nodeVar24 : vec2<f32>;
var<private> nodeVar25 : vec2<f32>;
var<private> nodeVar26 : vec2<f32>;
var<private> nodeVar27 : vec4<f32>;
var<private> nodeVar28 : vec4<f32>;
var<private> nodeVar29 : vec4<f32>;
var<private> nodeVar30 : f32;
var<private> nodeVar31 : f32;
var<private> nodeVar32 : vec4<f32>;
var<private> nodeVar33 : f32;
var<private> nodeVar34 : bool;
var<private> nodeVar35 : f32;
var<private> nodeVar36 : vec4<f32>;

// codes
fn tsl_clampWrapping_float( coord: f32 ) -> f32 { return clamp( coord, 0.0, 1.0 ); }
fn tsl_coord_clampS_clampT_2d( coord : vec2f ) -> vec2f {

	return vec2f(
		tsl_clampWrapping_float( coord.x ),
		tsl_clampWrapping_float( coord.y )
	);

}

fn getNeighborhoodStats ( uvCoord : vec2<f32>, centerSample : vec4<f32> ) -> vec4<f32> {

	var nodeVar0 : f32;
	var nodeVar1 : f32;
	var nodeVar2 : f32;
	var nodeVar3 : bool;
	var nodeVar4 : vec4<f32>;
	var nodeVar5 : vec4<f32>;
	var nodeVar6 : vec4<f32>;
	var nodeVar7 : vec4<f32>;

	nodeVar0 = 0.0;
	nodeVar1 = 0.0;
	nodeVar2 = 0.0;
	nodeVar3 = false;

	if ( ( object.nodeUniform0 > 0.0 ) ) {

		nodeVar2 = ( nodeVar2 + 1.0 );
		let nodeConst0 = dot( centerSample.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) );
		let nodeConst1 = ( nodeConst0 - nodeVar0 );
		nodeVar0 = ( nodeVar0 + ( nodeConst1 / nodeVar2 ) );
		nodeVar1 = ( nodeVar1 + ( nodeConst1 * ( nodeConst0 - nodeVar0 ) ) );
		

	}

	nodeVar4 = textureSample( nodeUniform1, nodeUniform1_sampler, ( uvCoord + ( vec2<f32>( -1.0, 0.0 ) / object.nodeUniform2 ) ) );
	let nodeConst2 = max( nodeVar4, vec4<f32>( 0.0 ) );

	if ( ( object.nodeUniform0 > 0.0 ) ) {

		nodeVar2 = ( nodeVar2 + 1.0 );
		let nodeConst3 = dot( nodeConst2.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) );
		let nodeConst4 = ( nodeConst3 - nodeVar0 );
		nodeVar0 = ( nodeVar0 + ( nodeConst4 / nodeVar2 ) );
		nodeVar1 = ( nodeVar1 + ( nodeConst4 * ( nodeConst3 - nodeVar0 ) ) );
		

	}

	nodeVar5 = textureSample( nodeUniform1, nodeUniform1_sampler, ( uvCoord + ( vec2<f32>( 1.0, 0.0 ) / object.nodeUniform2 ) ) );
	let nodeConst5 = max( nodeVar5, vec4<f32>( 0.0 ) );

	if ( ( object.nodeUniform0 > 0.0 ) ) {

		nodeVar2 = ( nodeVar2 + 1.0 );
		let nodeConst6 = dot( nodeConst5.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) );
		let nodeConst7 = ( nodeConst6 - nodeVar0 );
		nodeVar0 = ( nodeVar0 + ( nodeConst7 / nodeVar2 ) );
		nodeVar1 = ( nodeVar1 + ( nodeConst7 * ( nodeConst6 - nodeVar0 ) ) );
		

	}

	nodeVar6 = textureSample( nodeUniform1, nodeUniform1_sampler, ( uvCoord + ( vec2<f32>( 0.0, -1.0 ) / object.nodeUniform2 ) ) );
	let nodeConst8 = max( nodeVar6, vec4<f32>( 0.0 ) );

	if ( ( object.nodeUniform0 > 0.0 ) ) {

		nodeVar2 = ( nodeVar2 + 1.0 );
		let nodeConst9 = dot( nodeConst8.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) );
		let nodeConst10 = ( nodeConst9 - nodeVar0 );
		nodeVar0 = ( nodeVar0 + ( nodeConst10 / nodeVar2 ) );
		nodeVar1 = ( nodeVar1 + ( nodeConst10 * ( nodeConst9 - nodeVar0 ) ) );
		

	}

	nodeVar7 = textureSample( nodeUniform1, nodeUniform1_sampler, ( uvCoord + ( vec2<f32>( 0.0, 1.0 ) / object.nodeUniform2 ) ) );
	let nodeConst11 = max( nodeVar7, vec4<f32>( 0.0 ) );

	if ( ( object.nodeUniform0 > 0.0 ) ) {

		nodeVar2 = ( nodeVar2 + 1.0 );
		let nodeConst12 = dot( nodeConst11.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) );
		let nodeConst13 = ( nodeConst12 - nodeVar0 );
		nodeVar0 = ( nodeVar0 + ( nodeConst13 / nodeVar2 ) );
		nodeVar1 = ( nodeVar1 + ( nodeConst13 * ( nodeConst12 - nodeVar0 ) ) );
		

	}


	return vec4<f32>( 1.0, nodeVar0, sqrt( ( nodeVar1 / max( nodeVar2, 1.0 ) ) ), f32( nodeVar3 ) );

}


fn temporalWeight ( x : f32, strength : f32 ) -> f32 {

	


	return ( 1.0 / pow( x, strength ) );

}


fn getTemporalVarianceFactor ( frameNum : f32, strength : f32 ) -> f32 {

	


	return max( temporalWeight( frameNum, strength ), 0.05 );

}


fn specularLobeTanHalfAngle ( roughness : f32, percent : f32 ) -> f32 {

	


	return ( ( roughness * roughness ) * sqrt( ( percent / max( ( 1.0 - percent ), 0.000001 ) ) ) );

}


fn lobeNormalFalloff ( roughness : f32, aggressivity : f32, invNormalPhi : f32 ) -> f32 {

	

	let nodeConst0 = ( 1.0 / max( atan( specularLobeTanHalfAngle( roughness, clamp( mix( ( invNormalPhi * invNormalPhi ), 0.0, sqrt( aggressivity ) ), 0.1, 0.99 ) ) ), 0.0058823529411764705 ) );

	return ( ( nodeConst0 * nodeConst0 ) * 8.0 );

}


fn vogelDisk ( i : f32, radius : f32 ) -> vec2<f32> {

	

	let nodeConst0 = ( ( i + 0.5 ) * 2.399827721492203 );

	return ( vec2<f32>( cos( nodeConst0 ), sin( nodeConst0 ) ) * vec2<f32>( ( radius * sqrt( ( ( i + 0.5 ) / 8.0 ) ) ) ) );

}


fn tsl_mod_vec2( x : vec2f, y : vec2f ) -> vec2f { return x - y * floor( x / y ); }
fn planeDistance ( position : vec3<f32>, nPosition : vec3<f32>, normal : vec3<f32> ) -> f32 {

	


	return abs( dot( ( position - nPosition ), normal ) );

}


fn lobeNormalWeight ( viewNormal : vec3<f32>, nNormalV : vec3<f32>, lobeFalloff : f32 ) -> f32 {

	


	return exp( ( ( dot( viewNormal, nNormalV ) - 1.0 ) * lobeFalloff ) );

}


fn karisTemporalBlend ( denoisedRgb : vec3<f32>, denoisedRaw : vec3<f32>, a : f32, flickerSuppression : f32, adaptiveTrust : f32, nbhdMeanLuma : f32, nbhdStddevLuma : f32 ) -> vec3<f32> {

	

	let nodeConst0 = ( nbhdStddevLuma / max( nbhdMeanLuma, 0.0001 ) );
	let nodeConst1 = ( a * ( 1.0 - clamp( ( ( nodeConst0 * adaptiveTrust ) * ( 1.0 - a ) ), 0.0, 0.9 ) ) );
	let nodeConst2 = ( flickerSuppression * mix( ( 1.0 - adaptiveTrust ), 1.0, smoothstep( 0.1, 2.0, nodeConst0 ) ) );
	let nodeConst3 = ( ( 1.0 - nodeConst1 ) / ( ( ( dot( denoisedRgb, vec3<f32>( 0.2126, 0.7152, 0.0722 ) ) * nodeConst2 ) * 10.0 ) + 1.0 ) );
	let nodeConst4 = ( nodeConst1 / ( ( ( dot( denoisedRaw, vec3<f32>( 0.2126, 0.7152, 0.0722 ) ) * nodeConst2 ) * 10.0 ) + 1.0 ) );

	return ( ( ( denoisedRgb * vec3<f32>( nodeConst3 ) ) + ( denoisedRaw * vec3<f32>( nodeConst4 ) ) ) / vec3<f32>( max( ( nodeConst3 + nodeConst4 ), 0.000001 ) ) );

}




@fragment
fn main( @location( 0 ) nodeVarying0 : vec2<f32> ) -> OutputStruct {

	// flow
	// code

	nodeVar1 = textureDimensions( nodeUniform3, u32( 0 ) );
	nodeVar0 = textureLoad( nodeUniform3, vec2<u32>( clamp( floor( tsl_coord_clampS_clampT_2d( nodeVarying0 ) * vec2<f32>( nodeVar1 ) ), vec2<f32>( 0 ), vec2<f32>( nodeVar1 - vec2<u32>( 1, 1 ) ) ) ), u32( 0 ) );
	let nodeConst0 = nodeVar0;

	if ( ( nodeConst0 >= 1.0 ) ) {

		discard;
		

	} else {

		nodeVar2 = textureSample( nodeUniform4, nodeUniform4_sampler, nodeVarying0 );
		let nodeConst1 = ( ( nodeVar2.xyz * vec3<f32>( 2.0 ) ) - vec3<f32>( 1.0 ) );
		let nodeConst2 = normalize( ( vec4<f32>( nodeConst1, 0.0 ) * object.nodeUniform5 ).xyz );
		nodeVar3 = textureSample( nodeUniform1, nodeUniform1_sampler, nodeVarying0 );
		let nodeConst3 = max( max( nodeVar3, vec4<f32>( 0.0 ) ), vec4<f32>( 0.0 ) );
		let nodeConst4 = ( object.nodeUniform6 * vec4<f32>( vec3<f32>( ( ( vec2<f32>( nodeVarying0.x, ( 1.0 - nodeVarying0.y ) ) * vec2<f32>( 2.0 ) ) - vec2<f32>( 1.0 ) ), nodeConst0 ), 1.0 ) );
		let nodeConst5 = ( nodeConst4.xyz / vec3<f32>( nodeConst4.w ) );
		const nodeConst6 = vec2<f32>( 0.0, 1.0 );
		nodeVar4 = textureSample( nodeUniform1, nodeUniform1_sampler, nodeVarying0 );
		let nodeConst7 = max( nodeVar4, vec4<f32>( 0.0 ) );
		nodeVar5 = 1.0;
		nodeVar6 = 0.0;
		nodeVar7 = 0.0;
		nodeVar8 = false;

		if ( ( object.nodeUniform0 > 0.0 ) ) {

			let nodeConst8 = getNeighborhoodStats( nodeVarying0, nodeConst7 );
			nodeVar6 = nodeConst8.y;
			nodeVar7 = nodeConst8.z;
			

		}

		nodeVar9 = nodeConst3.xyz;
		nodeVar10 = 1.0;
		let nodeConst9 = ( 1.0 / nodeConst3.w );
		nodeVar11 = nodeConst9;
		nodeVar12 = 1.0;
		nodeVar13 = nodeConst7.xyz;
		nodeVar14 = 1.0;

		if ( ( length( nodeConst7.xyz ) < 0.0001 ) ) {

			nodeVar13 = vec3<f32>( 0.0, 0.0, 0.0 );
			nodeVar14 = 0.0;
			

		}

		let nodeConst10 = nodeConst7.w;
		nodeVar15 = ( object.nodeUniform7 * 0.1 );
		nodeVar15 = ( nodeVar15 * ( pow( nodeConst10, 2.0 ) * abs( nodeConst5.z ) ) );
		let nodeConst11 = ( 1.0 - getTemporalVarianceFactor( nodeConst9, ( 1.0 - object.nodeUniform8 ) ) );
		nodeVar15 = ( nodeVar15 * mix( 1.0, 0.001, nodeConst11 ) );
		nodeVar16 = vec3<f32>( 0.0, 0.0, 0.0 );
		nodeVar17 = vec3<f32>( 0.0, 0.0, 0.0 );
		nodeVar18 = normalize( cross( vec3<f32>( 0.0, 0.0, 1.0 ), nodeConst1 ) );

		if ( ( length( nodeVar18 ) < 0.000001 ) ) {

			nodeVar18 = normalize( cross( vec3<f32>( 0.0, 1.0, 0.0 ), nodeConst1 ) );
			

		}

		nodeVar16 = nodeVar18;
		nodeVar17 = normalize( cross( nodeConst1, nodeVar18 ) );
		nodeVar16 = ( nodeVar16 * vec3<f32>( nodeVar15 ) );
		nodeVar17 = ( nodeVar17 * vec3<f32>( nodeVar15 ) );
		const nodeConst12 = vec3<f32>( 0.0, 0.0, 0.0 );
		nodeVar19 = 1.0;
		nodeVar20 = vec2<f32>( 0.0, 0.0 );
		let nodeConst13 = lobeNormalFalloff( nodeConst6.y, nodeConst11, ( 1.0 - object.nodeUniform9 ) );

		for ( var i : i32 = 0; i < 8; i ++ ) {

			nodeVar21 = vogelDisk( f32( i ), 1.0 );
			let nodeConst14 = normalize( nodeVar21 );
			nodeVar22 = vec4<f32>( 0.0, 0.0, 0.0, 1.0 );
			let nodeConst15 = ( i32( object.nodeUniform10 ) + 83 );
			let nodeConst16 = tsl_mod_vec2( ( floor( ( nodeVarying0 * object.nodeUniform2 ) ) + floor( ( fract( vec2<f32>( ( f32( nodeConst15 ) * 0.7548776662 ), ( f32( nodeConst15 ) * 0.569840291 ) ) ) * vec2<f32>( 32.0 ) ) ) ), vec2<f32>( 32.0 ) );
			let nodeConst17 = ( ( ( nodeConst16.x * 0.7548776662466927 ) + ( nodeConst16.y * 0.5698402909980532 ) ) + 83.0 );
			nodeVar22 = vec4<f32>( fract( ( ( nodeConst17 * 1.324717957244746 ) * 0.7548776662466927 ) ), fract( ( ( nodeConst17 * 2.649435914489492 ) * 0.5698402909980532 ) ), fract( ( ( nodeConst17 * 3.974153871734238 ) * 0.419875421 ) ), fract( ( ( nodeConst17 * 5.298871828978984 ) * 0.43015970900194667 ) ) );
			let nodeConst18 = ( ( nodeVar22.x * 2.0 ) * 3.141592653589793 );

			if ( ( dot( nodeVar20, nodeVar20 ) > 0.001 ) ) {

				nodeVar23 = 1.0;

			} else {

				nodeVar23 = 0.0;

			}

			nodeVar24 = ( mat2x2<f32>( cos( nodeConst18 ), ( - sin( nodeConst18 ) ), sin( nodeConst18 ), cos( nodeConst18 ) ) * ( mix( nodeConst14, normalize( max( nodeVar20, vec2<f32>( 0.000001 ) ) ), ( ( object.nodeUniform11 * nodeConst11 ) * nodeVar23 ) ) * vec2<f32>( ( length( nodeVar21 ) * nodeVar19 ) ) ) );
			let nodeConst19 = ( object.nodeUniform12 * vec4<f32>( ( nodeConst5 + ( ( nodeVar17 * vec3<f32>( nodeVar24.x ) ) + ( nodeVar16 * vec3<f32>( nodeVar24.y ) ) ) ), 1.0 ) );
			nodeVar25 = ( ( ( nodeConst19.xy / vec2<f32>( nodeConst19.w ) ) * vec2<f32>( 0.5 ) ) + vec2<f32>( 0.5 ) );
			nodeVar26 = vec2<f32>( nodeVar25.x, ( 1.0 - nodeVar25.y ) );
			nodeVar26 = clamp( ( vec2<f32>( 1.0 ) - abs( ( vec2<f32>( 1.0 ) - abs( nodeVar26 ) ) ) ), vec2<f32>( 0.0 ), vec2<f32>( 1.0 ) );
			nodeVar27 = textureSample( nodeUniform1, nodeUniform1_sampler, nodeVar26 );
			let nodeConst20 = max( max( nodeVar27, vec4<f32>( 0.0 ) ), vec4<f32>( 0.0 ) );
			nodeVar28 = textureSample( nodeUniform1, nodeUniform1_sampler, nodeVar26 );
			nodeVar29 = max( max( nodeVar28, vec4<f32>( 0.0 ) ), vec4<f32>( 0.0 ) );
			nodeVar30 = textureLoad( nodeUniform3, vec2<u32>( clamp( floor( tsl_coord_clampS_clampT_2d( nodeVar26 ) * vec2<f32>( nodeVar1 ) ), vec2<f32>( 0 ), vec2<f32>( nodeVar1 - vec2<u32>( 1, 1 ) ) ) ), u32( 0 ) );
			let nodeConst21 = ( object.nodeUniform6 * vec4<f32>( vec3<f32>( ( ( vec2<f32>( nodeVar26.x, ( 1.0 - nodeVar26.y ) ) * vec2<f32>( 2.0 ) ) - vec2<f32>( 1.0 ) ), nodeVar30 ), 1.0 ) );
			let nodeConst22 = ( nodeConst21.xyz / vec3<f32>( nodeConst21.w ) );
			let nodeConst23 = abs( nodeConst22.z );
			nodeVar31 = 0.0;
			nodeVar31 = ( nodeVar31 + ( ( abs( ( dot( nodeVar29.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) ) - dot( nodeConst7.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) ) ) ) * object.nodeUniform13 ) * 10.0 ) );
			let nodeConst24 = pow( nodeConst10, 0.1 );
			nodeVar31 = ( nodeVar31 + ( ( ( nodeConst24 / ( ( nodeConst24 + pow( nodeVar29.w, 0.1 ) ) + 0.05 ) ) * object.nodeUniform14 ) * nodeConst11 ) );
			nodeVar32 = textureSample( nodeUniform4, nodeUniform4_sampler, nodeVar26 );
			nodeVar33 = ( exp( ( - ( ( nodeVar31 * nodeConst11 ) + ( planeDistance( nodeConst5, nodeConst22, nodeConst1 ) * ( ( ( object.nodeUniform15 * 500.0 ) * abs( nodeConst1.z ) ) / abs( nodeConst5.z ) ) ) ) ) ) * lobeNormalWeight( nodeConst2, normalize( ( vec4<f32>( ( ( nodeVar32.xyz * vec3<f32>( 2.0 ) ) - vec3<f32>( 1.0 ) ), 0.0 ) * object.nodeUniform5 ).xyz ), nodeConst13 ) );
			nodeVar19 = mix( nodeVar19, nodeVar33, object.nodeUniform11 );
			nodeVar20 = mix( nodeVar20, ( nodeConst14 * vec2<f32>( ( nodeVar33 - 0.5 ) ) ), 0.5 );
			nodeVar33 = ( nodeVar33 * mix( ( 1.0 / ( pow( dot( nodeVar29.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) ), 2.0 ) + 0.01 ) ), 1.0, min( ( nodeConst9 / 5.0 ), 1.0 ) ) );
			nodeVar13 = ( nodeVar13 + ( nodeVar29.xyz * vec3<f32>( nodeVar33 ) ) );
			nodeVar14 = ( nodeVar14 + nodeVar33 );
			nodeVar9 = ( nodeVar9 + ( nodeConst20.xyz * vec3<f32>( nodeVar33 ) ) );
			nodeVar10 = ( nodeVar10 + nodeVar33 );
			nodeVar34 = bool( object.nodeUniform16 );

			if ( nodeVar34 ) {


				if ( ( nodeConst20.w > nodeConst3.w ) ) {

					nodeVar35 = ( nodeVar33 * 0.33 );

				} else {

					nodeVar35 = 0.0;

				}

				nodeVar11 = ( nodeVar11 + ( ( 1.0 / nodeConst20.w ) * nodeVar35 ) );
				nodeVar12 = ( nodeVar12 + nodeVar35 );
				

			}


		}

		nodeVar9 = ( nodeVar9 / vec3<f32>( max( nodeVar10, 0.000001 ) ) );
		nodeVar9 = max( nodeVar9, vec3<f32>( 0.000001 ) );
		nodeVar13 = ( nodeVar13 / vec3<f32>( max( nodeVar14, 0.000001 ) ) );
		let nodeConst25 = ( 1.0 / max( ( nodeVar11 / max( nodeVar12, 0.000001 ) ), 0.000001 ) );
		nodeVar36 = vec4<f32>( karisTemporalBlend( nodeVar9, nodeVar13, nodeConst25, object.nodeUniform17, object.nodeUniform0, nodeVar6, nodeVar7 ), nodeConst25 );
		

	}


	// result

	output.color = nodeVar36;

	return output;

}
