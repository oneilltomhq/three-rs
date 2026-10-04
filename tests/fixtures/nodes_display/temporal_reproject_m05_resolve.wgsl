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
@binding( 6 ) @group( 0 ) var nodeUniform7_sampler : sampler;
@binding( 7 ) @group( 0 ) var nodeUniform7 : texture_2d<f32>;
@binding( 8 ) @group( 0 ) var nodeUniform10 : texture_depth_2d;
@binding( 9 ) @group( 0 ) var nodeUniform12_sampler : sampler;
@binding( 10 ) @group( 0 ) var nodeUniform12 : texture_2d<f32>;
@binding( 11 ) @group( 0 ) var nodeUniform14_sampler : sampler;
@binding( 12 ) @group( 0 ) var nodeUniform14 : texture_2d<f32>;

struct objectStruct {
	nodeUniform2 : vec2<f32>,
	nodeUniform4 : mat4x4<f32>,
	nodeUniform5 : mat4x4<f32>,
	nodeUniform6 : mat4x4<f32>,
	nodeUniform8 : mat4x4<f32>,
	nodeUniform9 : mat4x4<f32>,
	nodeUniform11 : mat3x3<f32>,
	nodeUniform13 : mat3x3<f32>,
	nodeUniform15 : mat3x3<f32>,
	nodeUniform16 : mat4x4<f32>,
	nodeUniform17 : f32,
	nodeUniform18 : f32,
	nodeUniform19 : f32
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
var<private> nodeVar8 : vec3<f32>;
var<private> nodeVar9 : vec4<f32>;
var<private> nodeVar10 : vec2<f32>;
var<private> nodeVar11 : vec2<f32>;
var<private> nodeVar12 : vec2<f32>;
var<private> nodeVar13 : StructType1;
var<private> nodeVar14 : vec4<f32>;
var<private> nodeVar15 : f32;
var<private> nodeVar16 : f32;
var<private> nodeVar17 : StructType0;
var<private> nodeVar18 : vec4<f32>;
var<private> nodeVar19 : vec4<f32>;
var<private> nodeVar20 : f32;
var<private> nodeVar21 : f32;
var<private> nodeVar22 : StructType0;
var<private> nodeVar23 : vec4<f32>;
var<private> nodeVar24 : vec4<f32>;
var<private> nodeVar25 : f32;
var<private> nodeVar26 : f32;
var<private> nodeVar27 : StructType0;
var<private> nodeVar28 : vec4<f32>;
var<private> nodeVar29 : vec4<f32>;
var<private> nodeVar30 : f32;
var<private> nodeVar31 : f32;
var<private> nodeVar32 : StructType0;
var<private> nodeVar33 : vec4<f32>;
var<private> nodeVar34 : vec4<f32>;
var<private> nodeVar35 : vec4<f32>;
var<private> nodeVar36 : f32;
var<private> nodeVar37 : f32;
var<private> nodeVar38 : vec2<f32>;
var<private> nodeVar39 : vec2<f32>;
var<private> nodeVar40 : vec3<f32>;
var<private> nodeVar41 : vec3<f32>;
var<private> nodeVar42 : vec3<f32>;
var<private> nodeVar43 : f32;
var<private> nodeVar44 : f32;
var<private> nodeVar45 : f32;
var<private> nodeVar46 : f32;
var<private> nodeVar47 : f32;
var<private> nodeVar48 : vec4<f32>;
var<private> nodeVar49 : vec4<f32>;
var<private> nodeVar50 : f32;
var<private> nodeVar51 : vec4<f32>;
var<private> nodeVar52 : vec4<f32>;
var<private> nodeVar53 : f32;
var<private> nodeVar54 : vec4<f32>;
var<private> nodeVar55 : vec4<f32>;
var<private> nodeVar56 : f32;
var<private> nodeVar57 : vec4<f32>;
var<private> nodeVar58 : vec4<f32>;
var<private> nodeVar59 : f32;
var<private> nodeVar60 : vec4<f32>;
var<private> nodeVar61 : vec4<f32>;
var<private> nodeVar62 : f32;
var<private> nodeVar63 : vec4<f32>;
var<private> nodeVar64 : vec4<f32>;
var<private> nodeVar65 : f32;
var<private> nodeVar66 : vec4<f32>;
var<private> nodeVar67 : vec4<f32>;
var<private> nodeVar68 : f32;
var<private> nodeVar69 : vec4<f32>;
var<private> nodeVar70 : vec4<f32>;
var<private> nodeVar71 : f32;
var<private> nodeVar72 : StructType2;
var<private> nodeVar73 : f32;
var<private> nodeVar74 : vec3<f32>;
var<private> nodeVar75 : vec3<f32>;
var<private> nodeVar76 : f32;

// codes
fn beautyTexelFromScreen ( screenTexel : vec2<i32>, beautySize : vec2<f32>, resolveSize : vec2<f32> ) -> vec2<i32> {

	


	return vec2<i32>( floor( ( ( vec2<f32>( screenTexel ) * beautySize ) / resolveSize ) ) );

}


fn velocityToUVOffset ( velocity : vec2<f32> ) -> vec2<f32> {

	


	return ( velocity * vec2<f32>( 0.5, -0.5 ) );

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
	nodeVar6 = normalize( ( vec4<f32>( nodeVar5, 0.0 ) * object.nodeUniform4 ).xyz );
	let nodeConst2 = ( object.nodeUniform5 * vec4<f32>( vec3<f32>( ( ( vec2<f32>( nodeVarying0.x, ( 1.0 - nodeVarying0.y ) ) * vec2<f32>( 2.0 ) ) - vec2<f32>( 1.0 ) ), nodeVar1 ), 1.0 ) );
	nodeVar7 = ( nodeConst2.xyz / vec3<f32>( nodeConst2.w ) );
	nodeVar8 = ( object.nodeUniform6 * vec4<f32>( nodeVar7, 1.0 ) ).xyz;
	nodeVar9 = textureLoad( nodeUniform7, nodeConst0, u32( 0u ) );
	nodeVar10 = velocityToUVOffset( nodeVar9.xy );
	nodeVar11 = ( nodeVarying0 - nodeVar10 );
	nodeVar12 = ( ( nodeVar11 * object.nodeUniform2 ) - vec2<f32>( 0.5 ) );
	let nodeConst3 = vec2<i32>( floor( nodeVar12 ) );
	let nodeConst4 = ( nodeConst3 + vec2<i32>( 0, 0 ) );
	let nodeConst5 = ( ( vec2<f32>( nodeConst4 ) + vec2<f32>( 0.5 ) ) / object.nodeUniform2 );
	nodeVar15 = textureLoad( nodeUniform10, vec2<i32>( ( object.nodeUniform11 * vec3<f32>( vec2<f32>( nodeConst4 ), 1.0 ) ).xy ), u32( 0u ) );
	let nodeConst6 = ( object.nodeUniform9 * vec4<f32>( vec3<f32>( ( ( vec2<f32>( nodeConst5.x, ( 1.0 - nodeConst5.y ) ) * vec2<f32>( 2.0 ) ) - vec2<f32>( 1.0 ) ), nodeVar15 ), 1.0 ) );
	let nodeConst7 = ( nodeConst6.xyz / vec3<f32>( nodeConst6.w ) );
	nodeVar16 = abs( dot( ( ( object.nodeUniform8 * vec4<f32>( nodeConst7, 1.0 ) ).xyz - nodeVar8 ), nodeVar6 ) );
	nodeVar16 = ( nodeVar16 / abs( nodeConst7.z ) );
	nodeVar18 = textureLoad( nodeUniform12, vec2<i32>( ( object.nodeUniform13 * vec3<f32>( vec2<f32>( nodeConst4 ), 1.0 ) ).xy ), u32( 0u ) );
	let nodeConst8 = fract( nodeVar12 );
	nodeVar19 = textureLoad( nodeUniform14, vec2<i32>( ( object.nodeUniform15 * vec3<f32>( vec2<f32>( nodeConst4 ), 1.0 ) ).xy ), u32( 0u ) );
	let nodeConst9 = ( ( 1.0 - smoothstep( 0.0, 0.01, nodeVar16 ) ) * smoothstep( 0.95, 0.999, dot( normalize( ( vec4<f32>( ( ( nodeVar19.xyz * vec3<f32>( 2.0 ) ) - vec3<f32>( 1.0 ) ), 0.0 ) * object.nodeUniform16 ).xyz ), nodeVar6 ) ) );
	let nodeConst10 = ( ( ( 1.0 - nodeConst8.x ) * ( 1.0 - nodeConst8.y ) ) * nodeConst9 );
	nodeVar17 = StructType0( ( max( nodeVar18, vec4<f32>( 0.0 ) ) * vec4<f32>( nodeConst10 ) ), nodeConst10, nodeConst9 );
	let nodeConst11 = ( nodeConst3 + vec2<i32>( 1, 0 ) );
	let nodeConst12 = ( ( vec2<f32>( nodeConst11 ) + vec2<f32>( 0.5 ) ) / object.nodeUniform2 );
	nodeVar20 = textureLoad( nodeUniform10, vec2<i32>( ( object.nodeUniform11 * vec3<f32>( vec2<f32>( nodeConst11 ), 1.0 ) ).xy ), u32( 0u ) );
	let nodeConst13 = ( object.nodeUniform9 * vec4<f32>( vec3<f32>( ( ( vec2<f32>( nodeConst12.x, ( 1.0 - nodeConst12.y ) ) * vec2<f32>( 2.0 ) ) - vec2<f32>( 1.0 ) ), nodeVar20 ), 1.0 ) );
	let nodeConst14 = ( nodeConst13.xyz / vec3<f32>( nodeConst13.w ) );
	nodeVar21 = abs( dot( ( ( object.nodeUniform8 * vec4<f32>( nodeConst14, 1.0 ) ).xyz - nodeVar8 ), nodeVar6 ) );
	nodeVar21 = ( nodeVar21 / abs( nodeConst14.z ) );
	nodeVar23 = textureLoad( nodeUniform12, vec2<i32>( ( object.nodeUniform13 * vec3<f32>( vec2<f32>( nodeConst11 ), 1.0 ) ).xy ), u32( 0u ) );
	nodeVar24 = textureLoad( nodeUniform14, vec2<i32>( ( object.nodeUniform15 * vec3<f32>( vec2<f32>( nodeConst11 ), 1.0 ) ).xy ), u32( 0u ) );
	let nodeConst15 = ( ( 1.0 - smoothstep( 0.0, 0.01, nodeVar21 ) ) * smoothstep( 0.95, 0.999, dot( normalize( ( vec4<f32>( ( ( nodeVar24.xyz * vec3<f32>( 2.0 ) ) - vec3<f32>( 1.0 ) ), 0.0 ) * object.nodeUniform16 ).xyz ), nodeVar6 ) ) );
	let nodeConst16 = ( ( nodeConst8.x * ( 1.0 - nodeConst8.y ) ) * nodeConst15 );
	nodeVar22 = StructType0( ( max( nodeVar23, vec4<f32>( 0.0 ) ) * vec4<f32>( nodeConst16 ) ), nodeConst16, nodeConst15 );
	let nodeConst17 = ( nodeConst3 + vec2<i32>( 0, 1 ) );
	let nodeConst18 = ( ( vec2<f32>( nodeConst17 ) + vec2<f32>( 0.5 ) ) / object.nodeUniform2 );
	nodeVar25 = textureLoad( nodeUniform10, vec2<i32>( ( object.nodeUniform11 * vec3<f32>( vec2<f32>( nodeConst17 ), 1.0 ) ).xy ), u32( 0u ) );
	let nodeConst19 = ( object.nodeUniform9 * vec4<f32>( vec3<f32>( ( ( vec2<f32>( nodeConst18.x, ( 1.0 - nodeConst18.y ) ) * vec2<f32>( 2.0 ) ) - vec2<f32>( 1.0 ) ), nodeVar25 ), 1.0 ) );
	let nodeConst20 = ( nodeConst19.xyz / vec3<f32>( nodeConst19.w ) );
	nodeVar26 = abs( dot( ( ( object.nodeUniform8 * vec4<f32>( nodeConst20, 1.0 ) ).xyz - nodeVar8 ), nodeVar6 ) );
	nodeVar26 = ( nodeVar26 / abs( nodeConst20.z ) );
	nodeVar28 = textureLoad( nodeUniform12, vec2<i32>( ( object.nodeUniform13 * vec3<f32>( vec2<f32>( nodeConst17 ), 1.0 ) ).xy ), u32( 0u ) );
	nodeVar29 = textureLoad( nodeUniform14, vec2<i32>( ( object.nodeUniform15 * vec3<f32>( vec2<f32>( nodeConst17 ), 1.0 ) ).xy ), u32( 0u ) );
	let nodeConst21 = ( ( 1.0 - smoothstep( 0.0, 0.01, nodeVar26 ) ) * smoothstep( 0.95, 0.999, dot( normalize( ( vec4<f32>( ( ( nodeVar29.xyz * vec3<f32>( 2.0 ) ) - vec3<f32>( 1.0 ) ), 0.0 ) * object.nodeUniform16 ).xyz ), nodeVar6 ) ) );
	let nodeConst22 = ( ( ( 1.0 - nodeConst8.x ) * nodeConst8.y ) * nodeConst21 );
	nodeVar27 = StructType0( ( max( nodeVar28, vec4<f32>( 0.0 ) ) * vec4<f32>( nodeConst22 ) ), nodeConst22, nodeConst21 );
	let nodeConst23 = ( nodeConst3 + vec2<i32>( 1, 1 ) );
	let nodeConst24 = ( ( vec2<f32>( nodeConst23 ) + vec2<f32>( 0.5 ) ) / object.nodeUniform2 );
	nodeVar30 = textureLoad( nodeUniform10, vec2<i32>( ( object.nodeUniform11 * vec3<f32>( vec2<f32>( nodeConst23 ), 1.0 ) ).xy ), u32( 0u ) );
	let nodeConst25 = ( object.nodeUniform9 * vec4<f32>( vec3<f32>( ( ( vec2<f32>( nodeConst24.x, ( 1.0 - nodeConst24.y ) ) * vec2<f32>( 2.0 ) ) - vec2<f32>( 1.0 ) ), nodeVar30 ), 1.0 ) );
	let nodeConst26 = ( nodeConst25.xyz / vec3<f32>( nodeConst25.w ) );
	nodeVar31 = abs( dot( ( ( object.nodeUniform8 * vec4<f32>( nodeConst26, 1.0 ) ).xyz - nodeVar8 ), nodeVar6 ) );
	nodeVar31 = ( nodeVar31 / abs( nodeConst26.z ) );
	nodeVar33 = textureLoad( nodeUniform12, vec2<i32>( ( object.nodeUniform13 * vec3<f32>( vec2<f32>( nodeConst23 ), 1.0 ) ).xy ), u32( 0u ) );
	nodeVar34 = textureLoad( nodeUniform14, vec2<i32>( ( object.nodeUniform15 * vec3<f32>( vec2<f32>( nodeConst23 ), 1.0 ) ).xy ), u32( 0u ) );
	let nodeConst27 = ( ( 1.0 - smoothstep( 0.0, 0.01, nodeVar31 ) ) * smoothstep( 0.95, 0.999, dot( normalize( ( vec4<f32>( ( ( nodeVar34.xyz * vec3<f32>( 2.0 ) ) - vec3<f32>( 1.0 ) ), 0.0 ) * object.nodeUniform16 ).xyz ), nodeVar6 ) ) );
	let nodeConst28 = ( ( nodeConst8.x * nodeConst8.y ) * nodeConst27 );
	nodeVar32 = StructType0( ( max( nodeVar33, vec4<f32>( 0.0 ) ) * vec4<f32>( nodeConst28 ) ), nodeConst28, nodeConst27 );
	let nodeConst29 = ( ( ( nodeVar17.weight + nodeVar22.weight ) + nodeVar27.weight ) + nodeVar32.weight );

	if ( ( nodeConst29 > 0.01 ) ) {

		nodeVar14 = ( ( ( ( nodeVar17.color + nodeVar22.color ) + nodeVar27.color ) + nodeVar32.color ) / vec4<f32>( nodeConst29 ) );

	} else {

		nodeVar14 = vec4<f32>( nodeVar3.xyz, 1.0 );

	}

	nodeVar13 = StructType1( nodeVar14, max( max( nodeVar17.confidence, nodeVar22.confidence ), max( nodeVar27.confidence, nodeVar32.confidence ) ), min( min( nodeVar17.confidence, nodeVar22.confidence ), min( nodeVar27.confidence, nodeVar32.confidence ) ) );
	nodeVar35 = nodeVar13.color;
	nodeVar36 = 1.0;
	nodeVar37 = 0.0;
	nodeVar38 = ( dpdx( nodeVar11 ) * object.nodeUniform2 );
	nodeVar39 = ( - dpdy( nodeVar11 ) * object.nodeUniform2 );
	let nodeConst30 = ( dot( nodeVar38, nodeVar38 ) + dot( nodeVar39, nodeVar39 ) );
	let nodeConst31 = ( ( nodeVar38.x * nodeVar39.y ) - ( nodeVar38.y * nodeVar39.x ) );
	let nodeConst32 = clamp( sqrt( max( ( ( nodeConst30 * 0.5 ) - sqrt( max( ( ( ( nodeConst30 * nodeConst30 ) * 0.25 ) - ( nodeConst31 * nodeConst31 ) ), 0.0 ) ) ), 0.0 ) ), 0.0, 1.0 );
	nodeVar36 = ( nodeVar36 * pow( nodeConst32, 2.0 ) );
	nodeVar40 = nodeVar35.xyz;
	let nodeConst33 = ( ( ( dot( nodeVar40, vec3<f32>( 0.2126, 0.7152, 0.0722 ) ) * object.nodeUniform17 ) * 10.0 ) + 1.0 );
	let nodeConst34 = ( nodeVar40 / vec3<f32>( nodeConst33 ) );
	let nodeConst35 = ( nodeVar3.xyz / vec3<f32>( ( ( ( dot( nodeVar3.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) ) * object.nodeUniform17 ) * 10.0 ) + 1.0 ) ) );
	let nodeConst36 = vec3<f32>( dot( nodeConst35, vec3<f32>( 0.25, 0.5, 0.25 ) ), dot( nodeConst35, vec3<f32>( 0.5, 0.0, -0.5 ) ), dot( nodeConst35, vec3<f32>( -0.25, 0.5, -0.25 ) ) );
	nodeVar41 = nodeConst36;
	nodeVar42 = ( nodeConst36 * nodeConst36 );
	nodeVar43 = 0.0;
	nodeVar44 = 0.0;
	nodeVar45 = 0.0;
	nodeVar46 = 0.0;

	if ( ( nodeVar3.w < 1000.0 ) ) {

		nodeVar43 = ( nodeVar43 + nodeVar3.w );
		nodeVar44 = ( nodeVar44 + 1.0 );
		nodeVar47 = ( nodeVar3.w - nodeVar45 );
		nodeVar45 = ( nodeVar45 + ( nodeVar47 / nodeVar44 ) );
		nodeVar46 = ( nodeVar46 + ( nodeVar47 * ( nodeVar3.w - nodeVar45 ) ) );
		

	}

	nodeVar48 = textureLoad( nodeUniform1, ( nodeConst1 + vec2<i32>( -1, -1 ) ), u32( 0u ) );
	nodeVar49 = max( nodeVar48, vec4<f32>( 0.0 ) );
	let nodeConst37 = ( nodeVar49.xyz / vec3<f32>( ( ( ( dot( nodeVar49.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) ) * object.nodeUniform17 ) * 10.0 ) + 1.0 ) ) );
	let nodeConst38 = vec3<f32>( dot( nodeConst37, vec3<f32>( 0.25, 0.5, 0.25 ) ), dot( nodeConst37, vec3<f32>( 0.5, 0.0, -0.5 ) ), dot( nodeConst37, vec3<f32>( -0.25, 0.5, -0.25 ) ) );
	nodeVar41 = ( nodeVar41 + nodeConst38 );
	nodeVar42 = ( nodeVar42 + ( nodeConst38 * nodeConst38 ) );

	if ( ( nodeVar49.w < 1000.0 ) ) {

		nodeVar43 = ( nodeVar43 + nodeVar49.w );
		nodeVar44 = ( nodeVar44 + 1.0 );
		nodeVar50 = ( nodeVar49.w - nodeVar45 );
		nodeVar45 = ( nodeVar45 + ( nodeVar50 / nodeVar44 ) );
		nodeVar46 = ( nodeVar46 + ( nodeVar50 * ( nodeVar49.w - nodeVar45 ) ) );
		

	}

	nodeVar51 = textureLoad( nodeUniform1, ( nodeConst1 + vec2<i32>( -1, 1 ) ), u32( 0u ) );
	nodeVar52 = max( nodeVar51, vec4<f32>( 0.0 ) );
	let nodeConst39 = ( nodeVar52.xyz / vec3<f32>( ( ( ( dot( nodeVar52.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) ) * object.nodeUniform17 ) * 10.0 ) + 1.0 ) ) );
	let nodeConst40 = vec3<f32>( dot( nodeConst39, vec3<f32>( 0.25, 0.5, 0.25 ) ), dot( nodeConst39, vec3<f32>( 0.5, 0.0, -0.5 ) ), dot( nodeConst39, vec3<f32>( -0.25, 0.5, -0.25 ) ) );
	nodeVar41 = ( nodeVar41 + nodeConst40 );
	nodeVar42 = ( nodeVar42 + ( nodeConst40 * nodeConst40 ) );

	if ( ( nodeVar52.w < 1000.0 ) ) {

		nodeVar43 = ( nodeVar43 + nodeVar52.w );
		nodeVar44 = ( nodeVar44 + 1.0 );
		nodeVar53 = ( nodeVar52.w - nodeVar45 );
		nodeVar45 = ( nodeVar45 + ( nodeVar53 / nodeVar44 ) );
		nodeVar46 = ( nodeVar46 + ( nodeVar53 * ( nodeVar52.w - nodeVar45 ) ) );
		

	}

	nodeVar54 = textureLoad( nodeUniform1, ( nodeConst1 + vec2<i32>( 1, -1 ) ), u32( 0u ) );
	nodeVar55 = max( nodeVar54, vec4<f32>( 0.0 ) );
	let nodeConst41 = ( nodeVar55.xyz / vec3<f32>( ( ( ( dot( nodeVar55.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) ) * object.nodeUniform17 ) * 10.0 ) + 1.0 ) ) );
	let nodeConst42 = vec3<f32>( dot( nodeConst41, vec3<f32>( 0.25, 0.5, 0.25 ) ), dot( nodeConst41, vec3<f32>( 0.5, 0.0, -0.5 ) ), dot( nodeConst41, vec3<f32>( -0.25, 0.5, -0.25 ) ) );
	nodeVar41 = ( nodeVar41 + nodeConst42 );
	nodeVar42 = ( nodeVar42 + ( nodeConst42 * nodeConst42 ) );

	if ( ( nodeVar55.w < 1000.0 ) ) {

		nodeVar43 = ( nodeVar43 + nodeVar55.w );
		nodeVar44 = ( nodeVar44 + 1.0 );
		nodeVar56 = ( nodeVar55.w - nodeVar45 );
		nodeVar45 = ( nodeVar45 + ( nodeVar56 / nodeVar44 ) );
		nodeVar46 = ( nodeVar46 + ( nodeVar56 * ( nodeVar55.w - nodeVar45 ) ) );
		

	}

	nodeVar57 = textureLoad( nodeUniform1, ( nodeConst1 + vec2<i32>( 1, 1 ) ), u32( 0u ) );
	nodeVar58 = max( nodeVar57, vec4<f32>( 0.0 ) );
	let nodeConst43 = ( nodeVar58.xyz / vec3<f32>( ( ( ( dot( nodeVar58.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) ) * object.nodeUniform17 ) * 10.0 ) + 1.0 ) ) );
	let nodeConst44 = vec3<f32>( dot( nodeConst43, vec3<f32>( 0.25, 0.5, 0.25 ) ), dot( nodeConst43, vec3<f32>( 0.5, 0.0, -0.5 ) ), dot( nodeConst43, vec3<f32>( -0.25, 0.5, -0.25 ) ) );
	nodeVar41 = ( nodeVar41 + nodeConst44 );
	nodeVar42 = ( nodeVar42 + ( nodeConst44 * nodeConst44 ) );

	if ( ( nodeVar58.w < 1000.0 ) ) {

		nodeVar43 = ( nodeVar43 + nodeVar58.w );
		nodeVar44 = ( nodeVar44 + 1.0 );
		nodeVar59 = ( nodeVar58.w - nodeVar45 );
		nodeVar45 = ( nodeVar45 + ( nodeVar59 / nodeVar44 ) );
		nodeVar46 = ( nodeVar46 + ( nodeVar59 * ( nodeVar58.w - nodeVar45 ) ) );
		

	}

	nodeVar60 = textureLoad( nodeUniform1, ( nodeConst1 + vec2<i32>( 1, 0 ) ), u32( 0u ) );
	nodeVar61 = max( nodeVar60, vec4<f32>( 0.0 ) );
	let nodeConst45 = ( nodeVar61.xyz / vec3<f32>( ( ( ( dot( nodeVar61.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) ) * object.nodeUniform17 ) * 10.0 ) + 1.0 ) ) );
	let nodeConst46 = vec3<f32>( dot( nodeConst45, vec3<f32>( 0.25, 0.5, 0.25 ) ), dot( nodeConst45, vec3<f32>( 0.5, 0.0, -0.5 ) ), dot( nodeConst45, vec3<f32>( -0.25, 0.5, -0.25 ) ) );
	nodeVar41 = ( nodeVar41 + nodeConst46 );
	nodeVar42 = ( nodeVar42 + ( nodeConst46 * nodeConst46 ) );

	if ( ( nodeVar61.w < 1000.0 ) ) {

		nodeVar43 = ( nodeVar43 + nodeVar61.w );
		nodeVar44 = ( nodeVar44 + 1.0 );
		nodeVar62 = ( nodeVar61.w - nodeVar45 );
		nodeVar45 = ( nodeVar45 + ( nodeVar62 / nodeVar44 ) );
		nodeVar46 = ( nodeVar46 + ( nodeVar62 * ( nodeVar61.w - nodeVar45 ) ) );
		

	}

	nodeVar63 = textureLoad( nodeUniform1, ( nodeConst1 + vec2<i32>( 0, -1 ) ), u32( 0u ) );
	nodeVar64 = max( nodeVar63, vec4<f32>( 0.0 ) );
	let nodeConst47 = ( nodeVar64.xyz / vec3<f32>( ( ( ( dot( nodeVar64.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) ) * object.nodeUniform17 ) * 10.0 ) + 1.0 ) ) );
	let nodeConst48 = vec3<f32>( dot( nodeConst47, vec3<f32>( 0.25, 0.5, 0.25 ) ), dot( nodeConst47, vec3<f32>( 0.5, 0.0, -0.5 ) ), dot( nodeConst47, vec3<f32>( -0.25, 0.5, -0.25 ) ) );
	nodeVar41 = ( nodeVar41 + nodeConst48 );
	nodeVar42 = ( nodeVar42 + ( nodeConst48 * nodeConst48 ) );

	if ( ( nodeVar64.w < 1000.0 ) ) {

		nodeVar43 = ( nodeVar43 + nodeVar64.w );
		nodeVar44 = ( nodeVar44 + 1.0 );
		nodeVar65 = ( nodeVar64.w - nodeVar45 );
		nodeVar45 = ( nodeVar45 + ( nodeVar65 / nodeVar44 ) );
		nodeVar46 = ( nodeVar46 + ( nodeVar65 * ( nodeVar64.w - nodeVar45 ) ) );
		

	}

	nodeVar66 = textureLoad( nodeUniform1, ( nodeConst1 + vec2<i32>( 0, 1 ) ), u32( 0u ) );
	nodeVar67 = max( nodeVar66, vec4<f32>( 0.0 ) );
	let nodeConst49 = ( nodeVar67.xyz / vec3<f32>( ( ( ( dot( nodeVar67.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) ) * object.nodeUniform17 ) * 10.0 ) + 1.0 ) ) );
	let nodeConst50 = vec3<f32>( dot( nodeConst49, vec3<f32>( 0.25, 0.5, 0.25 ) ), dot( nodeConst49, vec3<f32>( 0.5, 0.0, -0.5 ) ), dot( nodeConst49, vec3<f32>( -0.25, 0.5, -0.25 ) ) );
	nodeVar41 = ( nodeVar41 + nodeConst50 );
	nodeVar42 = ( nodeVar42 + ( nodeConst50 * nodeConst50 ) );

	if ( ( nodeVar67.w < 1000.0 ) ) {

		nodeVar43 = ( nodeVar43 + nodeVar67.w );
		nodeVar44 = ( nodeVar44 + 1.0 );
		nodeVar68 = ( nodeVar67.w - nodeVar45 );
		nodeVar45 = ( nodeVar45 + ( nodeVar68 / nodeVar44 ) );
		nodeVar46 = ( nodeVar46 + ( nodeVar68 * ( nodeVar67.w - nodeVar45 ) ) );
		

	}

	nodeVar69 = textureLoad( nodeUniform1, ( nodeConst1 + vec2<i32>( -1, 0 ) ), u32( 0u ) );
	nodeVar70 = max( nodeVar69, vec4<f32>( 0.0 ) );
	let nodeConst51 = ( nodeVar70.xyz / vec3<f32>( ( ( ( dot( nodeVar70.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) ) * object.nodeUniform17 ) * 10.0 ) + 1.0 ) ) );
	let nodeConst52 = vec3<f32>( dot( nodeConst51, vec3<f32>( 0.25, 0.5, 0.25 ) ), dot( nodeConst51, vec3<f32>( 0.5, 0.0, -0.5 ) ), dot( nodeConst51, vec3<f32>( -0.25, 0.5, -0.25 ) ) );
	nodeVar41 = ( nodeVar41 + nodeConst52 );
	nodeVar42 = ( nodeVar42 + ( nodeConst52 * nodeConst52 ) );

	if ( ( nodeVar70.w < 1000.0 ) ) {

		nodeVar43 = ( nodeVar43 + nodeVar70.w );
		nodeVar44 = ( nodeVar44 + 1.0 );
		nodeVar71 = ( nodeVar70.w - nodeVar45 );
		nodeVar45 = ( nodeVar45 + ( nodeVar71 / nodeVar44 ) );
		nodeVar46 = ( nodeVar46 + ( nodeVar71 * ( nodeVar70.w - nodeVar45 ) ) );
		

	}

	let nodeConst53 = ( nodeVar41 / vec3<f32>( 9.0 ) );

	if ( ( nodeVar44 < 0.5 ) ) {

		nodeVar73 = 10000.0;

	} else {

		nodeVar73 = ( nodeVar43 / max( nodeVar44, 0.0001 ) );

	}

	nodeVar72 = StructType2( nodeConst53, sqrt( max( ( ( nodeVar42 / vec3<f32>( 9.0 ) ) - ( nodeConst53 * nodeConst53 ) ), vec3<f32>( 0.0 ) ) ), nodeVar73, ( 1.0 - ( nodeVar44 / 9.0 ) ), max( sqrt( ( nodeVar46 / max( nodeVar44, 1.0 ) ) ), 0.001 ) );
	let nodeConst54 = clamp( ( length( ( nodeVar10 * object.nodeUniform2 ) ) / 128.0 ), 0.0, 1.0 );
	let nodeConst55 = ( 1.0 - nodeConst54 );
	let nodeConst56 = ( nodeVar72.stdColor * vec3<f32>( mix( 0.5, 1.0, ( nodeConst55 * nodeConst55 ) ) ) );
	let nodeConst57 = clipToAABB( vec3<f32>( dot( nodeConst34, vec3<f32>( 0.25, 0.5, 0.25 ) ), dot( nodeConst34, vec3<f32>( 0.5, 0.0, -0.5 ) ), dot( nodeConst34, vec3<f32>( -0.25, 0.5, -0.25 ) ) ), ( nodeVar72.mean - nodeConst56 ), ( nodeVar72.mean + nodeConst56 ) );
	nodeVar74 = ( vec3<f32>( ( ( nodeConst57.x + nodeConst57.y ) - nodeConst57.z ), ( nodeConst57.x + nodeConst57.z ), ( ( nodeConst57.x - nodeConst57.y ) - nodeConst57.z ) ) * vec3<f32>( nodeConst33 ) );
	let nodeConst58 = ( ( object.nodeUniform18 * max( min( ( nodeConst54 * 10.0 ), 1.0 ), 0.25 ) ) * ( 1.0 + clamp( ( ( 1.0 - nodeConst32 ) + ( 1.0 - nodeVar37 ) ), 0.0, 1.0 ) ) );
	nodeVar75 = mix( nodeVar35.xyz, nodeVar74, nodeConst58 );
	nodeVar35.x = nodeVar75[ 0 ];
	nodeVar35.y = nodeVar75[ 1 ];
	nodeVar35.z = nodeVar75[ 2 ];
	nodeVar36 = ( nodeVar36 * exp( ( - ( ( length( ( nodeVar35.xyz - nodeVar74 ) ) * nodeConst58 ) * 30.0 ) ) ) );
	nodeVar36 = ( nodeVar36 * mix( 1.0, ( ( nodeVar37 * 0.05 ) + 0.95 ), clamp( ( nodeConst54 * 100.0 ), 0.0, 1.0 ) ) );

	if ( ( nodeVar36 < 0.000001 ) ) {

		nodeVar35 = vec4<f32>( nodeVar3.xyz, 1.0 );
		

	}

	nodeVar76 = min( ( ( ( 1.0 / max( nodeVar35.w, 0.000001 ) ) * nodeVar36 ) + 1.0 ), object.nodeUniform19 );

	// result

	output.color = vec4<f32>( nodeVar35.xyz, ( 1.0 / nodeVar76 ) );

	return output;

}
