// Three.js r187dev - Node System

// global
diagnostic( off, derivative_uniformity );


// directives


// structs

struct StructType0 {
	closestDepth : f32,
	closestPositionTexel : vec2<f32>,
	farthestDepth : f32
};

struct OutputStruct {
	@location( 0 ) color: vec4<f32>
};
var<private> output : OutputStruct;

// uniforms
@binding( 0 ) @group( 0 ) var nodeUniform0_sampler : sampler;
@binding( 1 ) @group( 0 ) var nodeUniform0 : texture_2d<f32>;
@binding( 2 ) @group( 0 ) var nodeUniform1_sampler : sampler;
@binding( 3 ) @group( 0 ) var nodeUniform1 : texture_2d<f32>;
@binding( 4 ) @group( 0 ) var nodeUniform2 : texture_depth_2d;
@binding( 6 ) @group( 0 ) var nodeUniform7 : texture_depth_2d;
@binding( 7 ) @group( 0 ) var nodeUniform9_sampler : sampler;
@binding( 8 ) @group( 0 ) var nodeUniform9 : texture_2d<f32>;

struct objectStruct {
	nodeUniform3 : vec2<f32>,
	nodeUniform4 : mat4x4<f32>,
	nodeUniform5 : mat4x4<f32>,
	nodeUniform6 : mat4x4<f32>,
	nodeUniform8 : mat3x3<f32>,
	nodeUniform10 : mat3x3<f32>,
	nodeUniform11 : f32
};
@binding( 5 ) @group( 0 )
var<uniform> object : objectStruct;

// vars
var<private> DiffuseColor : vec4<f32>;
var<private> nodeVar0 : f32;
var<private> nodeVar1 : f32;
var<private> nodeVar2 : vec2<f32>;
var<private> nodeVar3 : f32;
var<private> nodeVar4 : vec2<f32>;
var<private> nodeVar5 : f32;
var<private> nodeVar6 : f32;
var<private> nodeVar7 : vec2<f32>;
var<private> nodeVar8 : f32;
var<private> nodeVar9 : f32;
var<private> nodeVar10 : vec2<f32>;
var<private> nodeVar11 : f32;
var<private> nodeVar12 : f32;
var<private> nodeVar13 : vec2<f32>;
var<private> nodeVar14 : f32;
var<private> nodeVar15 : f32;
var<private> nodeVar16 : vec2<f32>;
var<private> nodeVar17 : f32;
var<private> nodeVar18 : f32;
var<private> nodeVar19 : vec2<f32>;
var<private> nodeVar20 : f32;
var<private> nodeVar21 : f32;
var<private> nodeVar22 : vec2<f32>;
var<private> nodeVar23 : f32;
var<private> nodeVar24 : f32;
var<private> nodeVar25 : vec2<f32>;
var<private> nodeVar26 : f32;
var<private> nodeVar27 : f32;
var<private> nodeVar28 : vec2<f32>;
var<private> nodeVar29 : f32;
var<private> nodeVar30 : f32;
var<private> nodeVar31 : StructType0;
var<private> nodeVar32 : vec4<f32>;
var<private> nodeVar33 : f32;
var<private> nodeVar34 : f32;
var<private> nodeVar35 : vec2<u32>;
var<private> nodeVar36 : vec4<f32>;
var<private> nodeVar37 : vec4<f32>;
var<private> nodeVar38 : vec4<f32>;
var<private> nodeVar39 : vec4<f32>;
var<private> nodeVar40 : vec4<f32>;
var<private> nodeVar41 : vec4<f32>;
var<private> nodeVar42 : vec4<f32>;
var<private> nodeVar43 : vec4<f32>;
var<private> nodeVar44 : vec4<f32>;
var<private> nodeVar45 : vec4<f32>;
var<private> nodeVar46 : vec4<f32>;
var<private> nodeVar47 : vec4<f32>;
var<private> Output : vec4<f32>;

// codes
fn subpixelCorrection ( velocityUV : vec2<f32>, textureSize : vec2<i32> ) -> f32 {

	

	let nodeConst0 = abs( fract( ( velocityUV * vec2<f32>( textureSize ) ) ) );
	let nodeConst1 = max( nodeConst0, ( vec2<f32>( 1.0 ) - nodeConst0 ) );

	return ( ( 1.0 - ( nodeConst1.x * nodeConst1.y ) ) / 0.75 );

}


fn tsl_clampWrapping_float( coord: f32 ) -> f32 { return clamp( coord, 0.0, 1.0 ); }
fn tsl_coord_clampS_clampT_2d( coord : vec2f ) -> vec2f {

	return vec2f(
		tsl_clampWrapping_float( coord.x ),
		tsl_clampWrapping_float( coord.y )
	);

}

fn clipAABB ( currentColor : vec4<f32>, historyColor : vec4<f32>, minColor : vec4<f32>, maxColor : vec4<f32> ) -> vec4<f32> {

	var nodeVar0 : vec4<f32>;

	let nodeConst0 = ( ( maxColor.xyz + minColor.xyz ) * vec3<f32>( 0.5 ) );
	let nodeConst1 = ( ( ( maxColor.xyz - minColor.xyz ) * vec3<f32>( 0.5 ) ) + vec3<f32>( 1e-7 ) );
	let nodeConst2 = ( historyColor - vec4<f32>( nodeConst0, currentColor.w ) );
	let nodeConst3 = ( nodeConst2.xyz / nodeConst1 );
	let nodeConst4 = abs( nodeConst3 );
	let nodeConst5 = max( max( nodeConst4.x, nodeConst4.y ), nodeConst4.z );

	if ( ( nodeConst5 > 1.0 ) ) {

		nodeVar0 = ( vec4<f32>( nodeConst0, currentColor.w ) + ( nodeConst2 / vec4<f32>( nodeConst5 ) ) );

	} else {

		nodeVar0 = historyColor;

	}


	return nodeVar0;

}


fn flickerReduction ( currentColor : vec4<f32>, historyColor : vec4<f32>, currentWeight : f32 ) -> vec4<f32> {

	

	let nodeConst0 = ( currentColor * vec4<f32>( ( 1.0 / ( max( max( currentColor.x, currentColor.y ), currentColor.z ) + 1.0 ) ) ) );
	let nodeConst1 = ( historyColor * vec4<f32>( ( 1.0 / ( max( max( historyColor.x, historyColor.y ), historyColor.z ) + 1.0 ) ) ) );
	let nodeConst2 = dot( nodeConst0.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) );
	let nodeConst3 = dot( nodeConst1.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) );
	let nodeConst4 = ( currentWeight / ( nodeConst2 + 1.0 ) );
	let nodeConst5 = ( ( 1.0 - currentWeight ) / ( nodeConst3 + 1.0 ) );

	return ( ( ( currentColor * vec4<f32>( nodeConst4 ) ) + ( historyColor * vec4<f32>( nodeConst5 ) ) ) / vec4<f32>( max( ( nodeConst4 + nodeConst5 ), 0.00001 ) ) );

}




@fragment
fn main( @location( 0 ) nodeVarying0 : vec2<f32> ) -> OutputStruct {

	// flow
	// code

	nodeVar0 = 0.05;
	nodeVar1 = 2.0;
	nodeVar2 = vec2<f32>( 0.0, 0.0 );
	nodeVar3 = -1.0;
	let nodeConst0 = ( nodeVarying0 * vec2<f32>( textureDimensions( nodeUniform1, 0 ) ) );
	nodeVar4 = ( nodeConst0 + vec2<f32>( -1.0, -1.0 ) );
	nodeVar5 = textureLoad( nodeUniform2, vec2<i32>( nodeVar4 ), u32( 0u ) );
	nodeVar6 = nodeVar5;

	if ( ( nodeVar6 < nodeVar1 ) ) {

		nodeVar1 = nodeVar6;
		nodeVar2 = nodeVar4;
		

	}


	if ( ( nodeVar6 > nodeVar3 ) ) {

		nodeVar3 = nodeVar6;
		

	}

	nodeVar7 = ( nodeConst0 + vec2<f32>( -1.0, 0.0 ) );
	nodeVar8 = textureLoad( nodeUniform2, vec2<i32>( nodeVar7 ), u32( 0u ) );
	nodeVar9 = nodeVar8;

	if ( ( nodeVar9 < nodeVar1 ) ) {

		nodeVar1 = nodeVar9;
		nodeVar2 = nodeVar7;
		

	}


	if ( ( nodeVar9 > nodeVar3 ) ) {

		nodeVar3 = nodeVar9;
		

	}

	nodeVar10 = ( nodeConst0 + vec2<f32>( -1.0, 1.0 ) );
	nodeVar11 = textureLoad( nodeUniform2, vec2<i32>( nodeVar10 ), u32( 0u ) );
	nodeVar12 = nodeVar11;

	if ( ( nodeVar12 < nodeVar1 ) ) {

		nodeVar1 = nodeVar12;
		nodeVar2 = nodeVar10;
		

	}


	if ( ( nodeVar12 > nodeVar3 ) ) {

		nodeVar3 = nodeVar12;
		

	}

	nodeVar13 = ( nodeConst0 + vec2<f32>( 0.0, -1.0 ) );
	nodeVar14 = textureLoad( nodeUniform2, vec2<i32>( nodeVar13 ), u32( 0u ) );
	nodeVar15 = nodeVar14;

	if ( ( nodeVar15 < nodeVar1 ) ) {

		nodeVar1 = nodeVar15;
		nodeVar2 = nodeVar13;
		

	}


	if ( ( nodeVar15 > nodeVar3 ) ) {

		nodeVar3 = nodeVar15;
		

	}

	nodeVar16 = ( nodeConst0 + vec2<f32>( 0.0, 0.0 ) );
	nodeVar17 = textureLoad( nodeUniform2, vec2<i32>( nodeVar16 ), u32( 0u ) );
	nodeVar18 = nodeVar17;

	if ( ( nodeVar18 < nodeVar1 ) ) {

		nodeVar1 = nodeVar18;
		nodeVar2 = nodeVar16;
		

	}


	if ( ( nodeVar18 > nodeVar3 ) ) {

		nodeVar3 = nodeVar18;
		

	}

	nodeVar19 = ( nodeConst0 + vec2<f32>( 0.0, 1.0 ) );
	nodeVar20 = textureLoad( nodeUniform2, vec2<i32>( nodeVar19 ), u32( 0u ) );
	nodeVar21 = nodeVar20;

	if ( ( nodeVar21 < nodeVar1 ) ) {

		nodeVar1 = nodeVar21;
		nodeVar2 = nodeVar19;
		

	}


	if ( ( nodeVar21 > nodeVar3 ) ) {

		nodeVar3 = nodeVar21;
		

	}

	nodeVar22 = ( nodeConst0 + vec2<f32>( 1.0, -1.0 ) );
	nodeVar23 = textureLoad( nodeUniform2, vec2<i32>( nodeVar22 ), u32( 0u ) );
	nodeVar24 = nodeVar23;

	if ( ( nodeVar24 < nodeVar1 ) ) {

		nodeVar1 = nodeVar24;
		nodeVar2 = nodeVar22;
		

	}


	if ( ( nodeVar24 > nodeVar3 ) ) {

		nodeVar3 = nodeVar24;
		

	}

	nodeVar25 = ( nodeConst0 + vec2<f32>( 1.0, 0.0 ) );
	nodeVar26 = textureLoad( nodeUniform2, vec2<i32>( nodeVar25 ), u32( 0u ) );
	nodeVar27 = nodeVar26;

	if ( ( nodeVar27 < nodeVar1 ) ) {

		nodeVar1 = nodeVar27;
		nodeVar2 = nodeVar25;
		

	}


	if ( ( nodeVar27 > nodeVar3 ) ) {

		nodeVar3 = nodeVar27;
		

	}

	nodeVar28 = ( nodeConst0 + vec2<f32>( 1.0, 1.0 ) );
	nodeVar29 = textureLoad( nodeUniform2, vec2<i32>( nodeVar28 ), u32( 0u ) );
	nodeVar30 = nodeVar29;

	if ( ( nodeVar30 < nodeVar1 ) ) {

		nodeVar1 = nodeVar30;
		nodeVar2 = nodeVar28;
		

	}


	if ( ( nodeVar30 > nodeVar3 ) ) {

		nodeVar3 = nodeVar30;
		

	}

	nodeVar31 = StructType0( nodeVar1, nodeVar2, nodeVar3 );
	nodeVar32 = textureLoad( nodeUniform0, vec2<i32>( nodeVar31.closestPositionTexel ), u32( 0u ) );
	let nodeConst1 = ( nodeVar32.xy * vec2<f32>( 0.5, -0.5 ) );
	let nodeConst2 = subpixelCorrection( nodeConst1, vec2<i32>( textureDimensions( nodeUniform1, 0 ) ) );
	nodeVar0 = ( nodeVar0 + ( nodeConst2 * 0.25 ) );
	let nodeConst3 = ( nodeVarying0 - nodeConst1 );
	nodeVar35 = textureDimensions( nodeUniform7, u32( 0 ) );
	nodeVar34 = textureLoad( nodeUniform7, vec2<u32>( clamp( floor( tsl_coord_clampS_clampT_2d( ( object.nodeUniform8 * vec3<f32>( nodeConst3, 1.0 ) ).xy ) * vec2<f32>( nodeVar35 ) ), vec2<f32>( 0 ), vec2<f32>( nodeVar35 - vec2<u32>( 1, 1 ) ) ) ), u32( 0 ) );
	let nodeConst4 = ( object.nodeUniform6 * vec4<f32>( vec3<f32>( ( ( vec2<f32>( nodeConst3.x, ( 1.0 - nodeConst3.y ) ) * vec2<f32>( 2.0 ) ) - vec2<f32>( 1.0 ) ), nodeVar34 ), 1.0 ) );

	if ( ( ( all( ( nodeConst3 >= vec2<f32>( 0.0 ) ) ) && all( ( nodeConst3 <= vec2<f32>( 1.0 ) ) ) ) && ( ( ( nodeVar31.farthestDepth - nodeVar31.closestDepth ) > 0.001 ) || ( ! ( ( nodeVar31.closestDepth - ( ( ( object.nodeUniform3.x + ( object.nodeUniform4 * vec4<f32>( ( object.nodeUniform5 * vec4<f32>( ( nodeConst4.xyz / vec3<f32>( nodeConst4.w ) ), 1.0 ) ).xyz, 1.0 ) ).z ) * object.nodeUniform3.y ) / ( ( object.nodeUniform3.y - object.nodeUniform3.x ) * ( object.nodeUniform4 * vec4<f32>( ( object.nodeUniform5 * vec4<f32>( ( nodeConst4.xyz / vec3<f32>( nodeConst4.w ) ), 1.0 ) ).xyz, 1.0 ) ).z ) ) ) > 0.0005 ) ) ) ) ) {

		nodeVar33 = clamp( ( nodeVar0 + clamp( ( length( ( ( nodeVarying0 - nodeConst3 ) * vec2<f32>( textureDimensions( nodeUniform1, 0 ) ) ) ) / 128.0 ), 0.0, 1.0 ) ), 0.0, 1.0 );

	} else {

		nodeVar33 = 1.0;

	}

	nodeVar0 = nodeVar33;
	nodeVar36 = textureSample( nodeUniform1, nodeUniform1_sampler, nodeVarying0 );
	nodeVar37 = nodeVar36;
	nodeVar38 = ( nodeVar36 * nodeVar36 );
	nodeVar39 = textureLoad( nodeUniform1, vec2<i32>( nodeConst0 ) + vec2<i32>( -1, -1 ), u32( 0u ) );
	let nodeConst5 = max( nodeVar39, vec4<f32>( 0.0 ) );
	nodeVar37 = ( nodeVar37 + nodeConst5 );
	nodeVar38 = ( nodeVar38 + ( nodeConst5 * nodeConst5 ) );
	nodeVar40 = textureLoad( nodeUniform1, vec2<i32>( nodeConst0 ) + vec2<i32>( -1, 1 ), u32( 0u ) );
	let nodeConst6 = max( nodeVar40, vec4<f32>( 0.0 ) );
	nodeVar37 = ( nodeVar37 + nodeConst6 );
	nodeVar38 = ( nodeVar38 + ( nodeConst6 * nodeConst6 ) );
	nodeVar41 = textureLoad( nodeUniform1, vec2<i32>( nodeConst0 ) + vec2<i32>( 1, -1 ), u32( 0u ) );
	let nodeConst7 = max( nodeVar41, vec4<f32>( 0.0 ) );
	nodeVar37 = ( nodeVar37 + nodeConst7 );
	nodeVar38 = ( nodeVar38 + ( nodeConst7 * nodeConst7 ) );
	nodeVar42 = textureLoad( nodeUniform1, vec2<i32>( nodeConst0 ) + vec2<i32>( 1, 1 ), u32( 0u ) );
	let nodeConst8 = max( nodeVar42, vec4<f32>( 0.0 ) );
	nodeVar37 = ( nodeVar37 + nodeConst8 );
	nodeVar38 = ( nodeVar38 + ( nodeConst8 * nodeConst8 ) );
	nodeVar43 = textureLoad( nodeUniform1, vec2<i32>( nodeConst0 ) + vec2<i32>( 1, 0 ), u32( 0u ) );
	let nodeConst9 = max( nodeVar43, vec4<f32>( 0.0 ) );
	nodeVar37 = ( nodeVar37 + nodeConst9 );
	nodeVar38 = ( nodeVar38 + ( nodeConst9 * nodeConst9 ) );
	nodeVar44 = textureLoad( nodeUniform1, vec2<i32>( nodeConst0 ) + vec2<i32>( 0, -1 ), u32( 0u ) );
	let nodeConst10 = max( nodeVar44, vec4<f32>( 0.0 ) );
	nodeVar37 = ( nodeVar37 + nodeConst10 );
	nodeVar38 = ( nodeVar38 + ( nodeConst10 * nodeConst10 ) );
	nodeVar45 = textureLoad( nodeUniform1, vec2<i32>( nodeConst0 ) + vec2<i32>( 0, 1 ), u32( 0u ) );
	let nodeConst11 = max( nodeVar45, vec4<f32>( 0.0 ) );
	nodeVar37 = ( nodeVar37 + nodeConst11 );
	nodeVar38 = ( nodeVar38 + ( nodeConst11 * nodeConst11 ) );
	nodeVar46 = textureLoad( nodeUniform1, vec2<i32>( nodeConst0 ) + vec2<i32>( -1, 0 ), u32( 0u ) );
	let nodeConst12 = max( nodeVar46, vec4<f32>( 0.0 ) );
	nodeVar37 = ( nodeVar37 + nodeConst12 );
	nodeVar38 = ( nodeVar38 + ( nodeConst12 * nodeConst12 ) );
	let nodeConst13 = ( nodeVar37 / vec4<f32>( 9.0 ) );
	let nodeConst14 = ( 1.0 - clamp( ( length( ( ( nodeVarying0 - nodeConst3 ) * vec2<f32>( textureDimensions( nodeUniform1, 0 ) ) ) ) / 128.0 ), 0.0, 1.0 ) );
	let nodeConst15 = ( sqrt( max( ( ( nodeVar38 / vec4<f32>( 9.0 ) ) - ( nodeConst13 * nodeConst13 ) ), vec4<f32>( 0.0 ) ) ) * vec4<f32>( mix( 0.5, 1.0, ( nodeConst14 * nodeConst14 ) ) ) );
	let nodeConst16 = ( nodeConst13 - nodeConst15 );
	let nodeConst17 = ( nodeConst13 + nodeConst15 );
	nodeVar47 = textureSample( nodeUniform9, nodeUniform9_sampler, ( object.nodeUniform10 * vec3<f32>( nodeConst3, 1.0 ) ).xy );
	DiffuseColor = flickerReduction( nodeVar36, clipAABB( clamp( nodeConst13, nodeConst16, nodeConst17 ), nodeVar47, nodeConst16, nodeConst17 ), nodeVar0 );
	DiffuseColor.w = ( DiffuseColor.w * object.nodeUniform11 );
	DiffuseColor.w = 1.0;
	let nodeConst18 = max( vec4<f32>( DiffuseColor.xyz, DiffuseColor.w ), vec4<f32>( 0.0 ) );
	Output = nodeConst18;

	// result

	output.color = nodeConst18;

	return output;

}
