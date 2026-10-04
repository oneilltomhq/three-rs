// Three.js r187dev - Node System

// global
diagnostic( off, derivative_uniformity );


// directives


// structs

struct StructType0 {
	color : vec4<f32>,
	weight : f32,
	confidence : f32
};

struct StructType1 {
	color : vec4<f32>,
	tapConfidence : f32,
	minConfidence : f32
};

struct StructType2 {
	mean : vec3<f32>,
	stdColor : vec3<f32>,
	rayLength : f32,
	envProbability : f32,
	stdDevRayLength : f32
};

struct OutputStruct {
	@location( 0 ) color: vec4<f32>
};
var<private> output : OutputStruct;

// uniforms
@binding( 0 ) @group( 0 ) var nodeUniform0 : texture_depth_2d;
@binding( 1 ) @group( 0 ) var nodeUniform1_sampler : sampler;
@binding( 2 ) @group( 0 ) var nodeUniform1 : texture_2d<f32>;
@binding( 4 ) @group( 0 ) var nodeUniform3_sampler : sampler;
@binding( 5 ) @group( 0 ) var nodeUniform3 : texture_2d<f32>;
@binding( 6 ) @group( 0 ) var nodeUniform8_sampler : sampler;
@binding( 7 ) @group( 0 ) var nodeUniform8 : texture_2d<f32>;
@binding( 8 ) @group( 0 ) var nodeUniform11 : texture_depth_2d;
@binding( 9 ) @group( 0 ) var nodeUniform13_sampler : sampler;
@binding( 10 ) @group( 0 ) var nodeUniform13 : texture_2d<f32>;
@binding( 11 ) @group( 0 ) var nodeUniform15_sampler : sampler;
@binding( 12 ) @group( 0 ) var nodeUniform15 : texture_2d<f32>;

struct objectStruct {
	nodeUniform2 : vec2<f32>,
	nodeUniform4 : f32,
	nodeUniform5 : mat4x4<f32>,
	nodeUniform6 : mat4x4<f32>,
	nodeUniform7 : mat4x4<f32>,
	nodeUniform9 : mat4x4<f32>,
	nodeUniform10 : mat4x4<f32>,
	nodeUniform12 : mat3x3<f32>,
	nodeUniform14 : mat3x3<f32>,
	nodeUniform16 : mat3x3<f32>,
	nodeUniform17 : mat4x4<f32>,
	nodeUniform18 : vec3<f32>,
	nodeUniform19 : mat4x4<f32>,
	nodeUniform20 : u32,
	nodeUniform21 : f32,
	nodeUniform22 : f32
};
@binding( 3 ) @group( 0 )
var<uniform> object : objectStruct;

// vars
var<private> nodeVar0 : f32;
var<private> nodeVar1 : f32;
var<private> nodeVar2 : vec4<f32>;
var<private> nodeVar3 : vec4<f32>;
var<private> nodeVar4 : vec4<f32>;
var<private> nodeVar5 : vec3<f32>;
var<private> nodeVar6 : vec3<f32>;
var<private> nodeVar7 : vec3<f32>;
var<private> nodeVar8 : f32;
var<private> nodeVar9 : f32;
var<private> nodeVar10 : f32;
var<private> nodeVar11 : f32;
var<private> nodeVar12 : f32;
var<private> nodeVar13 : vec4<f32>;
var<private> nodeVar14 : vec4<f32>;
var<private> nodeVar15 : f32;
var<private> nodeVar16 : vec4<f32>;
var<private> nodeVar17 : vec4<f32>;
var<private> nodeVar18 : f32;
var<private> nodeVar19 : vec4<f32>;
var<private> nodeVar20 : vec4<f32>;
var<private> nodeVar21 : f32;
var<private> nodeVar22 : vec4<f32>;
var<private> nodeVar23 : vec4<f32>;
var<private> nodeVar24 : f32;
var<private> nodeVar25 : vec4<f32>;
var<private> nodeVar26 : vec4<f32>;
var<private> nodeVar27 : f32;
var<private> nodeVar28 : vec4<f32>;
var<private> nodeVar29 : vec4<f32>;
var<private> nodeVar30 : f32;
var<private> nodeVar31 : vec4<f32>;
var<private> nodeVar32 : vec4<f32>;
var<private> nodeVar33 : f32;
var<private> nodeVar34 : vec4<f32>;
var<private> nodeVar35 : vec4<f32>;
var<private> nodeVar36 : f32;
var<private> nodeVar37 : StructType2;
var<private> nodeVar38 : f32;
var<private> nodeVar40 : vec3<f32>;
var<private> nodeVar41 : vec3<f32>;
var<private> nodeVar42 : vec3<f32>;
var<private> nodeVar43 : vec4<f32>;
var<private> nodeVar44 : vec2<f32>;
var<private> nodeVar45 : vec2<f32>;
var<private> nodeVar46 : vec2<f32>;
var<private> nodeVar47 : StructType1;
var<private> nodeVar48 : vec4<f32>;
var<private> nodeVar49 : f32;
var<private> nodeVar50 : f32;
var<private> nodeVar51 : StructType0;
var<private> nodeVar52 : vec4<f32>;
var<private> nodeVar53 : vec4<f32>;
var<private> nodeVar54 : f32;
var<private> nodeVar55 : f32;
var<private> nodeVar56 : StructType0;
var<private> nodeVar57 : vec4<f32>;
var<private> nodeVar58 : vec4<f32>;
var<private> nodeVar59 : f32;
var<private> nodeVar60 : f32;
var<private> nodeVar61 : StructType0;
var<private> nodeVar62 : vec4<f32>;
var<private> nodeVar63 : vec4<f32>;
var<private> nodeVar64 : f32;
var<private> nodeVar65 : f32;
var<private> nodeVar66 : StructType0;
var<private> nodeVar67 : vec4<f32>;
var<private> nodeVar68 : vec4<f32>;
var<private> nodeVar69 : vec4<f32>;
var<private> nodeVar70 : f32;
var<private> nodeVar71 : f32;
var<private> nodeVar72 : vec3<f32>;
var<private> nodeVar73 : vec2<f32>;
var<private> nodeVar74 : vec2<f32>;
var<private> nodeVar75 : StructType1;
var<private> nodeVar76 : vec4<f32>;
var<private> nodeVar77 : f32;
var<private> nodeVar78 : f32;
var<private> nodeVar79 : StructType0;
var<private> nodeVar80 : vec4<f32>;
var<private> nodeVar81 : vec4<f32>;
var<private> nodeVar82 : f32;
var<private> nodeVar83 : f32;
var<private> nodeVar84 : StructType0;
var<private> nodeVar85 : vec4<f32>;
var<private> nodeVar86 : vec4<f32>;
var<private> nodeVar87 : f32;
var<private> nodeVar88 : f32;
var<private> nodeVar89 : StructType0;
var<private> nodeVar90 : vec4<f32>;
var<private> nodeVar91 : vec4<f32>;
var<private> nodeVar92 : f32;
var<private> nodeVar93 : f32;
var<private> nodeVar94 : StructType0;
var<private> nodeVar95 : vec4<f32>;
var<private> nodeVar96 : vec4<f32>;
var<private> nodeVar97 : f32;
var<private> nodeVar98 : bool;
var<private> nodeVar99 : f32;
var<private> nodeVar100 : vec4<f32>;
var<private> nodeVar101 : vec2<f32>;
var<private> nodeVar102 : vec2<f32>;
var<private> nodeVar103 : vec3<f32>;
var<private> nodeVar104 : vec3<f32>;
var<private> nodeVar105 : vec3<f32>;
var<private> nodeVar106 : f32;

// codes
fn beautyTexelFromScreen ( screenTexel : vec2<i32>, beautySize : vec2<f32>, resolveSize : vec2<f32> ) -> vec2<i32> {

	


	return vec2<i32>( floor( ( ( vec2<f32>( screenTexel ) * beautySize ) / resolveSize ) ) );

}


fn velocityToUVOffset ( velocity : vec2<f32> ) -> vec2<f32> {

	


	return ( velocity * vec2<f32>( 0.5, -0.5 ) );

}


fn projectWorldToUV ( worldPos : vec3<f32>, previousViewMatrix : mat4x4<f32>, previousProjectionMatrix : mat4x4<f32> ) -> vec2<f32> {

	var nodeVar0 : vec2<f32>;
	var nodeVar1 : vec4<f32>;
	var nodeVar2 : f32;

	nodeVar0 = vec2<f32>( -1.0, -1.0 );
	nodeVar1 = ( previousProjectionMatrix * ( previousViewMatrix * vec4<f32>( worldPos, 1.0 ) ) );
	nodeVar2 = nodeVar1.w;

	if ( ( abs( nodeVar2 ) > 0.00001 ) ) {

		nodeVar0 = ( ( ( nodeVar1.xyz / vec3<f32>( nodeVar2 ) ).xy * vec2<f32>( 0.5 ) ) + vec2<f32>( 0.5 ) );
		nodeVar0.y = ( 1.0 - nodeVar0.y );
		

	}


	return nodeVar0;

}


fn clipToAABB ( history : vec3<f32>, boxMin : vec3<f32>, boxMax : vec3<f32> ) -> vec3<f32> {

	var nodeVar0 : vec3<f32>;

	let nodeConst0 = ( ( boxMax + boxMin ) * vec3<f32>( 0.5 ) );
	let nodeConst1 = ( history - nodeConst0 );
	let nodeConst2 = abs( ( nodeConst1 / ( ( ( boxMax - boxMin ) * vec3<f32>( 0.5 ) ) + vec3<f32>( 1e-7 ) ) ) );
	let nodeConst3 = max( max( nodeConst2.x, nodeConst2.y ), nodeConst2.z );

	if ( ( nodeConst3 > 1.0 ) ) {

		nodeVar0 = ( nodeConst0 + ( nodeConst1 / vec3<f32>( nodeConst3 ) ) );

	} else {

		nodeVar0 = history;

	}


	return nodeVar0;

}




@fragment
fn main( @location( 0 ) nodeVarying0 : vec2<f32>,
	@builtin( position ) fragCoord : vec4<f32> ) -> OutputStruct {

	// flow
	// code

	let nodeConst0 = vec2<i32>( floor( ( fragCoord.xy - vec2<f32>( 0.5 ) ) ) );
	nodeVar0 = textureLoad( nodeUniform0, nodeConst0, u32( 0u ) );
	nodeVar1 = nodeVar0;

	if ( ( nodeVar1 >= 1.0 ) ) {

		discard;
		

	}

	let nodeConst1 = beautyTexelFromScreen( nodeConst0, vec2<f32>( textureDimensions( nodeUniform1, 0 ) ), object.nodeUniform2 );
	nodeVar2 = textureLoad( nodeUniform1, nodeConst1, u32( 0u ) );
	nodeVar3 = max( nodeVar2, vec4<f32>( 0.0 ) );
	nodeVar4 = textureLoad( nodeUniform3, nodeConst0, u32( 0u ) );
	nodeVar5 = ( ( nodeVar4.xyz * vec3<f32>( 2.0 ) ) - vec3<f32>( 1.0 ) );
	let nodeConst2 = ( nodeVar3.xyz / vec3<f32>( ( ( ( dot( nodeVar3.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) ) * object.nodeUniform4 ) * 10.0 ) + 1.0 ) ) );
	let nodeConst3 = vec3<f32>( dot( nodeConst2, vec3<f32>( 0.25, 0.5, 0.25 ) ), dot( nodeConst2, vec3<f32>( 0.5, 0.0, -0.5 ) ), dot( nodeConst2, vec3<f32>( -0.25, 0.5, -0.25 ) ) );
	nodeVar6 = nodeConst3;
	nodeVar7 = ( nodeConst3 * nodeConst3 );
	nodeVar8 = 0.0;
	nodeVar9 = 0.0;
	nodeVar10 = 0.0;
	nodeVar11 = 0.0;

	if ( ( nodeVar3.w < 1000.0 ) ) {

		nodeVar8 = ( nodeVar8 + nodeVar3.w );
		nodeVar9 = ( nodeVar9 + 1.0 );
		nodeVar12 = ( nodeVar3.w - nodeVar10 );
		nodeVar10 = ( nodeVar10 + ( nodeVar12 / nodeVar9 ) );
		nodeVar11 = ( nodeVar11 + ( nodeVar12 * ( nodeVar3.w - nodeVar10 ) ) );
		

	}

	nodeVar13 = textureLoad( nodeUniform1, ( nodeConst1 + vec2<i32>( -1, -1 ) ), u32( 0u ) );
	nodeVar14 = max( nodeVar13, vec4<f32>( 0.0 ) );
	let nodeConst4 = ( nodeVar14.xyz / vec3<f32>( ( ( ( dot( nodeVar14.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) ) * object.nodeUniform4 ) * 10.0 ) + 1.0 ) ) );
	let nodeConst5 = vec3<f32>( dot( nodeConst4, vec3<f32>( 0.25, 0.5, 0.25 ) ), dot( nodeConst4, vec3<f32>( 0.5, 0.0, -0.5 ) ), dot( nodeConst4, vec3<f32>( -0.25, 0.5, -0.25 ) ) );
	nodeVar6 = ( nodeVar6 + nodeConst5 );
	nodeVar7 = ( nodeVar7 + ( nodeConst5 * nodeConst5 ) );

	if ( ( nodeVar14.w < 1000.0 ) ) {

		nodeVar8 = ( nodeVar8 + nodeVar14.w );
		nodeVar9 = ( nodeVar9 + 1.0 );
		nodeVar15 = ( nodeVar14.w - nodeVar10 );
		nodeVar10 = ( nodeVar10 + ( nodeVar15 / nodeVar9 ) );
		nodeVar11 = ( nodeVar11 + ( nodeVar15 * ( nodeVar14.w - nodeVar10 ) ) );
		

	}

	nodeVar16 = textureLoad( nodeUniform1, ( nodeConst1 + vec2<i32>( -1, 1 ) ), u32( 0u ) );
	nodeVar17 = max( nodeVar16, vec4<f32>( 0.0 ) );
	let nodeConst6 = ( nodeVar17.xyz / vec3<f32>( ( ( ( dot( nodeVar17.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) ) * object.nodeUniform4 ) * 10.0 ) + 1.0 ) ) );
	let nodeConst7 = vec3<f32>( dot( nodeConst6, vec3<f32>( 0.25, 0.5, 0.25 ) ), dot( nodeConst6, vec3<f32>( 0.5, 0.0, -0.5 ) ), dot( nodeConst6, vec3<f32>( -0.25, 0.5, -0.25 ) ) );
	nodeVar6 = ( nodeVar6 + nodeConst7 );
	nodeVar7 = ( nodeVar7 + ( nodeConst7 * nodeConst7 ) );

	if ( ( nodeVar17.w < 1000.0 ) ) {

		nodeVar8 = ( nodeVar8 + nodeVar17.w );
		nodeVar9 = ( nodeVar9 + 1.0 );
		nodeVar18 = ( nodeVar17.w - nodeVar10 );
		nodeVar10 = ( nodeVar10 + ( nodeVar18 / nodeVar9 ) );
		nodeVar11 = ( nodeVar11 + ( nodeVar18 * ( nodeVar17.w - nodeVar10 ) ) );
		

	}

	nodeVar19 = textureLoad( nodeUniform1, ( nodeConst1 + vec2<i32>( 1, -1 ) ), u32( 0u ) );
	nodeVar20 = max( nodeVar19, vec4<f32>( 0.0 ) );
	let nodeConst8 = ( nodeVar20.xyz / vec3<f32>( ( ( ( dot( nodeVar20.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) ) * object.nodeUniform4 ) * 10.0 ) + 1.0 ) ) );
	let nodeConst9 = vec3<f32>( dot( nodeConst8, vec3<f32>( 0.25, 0.5, 0.25 ) ), dot( nodeConst8, vec3<f32>( 0.5, 0.0, -0.5 ) ), dot( nodeConst8, vec3<f32>( -0.25, 0.5, -0.25 ) ) );
	nodeVar6 = ( nodeVar6 + nodeConst9 );
	nodeVar7 = ( nodeVar7 + ( nodeConst9 * nodeConst9 ) );

	if ( ( nodeVar20.w < 1000.0 ) ) {

		nodeVar8 = ( nodeVar8 + nodeVar20.w );
		nodeVar9 = ( nodeVar9 + 1.0 );
		nodeVar21 = ( nodeVar20.w - nodeVar10 );
		nodeVar10 = ( nodeVar10 + ( nodeVar21 / nodeVar9 ) );
		nodeVar11 = ( nodeVar11 + ( nodeVar21 * ( nodeVar20.w - nodeVar10 ) ) );
		

	}

	nodeVar22 = textureLoad( nodeUniform1, ( nodeConst1 + vec2<i32>( 1, 1 ) ), u32( 0u ) );
	nodeVar23 = max( nodeVar22, vec4<f32>( 0.0 ) );
	let nodeConst10 = ( nodeVar23.xyz / vec3<f32>( ( ( ( dot( nodeVar23.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) ) * object.nodeUniform4 ) * 10.0 ) + 1.0 ) ) );
	let nodeConst11 = vec3<f32>( dot( nodeConst10, vec3<f32>( 0.25, 0.5, 0.25 ) ), dot( nodeConst10, vec3<f32>( 0.5, 0.0, -0.5 ) ), dot( nodeConst10, vec3<f32>( -0.25, 0.5, -0.25 ) ) );
	nodeVar6 = ( nodeVar6 + nodeConst11 );
	nodeVar7 = ( nodeVar7 + ( nodeConst11 * nodeConst11 ) );

	if ( ( nodeVar23.w < 1000.0 ) ) {

		nodeVar8 = ( nodeVar8 + nodeVar23.w );
		nodeVar9 = ( nodeVar9 + 1.0 );
		nodeVar24 = ( nodeVar23.w - nodeVar10 );
		nodeVar10 = ( nodeVar10 + ( nodeVar24 / nodeVar9 ) );
		nodeVar11 = ( nodeVar11 + ( nodeVar24 * ( nodeVar23.w - nodeVar10 ) ) );
		

	}

	nodeVar25 = textureLoad( nodeUniform1, ( nodeConst1 + vec2<i32>( 1, 0 ) ), u32( 0u ) );
	nodeVar26 = max( nodeVar25, vec4<f32>( 0.0 ) );
	let nodeConst12 = ( nodeVar26.xyz / vec3<f32>( ( ( ( dot( nodeVar26.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) ) * object.nodeUniform4 ) * 10.0 ) + 1.0 ) ) );
	let nodeConst13 = vec3<f32>( dot( nodeConst12, vec3<f32>( 0.25, 0.5, 0.25 ) ), dot( nodeConst12, vec3<f32>( 0.5, 0.0, -0.5 ) ), dot( nodeConst12, vec3<f32>( -0.25, 0.5, -0.25 ) ) );
	nodeVar6 = ( nodeVar6 + nodeConst13 );
	nodeVar7 = ( nodeVar7 + ( nodeConst13 * nodeConst13 ) );

	if ( ( nodeVar26.w < 1000.0 ) ) {

		nodeVar8 = ( nodeVar8 + nodeVar26.w );
		nodeVar9 = ( nodeVar9 + 1.0 );
		nodeVar27 = ( nodeVar26.w - nodeVar10 );
		nodeVar10 = ( nodeVar10 + ( nodeVar27 / nodeVar9 ) );
		nodeVar11 = ( nodeVar11 + ( nodeVar27 * ( nodeVar26.w - nodeVar10 ) ) );
		

	}

	nodeVar28 = textureLoad( nodeUniform1, ( nodeConst1 + vec2<i32>( 0, -1 ) ), u32( 0u ) );
	nodeVar29 = max( nodeVar28, vec4<f32>( 0.0 ) );
	let nodeConst14 = ( nodeVar29.xyz / vec3<f32>( ( ( ( dot( nodeVar29.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) ) * object.nodeUniform4 ) * 10.0 ) + 1.0 ) ) );
	let nodeConst15 = vec3<f32>( dot( nodeConst14, vec3<f32>( 0.25, 0.5, 0.25 ) ), dot( nodeConst14, vec3<f32>( 0.5, 0.0, -0.5 ) ), dot( nodeConst14, vec3<f32>( -0.25, 0.5, -0.25 ) ) );
	nodeVar6 = ( nodeVar6 + nodeConst15 );
	nodeVar7 = ( nodeVar7 + ( nodeConst15 * nodeConst15 ) );

	if ( ( nodeVar29.w < 1000.0 ) ) {

		nodeVar8 = ( nodeVar8 + nodeVar29.w );
		nodeVar9 = ( nodeVar9 + 1.0 );
		nodeVar30 = ( nodeVar29.w - nodeVar10 );
		nodeVar10 = ( nodeVar10 + ( nodeVar30 / nodeVar9 ) );
		nodeVar11 = ( nodeVar11 + ( nodeVar30 * ( nodeVar29.w - nodeVar10 ) ) );
		

	}

	nodeVar31 = textureLoad( nodeUniform1, ( nodeConst1 + vec2<i32>( 0, 1 ) ), u32( 0u ) );
	nodeVar32 = max( nodeVar31, vec4<f32>( 0.0 ) );
	let nodeConst16 = ( nodeVar32.xyz / vec3<f32>( ( ( ( dot( nodeVar32.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) ) * object.nodeUniform4 ) * 10.0 ) + 1.0 ) ) );
	let nodeConst17 = vec3<f32>( dot( nodeConst16, vec3<f32>( 0.25, 0.5, 0.25 ) ), dot( nodeConst16, vec3<f32>( 0.5, 0.0, -0.5 ) ), dot( nodeConst16, vec3<f32>( -0.25, 0.5, -0.25 ) ) );
	nodeVar6 = ( nodeVar6 + nodeConst17 );
	nodeVar7 = ( nodeVar7 + ( nodeConst17 * nodeConst17 ) );

	if ( ( nodeVar32.w < 1000.0 ) ) {

		nodeVar8 = ( nodeVar8 + nodeVar32.w );
		nodeVar9 = ( nodeVar9 + 1.0 );
		nodeVar33 = ( nodeVar32.w - nodeVar10 );
		nodeVar10 = ( nodeVar10 + ( nodeVar33 / nodeVar9 ) );
		nodeVar11 = ( nodeVar11 + ( nodeVar33 * ( nodeVar32.w - nodeVar10 ) ) );
		

	}

	nodeVar34 = textureLoad( nodeUniform1, ( nodeConst1 + vec2<i32>( -1, 0 ) ), u32( 0u ) );
	nodeVar35 = max( nodeVar34, vec4<f32>( 0.0 ) );
	let nodeConst18 = ( nodeVar35.xyz / vec3<f32>( ( ( ( dot( nodeVar35.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) ) * object.nodeUniform4 ) * 10.0 ) + 1.0 ) ) );
	let nodeConst19 = vec3<f32>( dot( nodeConst18, vec3<f32>( 0.25, 0.5, 0.25 ) ), dot( nodeConst18, vec3<f32>( 0.5, 0.0, -0.5 ) ), dot( nodeConst18, vec3<f32>( -0.25, 0.5, -0.25 ) ) );
	nodeVar6 = ( nodeVar6 + nodeConst19 );
	nodeVar7 = ( nodeVar7 + ( nodeConst19 * nodeConst19 ) );

	if ( ( nodeVar35.w < 1000.0 ) ) {

		nodeVar8 = ( nodeVar8 + nodeVar35.w );
		nodeVar9 = ( nodeVar9 + 1.0 );
		nodeVar36 = ( nodeVar35.w - nodeVar10 );
		nodeVar10 = ( nodeVar10 + ( nodeVar36 / nodeVar9 ) );
		nodeVar11 = ( nodeVar11 + ( nodeVar36 * ( nodeVar35.w - nodeVar10 ) ) );
		

	}

	let nodeConst20 = ( nodeVar6 / vec3<f32>( 9.0 ) );

	if ( ( nodeVar9 < 0.5 ) ) {

		nodeVar38 = 10000.0;

	} else {

		nodeVar38 = ( nodeVar8 / max( nodeVar9, 0.0001 ) );

	}

	nodeVar37 = StructType2( nodeConst20, sqrt( max( ( ( nodeVar7 / vec3<f32>( 9.0 ) ) - ( nodeConst20 * nodeConst20 ) ), vec3<f32>( 0.0 ) ) ), nodeVar38, ( 1.0 - ( nodeVar9 / 9.0 ) ), max( sqrt( ( nodeVar11 / max( nodeVar9, 1.0 ) ) ), 0.001 ) );
	var nodeVar39 : StructType2 = nodeVar37;
	nodeVar40 = normalize( ( vec4<f32>( nodeVar5, 0.0 ) * object.nodeUniform5 ).xyz );
	let nodeConst21 = ( object.nodeUniform6 * vec4<f32>( vec3<f32>( ( ( vec2<f32>( nodeVarying0.x, ( 1.0 - nodeVarying0.y ) ) * vec2<f32>( 2.0 ) ) - vec2<f32>( 1.0 ) ), nodeVar1 ), 1.0 ) );
	nodeVar41 = ( nodeConst21.xyz / vec3<f32>( nodeConst21.w ) );
	nodeVar42 = ( object.nodeUniform7 * vec4<f32>( nodeVar41, 1.0 ) ).xyz;
	nodeVar43 = textureLoad( nodeUniform8, nodeConst0, u32( 0u ) );
	nodeVar44 = velocityToUVOffset( nodeVar43.xy );
	nodeVar45 = ( nodeVarying0 - nodeVar44 );
	nodeVar46 = ( ( nodeVar45 * object.nodeUniform2 ) - vec2<f32>( 0.5 ) );
	let nodeConst22 = vec2<i32>( floor( nodeVar46 ) );
	let nodeConst23 = ( nodeConst22 + vec2<i32>( 0, 0 ) );
	let nodeConst24 = ( ( vec2<f32>( nodeConst23 ) + vec2<f32>( 0.5 ) ) / object.nodeUniform2 );
	nodeVar49 = textureLoad( nodeUniform11, vec2<i32>( ( object.nodeUniform12 * vec3<f32>( vec2<f32>( nodeConst23 ), 1.0 ) ).xy ), u32( 0u ) );
	let nodeConst25 = ( object.nodeUniform10 * vec4<f32>( vec3<f32>( ( ( vec2<f32>( nodeConst24.x, ( 1.0 - nodeConst24.y ) ) * vec2<f32>( 2.0 ) ) - vec2<f32>( 1.0 ) ), nodeVar49 ), 1.0 ) );
	let nodeConst26 = ( nodeConst25.xyz / vec3<f32>( nodeConst25.w ) );
	nodeVar50 = abs( dot( ( ( object.nodeUniform9 * vec4<f32>( nodeConst26, 1.0 ) ).xyz - nodeVar42 ), nodeVar40 ) );
	nodeVar50 = ( nodeVar50 / abs( nodeConst26.z ) );
	nodeVar52 = textureLoad( nodeUniform13, vec2<i32>( ( object.nodeUniform14 * vec3<f32>( vec2<f32>( nodeConst23 ), 1.0 ) ).xy ), u32( 0u ) );
	let nodeConst27 = fract( nodeVar46 );
	nodeVar53 = textureLoad( nodeUniform15, vec2<i32>( ( object.nodeUniform16 * vec3<f32>( vec2<f32>( nodeConst23 ), 1.0 ) ).xy ), u32( 0u ) );
	let nodeConst28 = ( ( 1.0 - smoothstep( 0.0, 0.01, nodeVar50 ) ) * smoothstep( 0.95, 0.999, dot( normalize( ( vec4<f32>( ( ( nodeVar53.xyz * vec3<f32>( 2.0 ) ) - vec3<f32>( 1.0 ) ), 0.0 ) * object.nodeUniform17 ).xyz ), nodeVar40 ) ) );
	let nodeConst29 = ( ( ( 1.0 - nodeConst27.x ) * ( 1.0 - nodeConst27.y ) ) * nodeConst28 );
	nodeVar51 = StructType0( ( max( nodeVar52, vec4<f32>( 0.0 ) ) * vec4<f32>( nodeConst29 ) ), nodeConst29, nodeConst28 );
	let nodeConst30 = ( nodeConst22 + vec2<i32>( 1, 0 ) );
	let nodeConst31 = ( ( vec2<f32>( nodeConst30 ) + vec2<f32>( 0.5 ) ) / object.nodeUniform2 );
	nodeVar54 = textureLoad( nodeUniform11, vec2<i32>( ( object.nodeUniform12 * vec3<f32>( vec2<f32>( nodeConst30 ), 1.0 ) ).xy ), u32( 0u ) );
	let nodeConst32 = ( object.nodeUniform10 * vec4<f32>( vec3<f32>( ( ( vec2<f32>( nodeConst31.x, ( 1.0 - nodeConst31.y ) ) * vec2<f32>( 2.0 ) ) - vec2<f32>( 1.0 ) ), nodeVar54 ), 1.0 ) );
	let nodeConst33 = ( nodeConst32.xyz / vec3<f32>( nodeConst32.w ) );
	nodeVar55 = abs( dot( ( ( object.nodeUniform9 * vec4<f32>( nodeConst33, 1.0 ) ).xyz - nodeVar42 ), nodeVar40 ) );
	nodeVar55 = ( nodeVar55 / abs( nodeConst33.z ) );
	nodeVar57 = textureLoad( nodeUniform13, vec2<i32>( ( object.nodeUniform14 * vec3<f32>( vec2<f32>( nodeConst30 ), 1.0 ) ).xy ), u32( 0u ) );
	nodeVar58 = textureLoad( nodeUniform15, vec2<i32>( ( object.nodeUniform16 * vec3<f32>( vec2<f32>( nodeConst30 ), 1.0 ) ).xy ), u32( 0u ) );
	let nodeConst34 = ( ( 1.0 - smoothstep( 0.0, 0.01, nodeVar55 ) ) * smoothstep( 0.95, 0.999, dot( normalize( ( vec4<f32>( ( ( nodeVar58.xyz * vec3<f32>( 2.0 ) ) - vec3<f32>( 1.0 ) ), 0.0 ) * object.nodeUniform17 ).xyz ), nodeVar40 ) ) );
	let nodeConst35 = ( ( nodeConst27.x * ( 1.0 - nodeConst27.y ) ) * nodeConst34 );
	nodeVar56 = StructType0( ( max( nodeVar57, vec4<f32>( 0.0 ) ) * vec4<f32>( nodeConst35 ) ), nodeConst35, nodeConst34 );
	let nodeConst36 = ( nodeConst22 + vec2<i32>( 0, 1 ) );
	let nodeConst37 = ( ( vec2<f32>( nodeConst36 ) + vec2<f32>( 0.5 ) ) / object.nodeUniform2 );
	nodeVar59 = textureLoad( nodeUniform11, vec2<i32>( ( object.nodeUniform12 * vec3<f32>( vec2<f32>( nodeConst36 ), 1.0 ) ).xy ), u32( 0u ) );
	let nodeConst38 = ( object.nodeUniform10 * vec4<f32>( vec3<f32>( ( ( vec2<f32>( nodeConst37.x, ( 1.0 - nodeConst37.y ) ) * vec2<f32>( 2.0 ) ) - vec2<f32>( 1.0 ) ), nodeVar59 ), 1.0 ) );
	let nodeConst39 = ( nodeConst38.xyz / vec3<f32>( nodeConst38.w ) );
	nodeVar60 = abs( dot( ( ( object.nodeUniform9 * vec4<f32>( nodeConst39, 1.0 ) ).xyz - nodeVar42 ), nodeVar40 ) );
	nodeVar60 = ( nodeVar60 / abs( nodeConst39.z ) );
	nodeVar62 = textureLoad( nodeUniform13, vec2<i32>( ( object.nodeUniform14 * vec3<f32>( vec2<f32>( nodeConst36 ), 1.0 ) ).xy ), u32( 0u ) );
	nodeVar63 = textureLoad( nodeUniform15, vec2<i32>( ( object.nodeUniform16 * vec3<f32>( vec2<f32>( nodeConst36 ), 1.0 ) ).xy ), u32( 0u ) );
	let nodeConst40 = ( ( 1.0 - smoothstep( 0.0, 0.01, nodeVar60 ) ) * smoothstep( 0.95, 0.999, dot( normalize( ( vec4<f32>( ( ( nodeVar63.xyz * vec3<f32>( 2.0 ) ) - vec3<f32>( 1.0 ) ), 0.0 ) * object.nodeUniform17 ).xyz ), nodeVar40 ) ) );
	let nodeConst41 = ( ( ( 1.0 - nodeConst27.x ) * nodeConst27.y ) * nodeConst40 );
	nodeVar61 = StructType0( ( max( nodeVar62, vec4<f32>( 0.0 ) ) * vec4<f32>( nodeConst41 ) ), nodeConst41, nodeConst40 );
	let nodeConst42 = ( nodeConst22 + vec2<i32>( 1, 1 ) );
	let nodeConst43 = ( ( vec2<f32>( nodeConst42 ) + vec2<f32>( 0.5 ) ) / object.nodeUniform2 );
	nodeVar64 = textureLoad( nodeUniform11, vec2<i32>( ( object.nodeUniform12 * vec3<f32>( vec2<f32>( nodeConst42 ), 1.0 ) ).xy ), u32( 0u ) );
	let nodeConst44 = ( object.nodeUniform10 * vec4<f32>( vec3<f32>( ( ( vec2<f32>( nodeConst43.x, ( 1.0 - nodeConst43.y ) ) * vec2<f32>( 2.0 ) ) - vec2<f32>( 1.0 ) ), nodeVar64 ), 1.0 ) );
	let nodeConst45 = ( nodeConst44.xyz / vec3<f32>( nodeConst44.w ) );
	nodeVar65 = abs( dot( ( ( object.nodeUniform9 * vec4<f32>( nodeConst45, 1.0 ) ).xyz - nodeVar42 ), nodeVar40 ) );
	nodeVar65 = ( nodeVar65 / abs( nodeConst45.z ) );
	nodeVar67 = textureLoad( nodeUniform13, vec2<i32>( ( object.nodeUniform14 * vec3<f32>( vec2<f32>( nodeConst42 ), 1.0 ) ).xy ), u32( 0u ) );
	nodeVar68 = textureLoad( nodeUniform15, vec2<i32>( ( object.nodeUniform16 * vec3<f32>( vec2<f32>( nodeConst42 ), 1.0 ) ).xy ), u32( 0u ) );
	let nodeConst46 = ( ( 1.0 - smoothstep( 0.0, 0.01, nodeVar65 ) ) * smoothstep( 0.95, 0.999, dot( normalize( ( vec4<f32>( ( ( nodeVar68.xyz * vec3<f32>( 2.0 ) ) - vec3<f32>( 1.0 ) ), 0.0 ) * object.nodeUniform17 ).xyz ), nodeVar40 ) ) );
	let nodeConst47 = ( ( nodeConst27.x * nodeConst27.y ) * nodeConst46 );
	nodeVar66 = StructType0( ( max( nodeVar67, vec4<f32>( 0.0 ) ) * vec4<f32>( nodeConst47 ) ), nodeConst47, nodeConst46 );
	let nodeConst48 = ( ( ( nodeVar51.weight + nodeVar56.weight ) + nodeVar61.weight ) + nodeVar66.weight );

	if ( ( nodeConst48 > 0.01 ) ) {

		nodeVar48 = ( ( ( ( nodeVar51.color + nodeVar56.color ) + nodeVar61.color ) + nodeVar66.color ) / vec4<f32>( nodeConst48 ) );

	} else {

		nodeVar48 = vec4<f32>( nodeVar3.xyz, 1.0 );

	}

	nodeVar47 = StructType1( nodeVar48, max( max( nodeVar51.confidence, nodeVar56.confidence ), max( nodeVar61.confidence, nodeVar66.confidence ) ), min( min( nodeVar51.confidence, nodeVar56.confidence ), min( nodeVar61.confidence, nodeVar66.confidence ) ) );
	nodeVar69 = nodeVar47.color;
	nodeVar70 = 1.0;
	nodeVar71 = 0.0;
	nodeVar72 = normalize( ( nodeVar42 - object.nodeUniform18 ) );
	let nodeConst49 = projectWorldToUV( ( nodeVar42 + ( nodeVar72 * vec3<f32>( nodeVar39.rayLength ) ) ), object.nodeUniform17, object.nodeUniform19 );
	nodeVar73 = nodeConst49;
	let nodeConst50 = clamp( ( length( ( nodeVar44 * object.nodeUniform2 ) ) / 128.0 ), 0.0, 1.0 );
	nodeVar39.stdDevRayLength = ( 1.0 - min( ( ( nodeVar39.stdDevRayLength * min( ( nodeConst50 * 100.0 ), 1.0 ) ) * 3.5 ), 1.0 ) );
	nodeVar74 = ( ( nodeVar73 * object.nodeUniform2 ) - vec2<f32>( 0.5 ) );
	let nodeConst51 = vec2<i32>( floor( nodeVar74 ) );
	let nodeConst52 = ( nodeConst51 + vec2<i32>( 0, 0 ) );
	let nodeConst53 = ( ( vec2<f32>( nodeConst52 ) + vec2<f32>( 0.5 ) ) / object.nodeUniform2 );
	nodeVar77 = textureLoad( nodeUniform11, vec2<i32>( ( object.nodeUniform12 * vec3<f32>( vec2<f32>( nodeConst52 ), 1.0 ) ).xy ), u32( 0u ) );
	let nodeConst54 = ( object.nodeUniform10 * vec4<f32>( vec3<f32>( ( ( vec2<f32>( nodeConst53.x, ( 1.0 - nodeConst53.y ) ) * vec2<f32>( 2.0 ) ) - vec2<f32>( 1.0 ) ), nodeVar77 ), 1.0 ) );
	let nodeConst55 = ( nodeConst54.xyz / vec3<f32>( nodeConst54.w ) );
	nodeVar78 = abs( dot( ( ( object.nodeUniform9 * vec4<f32>( nodeConst55, 1.0 ) ).xyz - nodeVar42 ), nodeVar40 ) );
	nodeVar78 = ( nodeVar78 / abs( nodeConst55.z ) );
	nodeVar80 = textureLoad( nodeUniform13, vec2<i32>( ( object.nodeUniform14 * vec3<f32>( vec2<f32>( nodeConst52 ), 1.0 ) ).xy ), u32( 0u ) );
	let nodeConst56 = fract( nodeVar74 );
	nodeVar81 = textureLoad( nodeUniform15, vec2<i32>( ( object.nodeUniform16 * vec3<f32>( vec2<f32>( nodeConst52 ), 1.0 ) ).xy ), u32( 0u ) );
	let nodeConst57 = ( ( 1.0 - smoothstep( 0.0, 0.01, nodeVar78 ) ) * smoothstep( 0.95, 0.999, dot( normalize( ( vec4<f32>( ( ( nodeVar81.xyz * vec3<f32>( 2.0 ) ) - vec3<f32>( 1.0 ) ), 0.0 ) * object.nodeUniform17 ).xyz ), nodeVar40 ) ) );
	let nodeConst58 = ( ( ( 1.0 - nodeConst56.x ) * ( 1.0 - nodeConst56.y ) ) * nodeConst57 );
	nodeVar79 = StructType0( ( max( nodeVar80, vec4<f32>( 0.0 ) ) * vec4<f32>( nodeConst58 ) ), nodeConst58, nodeConst57 );
	let nodeConst59 = ( nodeConst51 + vec2<i32>( 1, 0 ) );
	let nodeConst60 = ( ( vec2<f32>( nodeConst59 ) + vec2<f32>( 0.5 ) ) / object.nodeUniform2 );
	nodeVar82 = textureLoad( nodeUniform11, vec2<i32>( ( object.nodeUniform12 * vec3<f32>( vec2<f32>( nodeConst59 ), 1.0 ) ).xy ), u32( 0u ) );
	let nodeConst61 = ( object.nodeUniform10 * vec4<f32>( vec3<f32>( ( ( vec2<f32>( nodeConst60.x, ( 1.0 - nodeConst60.y ) ) * vec2<f32>( 2.0 ) ) - vec2<f32>( 1.0 ) ), nodeVar82 ), 1.0 ) );
	let nodeConst62 = ( nodeConst61.xyz / vec3<f32>( nodeConst61.w ) );
	nodeVar83 = abs( dot( ( ( object.nodeUniform9 * vec4<f32>( nodeConst62, 1.0 ) ).xyz - nodeVar42 ), nodeVar40 ) );
	nodeVar83 = ( nodeVar83 / abs( nodeConst62.z ) );
	nodeVar85 = textureLoad( nodeUniform13, vec2<i32>( ( object.nodeUniform14 * vec3<f32>( vec2<f32>( nodeConst59 ), 1.0 ) ).xy ), u32( 0u ) );
	nodeVar86 = textureLoad( nodeUniform15, vec2<i32>( ( object.nodeUniform16 * vec3<f32>( vec2<f32>( nodeConst59 ), 1.0 ) ).xy ), u32( 0u ) );
	let nodeConst63 = ( ( 1.0 - smoothstep( 0.0, 0.01, nodeVar83 ) ) * smoothstep( 0.95, 0.999, dot( normalize( ( vec4<f32>( ( ( nodeVar86.xyz * vec3<f32>( 2.0 ) ) - vec3<f32>( 1.0 ) ), 0.0 ) * object.nodeUniform17 ).xyz ), nodeVar40 ) ) );
	let nodeConst64 = ( ( nodeConst56.x * ( 1.0 - nodeConst56.y ) ) * nodeConst63 );
	nodeVar84 = StructType0( ( max( nodeVar85, vec4<f32>( 0.0 ) ) * vec4<f32>( nodeConst64 ) ), nodeConst64, nodeConst63 );
	let nodeConst65 = ( nodeConst51 + vec2<i32>( 0, 1 ) );
	let nodeConst66 = ( ( vec2<f32>( nodeConst65 ) + vec2<f32>( 0.5 ) ) / object.nodeUniform2 );
	nodeVar87 = textureLoad( nodeUniform11, vec2<i32>( ( object.nodeUniform12 * vec3<f32>( vec2<f32>( nodeConst65 ), 1.0 ) ).xy ), u32( 0u ) );
	let nodeConst67 = ( object.nodeUniform10 * vec4<f32>( vec3<f32>( ( ( vec2<f32>( nodeConst66.x, ( 1.0 - nodeConst66.y ) ) * vec2<f32>( 2.0 ) ) - vec2<f32>( 1.0 ) ), nodeVar87 ), 1.0 ) );
	let nodeConst68 = ( nodeConst67.xyz / vec3<f32>( nodeConst67.w ) );
	nodeVar88 = abs( dot( ( ( object.nodeUniform9 * vec4<f32>( nodeConst68, 1.0 ) ).xyz - nodeVar42 ), nodeVar40 ) );
	nodeVar88 = ( nodeVar88 / abs( nodeConst68.z ) );
	nodeVar90 = textureLoad( nodeUniform13, vec2<i32>( ( object.nodeUniform14 * vec3<f32>( vec2<f32>( nodeConst65 ), 1.0 ) ).xy ), u32( 0u ) );
	nodeVar91 = textureLoad( nodeUniform15, vec2<i32>( ( object.nodeUniform16 * vec3<f32>( vec2<f32>( nodeConst65 ), 1.0 ) ).xy ), u32( 0u ) );
	let nodeConst69 = ( ( 1.0 - smoothstep( 0.0, 0.01, nodeVar88 ) ) * smoothstep( 0.95, 0.999, dot( normalize( ( vec4<f32>( ( ( nodeVar91.xyz * vec3<f32>( 2.0 ) ) - vec3<f32>( 1.0 ) ), 0.0 ) * object.nodeUniform17 ).xyz ), nodeVar40 ) ) );
	let nodeConst70 = ( ( ( 1.0 - nodeConst56.x ) * nodeConst56.y ) * nodeConst69 );
	nodeVar89 = StructType0( ( max( nodeVar90, vec4<f32>( 0.0 ) ) * vec4<f32>( nodeConst70 ) ), nodeConst70, nodeConst69 );
	let nodeConst71 = ( nodeConst51 + vec2<i32>( 1, 1 ) );
	let nodeConst72 = ( ( vec2<f32>( nodeConst71 ) + vec2<f32>( 0.5 ) ) / object.nodeUniform2 );
	nodeVar92 = textureLoad( nodeUniform11, vec2<i32>( ( object.nodeUniform12 * vec3<f32>( vec2<f32>( nodeConst71 ), 1.0 ) ).xy ), u32( 0u ) );
	let nodeConst73 = ( object.nodeUniform10 * vec4<f32>( vec3<f32>( ( ( vec2<f32>( nodeConst72.x, ( 1.0 - nodeConst72.y ) ) * vec2<f32>( 2.0 ) ) - vec2<f32>( 1.0 ) ), nodeVar92 ), 1.0 ) );
	let nodeConst74 = ( nodeConst73.xyz / vec3<f32>( nodeConst73.w ) );
	nodeVar93 = abs( dot( ( ( object.nodeUniform9 * vec4<f32>( nodeConst74, 1.0 ) ).xyz - nodeVar42 ), nodeVar40 ) );
	nodeVar93 = ( nodeVar93 / abs( nodeConst74.z ) );
	nodeVar95 = textureLoad( nodeUniform13, vec2<i32>( ( object.nodeUniform14 * vec3<f32>( vec2<f32>( nodeConst71 ), 1.0 ) ).xy ), u32( 0u ) );
	nodeVar96 = textureLoad( nodeUniform15, vec2<i32>( ( object.nodeUniform16 * vec3<f32>( vec2<f32>( nodeConst71 ), 1.0 ) ).xy ), u32( 0u ) );
	let nodeConst75 = ( ( 1.0 - smoothstep( 0.0, 0.01, nodeVar93 ) ) * smoothstep( 0.95, 0.999, dot( normalize( ( vec4<f32>( ( ( nodeVar96.xyz * vec3<f32>( 2.0 ) ) - vec3<f32>( 1.0 ) ), 0.0 ) * object.nodeUniform17 ).xyz ), nodeVar40 ) ) );
	let nodeConst76 = ( ( nodeConst56.x * nodeConst56.y ) * nodeConst75 );
	nodeVar94 = StructType0( ( max( nodeVar95, vec4<f32>( 0.0 ) ) * vec4<f32>( nodeConst76 ) ), nodeConst76, nodeConst75 );
	let nodeConst77 = ( ( ( nodeVar79.weight + nodeVar84.weight ) + nodeVar89.weight ) + nodeVar94.weight );

	if ( ( nodeConst77 > 0.01 ) ) {

		nodeVar76 = ( ( ( ( nodeVar79.color + nodeVar84.color ) + nodeVar89.color ) + nodeVar94.color ) / vec4<f32>( nodeConst77 ) );

	} else {

		nodeVar76 = vec4<f32>( nodeVar3.xyz, 1.0 );

	}

	nodeVar75 = StructType1( nodeVar76, max( max( nodeVar79.confidence, nodeVar84.confidence ), max( nodeVar89.confidence, nodeVar94.confidence ) ), min( min( nodeVar79.confidence, nodeVar84.confidence ), min( nodeVar89.confidence, nodeVar94.confidence ) ) );
	nodeVar98 = bool( object.nodeUniform20 );

	if ( ( ( ( ( ( nodeVar73.x >= 0.0 ) && ( nodeVar73.x <= 1.0 ) ) && ( nodeVar73.y >= 0.0 ) ) && ( nodeVar73.y <= 1.0 ) ) && nodeVar98 ) ) {

		nodeVar97 = nodeVar75.tapConfidence;

	} else {

		nodeVar97 = 0.0;

	}

	let nodeConst78 = ( ( ( nodeVar75.minConfidence * nodeVar39.stdDevRayLength ) * ( 1.0 - clamp( ( length( fwidth( nodeVar40 ) ) * 50.0 ), 0.0, 1.0 ) ) ) * nodeVar97 );
	let nodeConst79 = ( nodeConst78 * ( 1.0 - ( nodeVar39.envProbability * nodeVar39.envProbability ) ) );

	if ( ( ( ( ( nodeVar45.x >= 0.0 ) && ( nodeVar45.x <= 1.0 ) ) && ( nodeVar45.y >= 0.0 ) ) && ( nodeVar45.y <= 1.0 ) ) ) {

		nodeVar99 = nodeVar47.tapConfidence;

	} else {

		nodeVar99 = 0.0;

	}

	let nodeConst80 = ( ( 1.0 - nodeConst79 ) * nodeVar99 );
	let nodeConst81 = max( ( nodeConst79 + nodeConst80 ), 0.000001 );
	nodeVar100 = vec4<f32>( ( ( ( max( nodeVar75.color.xyz, vec3<f32>( 0.0 ) ) * vec3<f32>( nodeConst79 ) ) + ( max( nodeVar47.color.xyz, vec3<f32>( 0.0 ) ) * vec3<f32>( nodeConst80 ) ) ) / vec3<f32>( nodeConst81 ) ), nodeVar47.color.w );

	if ( ( length( nodeVar100.xyz ) < 0.000001 ) ) {

		nodeVar100 = vec4<f32>( nodeVar3.xyz, 1.0 );
		

	}

	nodeVar69 = nodeVar100;
	nodeVar70 = ( ( ( nodeVar97 * nodeConst79 ) + ( nodeVar99 * nodeConst80 ) ) / nodeConst81 );
	nodeVar71 = nodeConst78;
	nodeVar101 = ( dpdx( nodeVar45 ) * object.nodeUniform2 );
	nodeVar102 = ( - dpdy( nodeVar45 ) * object.nodeUniform2 );
	let nodeConst82 = ( dot( nodeVar101, nodeVar101 ) + dot( nodeVar102, nodeVar102 ) );
	let nodeConst83 = ( ( nodeVar101.x * nodeVar102.y ) - ( nodeVar101.y * nodeVar102.x ) );
	let nodeConst84 = clamp( sqrt( max( ( ( nodeConst82 * 0.5 ) - sqrt( max( ( ( ( nodeConst82 * nodeConst82 ) * 0.25 ) - ( nodeConst83 * nodeConst83 ) ), 0.0 ) ) ), 0.0 ) ), 0.0, 1.0 );
	nodeVar70 = ( nodeVar70 * pow( nodeConst84, 2.0 ) );
	nodeVar103 = nodeVar69.xyz;
	let nodeConst85 = ( ( ( dot( nodeVar103, vec3<f32>( 0.2126, 0.7152, 0.0722 ) ) * object.nodeUniform4 ) * 10.0 ) + 1.0 );
	let nodeConst86 = ( nodeVar103 / vec3<f32>( nodeConst85 ) );
	let nodeConst87 = ( 1.0 - nodeConst50 );
	let nodeConst88 = ( nodeVar39.stdColor * vec3<f32>( mix( 0.5, 1.0, ( nodeConst87 * nodeConst87 ) ) ) );
	let nodeConst89 = clipToAABB( vec3<f32>( dot( nodeConst86, vec3<f32>( 0.25, 0.5, 0.25 ) ), dot( nodeConst86, vec3<f32>( 0.5, 0.0, -0.5 ) ), dot( nodeConst86, vec3<f32>( -0.25, 0.5, -0.25 ) ) ), ( nodeVar39.mean - nodeConst88 ), ( nodeVar39.mean + nodeConst88 ) );
	nodeVar104 = ( vec3<f32>( ( ( nodeConst89.x + nodeConst89.y ) - nodeConst89.z ), ( nodeConst89.x + nodeConst89.z ), ( ( nodeConst89.x - nodeConst89.y ) - nodeConst89.z ) ) * vec3<f32>( nodeConst85 ) );
	let nodeConst90 = ( ( object.nodeUniform21 * max( min( ( nodeConst50 * 10.0 ), 1.0 ), 0.25 ) ) * ( 1.0 + clamp( ( ( 1.0 - nodeConst84 ) + ( 1.0 - nodeVar71 ) ), 0.0, 1.0 ) ) );
	nodeVar105 = mix( nodeVar69.xyz, nodeVar104, nodeConst90 );
	nodeVar69.x = nodeVar105[ 0 ];
	nodeVar69.y = nodeVar105[ 1 ];
	nodeVar69.z = nodeVar105[ 2 ];
	nodeVar70 = ( nodeVar70 * exp( ( - ( ( length( ( nodeVar69.xyz - nodeVar104 ) ) * nodeConst90 ) * 30.0 ) ) ) );
	nodeVar70 = ( nodeVar70 * mix( 1.0, ( ( nodeVar71 * 0.05 ) + 0.95 ), clamp( ( nodeConst50 * 100.0 ), 0.0, 1.0 ) ) );

	if ( ( nodeVar70 < 0.000001 ) ) {

		nodeVar69 = vec4<f32>( nodeVar3.xyz, 1.0 );
		

	}

	nodeVar106 = min( ( ( ( 1.0 / max( nodeVar69.w, 0.000001 ) ) * nodeVar70 ) + 1.0 ), object.nodeUniform22 );

	if ( ( length( nodeVar3.xyz ) < 0.000001 ) ) {

		nodeVar106 = max( ( nodeVar106 - 1.0 ), 1.0 );
		

	}


	// result

	output.color = vec4<f32>( nodeVar69.xyz, ( 1.0 / nodeVar106 ) );

	return output;

}
