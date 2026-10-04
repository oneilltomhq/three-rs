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

struct OutputType {
	@location( 0 ) m0 : vec4<f32>,
	@location( 1 ) m1 : vec4<f32>,
	
};
var<private> output : OutputType;

// uniforms
@binding( 0 ) @group( 0 ) var nodeUniform0_sampler : sampler;
@binding( 1 ) @group( 0 ) var nodeUniform0 : texture_2d<f32>;
@binding( 3 ) @group( 0 ) var nodeUniform2_sampler : sampler;
@binding( 4 ) @group( 0 ) var nodeUniform2 : texture_2d<f32>;
@binding( 5 ) @group( 0 ) var nodeUniform3 : texture_depth_2d;
@binding( 6 ) @group( 0 ) var nodeUniform8 : texture_depth_2d;
@binding( 7 ) @group( 0 ) var nodeUniform10_sampler : sampler;
@binding( 8 ) @group( 0 ) var nodeUniform10 : texture_2d<f32>;
@binding( 9 ) @group( 0 ) var nodeUniform12_sampler : sampler;
@binding( 10 ) @group( 0 ) var nodeUniform12 : texture_2d<f32>;

struct objectStruct {
	nodeUniform1 : vec2<f32>,
	nodeUniform4 : vec2<f32>,
	nodeUniform5 : mat4x4<f32>,
	nodeUniform6 : mat4x4<f32>,
	nodeUniform7 : mat4x4<f32>,
	nodeUniform9 : mat3x3<f32>,
	nodeUniform11 : mat3x3<f32>,
	nodeUniform13 : mat3x3<f32>,
	nodeUniform14 : f32
};
@binding( 2 ) @group( 0 )
var<uniform> object : objectStruct;

// vars
var<private> DiffuseColor : vec4<f32>;
var<private> nodeVar0 : vec4<f32>;
var<private> nodeVar1 : f32;
var<private> nodeVar2 : vec4<f32>;
var<private> nodeVar3 : vec4<f32>;
var<private> nodeVar4 : vec4<f32>;
var<private> nodeVar5 : vec4<f32>;
var<private> nodeVar6 : vec4<f32>;
var<private> nodeVar7 : vec4<f32>;
var<private> nodeVar8 : vec4<f32>;
var<private> nodeVar9 : vec4<f32>;
var<private> nodeVar10 : vec4<f32>;
var<private> nodeVar11 : vec4<f32>;
var<private> nodeVar12 : vec4<f32>;
var<private> nodeVar13 : f32;
var<private> nodeVar14 : f32;
var<private> nodeVar15 : f32;
var<private> nodeVar16 : vec2<f32>;
var<private> nodeVar17 : f32;
var<private> nodeVar18 : vec2<f32>;
var<private> nodeVar19 : f32;
var<private> nodeVar20 : f32;
var<private> nodeVar21 : vec2<f32>;
var<private> nodeVar22 : f32;
var<private> nodeVar23 : f32;
var<private> nodeVar24 : vec2<f32>;
var<private> nodeVar25 : f32;
var<private> nodeVar26 : f32;
var<private> nodeVar27 : vec2<f32>;
var<private> nodeVar28 : f32;
var<private> nodeVar29 : f32;
var<private> nodeVar30 : vec2<f32>;
var<private> nodeVar31 : f32;
var<private> nodeVar32 : f32;
var<private> nodeVar33 : vec2<f32>;
var<private> nodeVar34 : f32;
var<private> nodeVar35 : f32;
var<private> nodeVar36 : vec2<f32>;
var<private> nodeVar37 : f32;
var<private> nodeVar38 : f32;
var<private> nodeVar39 : vec2<f32>;
var<private> nodeVar40 : f32;
var<private> nodeVar41 : f32;
var<private> nodeVar42 : vec2<f32>;
var<private> nodeVar43 : f32;
var<private> nodeVar44 : f32;
var<private> nodeVar45 : StructType0;
var<private> nodeVar46 : vec4<f32>;
var<private> nodeVar47 : f32;
var<private> nodeVar48 : vec2<u32>;
var<private> nodeVar49 : vec4<f32>;
var<private> nodeVar50 : vec4<f32>;
var<private> nodeVar51 : f32;
var<private> nodeVar52 : vec4<f32>;
var<private> nodeVar53 : f32;
var<private> nodeVar54 : vec4<f32>;
var<private> Output : vec4<f32>;

// codes
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
fn main( @location( 0 ) nodeVarying0 : vec2<f32> ) -> OutputType {

	// flow
	// code

	nodeVar0 = vec4<f32>( 0.0, 0.0, 0.0, 0.0 );
	nodeVar1 = 0.0;
	nodeVar2 = vec4<f32>( 0.0, 0.0, 0.0, 0.0 );
	nodeVar3 = vec4<f32>( 0.0, 0.0, 0.0, 0.0 );
	let nodeConst0 = vec2<f32>( textureDimensions( nodeUniform0, 0 ) );
	let nodeConst1 = ( nodeVarying0 * nodeConst0 );
	let nodeConst2 = round( ( nodeConst1 - ( vec2<f32>( 0.5, 0.5 ) + object.nodeUniform1 ) ) );
	let nodeConst3 = vec2<i32>( nodeConst2 );
	let nodeConst4 = ( nodeConst3 + vec2<i32>( -1, -1 ) );
	nodeVar4 = textureLoad( nodeUniform0, nodeConst4, u32( 0u ) );
	let nodeConst5 = max( nodeVar4, vec4<f32>( 0.0 ) );
	let nodeConst6 = ( nodeConst1 - ( vec2<f32>( nodeConst4 ) + ( vec2<f32>( 0.5, 0.5 ) + object.nodeUniform1 ) ) );
	let nodeConst7 = exp( ( dot( nodeConst6, nodeConst6 ) * -2.29 ) );
	nodeVar0 = ( nodeVar0 + ( nodeConst5 * vec4<f32>( nodeConst7 ) ) );
	nodeVar1 = ( nodeVar1 + nodeConst7 );
	nodeVar2 = ( nodeVar2 + nodeConst5 );
	nodeVar3 = ( nodeVar3 + ( nodeConst5 * nodeConst5 ) );
	let nodeConst8 = ( nodeConst3 + vec2<i32>( 0, -1 ) );
	nodeVar5 = textureLoad( nodeUniform0, nodeConst8, u32( 0u ) );
	let nodeConst9 = max( nodeVar5, vec4<f32>( 0.0 ) );
	let nodeConst10 = ( nodeConst1 - ( vec2<f32>( nodeConst8 ) + ( vec2<f32>( 0.5, 0.5 ) + object.nodeUniform1 ) ) );
	let nodeConst11 = exp( ( dot( nodeConst10, nodeConst10 ) * -2.29 ) );
	nodeVar0 = ( nodeVar0 + ( nodeConst9 * vec4<f32>( nodeConst11 ) ) );
	nodeVar1 = ( nodeVar1 + nodeConst11 );
	nodeVar2 = ( nodeVar2 + nodeConst9 );
	nodeVar3 = ( nodeVar3 + ( nodeConst9 * nodeConst9 ) );
	let nodeConst12 = ( nodeConst3 + vec2<i32>( 1, -1 ) );
	nodeVar6 = textureLoad( nodeUniform0, nodeConst12, u32( 0u ) );
	let nodeConst13 = max( nodeVar6, vec4<f32>( 0.0 ) );
	let nodeConst14 = ( nodeConst1 - ( vec2<f32>( nodeConst12 ) + ( vec2<f32>( 0.5, 0.5 ) + object.nodeUniform1 ) ) );
	let nodeConst15 = exp( ( dot( nodeConst14, nodeConst14 ) * -2.29 ) );
	nodeVar0 = ( nodeVar0 + ( nodeConst13 * vec4<f32>( nodeConst15 ) ) );
	nodeVar1 = ( nodeVar1 + nodeConst15 );
	nodeVar2 = ( nodeVar2 + nodeConst13 );
	nodeVar3 = ( nodeVar3 + ( nodeConst13 * nodeConst13 ) );
	let nodeConst16 = ( nodeConst3 + vec2<i32>( -1, 0 ) );
	nodeVar7 = textureLoad( nodeUniform0, nodeConst16, u32( 0u ) );
	let nodeConst17 = max( nodeVar7, vec4<f32>( 0.0 ) );
	let nodeConst18 = ( nodeConst1 - ( vec2<f32>( nodeConst16 ) + ( vec2<f32>( 0.5, 0.5 ) + object.nodeUniform1 ) ) );
	let nodeConst19 = exp( ( dot( nodeConst18, nodeConst18 ) * -2.29 ) );
	nodeVar0 = ( nodeVar0 + ( nodeConst17 * vec4<f32>( nodeConst19 ) ) );
	nodeVar1 = ( nodeVar1 + nodeConst19 );
	nodeVar2 = ( nodeVar2 + nodeConst17 );
	nodeVar3 = ( nodeVar3 + ( nodeConst17 * nodeConst17 ) );
	let nodeConst20 = ( nodeConst3 + vec2<i32>( 0, 0 ) );
	nodeVar8 = textureLoad( nodeUniform0, nodeConst20, u32( 0u ) );
	let nodeConst21 = max( nodeVar8, vec4<f32>( 0.0 ) );
	let nodeConst22 = ( nodeConst1 - ( vec2<f32>( nodeConst20 ) + ( vec2<f32>( 0.5, 0.5 ) + object.nodeUniform1 ) ) );
	let nodeConst23 = exp( ( dot( nodeConst22, nodeConst22 ) * -2.29 ) );
	nodeVar0 = ( nodeVar0 + ( nodeConst21 * vec4<f32>( nodeConst23 ) ) );
	nodeVar1 = ( nodeVar1 + nodeConst23 );
	nodeVar2 = ( nodeVar2 + nodeConst21 );
	nodeVar3 = ( nodeVar3 + ( nodeConst21 * nodeConst21 ) );
	let nodeConst24 = ( nodeConst3 + vec2<i32>( 1, 0 ) );
	nodeVar9 = textureLoad( nodeUniform0, nodeConst24, u32( 0u ) );
	let nodeConst25 = max( nodeVar9, vec4<f32>( 0.0 ) );
	let nodeConst26 = ( nodeConst1 - ( vec2<f32>( nodeConst24 ) + ( vec2<f32>( 0.5, 0.5 ) + object.nodeUniform1 ) ) );
	let nodeConst27 = exp( ( dot( nodeConst26, nodeConst26 ) * -2.29 ) );
	nodeVar0 = ( nodeVar0 + ( nodeConst25 * vec4<f32>( nodeConst27 ) ) );
	nodeVar1 = ( nodeVar1 + nodeConst27 );
	nodeVar2 = ( nodeVar2 + nodeConst25 );
	nodeVar3 = ( nodeVar3 + ( nodeConst25 * nodeConst25 ) );
	let nodeConst28 = ( nodeConst3 + vec2<i32>( -1, 1 ) );
	nodeVar10 = textureLoad( nodeUniform0, nodeConst28, u32( 0u ) );
	let nodeConst29 = max( nodeVar10, vec4<f32>( 0.0 ) );
	let nodeConst30 = ( nodeConst1 - ( vec2<f32>( nodeConst28 ) + ( vec2<f32>( 0.5, 0.5 ) + object.nodeUniform1 ) ) );
	let nodeConst31 = exp( ( dot( nodeConst30, nodeConst30 ) * -2.29 ) );
	nodeVar0 = ( nodeVar0 + ( nodeConst29 * vec4<f32>( nodeConst31 ) ) );
	nodeVar1 = ( nodeVar1 + nodeConst31 );
	nodeVar2 = ( nodeVar2 + nodeConst29 );
	nodeVar3 = ( nodeVar3 + ( nodeConst29 * nodeConst29 ) );
	let nodeConst32 = ( nodeConst3 + vec2<i32>( 0, 1 ) );
	nodeVar11 = textureLoad( nodeUniform0, nodeConst32, u32( 0u ) );
	let nodeConst33 = max( nodeVar11, vec4<f32>( 0.0 ) );
	let nodeConst34 = ( nodeConst1 - ( vec2<f32>( nodeConst32 ) + ( vec2<f32>( 0.5, 0.5 ) + object.nodeUniform1 ) ) );
	let nodeConst35 = exp( ( dot( nodeConst34, nodeConst34 ) * -2.29 ) );
	nodeVar0 = ( nodeVar0 + ( nodeConst33 * vec4<f32>( nodeConst35 ) ) );
	nodeVar1 = ( nodeVar1 + nodeConst35 );
	nodeVar2 = ( nodeVar2 + nodeConst33 );
	nodeVar3 = ( nodeVar3 + ( nodeConst33 * nodeConst33 ) );
	let nodeConst36 = ( nodeConst3 + vec2<i32>( 1, 1 ) );
	nodeVar12 = textureLoad( nodeUniform0, nodeConst36, u32( 0u ) );
	let nodeConst37 = max( nodeVar12, vec4<f32>( 0.0 ) );
	let nodeConst38 = ( nodeConst1 - ( vec2<f32>( nodeConst36 ) + ( vec2<f32>( 0.5, 0.5 ) + object.nodeUniform1 ) ) );
	let nodeConst39 = exp( ( dot( nodeConst38, nodeConst38 ) * -2.29 ) );
	nodeVar0 = ( nodeVar0 + ( nodeConst37 * vec4<f32>( nodeConst39 ) ) );
	nodeVar1 = ( nodeVar1 + nodeConst39 );
	nodeVar2 = ( nodeVar2 + nodeConst37 );
	nodeVar3 = ( nodeVar3 + ( nodeConst37 * nodeConst37 ) );
	let nodeConst40 = ( nodeVar2 / vec4<f32>( 9.0 ) );
	let nodeConst41 = dot( nodeConst40.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) );
	nodeVar13 = 0.025;
	nodeVar15 = 2.0;
	nodeVar16 = vec2<f32>( 0.0, 0.0 );
	nodeVar17 = -1.0;
	nodeVar18 = ( nodeConst2 + vec2<f32>( -1.0, -1.0 ) );
	nodeVar19 = textureLoad( nodeUniform3, vec2<i32>( nodeVar18 ), u32( 0u ) );
	nodeVar20 = nodeVar19;

	if ( ( nodeVar20 < nodeVar15 ) ) {

		nodeVar15 = nodeVar20;
		nodeVar16 = nodeVar18;
		

	}


	if ( ( nodeVar20 > nodeVar17 ) ) {

		nodeVar17 = nodeVar20;
		

	}

	nodeVar21 = ( nodeConst2 + vec2<f32>( -1.0, 0.0 ) );
	nodeVar22 = textureLoad( nodeUniform3, vec2<i32>( nodeVar21 ), u32( 0u ) );
	nodeVar23 = nodeVar22;

	if ( ( nodeVar23 < nodeVar15 ) ) {

		nodeVar15 = nodeVar23;
		nodeVar16 = nodeVar21;
		

	}


	if ( ( nodeVar23 > nodeVar17 ) ) {

		nodeVar17 = nodeVar23;
		

	}

	nodeVar24 = ( nodeConst2 + vec2<f32>( -1.0, 1.0 ) );
	nodeVar25 = textureLoad( nodeUniform3, vec2<i32>( nodeVar24 ), u32( 0u ) );
	nodeVar26 = nodeVar25;

	if ( ( nodeVar26 < nodeVar15 ) ) {

		nodeVar15 = nodeVar26;
		nodeVar16 = nodeVar24;
		

	}


	if ( ( nodeVar26 > nodeVar17 ) ) {

		nodeVar17 = nodeVar26;
		

	}

	nodeVar27 = ( nodeConst2 + vec2<f32>( 0.0, -1.0 ) );
	nodeVar28 = textureLoad( nodeUniform3, vec2<i32>( nodeVar27 ), u32( 0u ) );
	nodeVar29 = nodeVar28;

	if ( ( nodeVar29 < nodeVar15 ) ) {

		nodeVar15 = nodeVar29;
		nodeVar16 = nodeVar27;
		

	}


	if ( ( nodeVar29 > nodeVar17 ) ) {

		nodeVar17 = nodeVar29;
		

	}

	nodeVar30 = ( nodeConst2 + vec2<f32>( 0.0, 0.0 ) );
	nodeVar31 = textureLoad( nodeUniform3, vec2<i32>( nodeVar30 ), u32( 0u ) );
	nodeVar32 = nodeVar31;

	if ( ( nodeVar32 < nodeVar15 ) ) {

		nodeVar15 = nodeVar32;
		nodeVar16 = nodeVar30;
		

	}


	if ( ( nodeVar32 > nodeVar17 ) ) {

		nodeVar17 = nodeVar32;
		

	}

	nodeVar33 = ( nodeConst2 + vec2<f32>( 0.0, 1.0 ) );
	nodeVar34 = textureLoad( nodeUniform3, vec2<i32>( nodeVar33 ), u32( 0u ) );
	nodeVar35 = nodeVar34;

	if ( ( nodeVar35 < nodeVar15 ) ) {

		nodeVar15 = nodeVar35;
		nodeVar16 = nodeVar33;
		

	}


	if ( ( nodeVar35 > nodeVar17 ) ) {

		nodeVar17 = nodeVar35;
		

	}

	nodeVar36 = ( nodeConst2 + vec2<f32>( 1.0, -1.0 ) );
	nodeVar37 = textureLoad( nodeUniform3, vec2<i32>( nodeVar36 ), u32( 0u ) );
	nodeVar38 = nodeVar37;

	if ( ( nodeVar38 < nodeVar15 ) ) {

		nodeVar15 = nodeVar38;
		nodeVar16 = nodeVar36;
		

	}


	if ( ( nodeVar38 > nodeVar17 ) ) {

		nodeVar17 = nodeVar38;
		

	}

	nodeVar39 = ( nodeConst2 + vec2<f32>( 1.0, 0.0 ) );
	nodeVar40 = textureLoad( nodeUniform3, vec2<i32>( nodeVar39 ), u32( 0u ) );
	nodeVar41 = nodeVar40;

	if ( ( nodeVar41 < nodeVar15 ) ) {

		nodeVar15 = nodeVar41;
		nodeVar16 = nodeVar39;
		

	}


	if ( ( nodeVar41 > nodeVar17 ) ) {

		nodeVar17 = nodeVar41;
		

	}

	nodeVar42 = ( nodeConst2 + vec2<f32>( 1.0, 1.0 ) );
	nodeVar43 = textureLoad( nodeUniform3, vec2<i32>( nodeVar42 ), u32( 0u ) );
	nodeVar44 = nodeVar43;

	if ( ( nodeVar44 < nodeVar15 ) ) {

		nodeVar15 = nodeVar44;
		nodeVar16 = nodeVar42;
		

	}


	if ( ( nodeVar44 > nodeVar17 ) ) {

		nodeVar17 = nodeVar44;
		

	}

	nodeVar45 = StructType0( nodeVar15, nodeVar16, nodeVar17 );
	nodeVar46 = textureLoad( nodeUniform2, vec2<i32>( nodeVar45.closestPositionTexel ), u32( 0u ) );
	let nodeConst42 = ( nodeVarying0 - ( nodeVar46.xy * vec2<f32>( 0.5, -0.5 ) ) );
	let nodeConst43 = ( all( ( nodeConst42 >= vec2<f32>( 0.0 ) ) ) && all( ( nodeConst42 <= vec2<f32>( 1.0 ) ) ) );
	nodeVar48 = textureDimensions( nodeUniform8, u32( 0 ) );
	nodeVar47 = textureLoad( nodeUniform8, vec2<u32>( clamp( floor( tsl_coord_clampS_clampT_2d( ( object.nodeUniform9 * vec3<f32>( nodeConst42, 1.0 ) ).xy ) * vec2<f32>( nodeVar48 ) ), vec2<f32>( 0 ), vec2<f32>( nodeVar48 - vec2<u32>( 1, 1 ) ) ) ), u32( 0 ) );
	let nodeConst44 = ( object.nodeUniform7 * vec4<f32>( vec3<f32>( ( ( vec2<f32>( nodeConst42.x, ( 1.0 - nodeConst42.y ) ) * vec2<f32>( 2.0 ) ) - vec2<f32>( 1.0 ) ), nodeVar47 ), 1.0 ) );
	let nodeConst45 = ( ( ( object.nodeUniform4.x + ( object.nodeUniform5 * vec4<f32>( ( object.nodeUniform6 * vec4<f32>( ( nodeConst44.xyz / vec3<f32>( nodeConst44.w ) ), 1.0 ) ).xyz, 1.0 ) ).z ) * object.nodeUniform4.y ) / ( ( object.nodeUniform4.y - object.nodeUniform4.x ) * ( object.nodeUniform5 * vec4<f32>( ( object.nodeUniform6 * vec4<f32>( ( nodeConst44.xyz / vec3<f32>( nodeConst44.w ) ), 1.0 ) ).xyz, 1.0 ) ).z ) );
	let nodeConst46 = ( ( nodeVar45.closestDepth - nodeConst45 ) > 0.0005 );

	if ( ( nodeConst43 && ( ( ( nodeVar45.farthestDepth - nodeVar45.closestDepth ) > 0.001 ) || ( ! nodeConst46 ) ) ) ) {

		nodeVar14 = clamp( ( nodeVar13 + clamp( ( length( ( ( nodeVarying0 - nodeConst42 ) * nodeConst0 ) ) / 128.0 ), 0.0, 1.0 ) ), 0.0, 1.0 );

	} else {

		nodeVar14 = 1.0;

	}

	nodeVar13 = nodeVar14;
	let nodeConst47 = ( nodeVar0 / vec4<f32>( max( nodeVar1, 0.00001 ) ) );
	let nodeConst48 = ( 1.0 - clamp( ( length( ( ( nodeVarying0 - nodeConst42 ) * nodeConst0 ) ) / 128.0 ), 0.0, 1.0 ) );
	let nodeConst49 = ( sqrt( max( ( ( nodeVar3 / vec4<f32>( 9.0 ) ) - ( nodeConst40 * nodeConst40 ) ), vec4<f32>( 0.0 ) ) ) * vec4<f32>( mix( 0.5, 1.0, ( nodeConst48 * nodeConst48 ) ) ) );
	let nodeConst50 = ( nodeConst40 - nodeConst49 );
	let nodeConst51 = ( nodeConst40 + nodeConst49 );
	nodeVar50 = textureSample( nodeUniform10, nodeUniform10_sampler, ( object.nodeUniform11 * vec3<f32>( nodeConst42, 1.0 ) ).xy );

	if ( ( nodeConst43 && ( ! ( abs( ( nodeVar45.closestDepth - nodeConst45 ) ) > 0.0005 ) ) ) ) {

		nodeVar51 = smoothstep( 0.0, 0.2, ( abs( ( dot( nodeConst47.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) ) - nodeConst41 ) ) / nodeConst41 ) );

	} else {

		nodeVar51 = 0.0;

	}

	nodeVar52 = textureSample( nodeUniform12, nodeUniform12_sampler, ( object.nodeUniform13 * vec3<f32>( nodeVarying0, 1.0 ) ).xy );

	if ( nodeConst46 ) {

		nodeVar53 = 0.0;

	} else {

		nodeVar53 = 0.5;

	}

	let nodeConst52 = clamp( max( nodeVar51, ( nodeVar52.x * nodeVar53 ) ), 0.0, 1.0 );
	nodeVar49 = flickerReduction( nodeConst47, mix( clipAABB( clamp( nodeConst40, nodeConst50, nodeConst51 ), nodeVar50, nodeConst50, nodeConst51 ), nodeVar50, nodeConst52 ), nodeVar13 );
	nodeVar54 = vec4<f32>( nodeConst52 );
	DiffuseColor = vec4<f32>( 0.0, 0.0, 0.0, 0.0 );
	DiffuseColor.w = ( DiffuseColor.w * object.nodeUniform14 );
	DiffuseColor.w = 1.0;
	Output = max( vec4<f32>( DiffuseColor.xyz, DiffuseColor.w ), vec4<f32>( 0.0 ) );
	output.m0 = nodeVar49;
	output.m1 = nodeVar54;

	// result

	return output;

}
