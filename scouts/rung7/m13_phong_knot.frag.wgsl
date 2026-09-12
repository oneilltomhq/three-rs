// Three.js r186dev - Node System

// global
diagnostic( off, derivative_uniformity );


// structs

struct OutputStruct {
	@location( 0 ) color: vec4<f32>
};
var<private> output : OutputStruct;

// uniforms
@binding( 1 ) @group( 1 ) var nodeUniform16_sampler : sampler_comparison;
@binding( 2 ) @group( 1 ) var nodeUniform16 : texture_depth_2d;
@binding( 3 ) @group( 1 ) var nodeUniform32_sampler : sampler_comparison;
@binding( 4 ) @group( 1 ) var nodeUniform32 : texture_depth_2d;

struct objectStruct {
	nodeUniform0 : vec3<f32>,
	nodeUniform1 : f32,
	nodeUniform2 : f32,
	nodeUniform3 : vec3<f32>,
	nodeUniform4 : vec3<f32>,
	nodeUniform5 : f32,
	nodeUniform8 : mat3x3<f32>,
	nodeUniform10 : mat4x4<f32>
};
@binding( 0 ) @group( 1 )
var<uniform> object : objectStruct;

struct renderStruct {
	cameraProjectionMatrix : mat4x4<f32>,
	cameraViewMatrix : mat4x4<f32>,
	nodeUniform6 : vec3<f32>,
	nodeUniform20 : f32,
	nodeUniform21 : f32,
	nodeUniform24 : f32,
	nodeUniform25 : f32,
	nodeUniform11 : vec3<f32>,
	nodeUniform28 : vec3<f32>,
	nodeUniform9 : vec3<f32>,
	nodeUniform22 : vec3<f32>,
	nodeUniform23 : vec3<f32>,
	nodeUniform26 : vec3<f32>,
	nodeUniform27 : vec3<f32>,
	nodeUniform36 : vec3<f32>,
	nodeUniform37 : f32,
	nodeUniform38 : f32,
	nodeUniform12 : mat4x4<f32>,
	nodeUniform14 : f32,
	nodeUniform15 : f32,
	nodeUniform19 : f32,
	nodeUniform29 : mat4x4<f32>,
	nodeUniform30 : f32,
	nodeUniform31 : f32,
	nodeUniform35 : f32,
	nodeUniform17 : f32,
	nodeUniform18 : vec2<f32>,
	nodeUniform33 : f32,
	nodeUniform34 : vec2<f32>
};
@binding( 0 ) @group( 0 )
var<uniform> render : renderStruct;

// vars
var<private> DiffuseColor : vec4<f32>;
var<private> Shininess : f32;
var<private> SpecularColor : vec3<f32>;
var<private> EmissiveColor : vec3<f32>;
var<private> Output : vec4<f32>;
var<private> irradiance : vec3<f32>;
var<private> nodeVar0 : vec3<f32>;
var<private> directDiffuse : vec3<f32>;
var<private> normalViewGeometry : vec3<f32>;
var<private> NORMAL_normalView : vec3<f32>;
var<private> normalView : vec3<f32>;
var<private> nodeVar1 : vec3<f32>;
var<private> nodeVar2 : vec3<f32>;
var<private> nodeVar3 : f32;
var<private> shadowPositionWorld : vec3<f32>;
var<private> nodeVar4 : f32;
var<private> normalWorld : vec3<f32>;
var<private> nodeVar5 : vec4<f32>;
var<private> nodeVar6 : vec3<f32>;
var<private> nodeVar7 : vec3<f32>;
var<private> nodeVar8 : f32;
var<private> nodeVar9 : f32;
var<private> nodeVar10 : vec2<f32>;
var<private> nodeVar11 : f32;
var<private> nodeVar12 : vec2<f32>;
var<private> nodeVar13 : f32;
var<private> nodeVar14 : vec2<f32>;
var<private> nodeVar15 : f32;
var<private> nodeVar16 : vec2<f32>;
var<private> nodeVar17 : f32;
var<private> nodeVar18 : vec2<f32>;
var<private> nodeVar19 : f32;
var<private> nodeVar20 : f32;
var<private> nodeVar21 : vec3<f32>;
var<private> nodeVar22 : vec4<f32>;
var<private> nodeVar23 : vec4<f32>;
var<private> nodeVar24 : vec3<f32>;
var<private> nodeVar25 : vec3<f32>;
var<private> nodeVar26 : f32;
var<private> nodeVar27 : f32;
var<private> nodeVar28 : vec3<f32>;
var<private> nodeVar29 : f32;
var<private> nodeVar30 : f32;
var<private> nodeVar31 : f32;
var<private> nodeVar32 : f32;
var<private> nodeVar33 : vec3<f32>;
var<private> nodeVar34 : vec3<f32>;
var<private> nodeVar35 : vec3<f32>;
var<private> nodeVar36 : vec3<f32>;
var<private> nodeVar37 : vec3<f32>;
var<private> directSpecular : vec3<f32>;
var<private> positionViewDirection : vec3<f32>;
var<private> nodeVar38 : vec3<f32>;
var<private> nodeVar39 : f32;
var<private> nodeVar40 : f32;
var<private> nodeVar41 : vec3<f32>;
var<private> nodeVar42 : vec3<f32>;
var<private> nodeVar43 : vec3<f32>;
var<private> nodeVar44 : vec3<f32>;
var<private> nodeVar45 : vec3<f32>;
var<private> nodeVar46 : vec4<f32>;
var<private> nodeVar47 : vec4<f32>;
var<private> nodeVar48 : vec3<f32>;
var<private> nodeVar49 : vec3<f32>;
var<private> nodeVar50 : f32;
var<private> nodeVar51 : f32;
var<private> nodeVar52 : vec4<f32>;
var<private> nodeVar53 : vec3<f32>;
var<private> nodeVar54 : vec3<f32>;
var<private> nodeVar55 : f32;
var<private> nodeVar56 : f32;
var<private> nodeVar57 : vec2<f32>;
var<private> nodeVar58 : f32;
var<private> nodeVar59 : vec2<f32>;
var<private> nodeVar60 : f32;
var<private> nodeVar61 : vec2<f32>;
var<private> nodeVar62 : f32;
var<private> nodeVar63 : vec2<f32>;
var<private> nodeVar64 : f32;
var<private> nodeVar65 : vec2<f32>;
var<private> nodeVar66 : f32;
var<private> nodeVar67 : f32;
var<private> nodeVar68 : vec3<f32>;
var<private> nodeVar69 : vec3<f32>;
var<private> nodeVar70 : vec3<f32>;
var<private> nodeVar71 : vec3<f32>;
var<private> nodeVar72 : vec3<f32>;
var<private> nodeVar73 : f32;
var<private> nodeVar74 : f32;
var<private> nodeVar75 : vec3<f32>;
var<private> nodeVar76 : vec3<f32>;
var<private> nodeVar77 : vec3<f32>;
var<private> nodeVar78 : vec3<f32>;
var<private> indirectDiffuse : vec3<f32>;
var<private> nodeVar79 : vec4<f32>;
var<private> nodeVar80 : vec4<f32>;
var<private> nodeVar81 : vec4<f32>;
var<private> ambientOcclusion : f32;
var<private> nodeVar82 : vec3<f32>;
var<private> totalDiffuse : vec3<f32>;
var<private> nodeVar83 : vec3<f32>;
var<private> totalSpecular : vec3<f32>;
var<private> indirectSpecular : vec3<f32>;
var<private> nodeVar84 : vec3<f32>;
var<private> outgoingLight : vec3<f32>;
var<private> nodeVar85 : vec3<f32>;
var<private> nodeVar86 : vec4<f32>;

// codes
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


fn mx_trilerp_0 ( v0 : f32, v1 : f32, v2 : f32, v3 : f32, v4 : f32, v5 : f32, v6 : f32, v7 : f32, s : f32, t : f32, r : f32 ) -> f32 {

	var nodeVar0 : f32;
	var nodeVar1 : f32;
	var nodeVar2 : f32;
	var nodeVar3 : f32;
	var nodeVar4 : f32;
	var nodeVar5 : f32;
	var nodeVar6 : f32;
	var nodeVar7 : f32;
	var nodeVar8 : f32;
	var nodeVar9 : f32;
	var nodeVar10 : f32;
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

	return ( ( nodeVar13 * ( ( nodeVar12 * ( ( nodeVar10 * nodeVar11 ) + ( nodeVar9 * nodeVar2 ) ) ) + ( nodeVar1 * ( ( nodeVar8 * nodeVar11 ) + ( nodeVar7 * nodeVar2 ) ) ) ) ) + ( nodeVar0 * ( ( nodeVar12 * ( ( nodeVar6 * nodeVar11 ) + ( nodeVar5 * nodeVar2 ) ) ) + ( nodeVar1 * ( ( nodeVar4 * nodeVar11 ) + ( nodeVar3 * nodeVar2 ) ) ) ) ) );

}


fn mx_gradient_scale3d_0 ( v : f32 ) -> f32 {

	var nodeVar0 : f32;

	nodeVar0 = v;

	return ( 0.982 * nodeVar0 );

}


fn mx_perlin_noise_float_1 ( p : vec3<f32> ) -> f32 {

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
	var nodeVar13 : f32;

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
	nodeVar13 = mx_trilerp_0( mx_gradient_float_1( mx_hash_int_2( nodeVar1, nodeVar2, nodeVar3 ), nodeVar5, nodeVar7, nodeVar9 ), mx_gradient_float_1( mx_hash_int_2( ( nodeVar1 + 1 ), nodeVar2, nodeVar3 ), ( nodeVar5 - 1.0 ), nodeVar7, nodeVar9 ), mx_gradient_float_1( mx_hash_int_2( nodeVar1, ( nodeVar2 + 1 ), nodeVar3 ), nodeVar5, ( nodeVar7 - 1.0 ), nodeVar9 ), mx_gradient_float_1( mx_hash_int_2( ( nodeVar1 + 1 ), ( nodeVar2 + 1 ), nodeVar3 ), ( nodeVar5 - 1.0 ), ( nodeVar7 - 1.0 ), nodeVar9 ), mx_gradient_float_1( mx_hash_int_2( nodeVar1, nodeVar2, ( nodeVar3 + 1 ) ), nodeVar5, nodeVar7, ( nodeVar9 - 1.0 ) ), mx_gradient_float_1( mx_hash_int_2( ( nodeVar1 + 1 ), nodeVar2, ( nodeVar3 + 1 ) ), ( nodeVar5 - 1.0 ), nodeVar7, ( nodeVar9 - 1.0 ) ), mx_gradient_float_1( mx_hash_int_2( nodeVar1, ( nodeVar2 + 1 ), ( nodeVar3 + 1 ) ), nodeVar5, ( nodeVar7 - 1.0 ), ( nodeVar9 - 1.0 ) ), mx_gradient_float_1( mx_hash_int_2( ( nodeVar1 + 1 ), ( nodeVar2 + 1 ), ( nodeVar3 + 1 ) ), ( nodeVar5 - 1.0 ), ( nodeVar7 - 1.0 ), ( nodeVar9 - 1.0 ) ), nodeVar10, nodeVar11, nodeVar12 );

	return mx_gradient_scale3d_0( nodeVar13 );

}


fn mx_fractal_noise_float ( p : vec3<f32>, octaves : i32, lacunarity : f32, diminish : f32 ) -> f32 {

	var nodeVar0 : f32;
	var nodeVar1 : f32;
	var nodeVar2 : vec3<f32>;
	var nodeVar3 : f32;
	var nodeVar4 : f32;
	var nodeVar5 : i32;

	nodeVar0 = diminish;
	nodeVar1 = lacunarity;
	nodeVar2 = p;
	nodeVar3 = 0.0;
	nodeVar4 = 1.0;
	nodeVar5 = octaves;

	for ( var i : i32 = 0; i < nodeVar5; i ++ ) {

		nodeVar3 = ( nodeVar3 + ( nodeVar4 * mx_perlin_noise_float_1( nodeVar2 ) ) );
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
	@location( 1 ) positionLocal : vec3<f32>,
	@location( 2 ) v_positionWorld : vec3<f32>,
	@location( 3 ) v_positionViewDirection : vec3<f32>,
	@location( 4 ) v_normalViewGeometry : vec3<f32>,
	@builtin( position ) fragCoord : vec4<f32> ) -> OutputStruct {

	// flow
	// code


	if ( ( ! ( ( mx_fractal_noise_float( ( positionLocal * vec3<f32>( 0.1 ) ), 3, 2.0, 0.5 ) * 1.0 ) > 0.0 ) ) ) {

		discard;
		

	}

	DiffuseColor = vec4<f32>( object.nodeUniform0, 1.0 );
	DiffuseColor.w = ( DiffuseColor.w * object.nodeUniform1 );
	Shininess = max( object.nodeUniform2, 0.0001 );
	SpecularColor = object.nodeUniform3;
	EmissiveColor = ( object.nodeUniform4 * vec3<f32>( object.nodeUniform5 ) );
	irradiance = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar0 = ( irradiance + render.nodeUniform6 );
	irradiance = nodeVar0;
	directDiffuse = vec3<f32>( 0.0, 0.0, 0.0 );
	normalViewGeometry = normalize( v_normalViewGeometry );
	NORMAL_normalView = normalViewGeometry;
	normalView = NORMAL_normalView;
	nodeVar1 = ( render.nodeUniform9 - v_positionView );
	nodeVar2 = normalize( nodeVar1 );
	nodeVar3 = dot( normalView, nodeVar2 );
	shadowPositionWorld = v_positionWorld;
	normalWorld = normalize( ( vec4<f32>( normalView, 0.0 ) * render.cameraViewMatrix ).xyz );
	nodeVar5 = ( render.nodeUniform12 * vec4<f32>( ( shadowPositionWorld + ( normalWorld * vec3<f32>( render.nodeUniform14 ) ) ), 1.0 ) );
	nodeVar6 = ( nodeVar5.xyz / vec3<f32>( nodeVar5.w ) );
	nodeVar7 = vec3<f32>( nodeVar6.x, ( 1.0 - nodeVar6.y ), ( nodeVar6.z + render.nodeUniform15 ) );

	if ( ( ( ( ( ( nodeVar7.x >= 0.0 ) && ( nodeVar7.x <= 1.0 ) ) && ( nodeVar7.y >= 0.0 ) ) && ( nodeVar7.y <= 1.0 ) ) && ( nodeVar7.z <= 1.0 ) ) ) {

		nodeVar8 = ( interleavedGradientNoise( fragCoord.xy ) * 6.28318530718 );
		nodeVar9 = ( render.nodeUniform17 * ( vec2<f32>( 1.0, 1.0 ) / render.nodeUniform18 ).x );
		nodeVar10 = ( nodeVar7.xy + ( vogelDiskSample( 0, 5, nodeVar8 ) * vec2<f32>( nodeVar9 ) ) );
		nodeVar11 = textureSampleCompare( nodeUniform16, nodeUniform16_sampler, nodeVar10, nodeVar7.z );
		nodeVar12 = ( nodeVar7.xy + ( vogelDiskSample( 1, 5, nodeVar8 ) * vec2<f32>( nodeVar9 ) ) );
		nodeVar13 = textureSampleCompare( nodeUniform16, nodeUniform16_sampler, nodeVar12, nodeVar7.z );
		nodeVar14 = ( nodeVar7.xy + ( vogelDiskSample( 2, 5, nodeVar8 ) * vec2<f32>( nodeVar9 ) ) );
		nodeVar15 = textureSampleCompare( nodeUniform16, nodeUniform16_sampler, nodeVar14, nodeVar7.z );
		nodeVar16 = ( nodeVar7.xy + ( vogelDiskSample( 3, 5, nodeVar8 ) * vec2<f32>( nodeVar9 ) ) );
		nodeVar17 = textureSampleCompare( nodeUniform16, nodeUniform16_sampler, nodeVar16, nodeVar7.z );
		nodeVar18 = ( nodeVar7.xy + ( vogelDiskSample( 4, 5, nodeVar8 ) * vec2<f32>( nodeVar9 ) ) );
		nodeVar19 = textureSampleCompare( nodeUniform16, nodeUniform16_sampler, nodeVar18, nodeVar7.z );
		nodeVar4 = ( ( ( ( ( nodeVar11 + nodeVar13 ) + nodeVar15 ) + nodeVar17 ) + nodeVar19 ) * 0.2 );

	} else {

		nodeVar4 = 1.0;

	}

	nodeVar20 = mix( 1.0, nodeVar4, render.nodeUniform19 );
	nodeVar21 = ( render.nodeUniform22 - render.nodeUniform23 );
	nodeVar22 = vec4<f32>( nodeVar21, 0.0 );
	nodeVar23 = ( render.cameraViewMatrix * nodeVar22 );
	nodeVar24 = normalize( nodeVar23.xyz );
	nodeVar25 = nodeVar24;
	nodeVar26 = dot( nodeVar2, nodeVar25 );
	nodeVar27 = smoothstep( render.nodeUniform20, render.nodeUniform21, nodeVar26 );
	nodeVar28 = ( ( render.nodeUniform11 * vec3<f32>( nodeVar20 ) ) * vec3<f32>( nodeVar27 ) );

	if ( ( render.nodeUniform24 > 0.0 ) ) {

		nodeVar30 = length( nodeVar1 );
		nodeVar31 = ( nodeVar30 / render.nodeUniform24 );
		nodeVar32 = clamp( ( 1.0 - ( ( ( nodeVar31 * nodeVar31 ) * nodeVar31 ) * nodeVar31 ) ), 0.0, 1.0 );
		nodeVar29 = ( ( 1.0 / max( pow( nodeVar30, render.nodeUniform25 ), 0.01 ) ) * ( nodeVar32 * nodeVar32 ) );

	} else {

		nodeVar29 = ( 1.0 / max( pow( length( nodeVar1 ), render.nodeUniform25 ), 0.01 ) );

	}

	nodeVar33 = ( nodeVar28 * vec3<f32>( nodeVar29 ) );
	nodeVar34 = ( vec3<f32>( clamp( nodeVar3, 0.0, 1.0 ) ) * nodeVar33 );
	nodeVar35 = ( DiffuseColor.xyz * vec3<f32>( 0.3183098861837907 ) );
	nodeVar36 = ( nodeVar34 * nodeVar35 );
	nodeVar37 = ( directDiffuse + nodeVar36 );
	directDiffuse = nodeVar37;
	directSpecular = vec3<f32>( 0.0, 0.0, 0.0 );
	positionViewDirection = normalize( v_positionViewDirection );
	nodeVar38 = normalize( ( nodeVar2 + positionViewDirection ) );
	nodeVar39 = clamp( dot( positionViewDirection, nodeVar38 ), 0.0, 1.0 );
	nodeVar40 = exp2( ( ( ( nodeVar39 * -5.55473 ) - 6.98316 ) * nodeVar39 ) );
	nodeVar41 = ( ( ( ( SpecularColor * vec3<f32>( ( 1.0 - nodeVar40 ) ) ) + vec3<f32>( ( 1.0 * nodeVar40 ) ) ) * vec3<f32>( 0.25 ) ) * vec3<f32>( ( ( ( ( Shininess * 0.5 ) + 1.0 ) * 0.3183098861837907 ) * pow( clamp( dot( normalView, nodeVar38 ), 0.0, 1.0 ), Shininess ) ) ) );
	nodeVar42 = ( nodeVar34 * nodeVar41 );
	nodeVar43 = ( nodeVar42 * vec3<f32>( 1.0 ) );
	nodeVar44 = ( directSpecular + nodeVar43 );
	directSpecular = nodeVar44;
	nodeVar45 = ( render.nodeUniform26 - render.nodeUniform27 );
	nodeVar46 = vec4<f32>( nodeVar45, 0.0 );
	nodeVar47 = ( render.cameraViewMatrix * nodeVar46 );
	nodeVar48 = normalize( nodeVar47.xyz );
	nodeVar49 = nodeVar48;
	nodeVar50 = dot( normalView, nodeVar49 );
	shadowPositionWorld = v_positionWorld;
	nodeVar52 = ( render.nodeUniform29 * vec4<f32>( ( shadowPositionWorld + ( normalWorld * vec3<f32>( render.nodeUniform30 ) ) ), 1.0 ) );
	nodeVar53 = ( nodeVar52.xyz / vec3<f32>( nodeVar52.w ) );
	nodeVar54 = vec3<f32>( nodeVar53.x, ( 1.0 - nodeVar53.y ), ( nodeVar53.z + render.nodeUniform31 ) );

	if ( ( ( ( ( ( nodeVar54.x >= 0.0 ) && ( nodeVar54.x <= 1.0 ) ) && ( nodeVar54.y >= 0.0 ) ) && ( nodeVar54.y <= 1.0 ) ) && ( nodeVar54.z <= 1.0 ) ) ) {

		nodeVar55 = ( interleavedGradientNoise( fragCoord.xy ) * 6.28318530718 );
		nodeVar56 = ( render.nodeUniform33 * ( vec2<f32>( 1.0, 1.0 ) / render.nodeUniform34 ).x );
		nodeVar57 = ( nodeVar54.xy + ( vogelDiskSample( 0, 5, nodeVar55 ) * vec2<f32>( nodeVar56 ) ) );
		nodeVar58 = textureSampleCompare( nodeUniform32, nodeUniform32_sampler, nodeVar57, nodeVar54.z );
		nodeVar59 = ( nodeVar54.xy + ( vogelDiskSample( 1, 5, nodeVar55 ) * vec2<f32>( nodeVar56 ) ) );
		nodeVar60 = textureSampleCompare( nodeUniform32, nodeUniform32_sampler, nodeVar59, nodeVar54.z );
		nodeVar61 = ( nodeVar54.xy + ( vogelDiskSample( 2, 5, nodeVar55 ) * vec2<f32>( nodeVar56 ) ) );
		nodeVar62 = textureSampleCompare( nodeUniform32, nodeUniform32_sampler, nodeVar61, nodeVar54.z );
		nodeVar63 = ( nodeVar54.xy + ( vogelDiskSample( 3, 5, nodeVar55 ) * vec2<f32>( nodeVar56 ) ) );
		nodeVar64 = textureSampleCompare( nodeUniform32, nodeUniform32_sampler, nodeVar63, nodeVar54.z );
		nodeVar65 = ( nodeVar54.xy + ( vogelDiskSample( 4, 5, nodeVar55 ) * vec2<f32>( nodeVar56 ) ) );
		nodeVar66 = textureSampleCompare( nodeUniform32, nodeUniform32_sampler, nodeVar65, nodeVar54.z );
		nodeVar51 = ( ( ( ( ( nodeVar58 + nodeVar60 ) + nodeVar62 ) + nodeVar64 ) + nodeVar66 ) * 0.2 );

	} else {

		nodeVar51 = 1.0;

	}

	nodeVar67 = mix( 1.0, nodeVar51, render.nodeUniform35 );
	nodeVar68 = ( vec3<f32>( clamp( nodeVar50, 0.0, 1.0 ) ) * ( render.nodeUniform28 * vec3<f32>( nodeVar67 ) ) );
	nodeVar69 = ( DiffuseColor.xyz * vec3<f32>( 0.3183098861837907 ) );
	nodeVar70 = ( nodeVar68 * nodeVar69 );
	nodeVar71 = ( directDiffuse + nodeVar70 );
	directDiffuse = nodeVar71;
	nodeVar72 = normalize( ( nodeVar49 + positionViewDirection ) );
	nodeVar73 = clamp( dot( positionViewDirection, nodeVar72 ), 0.0, 1.0 );
	nodeVar74 = exp2( ( ( ( nodeVar73 * -5.55473 ) - 6.98316 ) * nodeVar73 ) );
	nodeVar75 = ( ( ( ( SpecularColor * vec3<f32>( ( 1.0 - nodeVar74 ) ) ) + vec3<f32>( ( 1.0 * nodeVar74 ) ) ) * vec3<f32>( 0.25 ) ) * vec3<f32>( ( ( ( ( Shininess * 0.5 ) + 1.0 ) * 0.3183098861837907 ) * pow( clamp( dot( normalView, nodeVar72 ), 0.0, 1.0 ), Shininess ) ) ) );
	nodeVar76 = ( nodeVar68 * nodeVar75 );
	nodeVar77 = ( nodeVar76 * vec3<f32>( 1.0 ) );
	nodeVar78 = ( directSpecular + nodeVar77 );
	directSpecular = nodeVar78;
	indirectDiffuse = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar79 = ( DiffuseColor * vec4<f32>( 0.3183098861837907 ) );
	nodeVar80 = ( vec4<f32>( irradiance, 1.0 ) * nodeVar79 );
	nodeVar81 = ( vec4<f32>( indirectDiffuse, 1.0 ) + nodeVar80 );
	indirectDiffuse = nodeVar81.xyz;
	ambientOcclusion = 1.0;
	nodeVar82 = ( indirectDiffuse * vec3<f32>( ambientOcclusion ) );
	indirectDiffuse = nodeVar82;
	nodeVar83 = ( directDiffuse + indirectDiffuse );
	totalDiffuse = nodeVar83;
	indirectSpecular = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar84 = ( directSpecular + indirectSpecular );
	totalSpecular = nodeVar84;
	nodeVar85 = ( totalDiffuse + totalSpecular );
	outgoingLight = nodeVar85;
	Output = max( vec4<f32>( ( outgoingLight + EmissiveColor ), DiffuseColor.w ), vec4<f32>( 0.0 ) );
	nodeVar86 = vec4<f32>( mix( Output.xyz, render.nodeUniform36, smoothstep( render.nodeUniform37, render.nodeUniform38, ( - v_positionView.z ) ) ), Output.w );
	Output = nodeVar86;

	// result

	output.color = nodeVar86;

	return output;

}
