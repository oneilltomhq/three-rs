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
@binding( 6 ) @group( 0 ) var nodeUniform6_sampler : sampler;
@binding( 7 ) @group( 0 ) var nodeUniform6 : texture_2d<f32>;
@binding( 8 ) @group( 0 ) var nodeUniform8_sampler : sampler;
@binding( 9 ) @group( 0 ) var nodeUniform8 : texture_2d<f32>;

struct objectStruct {
	nodeUniform0 : f32,
	nodeUniform2 : vec2<f32>,
	nodeUniform5 : mat4x4<f32>,
	nodeUniform7 : mat4x4<f32>,
	nodeUniform9 : f32,
	nodeUniform10 : f32,
	nodeUniform11 : f32,
	nodeUniform12 : f32,
	nodeUniform13 : f32,
	nodeUniform14 : f32,
	nodeUniform15 : mat4x4<f32>,
	nodeUniform16 : f32,
	nodeUniform17 : f32,
	nodeUniform18 : f32,
	nodeUniform19 : f32,
	nodeUniform20 : f32,
	nodeUniform21 : u32,
	nodeUniform22 : f32
};
@binding( 0 ) @group( 0 )
var<uniform> object : objectStruct;

// vars
var<private> nodeVar0 : f32;
var<private> nodeVar1 : vec2<u32>;
var<private> nodeVar2 : vec4<f32>;
var<private> nodeVar3 : vec4<f32>;
var<private> nodeVar4 : vec4<f32>;
var<private> nodeVar5 : vec4<f32>;
var<private> nodeVar6 : vec4<f32>;
var<private> nodeVar7 : f32;
var<private> nodeVar8 : f32;
var<private> nodeVar9 : f32;
var<private> nodeVar10 : bool;
var<private> nodeVar11 : vec3<f32>;
var<private> nodeVar12 : f32;
var<private> nodeVar13 : f32;
var<private> nodeVar14 : f32;
var<private> nodeVar15 : vec3<f32>;
var<private> nodeVar16 : f32;
var<private> nodeVar17 : f32;
var<private> nodeVar18 : vec3<f32>;
var<private> nodeVar19 : vec3<f32>;
var<private> nodeVar20 : vec4<f32>;
var<private> nodeVar21 : f32;
var<private> nodeVar22 : vec2<f32>;
var<private> nodeVar23 : vec2<f32>;
var<private> nodeVar24 : vec4<f32>;
var<private> nodeVar25 : f32;
var<private> nodeVar26 : vec2<f32>;
var<private> nodeVar27 : vec2<f32>;
var<private> nodeVar28 : vec2<f32>;
var<private> nodeVar29 : vec4<f32>;
var<private> nodeVar30 : vec4<f32>;
var<private> nodeVar31 : vec4<f32>;
var<private> nodeVar32 : f32;
var<private> nodeVar33 : f32;
var<private> nodeVar34 : vec4<f32>;
var<private> nodeVar35 : f32;
var<private> nodeVar36 : vec4<f32>;
var<private> nodeVar37 : vec4<f32>;
var<private> nodeVar38 : vec4<f32>;
var<private> nodeVar39 : f32;
var<private> nodeVar40 : bool;
var<private> nodeVar41 : f32;
var<private> nodeVar42 : vec4<f32>;

// codes
fn computeFrustumSize ( viewZ : f32, tanHalfFovY : f32 ) -> f32 {

	


	return ( ( 2.0 * viewZ ) * tanHalfFovY );

}


fn temporalWeight ( x : f32, strength : f32 ) -> f32 {

	


	return ( 1.0 / pow( x, strength ) );

}


fn specularLobeTanHalfAngle ( roughness : f32, percent : f32 ) -> f32 {

	


	return ( ( roughness * roughness ) * sqrt( ( percent / max( ( 1.0 - percent ), 0.000001 ) ) ) );

}


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
	var nodeVar3 : f32;
	var nodeVar4 : f32;
	var nodeVar5 : bool;
	var nodeVar6 : f32;
	var nodeVar7 : vec4<f32>;
	var nodeVar8 : f32;
	var nodeVar9 : vec4<f32>;
	var nodeVar10 : f32;
	var nodeVar11 : vec4<f32>;
	var nodeVar12 : f32;
	var nodeVar13 : vec4<f32>;
	var nodeVar14 : f32;

	nodeVar0 = 0.0;
	nodeVar1 = 0.0;
	nodeVar2 = 0.0;
	nodeVar3 = 0.0;
	nodeVar4 = 0.0;
	nodeVar5 = false;
	nodeVar6 = centerSample.w;

	if ( ( nodeVar6 > 1000.0 ) ) {

		nodeVar6 = 0.25;
		nodeVar5 = true;
		

	}

	let nodeConst0 = ( 1.0 / ( nodeVar6 + 0.001 ) );
	nodeVar0 = ( nodeVar0 + ( nodeVar6 * nodeConst0 ) );
	nodeVar1 = ( nodeVar1 + nodeConst0 );

	if ( ( object.nodeUniform0 > 0.0 ) ) {

		nodeVar4 = ( nodeVar4 + 1.0 );
		let nodeConst1 = dot( centerSample.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) );
		let nodeConst2 = ( nodeConst1 - nodeVar2 );
		nodeVar2 = ( nodeVar2 + ( nodeConst2 / nodeVar4 ) );
		nodeVar3 = ( nodeVar3 + ( nodeConst2 * ( nodeConst1 - nodeVar2 ) ) );
		

	}

	nodeVar7 = textureSample( nodeUniform1, nodeUniform1_sampler, ( uvCoord + ( vec2<f32>( -1.0, 0.0 ) / object.nodeUniform2 ) ) );
	let nodeConst3 = max( nodeVar7, vec4<f32>( 0.0 ) );
	nodeVar8 = nodeConst3.w;

	if ( ( nodeVar8 > 1000.0 ) ) {

		nodeVar8 = 0.25;
		nodeVar5 = true;
		

	}

	let nodeConst4 = ( 1.0 / ( nodeVar8 + 0.001 ) );
	nodeVar0 = ( nodeVar0 + ( nodeVar8 * nodeConst4 ) );
	nodeVar1 = ( nodeVar1 + nodeConst4 );

	if ( ( object.nodeUniform0 > 0.0 ) ) {

		nodeVar4 = ( nodeVar4 + 1.0 );
		let nodeConst5 = dot( nodeConst3.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) );
		let nodeConst6 = ( nodeConst5 - nodeVar2 );
		nodeVar2 = ( nodeVar2 + ( nodeConst6 / nodeVar4 ) );
		nodeVar3 = ( nodeVar3 + ( nodeConst6 * ( nodeConst5 - nodeVar2 ) ) );
		

	}

	nodeVar9 = textureSample( nodeUniform1, nodeUniform1_sampler, ( uvCoord + ( vec2<f32>( 1.0, 0.0 ) / object.nodeUniform2 ) ) );
	let nodeConst7 = max( nodeVar9, vec4<f32>( 0.0 ) );
	nodeVar10 = nodeConst7.w;

	if ( ( nodeVar10 > 1000.0 ) ) {

		nodeVar10 = 0.25;
		nodeVar5 = true;
		

	}

	let nodeConst8 = ( 1.0 / ( nodeVar10 + 0.001 ) );
	nodeVar0 = ( nodeVar0 + ( nodeVar10 * nodeConst8 ) );
	nodeVar1 = ( nodeVar1 + nodeConst8 );

	if ( ( object.nodeUniform0 > 0.0 ) ) {

		nodeVar4 = ( nodeVar4 + 1.0 );
		let nodeConst9 = dot( nodeConst7.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) );
		let nodeConst10 = ( nodeConst9 - nodeVar2 );
		nodeVar2 = ( nodeVar2 + ( nodeConst10 / nodeVar4 ) );
		nodeVar3 = ( nodeVar3 + ( nodeConst10 * ( nodeConst9 - nodeVar2 ) ) );
		

	}

	nodeVar11 = textureSample( nodeUniform1, nodeUniform1_sampler, ( uvCoord + ( vec2<f32>( 0.0, -1.0 ) / object.nodeUniform2 ) ) );
	let nodeConst11 = max( nodeVar11, vec4<f32>( 0.0 ) );
	nodeVar12 = nodeConst11.w;

	if ( ( nodeVar12 > 1000.0 ) ) {

		nodeVar12 = 0.25;
		nodeVar5 = true;
		

	}

	let nodeConst12 = ( 1.0 / ( nodeVar12 + 0.001 ) );
	nodeVar0 = ( nodeVar0 + ( nodeVar12 * nodeConst12 ) );
	nodeVar1 = ( nodeVar1 + nodeConst12 );

	if ( ( object.nodeUniform0 > 0.0 ) ) {

		nodeVar4 = ( nodeVar4 + 1.0 );
		let nodeConst13 = dot( nodeConst11.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) );
		let nodeConst14 = ( nodeConst13 - nodeVar2 );
		nodeVar2 = ( nodeVar2 + ( nodeConst14 / nodeVar4 ) );
		nodeVar3 = ( nodeVar3 + ( nodeConst14 * ( nodeConst13 - nodeVar2 ) ) );
		

	}

	nodeVar13 = textureSample( nodeUniform1, nodeUniform1_sampler, ( uvCoord + ( vec2<f32>( 0.0, 1.0 ) / object.nodeUniform2 ) ) );
	let nodeConst15 = max( nodeVar13, vec4<f32>( 0.0 ) );
	nodeVar14 = nodeConst15.w;

	if ( ( nodeVar14 > 1000.0 ) ) {

		nodeVar14 = 0.25;
		nodeVar5 = true;
		

	}

	let nodeConst16 = ( 1.0 / ( nodeVar14 + 0.001 ) );
	nodeVar0 = ( nodeVar0 + ( nodeVar14 * nodeConst16 ) );
	nodeVar1 = ( nodeVar1 + nodeConst16 );

	if ( ( object.nodeUniform0 > 0.0 ) ) {

		nodeVar4 = ( nodeVar4 + 1.0 );
		let nodeConst17 = dot( nodeConst15.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) );
		let nodeConst18 = ( nodeConst17 - nodeVar2 );
		nodeVar2 = ( nodeVar2 + ( nodeConst18 / nodeVar4 ) );
		nodeVar3 = ( nodeVar3 + ( nodeConst18 * ( nodeConst17 - nodeVar2 ) ) );
		

	}


	return vec4<f32>( ( nodeVar0 / nodeVar1 ), nodeVar2, sqrt( ( nodeVar3 / max( nodeVar4, 1.0 ) ) ), f32( nodeVar5 ) );

}


fn computeHitDistFactor ( worldRayLength : f32, viewZ : f32, tanHalfFovY : f32 ) -> f32 {

	


	return clamp( ( worldRayLength / max( computeFrustumSize( viewZ, tanHalfFovY ), 0.000001 ) ), 0.0, 1.0 );

}


fn getTemporalVarianceFactor ( frameNum : f32, strength : f32 ) -> f32 {

	


	return max( temporalWeight( frameNum, strength ), 0.05 );

}


fn getSpecularDominantDirection ( N : vec3<f32>, V : vec3<f32>, roughness : f32 ) -> vec3<f32> {

	


	return normalize( mix( N, reflect( ( - V ), N ), ( 1.0 - roughness ) ) );

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
fn diffuseColorDistance ( a : vec3<f32>, b : vec3<f32>, compressLuma : f32 ) -> f32 {

	

	let nodeConst0 = vec3<f32>( dot( a, vec3<f32>( 0.25, 0.5, 0.25 ) ), ( a.x - a.z ), ( a.y - ( ( a.x + a.z ) * 0.5 ) ) );
	let nodeConst1 = vec3<f32>( dot( b, vec3<f32>( 0.25, 0.5, 0.25 ) ), ( b.x - b.z ), ( b.y - ( ( b.x + b.z ) * 0.5 ) ) );

	return ( abs( ( mix( nodeConst0.x, log( ( nodeConst0.x + 1.0 ) ), compressLuma ) - mix( nodeConst1.x, log( ( nodeConst1.x + 1.0 ) ), compressLuma ) ) ) + ( length( vec2<f32>( ( nodeConst0.y - nodeConst1.y ), ( nodeConst0.z - nodeConst1.z ) ) ) * 2.0 ) );

}


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
		nodeVar3 = textureSample( nodeUniform6, nodeUniform6_sampler, nodeVarying0 );
		let nodeConst3 = max( max( nodeVar3, vec4<f32>( 0.0 ) ), vec4<f32>( 0.0 ) );
		let nodeConst4 = ( object.nodeUniform7 * vec4<f32>( vec3<f32>( ( ( vec2<f32>( nodeVarying0.x, ( 1.0 - nodeVarying0.y ) ) * vec2<f32>( 2.0 ) ) - vec2<f32>( 1.0 ) ), nodeConst0 ), 1.0 ) );
		let nodeConst5 = ( nodeConst4.xyz / vec3<f32>( nodeConst4.w ) );
		nodeVar4 = textureSample( nodeUniform8, nodeUniform8_sampler, nodeVarying0 );
		nodeVar5 = textureSample( nodeUniform4, nodeUniform4_sampler, nodeVarying0 );
		let nodeConst6 = vec2<f32>( nodeVar4.w, nodeVar5.w );
		nodeVar6 = textureSample( nodeUniform1, nodeUniform1_sampler, nodeVarying0 );
		let nodeConst7 = max( nodeVar6, vec4<f32>( 0.0 ) );
		nodeVar7 = 1.0;
		nodeVar8 = 0.0;
		nodeVar9 = 0.0;
		nodeVar10 = false;
		let nodeConst8 = getNeighborhoodStats( nodeVarying0, nodeConst7 );
		nodeVar7 = nodeConst8.x;
		nodeVar8 = nodeConst8.y;
		nodeVar9 = nodeConst8.z;
		nodeVar10 = ( nodeConst8.w > 0.5 );
		let nodeConst9 = tan( ( object.nodeUniform9 * 0.5 ) );
		let nodeConst10 = computeHitDistFactor( nodeVar7, abs( nodeConst5.z ), nodeConst9 );
		nodeVar11 = nodeConst3.xyz;
		nodeVar12 = 1.0;
		let nodeConst11 = ( 1.0 / nodeConst3.w );
		nodeVar13 = nodeConst11;
		nodeVar14 = 1.0;
		nodeVar15 = nodeConst7.xyz;
		nodeVar16 = 1.0;

		if ( ( length( nodeConst7.xyz ) < 0.0001 ) ) {

			nodeVar15 = vec3<f32>( 0.0, 0.0, 0.0 );
			nodeVar16 = 0.0;
			

		}

		nodeVar17 = ( object.nodeUniform10 * 0.1 );
		nodeVar17 = ( nodeVar17 * ( nodeVar7 * abs( nodeConst5.z ) ) );
		nodeVar17 = ( nodeVar17 * max( sqrt( nodeConst6.y ), 0.01 ) );
		let nodeConst12 = ( 1.0 - getTemporalVarianceFactor( nodeConst11, ( 1.0 - object.nodeUniform11 ) ) );
		nodeVar17 = ( nodeVar17 * mix( 1.0, 0.001, nodeConst12 ) );
		nodeVar18 = vec3<f32>( 0.0, 0.0, 0.0 );
		nodeVar19 = vec3<f32>( 0.0, 0.0, 0.0 );
		let nodeConst13 = reflect( ( - getSpecularDominantDirection( nodeConst1, ( - normalize( nodeConst5 ) ), nodeConst6.y ) ), nodeConst1 );
		let nodeConst14 = normalize( cross( nodeConst1, nodeConst13 ) );
		nodeVar18 = ( nodeConst14 * vec3<f32>( mix( 1.0, nodeConst6.y, clamp( ( acos( abs( nodeConst1.z ) ) / 1.5707963267948966 ), 0.0, 1.0 ) ) ) );
		nodeVar19 = cross( nodeConst13, nodeConst14 );
		nodeVar18 = ( nodeVar18 * vec3<f32>( nodeVar17 ) );
		nodeVar19 = ( nodeVar19 * vec3<f32>( nodeVar17 ) );
		nodeVar20 = textureSample( nodeUniform8, nodeUniform8_sampler, nodeVarying0 );
		let nodeConst15 = nodeVar20.xyz;
		nodeVar21 = 1.0;
		nodeVar22 = vec2<f32>( 0.0, 0.0 );
		let nodeConst16 = lobeNormalFalloff( nodeConst6.y, nodeConst12, ( 1.0 - object.nodeUniform12 ) );

		for ( var i : i32 = 0; i < 8; i ++ ) {

			nodeVar23 = vogelDisk( f32( i ), 1.0 );
			let nodeConst17 = normalize( nodeVar23 );
			nodeVar24 = vec4<f32>( 0.0, 0.0, 0.0, 1.0 );
			let nodeConst18 = ( i32( object.nodeUniform13 ) + 83 );
			let nodeConst19 = tsl_mod_vec2( ( floor( ( nodeVarying0 * object.nodeUniform2 ) ) + floor( ( fract( vec2<f32>( ( f32( nodeConst18 ) * 0.7548776662 ), ( f32( nodeConst18 ) * 0.569840291 ) ) ) * vec2<f32>( 32.0 ) ) ) ), vec2<f32>( 32.0 ) );
			let nodeConst20 = ( ( ( nodeConst19.x * 0.7548776662466927 ) + ( nodeConst19.y * 0.5698402909980532 ) ) + 83.0 );
			nodeVar24 = vec4<f32>( fract( ( ( nodeConst20 * 1.324717957244746 ) * 0.7548776662466927 ) ), fract( ( ( nodeConst20 * 2.649435914489492 ) * 0.5698402909980532 ) ), fract( ( ( nodeConst20 * 3.974153871734238 ) * 0.419875421 ) ), fract( ( ( nodeConst20 * 5.298871828978984 ) * 0.43015970900194667 ) ) );
			let nodeConst21 = ( ( nodeVar24.x * 2.0 ) * 3.141592653589793 );

			if ( ( dot( nodeVar22, nodeVar22 ) > 0.001 ) ) {

				nodeVar25 = 1.0;

			} else {

				nodeVar25 = 0.0;

			}

			nodeVar26 = ( mat2x2<f32>( cos( nodeConst21 ), ( - sin( nodeConst21 ) ), sin( nodeConst21 ), cos( nodeConst21 ) ) * ( mix( nodeConst17, normalize( max( nodeVar22, vec2<f32>( 0.000001 ) ) ), ( ( object.nodeUniform14 * nodeConst12 ) * nodeVar25 ) ) * vec2<f32>( ( length( nodeVar23 ) * nodeVar21 ) ) ) );
			let nodeConst22 = ( object.nodeUniform15 * vec4<f32>( ( nodeConst5 + ( ( nodeVar19 * vec3<f32>( nodeVar26.x ) ) + ( nodeVar18 * vec3<f32>( nodeVar26.y ) ) ) ), 1.0 ) );
			nodeVar27 = ( ( ( nodeConst22.xy / vec2<f32>( nodeConst22.w ) ) * vec2<f32>( 0.5 ) ) + vec2<f32>( 0.5 ) );
			nodeVar28 = vec2<f32>( nodeVar27.x, ( 1.0 - nodeVar27.y ) );
			nodeVar28 = clamp( ( vec2<f32>( 1.0 ) - abs( ( vec2<f32>( 1.0 ) - abs( nodeVar28 ) ) ) ), vec2<f32>( 0.0 ), vec2<f32>( 1.0 ) );
			nodeVar29 = textureSample( nodeUniform6, nodeUniform6_sampler, nodeVar28 );
			let nodeConst23 = max( max( nodeVar29, vec4<f32>( 0.0 ) ), vec4<f32>( 0.0 ) );
			nodeVar30 = textureSample( nodeUniform1, nodeUniform1_sampler, nodeVar28 );
			nodeVar31 = max( max( nodeVar30, vec4<f32>( 0.0 ) ), vec4<f32>( 0.0 ) );
			nodeVar32 = textureLoad( nodeUniform3, vec2<u32>( clamp( floor( tsl_coord_clampS_clampT_2d( nodeVar28 ) * vec2<f32>( nodeVar1 ) ), vec2<f32>( 0 ), vec2<f32>( nodeVar1 - vec2<u32>( 1, 1 ) ) ) ), u32( 0 ) );
			let nodeConst24 = ( object.nodeUniform7 * vec4<f32>( vec3<f32>( ( ( vec2<f32>( nodeVar28.x, ( 1.0 - nodeVar28.y ) ) * vec2<f32>( 2.0 ) ) - vec2<f32>( 1.0 ) ), nodeVar32 ), 1.0 ) );
			let nodeConst25 = ( nodeConst24.xyz / vec3<f32>( nodeConst24.w ) );
			let nodeConst26 = abs( nodeConst25.z );
			nodeVar33 = 0.0;
			nodeVar33 = ( nodeVar33 + ( ( abs( ( dot( nodeVar31.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) ) - dot( nodeConst7.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) ) ) ) * object.nodeUniform16 ) * 10.0 ) );
			nodeVar34 = textureSample( nodeUniform8, nodeUniform8_sampler, nodeVar28 );
			nodeVar33 = ( nodeVar33 + ( ( diffuseColorDistance( nodeConst15, nodeVar34.xyz, 0.0 ) * object.nodeUniform17 ) * nodeConst6.x ) );

			if ( ( ( nodeVar31.w > 1000.0 ) && nodeVar10 ) ) {

				nodeVar35 = 1.0;

			} else {

				nodeVar35 = ( ( abs( ( nodeConst10 - computeHitDistFactor( nodeVar31.w, nodeConst26, nodeConst9 ) ) ) * object.nodeUniform18 ) / abs( nodeConst5.z ) );

			}

			nodeVar33 = ( nodeVar33 + nodeVar35 );
			nodeVar36 = textureSample( nodeUniform8, nodeUniform8_sampler, nodeVar28 );
			nodeVar37 = textureSample( nodeUniform4, nodeUniform4_sampler, nodeVar28 );
			nodeVar33 = ( nodeVar33 + ( abs( ( nodeConst6.y - vec2<f32>( nodeVar36.w, nodeVar37.w ).y ) ) * object.nodeUniform19 ) );
			nodeVar38 = textureSample( nodeUniform4, nodeUniform4_sampler, nodeVar28 );
			nodeVar39 = ( exp( ( - ( ( nodeVar33 * nodeConst12 ) + ( planeDistance( nodeConst5, nodeConst25, nodeConst1 ) * ( ( ( object.nodeUniform20 * 500.0 ) * abs( nodeConst1.z ) ) / abs( nodeConst5.z ) ) ) ) ) ) * lobeNormalWeight( nodeConst2, normalize( ( vec4<f32>( ( ( nodeVar38.xyz * vec3<f32>( 2.0 ) ) - vec3<f32>( 1.0 ) ), 0.0 ) * object.nodeUniform5 ).xyz ), nodeConst16 ) );
			nodeVar21 = mix( nodeVar21, nodeVar39, object.nodeUniform14 );
			nodeVar22 = mix( nodeVar22, ( nodeConst17 * vec2<f32>( ( nodeVar39 - 0.5 ) ) ), 0.5 );
			nodeVar39 = ( nodeVar39 * mix( ( 1.0 / ( pow( dot( nodeVar31.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) ), 2.0 ) + 0.01 ) ), 1.0, min( ( nodeConst11 / 5.0 ), 1.0 ) ) );
			nodeVar15 = ( nodeVar15 + ( nodeVar31.xyz * vec3<f32>( nodeVar39 ) ) );
			nodeVar16 = ( nodeVar16 + nodeVar39 );
			nodeVar11 = ( nodeVar11 + ( nodeConst23.xyz * vec3<f32>( nodeVar39 ) ) );
			nodeVar12 = ( nodeVar12 + nodeVar39 );
			nodeVar40 = bool( object.nodeUniform21 );

			if ( nodeVar40 ) {


				if ( ( nodeConst23.w > nodeConst3.w ) ) {

					nodeVar41 = ( nodeVar39 * 0.33 );

				} else {

					nodeVar41 = 0.0;

				}

				nodeVar13 = ( nodeVar13 + ( ( 1.0 / nodeConst23.w ) * nodeVar41 ) );
				nodeVar14 = ( nodeVar14 + nodeVar41 );
				

			}


		}

		nodeVar11 = ( nodeVar11 / vec3<f32>( max( nodeVar12, 0.000001 ) ) );
		nodeVar11 = max( nodeVar11, vec3<f32>( 0.000001 ) );
		nodeVar15 = ( nodeVar15 / vec3<f32>( max( nodeVar16, 0.000001 ) ) );
		let nodeConst27 = ( 1.0 / max( ( nodeVar13 / max( nodeVar14, 0.000001 ) ), 0.000001 ) );
		nodeVar42 = vec4<f32>( karisTemporalBlend( nodeVar11, nodeVar15, nodeConst27, object.nodeUniform22, object.nodeUniform0, nodeVar8, nodeVar9 ), nodeConst27 );
		

	}


	// result

	output.color = nodeVar42;

	return output;

}
