// Three.js r186dev - Node System

// global
diagnostic( off, derivative_uniformity );


// structs

struct OutputStruct {
	@location( 0 ) color: vec4<f32>
};
var<private> output : OutputStruct;

// uniforms
@binding( 1 ) @group( 1 ) var nodeUniform15_sampler : sampler_comparison;
@binding( 2 ) @group( 1 ) var nodeUniform15 : texture_depth_2d;
@binding( 3 ) @group( 1 ) var nodeUniform31_sampler : sampler_comparison;
@binding( 4 ) @group( 1 ) var nodeUniform31 : texture_depth_2d;

struct objectStruct {
	nodeUniform0 : mat4x4<f32>,
	nodeUniform1 : f32,
	nodeUniform2 : f32,
	nodeUniform3 : vec3<f32>,
	nodeUniform4 : vec3<f32>,
	nodeUniform5 : f32,
	nodeUniform8 : mat3x3<f32>
};
@binding( 0 ) @group( 1 )
var<uniform> object : objectStruct;

struct renderStruct {
	cameraProjectionMatrix : mat4x4<f32>,
	cameraViewMatrix : mat4x4<f32>,
	nodeUniform6 : vec3<f32>,
	nodeUniform19 : f32,
	nodeUniform20 : f32,
	nodeUniform23 : f32,
	nodeUniform24 : f32,
	nodeUniform10 : vec3<f32>,
	nodeUniform27 : vec3<f32>,
	nodeUniform9 : vec3<f32>,
	nodeUniform21 : vec3<f32>,
	nodeUniform22 : vec3<f32>,
	nodeUniform25 : vec3<f32>,
	nodeUniform26 : vec3<f32>,
	nodeUniform35 : vec3<f32>,
	nodeUniform36 : f32,
	nodeUniform37 : f32,
	nodeUniform11 : mat4x4<f32>,
	nodeUniform13 : f32,
	nodeUniform14 : f32,
	nodeUniform16 : f32,
	nodeUniform17 : vec2<f32>,
	nodeUniform18 : f32,
	nodeUniform28 : mat4x4<f32>,
	nodeUniform29 : f32,
	nodeUniform30 : f32,
	nodeUniform32 : f32,
	nodeUniform33 : vec2<f32>,
	nodeUniform34 : f32
};
@binding( 0 ) @group( 0 )
var<uniform> render : renderStruct;

// vars
var<private> DiffuseColor : vec4<f32>;
var<private> nodeVar0 : vec3<f32>;
var<private> nodeVar1 : vec2<f32>;
var<private> Shininess : f32;
var<private> SpecularColor : vec3<f32>;
var<private> EmissiveColor : vec3<f32>;
var<private> Output : vec4<f32>;
var<private> irradiance : vec3<f32>;
var<private> nodeVar2 : vec3<f32>;
var<private> directDiffuse : vec3<f32>;
var<private> normalViewGeometry : vec3<f32>;
var<private> NORMAL_normalView : vec3<f32>;
var<private> normalView : vec3<f32>;
var<private> nodeVar3 : vec3<f32>;
var<private> nodeVar4 : vec3<f32>;
var<private> nodeVar5 : f32;
var<private> shadowPositionWorld : vec3<f32>;
var<private> nodeVar6 : vec3<f32>;
var<private> nodeVar7 : vec2<f32>;
var<private> nodeVar8 : f32;
var<private> normalWorld : vec3<f32>;
var<private> nodeVar9 : vec4<f32>;
var<private> nodeVar10 : vec3<f32>;
var<private> nodeVar11 : vec3<f32>;
var<private> nodeVar12 : f32;
var<private> nodeVar13 : f32;
var<private> nodeVar14 : vec2<f32>;
var<private> nodeVar15 : f32;
var<private> nodeVar16 : vec2<f32>;
var<private> nodeVar17 : f32;
var<private> nodeVar18 : vec2<f32>;
var<private> nodeVar19 : f32;
var<private> nodeVar20 : vec2<f32>;
var<private> nodeVar21 : f32;
var<private> nodeVar22 : vec2<f32>;
var<private> nodeVar23 : f32;
var<private> nodeVar24 : f32;
var<private> nodeVar25 : vec3<f32>;
var<private> nodeVar26 : vec3<f32>;
var<private> nodeVar27 : vec4<f32>;
var<private> nodeVar28 : vec4<f32>;
var<private> nodeVar29 : vec3<f32>;
var<private> nodeVar30 : vec3<f32>;
var<private> nodeVar31 : f32;
var<private> nodeVar32 : f32;
var<private> nodeVar33 : vec3<f32>;
var<private> nodeVar34 : f32;
var<private> nodeVar35 : f32;
var<private> nodeVar36 : f32;
var<private> nodeVar37 : f32;
var<private> nodeVar38 : vec3<f32>;
var<private> nodeVar39 : vec3<f32>;
var<private> nodeVar40 : vec3<f32>;
var<private> nodeVar41 : vec3<f32>;
var<private> nodeVar42 : vec3<f32>;
var<private> directSpecular : vec3<f32>;
var<private> positionViewDirection : vec3<f32>;
var<private> nodeVar43 : vec3<f32>;
var<private> nodeVar44 : f32;
var<private> nodeVar45 : f32;
var<private> nodeVar46 : vec3<f32>;
var<private> nodeVar47 : vec3<f32>;
var<private> nodeVar48 : vec3<f32>;
var<private> nodeVar49 : vec3<f32>;
var<private> nodeVar50 : vec3<f32>;
var<private> nodeVar51 : vec4<f32>;
var<private> nodeVar52 : vec4<f32>;
var<private> nodeVar53 : vec3<f32>;
var<private> nodeVar54 : vec3<f32>;
var<private> nodeVar55 : f32;
var<private> nodeVar56 : f32;
var<private> nodeVar57 : vec4<f32>;
var<private> nodeVar58 : vec3<f32>;
var<private> nodeVar59 : vec3<f32>;
var<private> nodeVar60 : f32;
var<private> nodeVar61 : f32;
var<private> nodeVar62 : vec2<f32>;
var<private> nodeVar63 : f32;
var<private> nodeVar64 : vec2<f32>;
var<private> nodeVar65 : f32;
var<private> nodeVar66 : vec2<f32>;
var<private> nodeVar67 : f32;
var<private> nodeVar68 : vec2<f32>;
var<private> nodeVar69 : f32;
var<private> nodeVar70 : vec2<f32>;
var<private> nodeVar71 : f32;
var<private> nodeVar72 : f32;
var<private> nodeVar73 : vec3<f32>;
var<private> nodeVar74 : vec3<f32>;
var<private> nodeVar75 : vec3<f32>;
var<private> nodeVar76 : vec3<f32>;
var<private> nodeVar77 : vec3<f32>;
var<private> nodeVar78 : vec3<f32>;
var<private> nodeVar79 : f32;
var<private> nodeVar80 : f32;
var<private> nodeVar81 : vec3<f32>;
var<private> nodeVar82 : vec3<f32>;
var<private> nodeVar83 : vec3<f32>;
var<private> nodeVar84 : vec3<f32>;
var<private> indirectDiffuse : vec3<f32>;
var<private> nodeVar85 : vec4<f32>;
var<private> nodeVar86 : vec4<f32>;
var<private> nodeVar87 : vec4<f32>;
var<private> ambientOcclusion : f32;
var<private> nodeVar88 : vec3<f32>;
var<private> totalDiffuse : vec3<f32>;
var<private> nodeVar89 : vec3<f32>;
var<private> totalSpecular : vec3<f32>;
var<private> indirectSpecular : vec3<f32>;
var<private> nodeVar90 : vec3<f32>;
var<private> outgoingLight : vec3<f32>;
var<private> nodeVar91 : vec3<f32>;
var<private> nodeVar92 : vec4<f32>;

// codes
fn mx_rotl32 ( x : u32, k : i32 ) -> u32 {

	var nodeVar0 : i32;
	var nodeVar1 : u32;

	nodeVar0 = k;
	nodeVar1 = x;

	return ( ( nodeVar1 << u32( nodeVar0 ) ) | ( nodeVar1 >> u32( ( 32 - nodeVar0 ) ) ) );

}


fn mx_bjfinal ( a : u32, b : u32, c : u32 ) -> u32 {

	var nodeVar0 : u32;
	var nodeVar1 : u32;
	var nodeVar2 : u32;

	nodeVar0 = c;
	nodeVar1 = b;
	nodeVar2 = a;
	nodeVar0 = ( nodeVar0 ^ nodeVar1 );
	nodeVar0 = ( nodeVar0 - mx_rotl32( nodeVar1, 14 ) );
	nodeVar2 = ( nodeVar2 ^ nodeVar0 );
	nodeVar2 = ( nodeVar2 - mx_rotl32( nodeVar0, 11 ) );
	nodeVar1 = ( nodeVar1 ^ nodeVar2 );
	nodeVar1 = ( nodeVar1 - mx_rotl32( nodeVar2, 25 ) );
	nodeVar0 = ( nodeVar0 ^ nodeVar1 );
	nodeVar0 = ( nodeVar0 - mx_rotl32( nodeVar1, 16 ) );
	nodeVar2 = ( nodeVar2 ^ nodeVar0 );
	nodeVar2 = ( nodeVar2 - mx_rotl32( nodeVar0, 4 ) );
	nodeVar1 = ( nodeVar1 ^ nodeVar2 );
	nodeVar1 = ( nodeVar1 - mx_rotl32( nodeVar2, 14 ) );
	nodeVar0 = ( nodeVar0 ^ nodeVar1 );
	nodeVar0 = ( nodeVar0 - mx_rotl32( nodeVar1, 24 ) );

	return nodeVar0;

}


fn mx_hash_int_2 ( x : i32, y : i32, z : i32 ) -> u32 {

	var nodeVar0 : i32;
	var nodeVar1 : i32;
	var nodeVar2 : i32;
	var nodeVar3 : u32;
	var nodeVar4 : u32;
	var nodeVar5 : u32;
	var nodeVar6 : u32;

	nodeVar0 = z;
	nodeVar1 = y;
	nodeVar2 = x;
	nodeVar3 = 3u;
	nodeVar4 = 0u;
	nodeVar5 = 0u;
	nodeVar6 = 0u;
	nodeVar6 = ( ( 3735928559u + ( nodeVar3 << 2u ) ) + 13u );
	nodeVar5 = nodeVar6;
	nodeVar4 = nodeVar5;
	nodeVar4 = ( nodeVar4 + u32( nodeVar2 ) );
	nodeVar5 = ( nodeVar5 + u32( nodeVar1 ) );
	nodeVar6 = ( nodeVar6 + u32( nodeVar0 ) );

	return mx_bjfinal( nodeVar4, nodeVar5, nodeVar6 );

}


fn mx_select ( b : bool, t : f32, f : f32 ) -> f32 {

	var nodeVar0 : f32;
	var nodeVar1 : f32;
	var nodeVar2 : bool;
	var nodeVar3 : f32;

	nodeVar0 = f;
	nodeVar1 = t;
	nodeVar2 = b;

	return select( nodeVar0, nodeVar1, nodeVar2 );

}


fn mx_negate_if ( val : f32, b : bool ) -> f32 {

	var nodeVar0 : bool;
	var nodeVar1 : f32;
	var nodeVar2 : f32;

	nodeVar0 = b;
	nodeVar1 = val;

	return select( nodeVar1, ( - nodeVar1 ), nodeVar0 );

}


fn mx_gradient_float_1 ( hash : u32, x : f32, y : f32, z : f32 ) -> f32 {

	var nodeVar0 : f32;
	var nodeVar1 : f32;
	var nodeVar2 : f32;
	var nodeVar3 : u32;
	var nodeVar4 : u32;
	var nodeVar5 : f32;
	var nodeVar6 : f32;

	nodeVar0 = z;
	nodeVar1 = y;
	nodeVar2 = x;
	nodeVar3 = hash;
	nodeVar4 = ( nodeVar3 & 15u );
	nodeVar5 = mx_select( ( nodeVar4 < 8u ), nodeVar2, nodeVar1 );
	nodeVar6 = mx_select( ( nodeVar4 < 4u ), nodeVar1, mx_select( ( ( nodeVar4 == 12u ) || ( nodeVar4 == 14u ) ), nodeVar2, nodeVar0 ) );

	return ( mx_negate_if( nodeVar5, bool( ( nodeVar4 & 1u ) ) ) + mx_negate_if( nodeVar6, bool( ( nodeVar4 & 2u ) ) ) );

}


fn mx_floor ( x : f32 ) -> i32 {

	var nodeVar0 : f32;

	nodeVar0 = x;

	return i32( floor( nodeVar0 ) );

}


fn mx_fade ( t : f32 ) -> f32 {

	var nodeVar0 : f32;

	nodeVar0 = t;

	return ( ( ( nodeVar0 * nodeVar0 ) * nodeVar0 ) * ( ( nodeVar0 * ( ( nodeVar0 * 6.0 ) - 15.0 ) ) + 10.0 ) );

}


fn mx_hash_vec3_1 ( x : i32, y : i32, z : i32 ) -> vec3<u32> {

	var nodeVar0 : i32;
	var nodeVar1 : i32;
	var nodeVar2 : i32;
	var nodeVar3 : u32;
	var nodeVar4 : vec3<u32>;

	nodeVar0 = z;
	nodeVar1 = y;
	nodeVar2 = x;
	nodeVar3 = mx_hash_int_2( nodeVar2, nodeVar1, nodeVar0 );
	nodeVar4 = vec3<u32>( 0u, 0u, 0u );
	nodeVar4.x = ( nodeVar3 & 255u );
	nodeVar4.y = ( ( nodeVar3 >> 8u ) & 255u );
	nodeVar4.z = ( ( nodeVar3 >> 16u ) & 255u );

	return nodeVar4;

}


fn mx_gradient_vec3_1 ( hash : vec3<u32>, x : f32, y : f32, z : f32 ) -> vec3<f32> {

	var nodeVar0 : f32;
	var nodeVar1 : f32;
	var nodeVar2 : f32;
	var nodeVar3 : vec3<u32>;

	nodeVar0 = z;
	nodeVar1 = y;
	nodeVar2 = x;
	nodeVar3 = hash;

	return vec3<f32>( mx_gradient_float_1( nodeVar3.x, nodeVar2, nodeVar1, nodeVar0 ), mx_gradient_float_1( nodeVar3.y, nodeVar2, nodeVar1, nodeVar0 ), mx_gradient_float_1( nodeVar3.z, nodeVar2, nodeVar1, nodeVar0 ) );

}


fn mx_trilerp_1 ( v0 : vec3<f32>, v1 : vec3<f32>, v2 : vec3<f32>, v3 : vec3<f32>, v4 : vec3<f32>, v5 : vec3<f32>, v6 : vec3<f32>, v7 : vec3<f32>, s : f32, t : f32, r : f32 ) -> vec3<f32> {

	var nodeVar0 : f32;
	var nodeVar1 : f32;
	var nodeVar2 : f32;
	var nodeVar3 : vec3<f32>;
	var nodeVar4 : vec3<f32>;
	var nodeVar5 : vec3<f32>;
	var nodeVar6 : vec3<f32>;
	var nodeVar7 : vec3<f32>;
	var nodeVar8 : vec3<f32>;
	var nodeVar9 : vec3<f32>;
	var nodeVar10 : vec3<f32>;
	var nodeVar11 : f32;
	var nodeVar12 : f32;
	var nodeVar13 : f32;

	nodeVar0 = r;
	nodeVar1 = t;
	nodeVar2 = s;
	nodeVar3 = v7;
	nodeVar4 = v6;
	nodeVar5 = v5;
	nodeVar6 = v4;
	nodeVar7 = v3;
	nodeVar8 = v2;
	nodeVar9 = v1;
	nodeVar10 = v0;
	nodeVar11 = ( 1.0 - nodeVar2 );
	nodeVar12 = ( 1.0 - nodeVar1 );
	nodeVar13 = ( 1.0 - nodeVar0 );

	return ( ( vec3<f32>( nodeVar13 ) * ( ( vec3<f32>( nodeVar12 ) * ( ( nodeVar10 * vec3<f32>( nodeVar11 ) ) + ( nodeVar9 * vec3<f32>( nodeVar2 ) ) ) ) + ( vec3<f32>( nodeVar1 ) * ( ( nodeVar8 * vec3<f32>( nodeVar11 ) ) + ( nodeVar7 * vec3<f32>( nodeVar2 ) ) ) ) ) ) + ( vec3<f32>( nodeVar0 ) * ( ( vec3<f32>( nodeVar12 ) * ( ( nodeVar6 * vec3<f32>( nodeVar11 ) ) + ( nodeVar5 * vec3<f32>( nodeVar2 ) ) ) ) + ( vec3<f32>( nodeVar1 ) * ( ( nodeVar4 * vec3<f32>( nodeVar11 ) ) + ( nodeVar3 * vec3<f32>( nodeVar2 ) ) ) ) ) ) );

}


fn mx_gradient_scale3d_1 ( v : vec3<f32> ) -> vec3<f32> {

	var nodeVar0 : vec3<f32>;

	nodeVar0 = v;

	return ( vec3<f32>( 0.982 ) * nodeVar0 );

}


fn mx_perlin_noise_vec3_1 ( p : vec3<f32> ) -> vec3<f32> {

	var nodeVar0 : vec3<f32>;
	var nodeVar1 : i32;
	var nodeVar2 : i32;
	var nodeVar3 : i32;
	var nodeVar4 : f32;
	var nodeVar5 : f32;
	var nodeVar6 : f32;
	var nodeVar7 : f32;
	var nodeVar8 : f32;
	var nodeVar9 : f32;
	var nodeVar10 : f32;
	var nodeVar11 : f32;
	var nodeVar12 : f32;
	var nodeVar13 : vec3<f32>;

	nodeVar0 = p;
	nodeVar1 = 0;
	nodeVar2 = 0;
	nodeVar3 = 0;
	nodeVar4 = nodeVar0.x;
	nodeVar1 = mx_floor( nodeVar4 );
	nodeVar5 = ( nodeVar4 - f32( nodeVar1 ) );
	nodeVar6 = nodeVar0.y;
	nodeVar2 = mx_floor( nodeVar6 );
	nodeVar7 = ( nodeVar6 - f32( nodeVar2 ) );
	nodeVar8 = nodeVar0.z;
	nodeVar3 = mx_floor( nodeVar8 );
	nodeVar9 = ( nodeVar8 - f32( nodeVar3 ) );
	nodeVar10 = mx_fade( nodeVar5 );
	nodeVar11 = mx_fade( nodeVar7 );
	nodeVar12 = mx_fade( nodeVar9 );
	nodeVar13 = mx_trilerp_1( mx_gradient_vec3_1( mx_hash_vec3_1( nodeVar1, nodeVar2, nodeVar3 ), nodeVar5, nodeVar7, nodeVar9 ), mx_gradient_vec3_1( mx_hash_vec3_1( ( nodeVar1 + 1 ), nodeVar2, nodeVar3 ), ( nodeVar5 - 1.0 ), nodeVar7, nodeVar9 ), mx_gradient_vec3_1( mx_hash_vec3_1( nodeVar1, ( nodeVar2 + 1 ), nodeVar3 ), nodeVar5, ( nodeVar7 - 1.0 ), nodeVar9 ), mx_gradient_vec3_1( mx_hash_vec3_1( ( nodeVar1 + 1 ), ( nodeVar2 + 1 ), nodeVar3 ), ( nodeVar5 - 1.0 ), ( nodeVar7 - 1.0 ), nodeVar9 ), mx_gradient_vec3_1( mx_hash_vec3_1( nodeVar1, nodeVar2, ( nodeVar3 + 1 ) ), nodeVar5, nodeVar7, ( nodeVar9 - 1.0 ) ), mx_gradient_vec3_1( mx_hash_vec3_1( ( nodeVar1 + 1 ), nodeVar2, ( nodeVar3 + 1 ) ), ( nodeVar5 - 1.0 ), nodeVar7, ( nodeVar9 - 1.0 ) ), mx_gradient_vec3_1( mx_hash_vec3_1( nodeVar1, ( nodeVar2 + 1 ), ( nodeVar3 + 1 ) ), nodeVar5, ( nodeVar7 - 1.0 ), ( nodeVar9 - 1.0 ) ), mx_gradient_vec3_1( mx_hash_vec3_1( ( nodeVar1 + 1 ), ( nodeVar2 + 1 ), ( nodeVar3 + 1 ) ), ( nodeVar5 - 1.0 ), ( nodeVar7 - 1.0 ), ( nodeVar9 - 1.0 ) ), nodeVar10, nodeVar11, nodeVar12 );

	return mx_gradient_scale3d_1( nodeVar13 );

}


fn mx_fractal_noise_vec3 ( p : vec3<f32>, octaves : i32, lacunarity : f32, diminish : f32 ) -> vec3<f32> {

	var nodeVar0 : f32;
	var nodeVar1 : f32;
	var nodeVar2 : vec3<f32>;
	var nodeVar3 : vec3<f32>;
	var nodeVar4 : f32;
	var nodeVar5 : i32;

	nodeVar0 = diminish;
	nodeVar1 = lacunarity;
	nodeVar2 = p;
	nodeVar3 = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar4 = 1.0;
	nodeVar5 = octaves;

	for ( var i : i32 = 0; i < nodeVar5; i ++ ) {

		nodeVar3 = ( nodeVar3 + ( vec3<f32>( nodeVar4 ) * mx_perlin_noise_vec3_1( nodeVar2 ) ) );
		nodeVar4 = ( nodeVar4 * nodeVar0 );
		nodeVar2 = ( nodeVar2 * vec3<f32>( nodeVar1 ) );

	}


	return nodeVar3;

}


fn interleavedGradientNoise ( position : vec2<f32> ) -> f32 {

	


	return fract( ( 52.9829189 * fract( dot( position, vec2<f32>( 0.06711056, 0.00583715 ) ) ) ) );

}


fn vogelDiskSample ( sampleIndex : i32, samplesCount : i32, phi : f32 ) -> vec2<f32> {

	var nodeVar0 : f32;

	nodeVar0 = ( ( f32( sampleIndex ) * 2.399963229728653 ) + phi );

	return ( vec2<f32>( cos( nodeVar0 ), sin( nodeVar0 ) ) * vec2<f32>( sqrt( ( ( f32( sampleIndex ) + 0.5 ) / f32( samplesCount ) ) ) ) );

}




@fragment
fn main( @location( 0 ) v_positionView : vec3<f32>,
	@location( 1 ) v_positionWorld : vec3<f32>,
	@location( 2 ) v_positionViewDirection : vec3<f32>,
	@location( 3 ) v_normalViewGeometry : vec3<f32>,
	@builtin( position ) fragCoord : vec4<f32> ) -> OutputStruct {

	// flow
	// code

	nodeVar0 = v_positionWorld;
	nodeVar1 = ( nodeVar0.xz + clamp( ( mx_fractal_noise_vec3( ( v_positionWorld * vec3<f32>( 2.0 ) ), 3, 2.0, 0.5 ) * vec3<f32>( 1.0 ) ), vec3<f32>( 0.0 ), vec3<f32>( 1.0 ) ).xz );
	nodeVar0.x = nodeVar1[ 0 ];
	nodeVar0.z = nodeVar1[ 1 ];
	DiffuseColor = vec4<f32>( ( ( clamp( ( mx_fractal_noise_vec3( ( v_positionWorld * vec3<f32>( 2.0 ) ), 3, 2.0, 0.5 ) * vec3<f32>( 1.0 ) ), vec3<f32>( 0.0 ), vec3<f32>( 1.0 ) ).zzz * vec3<f32>( 0.2 ) ) + vec3<f32>( 0.5 ) ), 1.0 );
	DiffuseColor.w = ( DiffuseColor.w * object.nodeUniform1 );
	DiffuseColor.w = 1.0;
	Shininess = max( object.nodeUniform2, 0.0001 );
	SpecularColor = object.nodeUniform3;
	EmissiveColor = ( object.nodeUniform4 * vec3<f32>( object.nodeUniform5 ) );
	irradiance = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar2 = ( irradiance + render.nodeUniform6 );
	irradiance = nodeVar2;
	directDiffuse = vec3<f32>( 0.0, 0.0, 0.0 );
	normalViewGeometry = normalize( v_normalViewGeometry );
	NORMAL_normalView = normalViewGeometry;
	normalView = NORMAL_normalView;
	nodeVar3 = ( render.nodeUniform9 - v_positionView );
	nodeVar4 = normalize( nodeVar3 );
	nodeVar5 = dot( normalView, nodeVar4 );
	nodeVar6 = v_positionWorld;
	nodeVar7 = ( nodeVar6.xz + clamp( ( mx_fractal_noise_vec3( ( v_positionWorld * vec3<f32>( 2.0 ) ), 3, 2.0, 0.5 ) * vec3<f32>( 1.0 ) ), vec3<f32>( 0.0 ), vec3<f32>( 1.0 ) ).xz );
	nodeVar6.x = nodeVar7[ 0 ];
	nodeVar6.z = nodeVar7[ 1 ];
	shadowPositionWorld = nodeVar6;
	normalWorld = normalize( ( vec4<f32>( normalView, 0.0 ) * render.cameraViewMatrix ).xyz );
	nodeVar9 = ( render.nodeUniform11 * vec4<f32>( ( shadowPositionWorld + ( normalWorld * vec3<f32>( render.nodeUniform13 ) ) ), 1.0 ) );
	nodeVar10 = ( nodeVar9.xyz / vec3<f32>( nodeVar9.w ) );
	nodeVar11 = vec3<f32>( nodeVar10.x, ( 1.0 - nodeVar10.y ), ( nodeVar10.z + render.nodeUniform14 ) );

	if ( ( ( ( ( ( nodeVar11.x >= 0.0 ) && ( nodeVar11.x <= 1.0 ) ) && ( nodeVar11.y >= 0.0 ) ) && ( nodeVar11.y <= 1.0 ) ) && ( nodeVar11.z <= 1.0 ) ) ) {

		nodeVar12 = ( interleavedGradientNoise( fragCoord.xy ) * 6.28318530718 );
		nodeVar13 = ( render.nodeUniform16 * ( vec2<f32>( 1.0, 1.0 ) / render.nodeUniform17 ).x );
		nodeVar14 = ( nodeVar11.xy + ( vogelDiskSample( 0, 5, nodeVar12 ) * vec2<f32>( nodeVar13 ) ) );
		nodeVar15 = textureSampleCompare( nodeUniform15, nodeUniform15_sampler, nodeVar14, nodeVar11.z );
		nodeVar16 = ( nodeVar11.xy + ( vogelDiskSample( 1, 5, nodeVar12 ) * vec2<f32>( nodeVar13 ) ) );
		nodeVar17 = textureSampleCompare( nodeUniform15, nodeUniform15_sampler, nodeVar16, nodeVar11.z );
		nodeVar18 = ( nodeVar11.xy + ( vogelDiskSample( 2, 5, nodeVar12 ) * vec2<f32>( nodeVar13 ) ) );
		nodeVar19 = textureSampleCompare( nodeUniform15, nodeUniform15_sampler, nodeVar18, nodeVar11.z );
		nodeVar20 = ( nodeVar11.xy + ( vogelDiskSample( 3, 5, nodeVar12 ) * vec2<f32>( nodeVar13 ) ) );
		nodeVar21 = textureSampleCompare( nodeUniform15, nodeUniform15_sampler, nodeVar20, nodeVar11.z );
		nodeVar22 = ( nodeVar11.xy + ( vogelDiskSample( 4, 5, nodeVar12 ) * vec2<f32>( nodeVar13 ) ) );
		nodeVar23 = textureSampleCompare( nodeUniform15, nodeUniform15_sampler, nodeVar22, nodeVar11.z );
		nodeVar8 = ( ( ( ( ( nodeVar15 + nodeVar17 ) + nodeVar19 ) + nodeVar21 ) + nodeVar23 ) * 0.2 );

	} else {

		nodeVar8 = 1.0;

	}

	nodeVar24 = mix( 1.0, nodeVar8, render.nodeUniform18 );
	nodeVar25 = ( render.nodeUniform10 * vec3<f32>( nodeVar24 ) );
	nodeVar26 = ( render.nodeUniform21 - render.nodeUniform22 );
	nodeVar27 = vec4<f32>( nodeVar26, 0.0 );
	nodeVar28 = ( render.cameraViewMatrix * nodeVar27 );
	nodeVar29 = normalize( nodeVar28.xyz );
	nodeVar30 = nodeVar29;
	nodeVar31 = dot( nodeVar4, nodeVar30 );
	nodeVar32 = smoothstep( render.nodeUniform19, render.nodeUniform20, nodeVar31 );
	nodeVar33 = ( nodeVar25 * vec3<f32>( nodeVar32 ) );

	if ( ( render.nodeUniform23 > 0.0 ) ) {

		nodeVar35 = length( nodeVar3 );
		nodeVar36 = ( nodeVar35 / render.nodeUniform23 );
		nodeVar37 = clamp( ( 1.0 - ( ( ( nodeVar36 * nodeVar36 ) * nodeVar36 ) * nodeVar36 ) ), 0.0, 1.0 );
		nodeVar34 = ( ( 1.0 / max( pow( nodeVar35, render.nodeUniform24 ), 0.01 ) ) * ( nodeVar37 * nodeVar37 ) );

	} else {

		nodeVar34 = ( 1.0 / max( pow( length( nodeVar3 ), render.nodeUniform24 ), 0.01 ) );

	}

	nodeVar38 = ( nodeVar33 * vec3<f32>( nodeVar34 ) );
	nodeVar39 = ( vec3<f32>( clamp( nodeVar5, 0.0, 1.0 ) ) * nodeVar38 );
	nodeVar40 = ( DiffuseColor.xyz * vec3<f32>( 0.3183098861837907 ) );
	nodeVar41 = ( nodeVar39 * nodeVar40 );
	nodeVar42 = ( directDiffuse + nodeVar41 );
	directDiffuse = nodeVar42;
	directSpecular = vec3<f32>( 0.0, 0.0, 0.0 );
	positionViewDirection = normalize( v_positionViewDirection );
	nodeVar43 = normalize( ( nodeVar4 + positionViewDirection ) );
	nodeVar44 = clamp( dot( positionViewDirection, nodeVar43 ), 0.0, 1.0 );
	nodeVar45 = exp2( ( ( ( nodeVar44 * -5.55473 ) - 6.98316 ) * nodeVar44 ) );
	nodeVar46 = ( ( ( ( SpecularColor * vec3<f32>( ( 1.0 - nodeVar45 ) ) ) + vec3<f32>( ( 1.0 * nodeVar45 ) ) ) * vec3<f32>( 0.25 ) ) * vec3<f32>( ( ( ( ( Shininess * 0.5 ) + 1.0 ) * 0.3183098861837907 ) * pow( clamp( dot( normalView, nodeVar43 ), 0.0, 1.0 ), Shininess ) ) ) );
	nodeVar47 = ( nodeVar39 * nodeVar46 );
	nodeVar48 = ( nodeVar47 * vec3<f32>( 1.0 ) );
	nodeVar49 = ( directSpecular + nodeVar48 );
	directSpecular = nodeVar49;
	nodeVar50 = ( render.nodeUniform25 - render.nodeUniform26 );
	nodeVar51 = vec4<f32>( nodeVar50, 0.0 );
	nodeVar52 = ( render.cameraViewMatrix * nodeVar51 );
	nodeVar53 = normalize( nodeVar52.xyz );
	nodeVar54 = nodeVar53;
	nodeVar55 = dot( normalView, nodeVar54 );
	shadowPositionWorld = nodeVar6;
	nodeVar57 = ( render.nodeUniform28 * vec4<f32>( ( shadowPositionWorld + ( normalWorld * vec3<f32>( render.nodeUniform29 ) ) ), 1.0 ) );
	nodeVar58 = ( nodeVar57.xyz / vec3<f32>( nodeVar57.w ) );
	nodeVar59 = vec3<f32>( nodeVar58.x, ( 1.0 - nodeVar58.y ), ( nodeVar58.z + render.nodeUniform30 ) );

	if ( ( ( ( ( ( nodeVar59.x >= 0.0 ) && ( nodeVar59.x <= 1.0 ) ) && ( nodeVar59.y >= 0.0 ) ) && ( nodeVar59.y <= 1.0 ) ) && ( nodeVar59.z <= 1.0 ) ) ) {

		nodeVar60 = ( interleavedGradientNoise( fragCoord.xy ) * 6.28318530718 );
		nodeVar61 = ( render.nodeUniform32 * ( vec2<f32>( 1.0, 1.0 ) / render.nodeUniform33 ).x );
		nodeVar62 = ( nodeVar59.xy + ( vogelDiskSample( 0, 5, nodeVar60 ) * vec2<f32>( nodeVar61 ) ) );
		nodeVar63 = textureSampleCompare( nodeUniform31, nodeUniform31_sampler, nodeVar62, nodeVar59.z );
		nodeVar64 = ( nodeVar59.xy + ( vogelDiskSample( 1, 5, nodeVar60 ) * vec2<f32>( nodeVar61 ) ) );
		nodeVar65 = textureSampleCompare( nodeUniform31, nodeUniform31_sampler, nodeVar64, nodeVar59.z );
		nodeVar66 = ( nodeVar59.xy + ( vogelDiskSample( 2, 5, nodeVar60 ) * vec2<f32>( nodeVar61 ) ) );
		nodeVar67 = textureSampleCompare( nodeUniform31, nodeUniform31_sampler, nodeVar66, nodeVar59.z );
		nodeVar68 = ( nodeVar59.xy + ( vogelDiskSample( 3, 5, nodeVar60 ) * vec2<f32>( nodeVar61 ) ) );
		nodeVar69 = textureSampleCompare( nodeUniform31, nodeUniform31_sampler, nodeVar68, nodeVar59.z );
		nodeVar70 = ( nodeVar59.xy + ( vogelDiskSample( 4, 5, nodeVar60 ) * vec2<f32>( nodeVar61 ) ) );
		nodeVar71 = textureSampleCompare( nodeUniform31, nodeUniform31_sampler, nodeVar70, nodeVar59.z );
		nodeVar56 = ( ( ( ( ( nodeVar63 + nodeVar65 ) + nodeVar67 ) + nodeVar69 ) + nodeVar71 ) * 0.2 );

	} else {

		nodeVar56 = 1.0;

	}

	nodeVar72 = mix( 1.0, nodeVar56, render.nodeUniform34 );
	nodeVar73 = ( render.nodeUniform27 * vec3<f32>( nodeVar72 ) );
	nodeVar74 = ( vec3<f32>( clamp( nodeVar55, 0.0, 1.0 ) ) * nodeVar73 );
	nodeVar75 = ( DiffuseColor.xyz * vec3<f32>( 0.3183098861837907 ) );
	nodeVar76 = ( nodeVar74 * nodeVar75 );
	nodeVar77 = ( directDiffuse + nodeVar76 );
	directDiffuse = nodeVar77;
	nodeVar78 = normalize( ( nodeVar54 + positionViewDirection ) );
	nodeVar79 = clamp( dot( positionViewDirection, nodeVar78 ), 0.0, 1.0 );
	nodeVar80 = exp2( ( ( ( nodeVar79 * -5.55473 ) - 6.98316 ) * nodeVar79 ) );
	nodeVar81 = ( ( ( ( SpecularColor * vec3<f32>( ( 1.0 - nodeVar80 ) ) ) + vec3<f32>( ( 1.0 * nodeVar80 ) ) ) * vec3<f32>( 0.25 ) ) * vec3<f32>( ( ( ( ( Shininess * 0.5 ) + 1.0 ) * 0.3183098861837907 ) * pow( clamp( dot( normalView, nodeVar78 ), 0.0, 1.0 ), Shininess ) ) ) );
	nodeVar82 = ( nodeVar74 * nodeVar81 );
	nodeVar83 = ( nodeVar82 * vec3<f32>( 1.0 ) );
	nodeVar84 = ( directSpecular + nodeVar83 );
	directSpecular = nodeVar84;
	indirectDiffuse = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar85 = ( DiffuseColor * vec4<f32>( 0.3183098861837907 ) );
	nodeVar86 = ( vec4<f32>( irradiance, 1.0 ) * nodeVar85 );
	nodeVar87 = ( vec4<f32>( indirectDiffuse, 1.0 ) + nodeVar86 );
	indirectDiffuse = nodeVar87.xyz;
	ambientOcclusion = 1.0;
	nodeVar88 = ( indirectDiffuse * vec3<f32>( ambientOcclusion ) );
	indirectDiffuse = nodeVar88;
	nodeVar89 = ( directDiffuse + indirectDiffuse );
	totalDiffuse = nodeVar89;
	indirectSpecular = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar90 = ( directSpecular + indirectSpecular );
	totalSpecular = nodeVar90;
	nodeVar91 = ( totalDiffuse + totalSpecular );
	outgoingLight = nodeVar91;
	Output = max( vec4<f32>( ( outgoingLight + EmissiveColor ), DiffuseColor.w ), vec4<f32>( 0.0 ) );
	nodeVar92 = vec4<f32>( mix( Output.xyz, render.nodeUniform35, smoothstep( render.nodeUniform36, render.nodeUniform37, ( - v_positionView.z ) ) ), Output.w );
	Output = nodeVar92;

	// result

	output.color = nodeVar92;

	return output;

}
