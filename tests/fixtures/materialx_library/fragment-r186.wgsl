// Three.js r186 - Node System

// global
diagnostic( off, derivative_uniformity );


// structs

struct OutputStruct {
	@location( 0 ) color: vec4<f32>
};
var<private> output : OutputStruct;

// uniforms

struct renderStruct {
	nodeUniform0 : f32,
	cameraProjectionMatrix : mat4x4<f32>,
	cameraViewMatrix : mat4x4<f32>
};
@binding( 0 ) @group( 0 )
var<uniform> render : renderStruct;

struct objectStruct {
	nodeUniform1 : f32,
	nodeUniform4 : mat4x4<f32>
};
@binding( 0 ) @group( 1 )
var<uniform> object : objectStruct;

// vars
var<private> DiffuseColor : vec4<f32>;
var<private> Output : vec4<f32>;
var<private> nodeVar0 : vec4<f32>;

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


fn mx_hash_int_1 ( x : i32, y : i32 ) -> u32 {

	var nodeVar0 : i32;
	var nodeVar1 : i32;
	var nodeVar2 : u32;
	var nodeVar3 : u32;
	var nodeVar4 : u32;
	var nodeVar5 : u32;

	nodeVar0 = y;
	nodeVar1 = x;
	nodeVar2 = 2u;
	nodeVar3 = 0u;
	nodeVar4 = 0u;
	nodeVar5 = 0u;
	nodeVar5 = ( ( 3735928559u + ( nodeVar2 << 2u ) ) + 13u );
	nodeVar4 = nodeVar5;
	nodeVar3 = nodeVar4;
	nodeVar3 = ( nodeVar3 + u32( nodeVar1 ) );
	nodeVar4 = ( nodeVar4 + u32( nodeVar0 ) );

	return mx_bjfinal( nodeVar3, nodeVar4, nodeVar5 );

}


fn mx_gradient_float_0 ( hash : u32, x : f32, y : f32 ) -> f32 {

	var nodeVar0 : f32;
	var nodeVar1 : f32;
	var nodeVar2 : u32;
	var nodeVar3 : u32;
	var nodeVar4 : f32;
	var nodeVar5 : f32;

	nodeVar0 = y;
	nodeVar1 = x;
	nodeVar2 = hash;
	nodeVar3 = ( nodeVar2 & 7u );
	nodeVar4 = mx_select( ( nodeVar3 < 4u ), nodeVar1, nodeVar0 );
	nodeVar5 = ( 2.0 * mx_select( ( nodeVar3 < 4u ), nodeVar0, nodeVar1 ) );

	return ( mx_negate_if( nodeVar4, bool( ( nodeVar3 & 1u ) ) ) + mx_negate_if( nodeVar5, bool( ( nodeVar3 & 2u ) ) ) );

}


fn mx_bilerp_0 ( v0 : f32, v1 : f32, v2 : f32, v3 : f32, s : f32, t : f32 ) -> f32 {

	var nodeVar0 : f32;
	var nodeVar1 : f32;
	var nodeVar2 : f32;
	var nodeVar3 : f32;
	var nodeVar4 : f32;
	var nodeVar5 : f32;
	var nodeVar6 : f32;

	nodeVar0 = t;
	nodeVar1 = s;
	nodeVar2 = v3;
	nodeVar3 = v2;
	nodeVar4 = v1;
	nodeVar5 = v0;
	nodeVar6 = ( 1.0 - nodeVar1 );

	return ( ( ( 1.0 - nodeVar0 ) * ( ( nodeVar5 * nodeVar6 ) + ( nodeVar4 * nodeVar1 ) ) ) + ( nodeVar0 * ( ( nodeVar3 * nodeVar6 ) + ( nodeVar2 * nodeVar1 ) ) ) );

}


fn mx_gradient_scale2d_0 ( v : f32 ) -> f32 {

	var nodeVar0 : f32;

	nodeVar0 = v;

	return ( 0.6616 * nodeVar0 );

}


fn mx_perlin_noise_float_0 ( p : vec2<f32> ) -> f32 {

	var nodeVar0 : vec2<f32>;
	var nodeVar1 : i32;
	var nodeVar2 : i32;
	var nodeVar3 : f32;
	var nodeVar4 : f32;
	var nodeVar5 : f32;
	var nodeVar6 : f32;
	var nodeVar7 : f32;
	var nodeVar8 : f32;
	var nodeVar9 : f32;

	nodeVar0 = p;
	nodeVar1 = 0;
	nodeVar2 = 0;
	nodeVar3 = nodeVar0.x;
	nodeVar1 = mx_floor( nodeVar3 );
	nodeVar4 = ( nodeVar3 - f32( nodeVar1 ) );
	nodeVar5 = nodeVar0.y;
	nodeVar2 = mx_floor( nodeVar5 );
	nodeVar6 = ( nodeVar5 - f32( nodeVar2 ) );
	nodeVar7 = mx_fade( nodeVar4 );
	nodeVar8 = mx_fade( nodeVar6 );
	nodeVar9 = mx_bilerp_0( mx_gradient_float_0( mx_hash_int_1( nodeVar1, nodeVar2 ), nodeVar4, nodeVar6 ), mx_gradient_float_0( mx_hash_int_1( ( nodeVar1 + 1 ), nodeVar2 ), ( nodeVar4 - 1.0 ), nodeVar6 ), mx_gradient_float_0( mx_hash_int_1( nodeVar1, ( nodeVar2 + 1 ) ), nodeVar4, ( nodeVar6 - 1.0 ) ), mx_gradient_float_0( mx_hash_int_1( ( nodeVar1 + 1 ), ( nodeVar2 + 1 ) ), ( nodeVar4 - 1.0 ), ( nodeVar6 - 1.0 ) ), nodeVar7, nodeVar8 );

	return mx_gradient_scale2d_0( nodeVar9 );

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


fn mx_hash_vec3_0 ( x : i32, y : i32 ) -> vec3<u32> {

	var nodeVar0 : i32;
	var nodeVar1 : i32;
	var nodeVar2 : u32;
	var nodeVar3 : vec3<u32>;

	nodeVar0 = y;
	nodeVar1 = x;
	nodeVar2 = mx_hash_int_1( nodeVar1, nodeVar0 );
	nodeVar3 = vec3<u32>( 0u, 0u, 0u );
	nodeVar3.x = ( nodeVar2 & 255u );
	nodeVar3.y = ( ( nodeVar2 >> 8u ) & 255u );
	nodeVar3.z = ( ( nodeVar2 >> 16u ) & 255u );

	return nodeVar3;

}


fn mx_gradient_vec3_0 ( hash : vec3<u32>, x : f32, y : f32 ) -> vec3<f32> {

	var nodeVar0 : f32;
	var nodeVar1 : f32;
	var nodeVar2 : vec3<u32>;

	nodeVar0 = y;
	nodeVar1 = x;
	nodeVar2 = hash;

	return vec3<f32>( mx_gradient_float_0( nodeVar2.x, nodeVar1, nodeVar0 ), mx_gradient_float_0( nodeVar2.y, nodeVar1, nodeVar0 ), mx_gradient_float_0( nodeVar2.z, nodeVar1, nodeVar0 ) );

}


fn mx_bilerp_1 ( v0 : vec3<f32>, v1 : vec3<f32>, v2 : vec3<f32>, v3 : vec3<f32>, s : f32, t : f32 ) -> vec3<f32> {

	var nodeVar0 : f32;
	var nodeVar1 : f32;
	var nodeVar2 : vec3<f32>;
	var nodeVar3 : vec3<f32>;
	var nodeVar4 : vec3<f32>;
	var nodeVar5 : vec3<f32>;
	var nodeVar6 : f32;

	nodeVar0 = t;
	nodeVar1 = s;
	nodeVar2 = v3;
	nodeVar3 = v2;
	nodeVar4 = v1;
	nodeVar5 = v0;
	nodeVar6 = ( 1.0 - nodeVar1 );

	return ( ( vec3<f32>( ( 1.0 - nodeVar0 ) ) * ( ( nodeVar5 * vec3<f32>( nodeVar6 ) ) + ( nodeVar4 * vec3<f32>( nodeVar1 ) ) ) ) + ( vec3<f32>( nodeVar0 ) * ( ( nodeVar3 * vec3<f32>( nodeVar6 ) ) + ( nodeVar2 * vec3<f32>( nodeVar1 ) ) ) ) );

}


fn mx_gradient_scale2d_1 ( v : vec3<f32> ) -> vec3<f32> {

	var nodeVar0 : vec3<f32>;

	nodeVar0 = v;

	return ( vec3<f32>( 0.6616 ) * nodeVar0 );

}


fn mx_perlin_noise_vec3_0 ( p : vec2<f32> ) -> vec3<f32> {

	var nodeVar0 : vec2<f32>;
	var nodeVar1 : i32;
	var nodeVar2 : i32;
	var nodeVar3 : f32;
	var nodeVar4 : f32;
	var nodeVar5 : f32;
	var nodeVar6 : f32;
	var nodeVar7 : f32;
	var nodeVar8 : f32;
	var nodeVar9 : vec3<f32>;

	nodeVar0 = p;
	nodeVar1 = 0;
	nodeVar2 = 0;
	nodeVar3 = nodeVar0.x;
	nodeVar1 = mx_floor( nodeVar3 );
	nodeVar4 = ( nodeVar3 - f32( nodeVar1 ) );
	nodeVar5 = nodeVar0.y;
	nodeVar2 = mx_floor( nodeVar5 );
	nodeVar6 = ( nodeVar5 - f32( nodeVar2 ) );
	nodeVar7 = mx_fade( nodeVar4 );
	nodeVar8 = mx_fade( nodeVar6 );
	nodeVar9 = mx_bilerp_1( mx_gradient_vec3_0( mx_hash_vec3_0( nodeVar1, nodeVar2 ), nodeVar4, nodeVar6 ), mx_gradient_vec3_0( mx_hash_vec3_0( ( nodeVar1 + 1 ), nodeVar2 ), ( nodeVar4 - 1.0 ), nodeVar6 ), mx_gradient_vec3_0( mx_hash_vec3_0( nodeVar1, ( nodeVar2 + 1 ) ), nodeVar4, ( nodeVar6 - 1.0 ) ), mx_gradient_vec3_0( mx_hash_vec3_0( ( nodeVar1 + 1 ), ( nodeVar2 + 1 ) ), ( nodeVar4 - 1.0 ), ( nodeVar6 - 1.0 ) ), nodeVar7, nodeVar8 );

	return mx_gradient_scale2d_1( nodeVar9 );

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


fn mx_bits_to_01 ( bits : u32 ) -> f32 {

	var nodeVar0 : u32;

	nodeVar0 = bits;

	return ( f32( nodeVar0 ) / 4294967295.0 );

}


fn mx_cell_noise_float_1 ( p : vec2<f32> ) -> f32 {

	var nodeVar0 : vec2<f32>;
	var nodeVar1 : i32;
	var nodeVar2 : i32;

	nodeVar0 = p;
	nodeVar1 = mx_floor( nodeVar0.x );
	nodeVar2 = mx_floor( nodeVar0.y );

	return mx_bits_to_01( mx_hash_int_1( nodeVar1, nodeVar2 ) );

}


fn mx_cell_noise_float_2 ( p : vec3<f32> ) -> f32 {

	var nodeVar0 : vec3<f32>;
	var nodeVar1 : i32;
	var nodeVar2 : i32;
	var nodeVar3 : i32;

	nodeVar0 = p;
	nodeVar1 = mx_floor( nodeVar0.x );
	nodeVar2 = mx_floor( nodeVar0.y );
	nodeVar3 = mx_floor( nodeVar0.z );

	return mx_bits_to_01( mx_hash_int_2( nodeVar1, nodeVar2, nodeVar3 ) );

}


fn mx_cell_noise_vec3_1 ( p : vec2<f32> ) -> vec3<f32> {

	var nodeVar0 : vec2<f32>;
	var nodeVar1 : i32;
	var nodeVar2 : i32;

	nodeVar0 = p;
	nodeVar1 = mx_floor( nodeVar0.x );
	nodeVar2 = mx_floor( nodeVar0.y );

	return vec3<f32>( mx_bits_to_01( mx_hash_int_2( nodeVar1, nodeVar2, 0 ) ), mx_bits_to_01( mx_hash_int_2( nodeVar1, nodeVar2, 1 ) ), mx_bits_to_01( mx_hash_int_2( nodeVar1, nodeVar2, 2 ) ) );

}


fn mx_bjmix ( a : u32, b : u32, c : u32 ) -> vec3<u32> {

	var nodeVar0 : u32;
	var nodeVar1 : u32;
	var nodeVar2 : u32;

	nodeVar0 = a;
	nodeVar1 = b;
	nodeVar2 = c;
	nodeVar0 = ( nodeVar0 - nodeVar2 );
	nodeVar0 = ( nodeVar0 ^ mx_rotl32( nodeVar2, 4 ) );
	nodeVar2 = ( nodeVar2 + nodeVar1 );
	nodeVar1 = ( nodeVar1 - nodeVar0 );
	nodeVar1 = ( nodeVar1 ^ mx_rotl32( nodeVar0, 6 ) );
	nodeVar0 = ( nodeVar0 + nodeVar2 );
	nodeVar2 = ( nodeVar2 - nodeVar1 );
	nodeVar2 = ( nodeVar2 ^ mx_rotl32( nodeVar1, 8 ) );
	nodeVar1 = ( nodeVar1 + nodeVar0 );
	nodeVar0 = ( nodeVar0 - nodeVar2 );
	nodeVar0 = ( nodeVar0 ^ mx_rotl32( nodeVar2, 16 ) );
	nodeVar2 = ( nodeVar2 + nodeVar1 );
	nodeVar1 = ( nodeVar1 - nodeVar0 );
	nodeVar1 = ( nodeVar1 ^ mx_rotl32( nodeVar0, 19 ) );
	nodeVar0 = ( nodeVar0 + nodeVar2 );
	nodeVar2 = ( nodeVar2 - nodeVar1 );
	nodeVar2 = ( nodeVar2 ^ mx_rotl32( nodeVar1, 4 ) );
	nodeVar1 = ( nodeVar1 + nodeVar0 );

	return vec3<u32>( nodeVar0, nodeVar1, nodeVar2 );

}


fn mx_cell_noise_vec3_2 ( p : vec3<f32> ) -> vec3<f32> {

	var nodeVar0 : vec3<f32>;
	var nodeVar1 : i32;
	var nodeVar2 : i32;
	var nodeVar3 : i32;
	var nodeVar4 : u32;
	var nodeVar5 : u32;
	var nodeVar6 : u32;
	var nodeVar7 : u32;
	var nodeVar8 : vec3<u32>;

	nodeVar0 = p;
	nodeVar1 = i32( floor( nodeVar0.x ) );
	nodeVar2 = i32( floor( nodeVar0.y ) );
	nodeVar3 = i32( floor( nodeVar0.z ) );
	nodeVar4 = 3735928588u;
	nodeVar5 = 0u;
	nodeVar6 = 0u;
	nodeVar7 = 0u;
	nodeVar7 = nodeVar4;
	nodeVar6 = nodeVar7;
	nodeVar5 = nodeVar6;
	nodeVar5 = ( nodeVar5 + u32( nodeVar1 ) );
	nodeVar6 = ( nodeVar6 + u32( nodeVar2 ) );
	nodeVar7 = ( nodeVar7 + u32( nodeVar3 ) );
	nodeVar8 = mx_bjmix( nodeVar5, nodeVar6, nodeVar7 );

	return vec3<f32>( mx_bits_to_01( mx_bjfinal( nodeVar8.x, nodeVar8.y, nodeVar8.z ) ), mx_bits_to_01( mx_bjfinal( ( nodeVar8.x + 1u ), nodeVar8.y, nodeVar8.z ) ), mx_bits_to_01( mx_bjfinal( ( nodeVar8.x + 2u ), nodeVar8.y, nodeVar8.z ) ) );

}


fn mx_worley_distance_1 ( p : vec3<f32>, x : i32, y : i32, z : i32, xoff : i32, yoff : i32, zoff : i32, jitter : f32, metric : i32 ) -> f32 {

	var nodeVar0 : i32;
	var nodeVar1 : f32;
	var nodeVar2 : i32;
	var nodeVar3 : i32;
	var nodeVar4 : i32;
	var nodeVar5 : i32;
	var nodeVar6 : i32;
	var nodeVar7 : i32;
	var nodeVar8 : vec3<f32>;
	var nodeVar9 : vec3<f32>;
	var nodeVar10 : i32;
	var nodeVar11 : i32;
	var nodeVar12 : i32;
	var nodeVar13 : u32;
	var nodeVar14 : u32;
	var nodeVar15 : u32;
	var nodeVar16 : u32;
	var nodeVar17 : vec3<u32>;
	var nodeVar18 : vec3<f32>;
	var nodeVar19 : vec3<f32>;
	var nodeVar20 : vec3<f32>;

	nodeVar0 = metric;
	nodeVar1 = jitter;
	nodeVar2 = zoff;
	nodeVar3 = yoff;
	nodeVar4 = xoff;
	nodeVar5 = z;
	nodeVar6 = y;
	nodeVar7 = x;
	nodeVar8 = p;
	nodeVar9 = vec3<f32>( f32( ( nodeVar7 + nodeVar4 ) ), f32( ( nodeVar6 + nodeVar3 ) ), f32( ( nodeVar5 + nodeVar2 ) ) );
	nodeVar10 = i32( floor( nodeVar9.x ) );
	nodeVar11 = i32( floor( nodeVar9.y ) );
	nodeVar12 = i32( floor( nodeVar9.z ) );
	nodeVar13 = 3735928588u;
	nodeVar14 = 0u;
	nodeVar15 = 0u;
	nodeVar16 = 0u;
	nodeVar16 = nodeVar13;
	nodeVar15 = nodeVar16;
	nodeVar14 = nodeVar15;
	nodeVar14 = ( nodeVar14 + u32( nodeVar10 ) );
	nodeVar15 = ( nodeVar15 + u32( nodeVar11 ) );
	nodeVar16 = ( nodeVar16 + u32( nodeVar12 ) );
	nodeVar17 = mx_bjmix( nodeVar14, nodeVar15, nodeVar16 );
	nodeVar18 = vec3<f32>( mx_bits_to_01( mx_bjfinal( nodeVar17.x, nodeVar17.y, nodeVar17.z ) ), mx_bits_to_01( mx_bjfinal( ( nodeVar17.x + 1u ), nodeVar17.y, nodeVar17.z ) ), mx_bits_to_01( mx_bjfinal( ( nodeVar17.x + 2u ), nodeVar17.y, nodeVar17.z ) ) );
	nodeVar18 = ( nodeVar18 - vec3<f32>( 0.5 ) );
	nodeVar18 = ( nodeVar18 * vec3<f32>( nodeVar1 ) );
	nodeVar18 = ( nodeVar18 + vec3<f32>( 0.5 ) );
	nodeVar19 = ( vec3<f32>( f32( nodeVar7 ), f32( nodeVar6 ), f32( nodeVar5 ) ) + nodeVar18 );
	nodeVar20 = ( nodeVar19 - nodeVar8 );

	if ( ( nodeVar0 == 2 ) ) {

		return ( ( abs( nodeVar20.x ) + abs( nodeVar20.y ) ) + abs( nodeVar20.z ) );

	}


	if ( ( nodeVar0 == 3 ) ) {

		return max( max( abs( nodeVar20.x ), abs( nodeVar20.y ) ), abs( nodeVar20.z ) );

	}


	return dot( nodeVar20, nodeVar20 );

}


fn mx_worley_distance_0 ( p : vec2<f32>, x : i32, y : i32, xoff : i32, yoff : i32, jitter : f32, metric : i32 ) -> f32 {

	var nodeVar0 : i32;
	var nodeVar1 : f32;
	var nodeVar2 : i32;
	var nodeVar3 : i32;
	var nodeVar4 : i32;
	var nodeVar5 : i32;
	var nodeVar6 : vec2<f32>;
	var nodeVar7 : vec3<f32>;
	var nodeVar8 : vec2<f32>;
	var nodeVar9 : vec2<f32>;
	var nodeVar10 : vec2<f32>;

	nodeVar0 = metric;
	nodeVar1 = jitter;
	nodeVar2 = yoff;
	nodeVar3 = xoff;
	nodeVar4 = y;
	nodeVar5 = x;
	nodeVar6 = p;
	nodeVar7 = mx_cell_noise_vec3_1( vec2<f32>( f32( ( nodeVar5 + nodeVar3 ) ), f32( ( nodeVar4 + nodeVar2 ) ) ) );
	nodeVar8 = vec2<f32>( nodeVar7.x, nodeVar7.y );
	nodeVar8 = ( nodeVar8 - vec2<f32>( 0.5 ) );
	nodeVar8 = ( nodeVar8 * vec2<f32>( nodeVar1 ) );
	nodeVar8 = ( nodeVar8 + vec2<f32>( 0.5 ) );
	nodeVar9 = ( vec2<f32>( f32( nodeVar5 ), f32( nodeVar4 ) ) + nodeVar8 );
	nodeVar10 = ( nodeVar9 - nodeVar6 );

	if ( ( nodeVar0 == 2 ) ) {

		return ( abs( nodeVar10.x ) + abs( nodeVar10.y ) );

	}


	if ( ( nodeVar0 == 3 ) ) {

		return max( abs( nodeVar10.x ), abs( nodeVar10.y ) );

	}


	return dot( nodeVar10, nodeVar10 );

}


fn mx_worley_noise_vec2_0 ( p : vec2<f32>, jitter : f32, metric : i32 ) -> vec2<f32> {

	var nodeVar0 : i32;
	var nodeVar1 : f32;
	var nodeVar2 : vec2<f32>;
	var nodeVar3 : i32;
	var nodeVar4 : i32;
	var nodeVar5 : f32;
	var nodeVar6 : f32;
	var nodeVar7 : vec2<f32>;
	var nodeVar8 : vec2<f32>;
	var nodeVar9 : f32;

	nodeVar0 = metric;
	nodeVar1 = jitter;
	nodeVar2 = p;
	nodeVar3 = 0;
	nodeVar4 = 0;
	nodeVar5 = nodeVar2.x;
	nodeVar3 = mx_floor( nodeVar5 );
	nodeVar6 = nodeVar2.y;
	nodeVar4 = mx_floor( nodeVar6 );
	nodeVar7 = vec2<f32>( ( nodeVar5 - f32( nodeVar3 ) ), ( nodeVar6 - f32( nodeVar4 ) ) );
	nodeVar8 = vec2<f32>( 1000000.0, 1000000.0 );

	for ( var x : i32 = -1; x <= 1; x ++ ) {


		for ( var y : i32 = -1; y <= 1; y ++ ) {

			nodeVar9 = mx_worley_distance_0( nodeVar7, x, y, nodeVar3, nodeVar4, nodeVar1, nodeVar0 );

			if ( ( nodeVar9 < nodeVar8.x ) ) {

				nodeVar8.y = nodeVar8.x;
				nodeVar8.x = nodeVar9;
				

			} else {


				if ( ( nodeVar9 < nodeVar8.y ) ) {

					nodeVar8.y = nodeVar9;
					

				}

				

			}


		}


	}


	if ( ( nodeVar0 == 0 ) ) {

		nodeVar8 = sqrt( nodeVar8 );
		

	}


	return nodeVar8;

}


fn mx_worley_noise_vec2_1 ( p : vec3<f32>, jitter : f32, metric : i32 ) -> vec2<f32> {

	var nodeVar0 : i32;
	var nodeVar1 : f32;
	var nodeVar2 : vec3<f32>;
	var nodeVar3 : i32;
	var nodeVar4 : i32;
	var nodeVar5 : i32;
	var nodeVar6 : f32;
	var nodeVar7 : f32;
	var nodeVar8 : f32;
	var nodeVar9 : vec3<f32>;
	var nodeVar10 : vec2<f32>;
	var nodeVar11 : f32;

	nodeVar0 = metric;
	nodeVar1 = jitter;
	nodeVar2 = p;
	nodeVar3 = 0;
	nodeVar4 = 0;
	nodeVar5 = 0;
	nodeVar6 = nodeVar2.x;
	nodeVar3 = mx_floor( nodeVar6 );
	nodeVar7 = nodeVar2.y;
	nodeVar4 = mx_floor( nodeVar7 );
	nodeVar8 = nodeVar2.z;
	nodeVar5 = mx_floor( nodeVar8 );
	nodeVar9 = vec3<f32>( ( nodeVar6 - f32( nodeVar3 ) ), ( nodeVar7 - f32( nodeVar4 ) ), ( nodeVar8 - f32( nodeVar5 ) ) );
	nodeVar10 = vec2<f32>( 1000000.0, 1000000.0 );

	for ( var x : i32 = -1; x <= 1; x ++ ) {


		for ( var y : i32 = -1; y <= 1; y ++ ) {


			for ( var z : i32 = -1; z <= 1; z ++ ) {

				nodeVar11 = mx_worley_distance_1( nodeVar9, x, y, z, nodeVar3, nodeVar4, nodeVar5, nodeVar1, nodeVar0 );

				if ( ( nodeVar11 < nodeVar10.x ) ) {

					nodeVar10.y = nodeVar10.x;
					nodeVar10.x = nodeVar11;
					

				} else {


					if ( ( nodeVar11 < nodeVar10.y ) ) {

						nodeVar10.y = nodeVar11;
						

					}

					

				}


			}


		}


	}


	if ( ( nodeVar0 == 0 ) ) {

		nodeVar10 = sqrt( nodeVar10 );
		

	}


	return nodeVar10;

}


fn mx_worley_noise_vec3_0 ( p : vec2<f32>, jitter : f32, metric : i32 ) -> vec3<f32> {

	var nodeVar0 : i32;
	var nodeVar1 : f32;
	var nodeVar2 : vec2<f32>;
	var nodeVar3 : i32;
	var nodeVar4 : i32;
	var nodeVar5 : f32;
	var nodeVar6 : f32;
	var nodeVar7 : vec2<f32>;
	var nodeVar8 : vec3<f32>;
	var nodeVar9 : f32;

	nodeVar0 = metric;
	nodeVar1 = jitter;
	nodeVar2 = p;
	nodeVar3 = 0;
	nodeVar4 = 0;
	nodeVar5 = nodeVar2.x;
	nodeVar3 = mx_floor( nodeVar5 );
	nodeVar6 = nodeVar2.y;
	nodeVar4 = mx_floor( nodeVar6 );
	nodeVar7 = vec2<f32>( ( nodeVar5 - f32( nodeVar3 ) ), ( nodeVar6 - f32( nodeVar4 ) ) );
	nodeVar8 = vec3<f32>( 1000000.0, 1000000.0, 1000000.0 );

	for ( var x : i32 = -1; x <= 1; x ++ ) {


		for ( var y : i32 = -1; y <= 1; y ++ ) {

			nodeVar9 = mx_worley_distance_0( nodeVar7, x, y, nodeVar3, nodeVar4, nodeVar1, nodeVar0 );

			if ( ( nodeVar9 < nodeVar8.x ) ) {

				nodeVar8.z = nodeVar8.y;
				nodeVar8.y = nodeVar8.x;
				nodeVar8.x = nodeVar9;
				

			} else {


				if ( ( nodeVar9 < nodeVar8.y ) ) {

					nodeVar8.z = nodeVar8.y;
					nodeVar8.y = nodeVar9;
					

				} else {


					if ( ( nodeVar9 < nodeVar8.z ) ) {

						nodeVar8.z = nodeVar9;
						

					}

					

				}

				

			}


		}


	}


	if ( ( nodeVar0 == 0 ) ) {

		nodeVar8 = sqrt( nodeVar8 );
		

	}


	return nodeVar8;

}


fn mx_worley_noise_vec3_1 ( p : vec3<f32>, jitter : f32, metric : i32 ) -> vec3<f32> {

	var nodeVar0 : i32;
	var nodeVar1 : f32;
	var nodeVar2 : vec3<f32>;
	var nodeVar3 : i32;
	var nodeVar4 : i32;
	var nodeVar5 : i32;
	var nodeVar6 : f32;
	var nodeVar7 : f32;
	var nodeVar8 : f32;
	var nodeVar9 : vec3<f32>;
	var nodeVar10 : vec3<f32>;
	var nodeVar11 : f32;

	nodeVar0 = metric;
	nodeVar1 = jitter;
	nodeVar2 = p;
	nodeVar3 = 0;
	nodeVar4 = 0;
	nodeVar5 = 0;
	nodeVar6 = nodeVar2.x;
	nodeVar3 = mx_floor( nodeVar6 );
	nodeVar7 = nodeVar2.y;
	nodeVar4 = mx_floor( nodeVar7 );
	nodeVar8 = nodeVar2.z;
	nodeVar5 = mx_floor( nodeVar8 );
	nodeVar9 = vec3<f32>( ( nodeVar6 - f32( nodeVar3 ) ), ( nodeVar7 - f32( nodeVar4 ) ), ( nodeVar8 - f32( nodeVar5 ) ) );
	nodeVar10 = vec3<f32>( 1000000.0, 1000000.0, 1000000.0 );

	for ( var x : i32 = -1; x <= 1; x ++ ) {


		for ( var y : i32 = -1; y <= 1; y ++ ) {


			for ( var z : i32 = -1; z <= 1; z ++ ) {

				nodeVar11 = mx_worley_distance_1( nodeVar9, x, y, z, nodeVar3, nodeVar4, nodeVar5, nodeVar1, nodeVar0 );

				if ( ( nodeVar11 < nodeVar10.x ) ) {

					nodeVar10.z = nodeVar10.y;
					nodeVar10.y = nodeVar10.x;
					nodeVar10.x = nodeVar11;
					

				} else {


					if ( ( nodeVar11 < nodeVar10.y ) ) {

						nodeVar10.z = nodeVar10.y;
						nodeVar10.y = nodeVar11;
						

					} else {


						if ( ( nodeVar11 < nodeVar10.z ) ) {

							nodeVar10.z = nodeVar11;
							

						}

						

					}

					

				}


			}


		}


	}


	if ( ( nodeVar0 == 0 ) ) {

		nodeVar10 = sqrt( nodeVar10 );
		

	}


	return nodeVar10;

}


fn mx_worley_noise_vec3_style_0 ( p : vec2<f32>, jitter : f32, style : i32, metric : i32 ) -> vec3<f32> {

	var nodeVar0 : i32;
	var nodeVar1 : i32;
	var nodeVar2 : f32;
	var nodeVar3 : vec2<f32>;
	var nodeVar4 : i32;
	var nodeVar5 : i32;
	var nodeVar6 : f32;
	var nodeVar7 : f32;
	var nodeVar8 : vec2<f32>;
	var nodeVar9 : f32;
	var nodeVar10 : vec2<f32>;
	var nodeVar11 : f32;
	var nodeVar12 : vec3<f32>;
	var nodeVar13 : vec2<f32>;
	var nodeVar14 : vec2<f32>;
	var nodeVar15 : vec3<f32>;

	nodeVar0 = metric;
	nodeVar1 = style;
	nodeVar2 = jitter;
	nodeVar3 = p;
	nodeVar4 = 0;
	nodeVar5 = 0;
	nodeVar6 = nodeVar3.x;
	nodeVar4 = mx_floor( nodeVar6 );
	nodeVar7 = nodeVar3.y;
	nodeVar5 = mx_floor( nodeVar7 );
	nodeVar8 = vec2<f32>( ( nodeVar6 - f32( nodeVar4 ) ), ( nodeVar7 - f32( nodeVar5 ) ) );
	nodeVar9 = 1000000.0;
	nodeVar10 = vec2<f32>( 0.0, 0.0 );

	for ( var x : i32 = -1; x <= 1; x ++ ) {


		for ( var y : i32 = -1; y <= 1; y ++ ) {

			nodeVar11 = mx_worley_distance_0( nodeVar8, x, y, nodeVar4, nodeVar5, nodeVar2, nodeVar0 );
			nodeVar12 = mx_cell_noise_vec3_1( vec2<f32>( f32( ( nodeVar4 + x ) ), f32( ( nodeVar5 + y ) ) ) );
			nodeVar13 = vec2<f32>( nodeVar12.x, nodeVar12.y );
			nodeVar13 = ( nodeVar13 - vec2<f32>( 0.5 ) );
			nodeVar13 = ( nodeVar13 * vec2<f32>( nodeVar2 ) );
			nodeVar13 = ( nodeVar13 + vec2<f32>( 0.5 ) );
			nodeVar14 = ( ( vec2<f32>( f32( x ), f32( y ) ) + nodeVar13 ) - nodeVar8 );

			if ( ( nodeVar11 < nodeVar9 ) ) {

				nodeVar9 = nodeVar11;
				nodeVar10 = nodeVar14;
				

			}


		}


	}

	nodeVar15 = mx_worley_noise_vec3_0( nodeVar3, nodeVar2, nodeVar0 );

	if ( ( nodeVar1 == 1 ) ) {

		nodeVar15 = mx_cell_noise_vec3_1( ( nodeVar10 + nodeVar3 ) );
		

	}


	return nodeVar15;

}


fn mx_worley_noise_vec3_style_1 ( p : vec3<f32>, jitter : f32, style : i32, metric : i32 ) -> vec3<f32> {

	var nodeVar0 : i32;
	var nodeVar1 : i32;
	var nodeVar2 : f32;
	var nodeVar3 : vec3<f32>;
	var nodeVar4 : i32;
	var nodeVar5 : i32;
	var nodeVar6 : i32;
	var nodeVar7 : f32;
	var nodeVar8 : f32;
	var nodeVar9 : f32;
	var nodeVar10 : vec3<f32>;
	var nodeVar11 : f32;
	var nodeVar12 : vec3<f32>;
	var nodeVar13 : f32;
	var nodeVar14 : vec3<f32>;
	var nodeVar15 : i32;
	var nodeVar16 : i32;
	var nodeVar17 : i32;
	var nodeVar18 : u32;
	var nodeVar19 : u32;
	var nodeVar20 : u32;
	var nodeVar21 : u32;
	var nodeVar22 : vec3<u32>;
	var nodeVar23 : vec3<f32>;
	var nodeVar24 : vec3<f32>;
	var nodeVar25 : vec3<f32>;
	var nodeVar26 : vec3<f32>;
	var nodeVar27 : i32;
	var nodeVar28 : i32;
	var nodeVar29 : i32;
	var nodeVar30 : u32;
	var nodeVar31 : u32;
	var nodeVar32 : u32;
	var nodeVar33 : u32;
	var nodeVar34 : vec3<u32>;

	nodeVar0 = metric;
	nodeVar1 = style;
	nodeVar2 = jitter;
	nodeVar3 = p;
	nodeVar4 = 0;
	nodeVar5 = 0;
	nodeVar6 = 0;
	nodeVar7 = nodeVar3.x;
	nodeVar4 = mx_floor( nodeVar7 );
	nodeVar8 = nodeVar3.y;
	nodeVar5 = mx_floor( nodeVar8 );
	nodeVar9 = nodeVar3.z;
	nodeVar6 = mx_floor( nodeVar9 );
	nodeVar10 = vec3<f32>( ( nodeVar7 - f32( nodeVar4 ) ), ( nodeVar8 - f32( nodeVar5 ) ), ( nodeVar9 - f32( nodeVar6 ) ) );
	nodeVar11 = 1000000.0;
	nodeVar12 = vec3<f32>( 0.0, 0.0, 0.0 );

	for ( var x : i32 = -1; x <= 1; x ++ ) {


		for ( var y : i32 = -1; y <= 1; y ++ ) {


			for ( var z : i32 = -1; z <= 1; z ++ ) {

				nodeVar13 = mx_worley_distance_1( nodeVar10, x, y, z, nodeVar4, nodeVar5, nodeVar6, nodeVar2, nodeVar0 );
				nodeVar14 = vec3<f32>( f32( ( nodeVar4 + x ) ), f32( ( nodeVar5 + y ) ), f32( ( nodeVar6 + z ) ) );
				nodeVar15 = i32( floor( nodeVar14.x ) );
				nodeVar16 = i32( floor( nodeVar14.y ) );
				nodeVar17 = i32( floor( nodeVar14.z ) );
				nodeVar18 = 3735928588u;
				nodeVar19 = 0u;
				nodeVar20 = 0u;
				nodeVar21 = 0u;
				nodeVar21 = nodeVar18;
				nodeVar20 = nodeVar21;
				nodeVar19 = nodeVar20;
				nodeVar19 = ( nodeVar19 + u32( nodeVar15 ) );
				nodeVar20 = ( nodeVar20 + u32( nodeVar16 ) );
				nodeVar21 = ( nodeVar21 + u32( nodeVar17 ) );
				nodeVar22 = mx_bjmix( nodeVar19, nodeVar20, nodeVar21 );
				nodeVar23 = vec3<f32>( mx_bits_to_01( mx_bjfinal( nodeVar22.x, nodeVar22.y, nodeVar22.z ) ), mx_bits_to_01( mx_bjfinal( ( nodeVar22.x + 1u ), nodeVar22.y, nodeVar22.z ) ), mx_bits_to_01( mx_bjfinal( ( nodeVar22.x + 2u ), nodeVar22.y, nodeVar22.z ) ) );
				nodeVar23 = ( nodeVar23 - vec3<f32>( 0.5 ) );
				nodeVar23 = ( nodeVar23 * vec3<f32>( nodeVar2 ) );
				nodeVar23 = ( nodeVar23 + vec3<f32>( 0.5 ) );
				nodeVar24 = ( ( vec3<f32>( f32( x ), f32( y ), f32( z ) ) + nodeVar23 ) - nodeVar10 );

				if ( ( nodeVar13 < nodeVar11 ) ) {

					nodeVar11 = nodeVar13;
					nodeVar12 = nodeVar24;
					

				}


			}


		}


	}

	nodeVar25 = mx_worley_noise_vec3_1( nodeVar3, nodeVar2, nodeVar0 );

	if ( ( nodeVar1 == 1 ) ) {

		nodeVar26 = ( nodeVar12 + nodeVar3 );
		nodeVar27 = i32( floor( nodeVar26.x ) );
		nodeVar28 = i32( floor( nodeVar26.y ) );
		nodeVar29 = i32( floor( nodeVar26.z ) );
		nodeVar30 = 3735928588u;
		nodeVar31 = 0u;
		nodeVar32 = 0u;
		nodeVar33 = 0u;
		nodeVar33 = nodeVar30;
		nodeVar32 = nodeVar33;
		nodeVar31 = nodeVar32;
		nodeVar31 = ( nodeVar31 + u32( nodeVar27 ) );
		nodeVar32 = ( nodeVar32 + u32( nodeVar28 ) );
		nodeVar33 = ( nodeVar33 + u32( nodeVar29 ) );
		nodeVar34 = mx_bjmix( nodeVar31, nodeVar32, nodeVar33 );
		nodeVar25 = vec3<f32>( mx_bits_to_01( mx_bjfinal( nodeVar34.x, nodeVar34.y, nodeVar34.z ) ), mx_bits_to_01( mx_bjfinal( ( nodeVar34.x + 1u ), nodeVar34.y, nodeVar34.z ) ), mx_bits_to_01( mx_bjfinal( ( nodeVar34.x + 2u ), nodeVar34.y, nodeVar34.z ) ) );
		

	}


	return nodeVar25;

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


fn mx_fractal_noise_float_2d ( p : vec2<f32>, octaves : i32, lacunarity : f32, diminish : f32 ) -> f32 {

	var nodeVar0 : f32;
	var nodeVar1 : f32;
	var nodeVar2 : vec2<f32>;
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

		nodeVar3 = ( nodeVar3 + ( nodeVar4 * mx_perlin_noise_float_0( nodeVar2 ) ) );
		nodeVar4 = ( nodeVar4 * nodeVar0 );
		nodeVar2 = ( nodeVar2 * vec2<f32>( nodeVar1 ) );

	}


	return nodeVar3;

}


fn mx_fractal_noise_vec2 ( p : vec3<f32>, octaves : i32, lacunarity : f32, diminish : f32 ) -> vec2<f32> {

	var nodeVar0 : f32;
	var nodeVar1 : f32;
	var nodeVar2 : i32;
	var nodeVar3 : vec3<f32>;

	nodeVar0 = diminish;
	nodeVar1 = lacunarity;
	nodeVar2 = octaves;
	nodeVar3 = p;

	return vec2<f32>( mx_fractal_noise_float( nodeVar3, nodeVar2, nodeVar1, nodeVar0 ), mx_fractal_noise_float( ( nodeVar3 + vec3<f32>( f32( 19 ), f32( 193 ), f32( 17 ) ) ), nodeVar2, nodeVar1, nodeVar0 ) );

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


fn mx_fractal_noise_vec4 ( p : vec3<f32>, octaves : i32, lacunarity : f32, diminish : f32 ) -> vec4<f32> {

	var nodeVar0 : f32;
	var nodeVar1 : f32;
	var nodeVar2 : i32;
	var nodeVar3 : vec3<f32>;
	var nodeVar4 : vec3<f32>;
	var nodeVar5 : f32;

	nodeVar0 = diminish;
	nodeVar1 = lacunarity;
	nodeVar2 = octaves;
	nodeVar3 = p;
	nodeVar4 = mx_fractal_noise_vec3( nodeVar3, nodeVar2, nodeVar1, nodeVar0 );
	nodeVar5 = mx_fractal_noise_float( ( nodeVar3 + vec3<f32>( f32( 19 ), f32( 193 ), f32( 17 ) ) ), nodeVar2, nodeVar1, nodeVar0 );

	return vec4<f32>( nodeVar4, nodeVar5 );

}


fn mx_hsvtorgb ( hsv : vec3<f32> ) -> vec3<f32> {

	var nodeVar0 : vec3<f32>;
	var nodeVar1 : f32;
	var nodeVar2 : f32;
	var nodeVar3 : f32;
	var nodeVar4 : f32;
	var nodeVar5 : f32;

	nodeVar0 = vec3<f32>( 0.0, 0.0, 0.0 );

	if ( ( hsv.y < 0.0001 ) ) {

		nodeVar0 = vec3<f32>( hsv.z, hsv.z, hsv.z );
		

	} else {

		nodeVar1 = ( ( hsv.x - floor( hsv.x ) ) * 6.0 );

		if ( ( i32( trunc( nodeVar1 ) ) == 0 ) ) {

			nodeVar0 = vec3<f32>( hsv.z, ( hsv.z * ( 1.0 - ( hsv.y * ( 1.0 - ( nodeVar1 - f32( i32( trunc( nodeVar1 ) ) ) ) ) ) ) ), ( hsv.z * ( 1.0 - hsv.y ) ) );
			

		} else {


			if ( ( i32( trunc( nodeVar1 ) ) == 1 ) ) {

				nodeVar2 = ( nodeVar1 - f32( i32( trunc( nodeVar1 ) ) ) );
				nodeVar3 = ( hsv.z * ( 1.0 - hsv.y ) );
				nodeVar0 = vec3<f32>( ( hsv.z * ( 1.0 - ( hsv.y * nodeVar2 ) ) ), hsv.z, nodeVar3 );
				

			} else {


				if ( ( i32( trunc( nodeVar1 ) ) == 2 ) ) {

					nodeVar4 = ( hsv.z * ( 1.0 - ( hsv.y * ( 1.0 - nodeVar2 ) ) ) );
					nodeVar0 = vec3<f32>( nodeVar3, hsv.z, nodeVar4 );
					

				} else {


					if ( ( i32( trunc( nodeVar1 ) ) == 3 ) ) {

						nodeVar5 = ( hsv.z * ( 1.0 - ( hsv.y * nodeVar2 ) ) );
						nodeVar0 = vec3<f32>( nodeVar3, nodeVar5, hsv.z );
						

					} else {


						if ( ( i32( trunc( nodeVar1 ) ) == 4 ) ) {

							nodeVar0 = vec3<f32>( nodeVar4, nodeVar3, hsv.z );
							

						} else {

							nodeVar0 = vec3<f32>( hsv.z, nodeVar3, nodeVar5 );
							

						}

						

					}

					

				}

				

			}

			

		}

		

	}


	return nodeVar0;

}


fn mx_rgbtohsv ( c : vec3<f32> ) -> vec3<f32> {

	var nodeVar0 : vec3<f32>;
	var nodeVar1 : f32;
	var nodeVar2 : f32;
	var nodeVar3 : f32;
	var nodeVar4 : f32;
	var nodeVar5 : f32;
	var nodeVar6 : f32;
	var nodeVar7 : f32;
	var nodeVar8 : f32;
	var nodeVar9 : f32;

	nodeVar0 = c;
	nodeVar1 = nodeVar0.x;
	nodeVar2 = nodeVar0.y;
	nodeVar3 = nodeVar0.z;
	nodeVar4 = min( nodeVar1, min( nodeVar2, nodeVar3 ) );
	nodeVar5 = max( nodeVar1, max( nodeVar2, nodeVar3 ) );
	nodeVar6 = ( nodeVar5 - nodeVar4 );
	nodeVar7 = 0.0;
	nodeVar8 = 0.0;
	nodeVar9 = 0.0;
	nodeVar9 = nodeVar5;

	if ( ( nodeVar5 > 0.0 ) ) {

		nodeVar8 = ( nodeVar6 / nodeVar5 );
		

	} else {

		nodeVar8 = 0.0;
		

	}


	if ( ( nodeVar8 <= 0.0 ) ) {

		nodeVar7 = 0.0;
		

	} else {


		if ( ( nodeVar1 >= nodeVar5 ) ) {

			nodeVar7 = ( ( nodeVar2 - nodeVar3 ) / nodeVar6 );
			

		} else {


			if ( ( nodeVar2 >= nodeVar5 ) ) {

				nodeVar7 = ( 2.0 + ( ( nodeVar3 - nodeVar1 ) / nodeVar6 ) );
				

			} else {

				nodeVar7 = ( 4.0 + ( ( nodeVar1 - nodeVar2 ) / nodeVar6 ) );
				

			}

			

		}

		nodeVar7 = ( nodeVar7 * 0.16666666666666666 );

		if ( ( nodeVar7 < 0.0 ) ) {

			nodeVar7 = ( nodeVar7 + 1.0 );
			

		}

		

	}


	return vec3<f32>( nodeVar7, nodeVar8, nodeVar9 );

}


fn mx_srgb_texture_to_lin_rec709 ( color : vec3<f32> ) -> vec3<f32> {

	var nodeVar0 : vec3<f32>;
	var nodeVar1 : vec3<bool>;
	var nodeVar2 : vec3<f32>;
	var nodeVar3 : vec3<f32>;

	nodeVar0 = color;
	nodeVar1 = ( nodeVar0 > vec3<f32>( 0.04045, 0.04045, 0.04045 ) );
	nodeVar2 = ( nodeVar0 / vec3<f32>( 12.92 ) );
	nodeVar3 = pow( ( max( ( nodeVar0 + vec3<f32>( 0.055, 0.055, 0.055 ) ), vec3<f32>( 0.0, 0.0, 0.0 ) ) / vec3<f32>( 1.055 ) ), vec3<f32>( 2.4, 2.4, 2.4 ) );

	return mix( nodeVar2, nodeVar3, vec3<f32>( nodeVar1 ) );

}


fn t_noise_float_2 ( a0 : vec2<f32> ) -> f32 {

	


	return ( ( mx_perlin_noise_float_0( a0 ) * 1.0 ) + 0.0 );

}


fn t_noise_float_3 ( a0 : vec3<f32> ) -> f32 {

	


	return ( ( mx_perlin_noise_float_1( a0 ) * 1.0 ) + 0.0 );

}


fn t_noise_vec3_2 ( a0 : vec2<f32> ) -> vec3<f32> {

	


	return ( ( mx_perlin_noise_vec3_0( a0 ) * vec3<f32>( 1.0 ) ) + vec3<f32>( 0.0 ) );

}


fn t_noise_vec3_3 ( a0 : vec3<f32> ) -> vec3<f32> {

	


	return ( ( mx_perlin_noise_vec3_1( a0 ) * vec3<f32>( 2.0 ) ) + vec3<f32>( 0.5 ) );

}


fn t_noise_vec4_2 ( a0 : vec2<f32> ) -> vec4<f32> {

	


	return ( ( vec4<f32>( mx_perlin_noise_vec3_0( a0 ), mx_perlin_noise_float_0( ( a0 + vec2<f32>( 19.0, 73.0 ) ) ) ) * vec4<f32>( 1.0 ) ) + vec4<f32>( 0.0 ) );

}


fn t_noise_vec4_3 ( a0 : vec3<f32> ) -> vec4<f32> {

	


	return ( ( vec4<f32>( mx_perlin_noise_vec3_1( a0 ), mx_perlin_noise_float_1( ( a0 + vec3<f32>( vec2<f32>( 19.0, 73.0 ), 0.0 ) ) ) ) * vec4<f32>( 1.0 ) ) + vec4<f32>( 0.0 ) );

}


fn t_cell_noise_float_2 ( a0 : vec2<f32> ) -> f32 {

	


	return mx_cell_noise_float_1( a0 );

}


fn t_cell_noise_float_3 ( a0 : vec3<f32> ) -> f32 {

	


	return mx_cell_noise_float_2( a0 );

}


fn t_cell_noise_vec3_2 ( a0 : vec2<f32> ) -> vec3<f32> {

	


	return mx_cell_noise_vec3_1( a0 );

}


fn t_cell_noise_vec3_3 ( a0 : vec3<f32> ) -> vec3<f32> {

	


	return mx_cell_noise_vec3_2( a0 );

}


fn t_worley_noise_float_2 ( a0 : vec2<f32> ) -> f32 {

	var nodeVar0 : vec3<f32>;
	var nodeVar1 : f32;
	var nodeVar2 : i32;
	var nodeVar3 : i32;
	var nodeVar4 : i32;
	var nodeVar5 : i32;
	var nodeVar6 : f32;
	var nodeVar7 : f32;
	var nodeVar8 : f32;
	var nodeVar9 : vec3<f32>;
	var nodeVar10 : f32;
	var nodeVar11 : vec3<f32>;
	var nodeVar12 : f32;
	var nodeVar13 : vec3<f32>;
	var nodeVar14 : i32;
	var nodeVar15 : i32;
	var nodeVar16 : i32;
	var nodeVar17 : u32;
	var nodeVar18 : u32;
	var nodeVar19 : u32;
	var nodeVar20 : u32;
	var nodeVar21 : vec3<u32>;
	var nodeVar22 : vec3<f32>;
	var nodeVar23 : vec3<f32>;

	nodeVar0 = vec3<f32>( a0, 0.0 );
	nodeVar1 = 1.0;
	nodeVar2 = 0;
	nodeVar3 = 0;
	nodeVar4 = 0;
	nodeVar5 = 0;
	nodeVar6 = nodeVar0.x;
	nodeVar3 = mx_floor( nodeVar6 );
	nodeVar7 = nodeVar0.y;
	nodeVar4 = mx_floor( nodeVar7 );
	nodeVar8 = nodeVar0.z;
	nodeVar5 = mx_floor( nodeVar8 );
	nodeVar9 = vec3<f32>( ( nodeVar6 - f32( nodeVar3 ) ), ( nodeVar7 - f32( nodeVar4 ) ), ( nodeVar8 - f32( nodeVar5 ) ) );
	nodeVar10 = 1000000.0;
	nodeVar11 = vec3<f32>( 0.0, 0.0, 0.0 );

	for ( var x : i32 = -1; x <= 1; x ++ ) {


		for ( var y : i32 = -1; y <= 1; y ++ ) {


			for ( var z : i32 = -1; z <= 1; z ++ ) {

				nodeVar12 = mx_worley_distance_1( nodeVar9, x, y, z, nodeVar3, nodeVar4, nodeVar5, nodeVar1, 0 );
				nodeVar13 = vec3<f32>( f32( ( nodeVar3 + x ) ), f32( ( nodeVar4 + y ) ), f32( ( nodeVar5 + z ) ) );
				nodeVar14 = i32( floor( nodeVar13.x ) );
				nodeVar15 = i32( floor( nodeVar13.y ) );
				nodeVar16 = i32( floor( nodeVar13.z ) );
				nodeVar17 = 3735928588u;
				nodeVar18 = 0u;
				nodeVar19 = 0u;
				nodeVar20 = 0u;
				nodeVar20 = nodeVar17;
				nodeVar19 = nodeVar20;
				nodeVar18 = nodeVar19;
				nodeVar18 = ( nodeVar18 + u32( nodeVar14 ) );
				nodeVar19 = ( nodeVar19 + u32( nodeVar15 ) );
				nodeVar20 = ( nodeVar20 + u32( nodeVar16 ) );
				nodeVar21 = mx_bjmix( nodeVar18, nodeVar19, nodeVar20 );
				nodeVar22 = vec3<f32>( mx_bits_to_01( mx_bjfinal( nodeVar21.x, nodeVar21.y, nodeVar21.z ) ), mx_bits_to_01( mx_bjfinal( ( nodeVar21.x + 1u ), nodeVar21.y, nodeVar21.z ) ), mx_bits_to_01( mx_bjfinal( ( nodeVar21.x + 2u ), nodeVar21.y, nodeVar21.z ) ) );
				nodeVar22 = ( nodeVar22 - vec3<f32>( 0.5 ) );
				nodeVar22 = ( nodeVar22 * vec3<f32>( nodeVar1 ) );
				nodeVar22 = ( nodeVar22 + vec3<f32>( 0.5 ) );
				nodeVar23 = ( ( vec3<f32>( f32( x ), f32( y ), f32( z ) ) + nodeVar22 ) - nodeVar9 );

				if ( ( nodeVar12 < nodeVar10 ) ) {

					nodeVar10 = nodeVar12;
					nodeVar11 = nodeVar23;
					

				}


			}


		}


	}


	if ( ( nodeVar2 == 1 ) ) {

		nodeVar10 = mx_cell_noise_float_2( ( nodeVar11 + nodeVar0 ) );
		

	} else {

		nodeVar10 = sqrt( nodeVar10 );
		

	}


	return nodeVar10;

}


fn t_worley_noise_float_3 ( a0 : vec3<f32> ) -> f32 {

	var nodeVar0 : vec3<f32>;
	var nodeVar1 : f32;
	var nodeVar2 : i32;
	var nodeVar3 : i32;
	var nodeVar4 : i32;
	var nodeVar5 : i32;
	var nodeVar6 : f32;
	var nodeVar7 : f32;
	var nodeVar8 : f32;
	var nodeVar9 : vec3<f32>;
	var nodeVar10 : f32;
	var nodeVar11 : vec3<f32>;
	var nodeVar12 : f32;
	var nodeVar13 : vec3<f32>;
	var nodeVar14 : i32;
	var nodeVar15 : i32;
	var nodeVar16 : i32;
	var nodeVar17 : u32;
	var nodeVar18 : u32;
	var nodeVar19 : u32;
	var nodeVar20 : u32;
	var nodeVar21 : vec3<u32>;
	var nodeVar22 : vec3<f32>;
	var nodeVar23 : vec3<f32>;

	nodeVar0 = a0;
	nodeVar1 = 0.5;
	nodeVar2 = 1;
	nodeVar3 = 0;
	nodeVar4 = 0;
	nodeVar5 = 0;
	nodeVar6 = nodeVar0.x;
	nodeVar3 = mx_floor( nodeVar6 );
	nodeVar7 = nodeVar0.y;
	nodeVar4 = mx_floor( nodeVar7 );
	nodeVar8 = nodeVar0.z;
	nodeVar5 = mx_floor( nodeVar8 );
	nodeVar9 = vec3<f32>( ( nodeVar6 - f32( nodeVar3 ) ), ( nodeVar7 - f32( nodeVar4 ) ), ( nodeVar8 - f32( nodeVar5 ) ) );
	nodeVar10 = 1000000.0;
	nodeVar11 = vec3<f32>( 0.0, 0.0, 0.0 );

	for ( var x : i32 = -1; x <= 1; x ++ ) {


		for ( var y : i32 = -1; y <= 1; y ++ ) {


			for ( var z : i32 = -1; z <= 1; z ++ ) {

				nodeVar12 = mx_worley_distance_1( nodeVar9, x, y, z, nodeVar3, nodeVar4, nodeVar5, nodeVar1, 0 );
				nodeVar13 = vec3<f32>( f32( ( nodeVar3 + x ) ), f32( ( nodeVar4 + y ) ), f32( ( nodeVar5 + z ) ) );
				nodeVar14 = i32( floor( nodeVar13.x ) );
				nodeVar15 = i32( floor( nodeVar13.y ) );
				nodeVar16 = i32( floor( nodeVar13.z ) );
				nodeVar17 = 3735928588u;
				nodeVar18 = 0u;
				nodeVar19 = 0u;
				nodeVar20 = 0u;
				nodeVar20 = nodeVar17;
				nodeVar19 = nodeVar20;
				nodeVar18 = nodeVar19;
				nodeVar18 = ( nodeVar18 + u32( nodeVar14 ) );
				nodeVar19 = ( nodeVar19 + u32( nodeVar15 ) );
				nodeVar20 = ( nodeVar20 + u32( nodeVar16 ) );
				nodeVar21 = mx_bjmix( nodeVar18, nodeVar19, nodeVar20 );
				nodeVar22 = vec3<f32>( mx_bits_to_01( mx_bjfinal( nodeVar21.x, nodeVar21.y, nodeVar21.z ) ), mx_bits_to_01( mx_bjfinal( ( nodeVar21.x + 1u ), nodeVar21.y, nodeVar21.z ) ), mx_bits_to_01( mx_bjfinal( ( nodeVar21.x + 2u ), nodeVar21.y, nodeVar21.z ) ) );
				nodeVar22 = ( nodeVar22 - vec3<f32>( 0.5 ) );
				nodeVar22 = ( nodeVar22 * vec3<f32>( nodeVar1 ) );
				nodeVar22 = ( nodeVar22 + vec3<f32>( 0.5 ) );
				nodeVar23 = ( ( vec3<f32>( f32( x ), f32( y ), f32( z ) ) + nodeVar22 ) - nodeVar9 );

				if ( ( nodeVar12 < nodeVar10 ) ) {

					nodeVar10 = nodeVar12;
					nodeVar11 = nodeVar23;
					

				}


			}


		}


	}


	if ( ( nodeVar2 == 1 ) ) {

		nodeVar10 = mx_cell_noise_float_2( ( nodeVar11 + nodeVar0 ) );
		

	} else {

		nodeVar10 = sqrt( nodeVar10 );
		

	}


	return nodeVar10;

}


fn t_worley_noise_float_2d ( a0 : vec2<f32> ) -> f32 {

	var nodeVar0 : vec2<f32>;
	var nodeVar1 : f32;
	var nodeVar2 : i32;
	var nodeVar3 : vec2<f32>;
	var nodeVar4 : vec2<f32>;
	var nodeVar5 : f32;
	var nodeVar6 : vec2<f32>;
	var nodeVar7 : vec2<f32>;
	var nodeVar8 : vec2<f32>;
	var nodeVar9 : vec2<f32>;
	var nodeVar10 : vec2<f32>;
	var nodeVar11 : f32;

	nodeVar0 = a0;
	nodeVar1 = 1.0;
	nodeVar2 = 0;
	nodeVar3 = floor( nodeVar0 );
	nodeVar4 = vec2<f32>( fract( nodeVar0.x ), fract( nodeVar0.y ) );
	nodeVar5 = 1000000.0;
	nodeVar6 = vec2<f32>( 0.0, 0.0 );

	for ( var x : i32 = -1; x <= 1; x ++ ) {


		for ( var y : i32 = -1; y <= 1; y ++ ) {

			nodeVar7 = vec2<f32>( f32( x ), f32( y ) );
			nodeVar8 = vec2<f32>( ( nodeVar7.x + nodeVar3.x ), ( nodeVar7.y + nodeVar3.y ) );
			nodeVar9 = vec2<f32>( mx_cell_noise_float_2( vec3<f32>( nodeVar8.x, nodeVar8.y, 0.0 ) ), mx_cell_noise_float_2( vec3<f32>( nodeVar8.x, nodeVar8.y, 1.0 ) ) );
			nodeVar9 = ( nodeVar9 - vec2<f32>( 0.5 ) );
			nodeVar9 = ( nodeVar9 * vec2<f32>( nodeVar1 ) );
			nodeVar9 = ( nodeVar9 + vec2<f32>( 0.5 ) );
			nodeVar10 = ( ( nodeVar7 + nodeVar9 ) - nodeVar4 );
			nodeVar11 = dot( nodeVar10, nodeVar10 );

			if ( ( nodeVar11 < nodeVar5 ) ) {

				nodeVar5 = nodeVar11;
				nodeVar6 = nodeVar10;
				

			}


		}


	}


	if ( ( nodeVar2 == 1 ) ) {

		nodeVar5 = mx_cell_noise_float_1( ( nodeVar6 + nodeVar0 ) );
		

	} else {

		nodeVar5 = sqrt( nodeVar5 );
		

	}


	return nodeVar5;

}


fn t_worley_noise_float_3d ( a0 : vec3<f32> ) -> f32 {

	var nodeVar0 : vec3<f32>;
	var nodeVar1 : f32;
	var nodeVar2 : i32;
	var nodeVar3 : i32;
	var nodeVar4 : i32;
	var nodeVar5 : i32;
	var nodeVar6 : f32;
	var nodeVar7 : f32;
	var nodeVar8 : f32;
	var nodeVar9 : vec3<f32>;
	var nodeVar10 : f32;
	var nodeVar11 : vec3<f32>;
	var nodeVar12 : f32;
	var nodeVar13 : vec3<f32>;
	var nodeVar14 : i32;
	var nodeVar15 : i32;
	var nodeVar16 : i32;
	var nodeVar17 : u32;
	var nodeVar18 : u32;
	var nodeVar19 : u32;
	var nodeVar20 : u32;
	var nodeVar21 : vec3<u32>;
	var nodeVar22 : vec3<f32>;
	var nodeVar23 : vec3<f32>;

	nodeVar0 = a0;
	nodeVar1 = 1.0;
	nodeVar2 = 0;
	nodeVar3 = 0;
	nodeVar4 = 0;
	nodeVar5 = 0;
	nodeVar6 = nodeVar0.x;
	nodeVar3 = mx_floor( nodeVar6 );
	nodeVar7 = nodeVar0.y;
	nodeVar4 = mx_floor( nodeVar7 );
	nodeVar8 = nodeVar0.z;
	nodeVar5 = mx_floor( nodeVar8 );
	nodeVar9 = vec3<f32>( ( nodeVar6 - f32( nodeVar3 ) ), ( nodeVar7 - f32( nodeVar4 ) ), ( nodeVar8 - f32( nodeVar5 ) ) );
	nodeVar10 = 1000000.0;
	nodeVar11 = vec3<f32>( 0.0, 0.0, 0.0 );

	for ( var x : i32 = -1; x <= 1; x ++ ) {


		for ( var y : i32 = -1; y <= 1; y ++ ) {


			for ( var z : i32 = -1; z <= 1; z ++ ) {

				nodeVar12 = mx_worley_distance_1( nodeVar9, x, y, z, nodeVar3, nodeVar4, nodeVar5, nodeVar1, 0 );
				nodeVar13 = vec3<f32>( f32( ( nodeVar3 + x ) ), f32( ( nodeVar4 + y ) ), f32( ( nodeVar5 + z ) ) );
				nodeVar14 = i32( floor( nodeVar13.x ) );
				nodeVar15 = i32( floor( nodeVar13.y ) );
				nodeVar16 = i32( floor( nodeVar13.z ) );
				nodeVar17 = 3735928588u;
				nodeVar18 = 0u;
				nodeVar19 = 0u;
				nodeVar20 = 0u;
				nodeVar20 = nodeVar17;
				nodeVar19 = nodeVar20;
				nodeVar18 = nodeVar19;
				nodeVar18 = ( nodeVar18 + u32( nodeVar14 ) );
				nodeVar19 = ( nodeVar19 + u32( nodeVar15 ) );
				nodeVar20 = ( nodeVar20 + u32( nodeVar16 ) );
				nodeVar21 = mx_bjmix( nodeVar18, nodeVar19, nodeVar20 );
				nodeVar22 = vec3<f32>( mx_bits_to_01( mx_bjfinal( nodeVar21.x, nodeVar21.y, nodeVar21.z ) ), mx_bits_to_01( mx_bjfinal( ( nodeVar21.x + 1u ), nodeVar21.y, nodeVar21.z ) ), mx_bits_to_01( mx_bjfinal( ( nodeVar21.x + 2u ), nodeVar21.y, nodeVar21.z ) ) );
				nodeVar22 = ( nodeVar22 - vec3<f32>( 0.5 ) );
				nodeVar22 = ( nodeVar22 * vec3<f32>( nodeVar1 ) );
				nodeVar22 = ( nodeVar22 + vec3<f32>( 0.5 ) );
				nodeVar23 = ( ( vec3<f32>( f32( x ), f32( y ), f32( z ) ) + nodeVar22 ) - nodeVar9 );

				if ( ( nodeVar12 < nodeVar10 ) ) {

					nodeVar10 = nodeVar12;
					nodeVar11 = nodeVar23;
					

				}


			}


		}


	}


	if ( ( nodeVar2 == 1 ) ) {

		nodeVar10 = mx_cell_noise_float_2( ( nodeVar11 + nodeVar0 ) );
		

	} else {

		nodeVar10 = sqrt( nodeVar10 );
		

	}


	return nodeVar10;

}


fn t_worley_noise_vec2_2 ( a0 : vec2<f32> ) -> vec2<f32> {

	


	return mx_worley_noise_vec2_0( a0, 1.0, 1 );

}


fn t_worley_noise_vec2_3 ( a0 : vec3<f32> ) -> vec2<f32> {

	


	return mx_worley_noise_vec2_1( a0, 1.0, 1 );

}


fn t_worley_noise_vec3_2 ( a0 : vec2<f32> ) -> vec3<f32> {

	


	return mx_worley_noise_vec3_0( a0, 1.0, 1 );

}


fn t_worley_noise_vec3_3 ( a0 : vec3<f32> ) -> vec3<f32> {

	


	return mx_worley_noise_vec3_1( a0, 1.0, 2 );

}


fn t_worley_noise_vec3_style_2 ( a0 : vec2<f32> ) -> vec3<f32> {

	


	return mx_worley_noise_vec3_style_0( a0, 1.0, 0, 0 );

}


fn t_worley_noise_vec3_style_3 ( a0 : vec3<f32> ) -> vec3<f32> {

	


	return mx_worley_noise_vec3_style_1( a0, 1.0, 1, 0 );

}


fn t_unifiednoise2d ( a0 : vec2<f32> ) -> f32 {

	var nodeVar0 : i32;
	var nodeVar1 : vec2<f32>;
	var nodeVar2 : vec2<f32>;
	var nodeVar3 : vec2<f32>;
	var nodeVar4 : f32;
	var nodeVar5 : f32;
	var nodeVar6 : f32;
	var nodeVar7 : f32;
	var nodeVar8 : i32;
	var nodeVar9 : f32;
	var nodeVar10 : f32;
	var nodeVar11 : i32;
	var nodeVar12 : vec2<f32>;
	var nodeVar13 : vec2<f32>;
	var nodeVar14 : f32;
	var nodeVar15 : f32;
	var nodeVar16 : f32;
	var nodeVar17 : f32;
	var nodeVar18 : vec2<f32>;
	var nodeVar19 : vec3<f32>;
	var nodeVar20 : f32;
	var nodeVar21 : vec2<f32>;
	var nodeVar22 : f32;
	var nodeVar23 : i32;
	var nodeVar24 : vec2<f32>;
	var nodeVar25 : vec2<f32>;
	var nodeVar26 : f32;
	var nodeVar27 : vec2<f32>;
	var nodeVar28 : vec2<f32>;
	var nodeVar29 : vec2<f32>;
	var nodeVar30 : vec2<f32>;
	var nodeVar31 : vec2<f32>;
	var nodeVar32 : f32;
	var nodeVar33 : f32;
	var nodeVar34 : f32;
	var nodeVar35 : f32;

	nodeVar0 = 0;
	nodeVar1 = a0;
	nodeVar2 = vec2<f32>( 1.0, 1.0 );
	nodeVar3 = vec2<f32>( 0.0, 0.0 );
	nodeVar4 = 1.0;
	nodeVar5 = 0.0;
	nodeVar6 = 1.0;
	nodeVar7 = f32( false );
	nodeVar8 = 1;
	nodeVar9 = 2.0;
	nodeVar10 = 0.5;
	nodeVar11 = 0;
	nodeVar12 = ( nodeVar1 * nodeVar2 );
	nodeVar13 = ( nodeVar12 + nodeVar3 );
	nodeVar14 = ( ( nodeVar4 - 1.0 ) * 90000.0 );
	nodeVar15 = ( nodeVar14 * 0.017453292519943295 );
	nodeVar16 = cos( nodeVar15 );
	nodeVar17 = sin( nodeVar15 );
	nodeVar18 = vec2<f32>( ( ( nodeVar16 * nodeVar13.x ) + ( nodeVar17 * nodeVar13.y ) ), ( ( nodeVar16 * nodeVar13.y ) - ( nodeVar17 * nodeVar13.x ) ) );
	nodeVar19 = vec3<f32>( nodeVar13.x, nodeVar13.y, nodeVar14 );
	nodeVar20 = 0.0;

	if ( ( nodeVar0 == 0 ) ) {

		nodeVar20 = ( ( mx_perlin_noise_float_0( nodeVar18 ) * 0.5 ) + 0.5 );
		

	}


	if ( ( nodeVar0 == 1 ) ) {

		nodeVar20 = mx_cell_noise_float_1( nodeVar18 );
		

	}


	if ( ( nodeVar0 == 2 ) ) {

		nodeVar21 = nodeVar13;
		nodeVar22 = nodeVar4;
		nodeVar23 = nodeVar11;
		nodeVar24 = floor( nodeVar21 );
		nodeVar25 = vec2<f32>( fract( nodeVar21.x ), fract( nodeVar21.y ) );
		nodeVar26 = 1000000.0;
		nodeVar27 = vec2<f32>( 0.0, 0.0 );

		for ( var x : i32 = -1; x <= 1; x ++ ) {


			for ( var y : i32 = -1; y <= 1; y ++ ) {

				nodeVar28 = vec2<f32>( f32( x ), f32( y ) );
				nodeVar29 = vec2<f32>( ( nodeVar28.x + nodeVar24.x ), ( nodeVar28.y + nodeVar24.y ) );
				nodeVar30 = vec2<f32>( mx_cell_noise_float_2( vec3<f32>( nodeVar29.x, nodeVar29.y, 0.0 ) ), mx_cell_noise_float_2( vec3<f32>( nodeVar29.x, nodeVar29.y, 1.0 ) ) );
				nodeVar30 = ( nodeVar30 - vec2<f32>( 0.5 ) );
				nodeVar30 = ( nodeVar30 * vec2<f32>( nodeVar22 ) );
				nodeVar30 = ( nodeVar30 + vec2<f32>( 0.5 ) );
				nodeVar31 = ( ( nodeVar28 + nodeVar30 ) - nodeVar25 );
				nodeVar32 = dot( nodeVar31, nodeVar31 );

				if ( ( nodeVar32 < nodeVar26 ) ) {

					nodeVar26 = nodeVar32;
					nodeVar27 = nodeVar31;
					

				}


			}


		}


		if ( ( nodeVar23 == 1 ) ) {

			nodeVar26 = mx_cell_noise_float_1( ( nodeVar27 + nodeVar21 ) );
			

		} else {

			nodeVar26 = sqrt( nodeVar26 );
			

		}

		nodeVar20 = nodeVar26;
		

	}


	if ( ( nodeVar0 == 3 ) ) {

		nodeVar20 = mx_fractal_noise_float( nodeVar19, nodeVar8, nodeVar9, nodeVar10 );
		

	}

	nodeVar33 = ( nodeVar5 + ( nodeVar20 * ( nodeVar6 - nodeVar5 ) ) );
	nodeVar34 = clamp( nodeVar33, nodeVar5, nodeVar6 );
	nodeVar35 = nodeVar33;

	if ( ( nodeVar7 == 1.0 ) ) {

		nodeVar35 = nodeVar34;
		

	}


	return nodeVar35;

}


fn t_unifiednoise3d ( a0 : vec3<f32> ) -> f32 {

	var nodeVar0 : i32;
	var nodeVar1 : vec3<f32>;
	var nodeVar2 : vec3<f32>;
	var nodeVar3 : vec3<f32>;
	var nodeVar4 : f32;
	var nodeVar5 : f32;
	var nodeVar6 : f32;
	var nodeVar7 : f32;
	var nodeVar8 : i32;
	var nodeVar9 : f32;
	var nodeVar10 : f32;
	var nodeVar11 : i32;
	var nodeVar12 : vec3<f32>;
	var nodeVar13 : vec3<f32>;
	var nodeVar14 : f32;
	var nodeVar15 : f32;
	var nodeVar16 : f32;
	var nodeVar17 : vec3<f32>;
	var nodeVar18 : vec3<f32>;
	var nodeVar19 : f32;
	var nodeVar20 : vec3<f32>;
	var nodeVar21 : f32;
	var nodeVar22 : i32;
	var nodeVar23 : i32;
	var nodeVar24 : i32;
	var nodeVar25 : i32;
	var nodeVar26 : f32;
	var nodeVar27 : f32;
	var nodeVar28 : f32;
	var nodeVar29 : vec3<f32>;
	var nodeVar30 : f32;
	var nodeVar31 : vec3<f32>;
	var nodeVar32 : f32;
	var nodeVar33 : vec3<f32>;
	var nodeVar34 : i32;
	var nodeVar35 : i32;
	var nodeVar36 : i32;
	var nodeVar37 : u32;
	var nodeVar38 : u32;
	var nodeVar39 : u32;
	var nodeVar40 : u32;
	var nodeVar41 : vec3<u32>;
	var nodeVar42 : vec3<f32>;
	var nodeVar43 : vec3<f32>;
	var nodeVar44 : f32;
	var nodeVar45 : f32;
	var nodeVar46 : f32;

	nodeVar0 = 2;
	nodeVar1 = a0;
	nodeVar2 = vec3<f32>( 1.0, 1.0, 1.0 );
	nodeVar3 = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar4 = 1.0;
	nodeVar5 = 0.0;
	nodeVar6 = 1.0;
	nodeVar7 = f32( false );
	nodeVar8 = 1;
	nodeVar9 = 2.0;
	nodeVar10 = 0.5;
	nodeVar11 = 0;
	nodeVar12 = ( nodeVar1 * nodeVar2 );
	nodeVar13 = ( nodeVar12 + nodeVar3 );
	nodeVar14 = ( ( nodeVar4 - 1.0 ) * 90000.0 );
	nodeVar15 = ( nodeVar14 * 0.017453292519943295 );
	nodeVar16 = cos( nodeVar15 );
	nodeVar17 = normalize( vec3<f32>( 0.1, 1.0, 0.0 ) );
	nodeVar18 = ( ( ( nodeVar13 * vec3<f32>( nodeVar16 ) ) + ( cross( nodeVar13, nodeVar17 ) * vec3<f32>( sin( nodeVar15 ) ) ) ) + ( nodeVar17 * vec3<f32>( ( dot( nodeVar17, nodeVar13 ) * ( 1.0 - nodeVar16 ) ) ) ) );
	nodeVar19 = ( ( mx_perlin_noise_float_1( nodeVar18 ) * 0.5 ) + 0.5 );

	if ( ( nodeVar0 == 1 ) ) {

		nodeVar19 = mx_cell_noise_float_2( nodeVar18 );
		

	}


	if ( ( nodeVar0 == 2 ) ) {

		nodeVar20 = nodeVar13;
		nodeVar21 = nodeVar4;
		nodeVar22 = nodeVar11;
		nodeVar23 = 0;
		nodeVar24 = 0;
		nodeVar25 = 0;
		nodeVar26 = nodeVar20.x;
		nodeVar23 = mx_floor( nodeVar26 );
		nodeVar27 = nodeVar20.y;
		nodeVar24 = mx_floor( nodeVar27 );
		nodeVar28 = nodeVar20.z;
		nodeVar25 = mx_floor( nodeVar28 );
		nodeVar29 = vec3<f32>( ( nodeVar26 - f32( nodeVar23 ) ), ( nodeVar27 - f32( nodeVar24 ) ), ( nodeVar28 - f32( nodeVar25 ) ) );
		nodeVar30 = 1000000.0;
		nodeVar31 = vec3<f32>( 0.0, 0.0, 0.0 );

		for ( var x : i32 = -1; x <= 1; x ++ ) {


			for ( var y : i32 = -1; y <= 1; y ++ ) {


				for ( var z : i32 = -1; z <= 1; z ++ ) {

					nodeVar32 = mx_worley_distance_1( nodeVar29, x, y, z, nodeVar23, nodeVar24, nodeVar25, nodeVar21, 0 );
					nodeVar33 = vec3<f32>( f32( ( nodeVar23 + x ) ), f32( ( nodeVar24 + y ) ), f32( ( nodeVar25 + z ) ) );
					nodeVar34 = i32( floor( nodeVar33.x ) );
					nodeVar35 = i32( floor( nodeVar33.y ) );
					nodeVar36 = i32( floor( nodeVar33.z ) );
					nodeVar37 = 3735928588u;
					nodeVar38 = 0u;
					nodeVar39 = 0u;
					nodeVar40 = 0u;
					nodeVar40 = nodeVar37;
					nodeVar39 = nodeVar40;
					nodeVar38 = nodeVar39;
					nodeVar38 = ( nodeVar38 + u32( nodeVar34 ) );
					nodeVar39 = ( nodeVar39 + u32( nodeVar35 ) );
					nodeVar40 = ( nodeVar40 + u32( nodeVar36 ) );
					nodeVar41 = mx_bjmix( nodeVar38, nodeVar39, nodeVar40 );
					nodeVar42 = vec3<f32>( mx_bits_to_01( mx_bjfinal( nodeVar41.x, nodeVar41.y, nodeVar41.z ) ), mx_bits_to_01( mx_bjfinal( ( nodeVar41.x + 1u ), nodeVar41.y, nodeVar41.z ) ), mx_bits_to_01( mx_bjfinal( ( nodeVar41.x + 2u ), nodeVar41.y, nodeVar41.z ) ) );
					nodeVar42 = ( nodeVar42 - vec3<f32>( 0.5 ) );
					nodeVar42 = ( nodeVar42 * vec3<f32>( nodeVar21 ) );
					nodeVar42 = ( nodeVar42 + vec3<f32>( 0.5 ) );
					nodeVar43 = ( ( vec3<f32>( f32( x ), f32( y ), f32( z ) ) + nodeVar42 ) - nodeVar29 );

					if ( ( nodeVar32 < nodeVar30 ) ) {

						nodeVar30 = nodeVar32;
						nodeVar31 = nodeVar43;
						

					}


				}


			}


		}


		if ( ( nodeVar22 == 1 ) ) {

			nodeVar30 = mx_cell_noise_float_2( ( nodeVar31 + nodeVar20 ) );
			

		} else {

			nodeVar30 = sqrt( nodeVar30 );
			

		}

		nodeVar19 = nodeVar30;
		

	}


	if ( ( nodeVar0 == 3 ) ) {

		nodeVar19 = mx_fractal_noise_float( nodeVar18, nodeVar8, nodeVar9, nodeVar10 );
		

	}

	nodeVar44 = ( nodeVar5 + ( nodeVar19 * ( nodeVar6 - nodeVar5 ) ) );
	nodeVar45 = clamp( nodeVar44, nodeVar5, nodeVar6 );
	nodeVar46 = nodeVar44;

	if ( ( nodeVar7 == 1.0 ) ) {

		nodeVar46 = nodeVar45;
		

	}


	return nodeVar46;

}


fn t_fractal_noise_float_2d ( a0 : vec2<f32> ) -> f32 {

	


	return ( mx_fractal_noise_float_2d( a0, 3, 2.0, 0.5 ) * 1.0 );

}


fn t_fractal_noise_float ( a0 : vec3<f32> ) -> f32 {

	


	return ( mx_fractal_noise_float( a0, 3, 2.0, 0.5 ) * 1.0 );

}


fn t_fractal_noise_vec2 ( a0 : vec3<f32> ) -> vec2<f32> {

	


	return ( mx_fractal_noise_vec2( a0, 3, 2.0, 0.5 ) * vec2<f32>( 1.0 ) );

}


fn t_fractal_noise_vec3 ( a0 : vec3<f32> ) -> vec3<f32> {

	


	return ( mx_fractal_noise_vec3( a0, 3, 2.0, 0.5 ) * vec3<f32>( 1.0 ) );

}


fn t_fractal_noise_vec4 ( a0 : vec3<f32> ) -> vec4<f32> {

	


	return ( mx_fractal_noise_vec4( a0, 4, 2.5, 0.25 ) * vec4<f32>( 2.0 ) );

}


fn t_hsvtorgb ( a0 : vec3<f32> ) -> vec3<f32> {

	


	return mx_hsvtorgb( a0 );

}


fn t_rgbtohsv ( a0 : vec3<f32> ) -> vec3<f32> {

	


	return mx_rgbtohsv( a0 );

}


fn t_srgb_texture_to_lin_rec709 ( a0 : vec3<f32> ) -> vec3<f32> {

	


	return mx_srgb_texture_to_lin_rec709( a0 );

}


fn t_rotate2d ( a0 : vec2<f32>, a1 : f32 ) -> vec2<f32> {

	var nodeVar0 : f32;
	var nodeVar1 : f32;
	var nodeVar2 : f32;

	nodeVar0 = ( a1 * 0.017453292519943295 );
	nodeVar1 = cos( nodeVar0 );
	nodeVar2 = sin( nodeVar0 );

	return vec2<f32>( ( ( nodeVar1 * a0.x ) + ( nodeVar2 * a0.y ) ), ( ( nodeVar1 * a0.y ) - ( nodeVar2 * a0.x ) ) );

}


fn t_rotate3d ( a0 : vec3<f32>, a1 : f32 ) -> vec3<f32> {

	var nodeVar0 : f32;
	var nodeVar1 : f32;
	var nodeVar2 : vec3<f32>;

	nodeVar0 = ( a1 * 0.017453292519943295 );
	nodeVar1 = cos( nodeVar0 );
	nodeVar2 = normalize( vec3<f32>( 0.0, 1.0, 0.0 ) );

	return ( ( ( a0 * vec3<f32>( nodeVar1 ) ) + ( cross( a0, nodeVar2 ) * vec3<f32>( sin( nodeVar0 ) ) ) ) + ( nodeVar2 * vec3<f32>( ( dot( nodeVar2, a0 ) * ( 1.0 - nodeVar1 ) ) ) ) );

}


fn t_rotate3d_axis ( a0 : vec3<f32>, a1 : f32 ) -> vec3<f32> {

	var nodeVar0 : f32;
	var nodeVar1 : f32;
	var nodeVar2 : vec3<f32>;

	nodeVar0 = ( a1 * 0.017453292519943295 );
	nodeVar1 = cos( nodeVar0 );
	nodeVar2 = normalize( vec3<f32>( 1.0, 0.0, 0.0 ) );

	return ( ( ( a0 * vec3<f32>( nodeVar1 ) ) + ( cross( a0, nodeVar2 ) * vec3<f32>( sin( nodeVar0 ) ) ) ) + ( nodeVar2 * vec3<f32>( ( dot( nodeVar2, a0 ) * ( 1.0 - nodeVar1 ) ) ) ) );

}


fn t_aastep ( a0 : f32, a1 : f32 ) -> f32 {

	var nodeVar0 : f32;

	nodeVar0 = ( length( vec2<f32>( dpdx( a1 ), - dpdy( a1 ) ) ) * 0.7071067811865476 );

	return smoothstep( ( a0 - nodeVar0 ), ( a0 + nodeVar0 ), a1 );

}


fn t_ramplr ( a0 : vec3<f32>, a1 : vec3<f32>, a2 : vec2<f32> ) -> vec3<f32> {

	


	return mix( a0, a1, clamp( a2.x, 0.0, 1.0 ) );

}


fn t_ramptb ( a0 : vec3<f32>, a1 : vec3<f32> ) -> vec3<f32> {

	


	return mix( a0, a1, clamp( nodeVarying3.y, 0.0, 1.0 ) );

}


fn t_ramp4 ( a0 : vec3<f32>, a1 : vec3<f32>, a2 : vec3<f32>, a3 : vec3<f32>, a4 : vec2<f32> ) -> vec3<f32> {

	var nodeVar0 : f32;

	nodeVar0 = clamp( a4.x, 0.0, 1.0 );

	return mix( mix( a2, a3, nodeVar0 ), mix( a0, a1, nodeVar0 ), clamp( a4.y, 0.0, 1.0 ) );

}


fn t_splitlr ( a0 : vec3<f32>, a1 : vec3<f32>, a2 : f32, a3 : vec2<f32> ) -> vec3<f32> {

	var nodeVar0 : f32;

	nodeVar0 = ( length( vec2<f32>( dpdx( a3.x ), - dpdy( a3.x ) ) ) * 0.7071067811865476 );

	return mix( a0, a1, smoothstep( ( a2 - nodeVar0 ), ( a2 + nodeVar0 ), a3.x ) );

}


fn t_splittb ( a0 : vec3<f32>, a1 : vec3<f32>, a2 : f32 ) -> vec3<f32> {

	var nodeVar0 : f32;

	nodeVar0 = ( length( vec2<f32>( dpdx( nodeVarying3.y ), - dpdy( nodeVarying3.y ) ) ) * 0.7071067811865476 );

	return mix( a0, a1, smoothstep( ( a2 - nodeVar0 ), ( a2 + nodeVar0 ), nodeVarying3.y ) );

}


fn t_transform_uv ( a0 : f32, a1 : f32, a2 : vec2<f32> ) -> vec2<f32> {

	


	return ( ( a2 * vec2<f32>( a0 ) ) + vec2<f32>( a1 ) );

}


fn t_transform_uv_default (  ) -> vec2<f32> {

	


	return ( ( nodeVarying3 * vec2<f32>( 1.0 ) ) + vec2<f32>( 0.0 ) );

}


fn t_safepower ( a0 : f32, a1 : f32 ) -> f32 {

	


	return ( pow( abs( a0 ), a1 ) * sign( a0 ) );

}


fn t_contrast ( a0 : f32 ) -> f32 {

	


	return ( ( ( a0 - 0.5 ) * 1.0 ) + 0.5 );

}


fn t_contrast_args ( a0 : f32, a1 : f32, a2 : f32 ) -> f32 {

	


	return ( ( ( a0 - a2 ) * a1 ) + a2 );

}


fn t_smoothstep ( a0 : f32, a1 : f32, a2 : f32 ) -> f32 {

	var nodeVar0 : f32;

	nodeVar0 = clamp( ( ( a0 - a1 ) / max( abs( ( a2 - a1 ) ), 0.000001 ) ), 0.0, 1.0 );

	return mix( ( ( nodeVar0 * nodeVar0 ) * ( 3.0 - ( 2.0 * nodeVar0 ) ) ), step( a2, a0 ), step( a2, a1 ) );

}


fn t_add ( a0 : vec3<f32>, a1 : vec3<f32> ) -> vec3<f32> {

	


	return ( a0 + a1 );

}


fn t_subtract ( a0 : vec3<f32>, a1 : vec3<f32> ) -> vec3<f32> {

	


	return ( a0 - a1 );

}


fn t_multiply ( a0 : vec3<f32>, a1 : f32 ) -> vec3<f32> {

	


	return ( a0 * vec3<f32>( a1 ) );

}


fn t_divide ( a0 : vec3<f32>, a1 : f32 ) -> vec3<f32> {

	


	return ( a0 / vec3<f32>( a1 ) );

}


fn t_modulo ( a0 : vec3<f32>, a1 : f32 ) -> vec3<f32> {

	


	return ( a0 - ( vec3<f32>( a1 ) * floor( ( a0 / vec3<f32>( a1 ) ) ) ) );

}


fn t_modulo_default ( a0 : f32 ) -> f32 {

	


	return ( a0 - ( 1.0 * floor( ( a0 / 1.0 ) ) ) );

}


fn t_power ( a0 : f32, a1 : f32 ) -> f32 {

	


	return pow( a0, a1 );

}


fn t_atan2 ( a0 : f32, a1 : f32 ) -> f32 {

	


	return atan2( a0, a1 );

}


fn t_timer (  ) -> f32 {

	


	return render.nodeUniform0;

}


fn t_invert ( a0 : f32 ) -> f32 {

	


	return ( 1.0 - a0 );

}


fn t_ifgreater ( a0 : f32, a1 : f32, a2 : vec3<f32>, a3 : vec3<f32> ) -> vec3<f32> {

	


	return mix( a3, a2, f32( ( a0 > a1 ) ) );

}


fn t_ifgreatereq ( a0 : f32, a1 : f32, a2 : vec3<f32>, a3 : vec3<f32> ) -> vec3<f32> {

	


	return mix( a3, a2, f32( ( a0 >= a1 ) ) );

}


fn t_ifequal ( a0 : f32, a1 : f32, a2 : f32, a3 : f32 ) -> f32 {

	


	return mix( a3, a2, f32( ( a0 == a1 ) ) );

}


fn t_separate ( a0 : vec3<f32> ) -> f32 {

	


	return ( ( a0[ 1u ] + a0[ 2u ] ) + a0.x );

}


fn t_place2d ( a0 : vec2<f32>, a1 : f32 ) -> vec2<f32> {

	var nodeVar0 : f32;
	var nodeVar1 : f32;
	var nodeVar2 : vec2<f32>;
	var nodeVar3 : f32;
	var nodeVar4 : f32;
	var nodeVar5 : f32;
	var nodeVar6 : f32;

	nodeVar0 = ( a1 * 0.017453292519943295 );
	nodeVar1 = cos( nodeVar0 );
	nodeVar2 = ( a0 - vec2<f32>( 0.5, 0.5 ) );
	nodeVar3 = sin( nodeVar0 );
	nodeVar4 = ( a1 * 0.017453292519943295 );
	nodeVar5 = cos( nodeVar4 );
	nodeVar6 = sin( nodeVar4 );

	return mix( ( ( vec2<f32>( ( ( nodeVar1 * ( nodeVar2 / vec2<f32>( 2.0, 3.0 ) ).x ) + ( nodeVar3 * ( nodeVar2 / vec2<f32>( 2.0, 3.0 ) ).y ) ), ( ( nodeVar1 * ( nodeVar2 / vec2<f32>( 2.0, 3.0 ) ).y ) - ( nodeVar3 * ( nodeVar2 / vec2<f32>( 2.0, 3.0 ) ).x ) ) ) - vec2<f32>( 0.25, 0.0 ) ) + vec2<f32>( 0.5, 0.5 ) ), ( ( vec2<f32>( ( ( nodeVar5 * ( nodeVar2 - vec2<f32>( 0.25, 0.0 ) ).x ) + ( nodeVar6 * ( nodeVar2 - vec2<f32>( 0.25, 0.0 ) ).y ) ), ( ( nodeVar5 * ( nodeVar2 - vec2<f32>( 0.25, 0.0 ) ).y ) - ( nodeVar6 * ( nodeVar2 - vec2<f32>( 0.25, 0.0 ) ).x ) ) ) / vec2<f32>( 2.0, 3.0 ) ) + vec2<f32>( 0.5, 0.5 ) ), step( 0.5, 0.0 ) );

}


fn t_place2d_trs ( a0 : vec2<f32>, a1 : f32 ) -> vec2<f32> {

	var nodeVar0 : f32;
	var nodeVar1 : f32;
	var nodeVar2 : f32;

	nodeVar0 = ( a1 * 0.017453292519943295 );
	nodeVar1 = cos( nodeVar0 );
	nodeVar2 = sin( nodeVar0 );

	return ( ( vec2<f32>( ( ( nodeVar1 * ( ( a0 - vec2<f32>( 0.5, 0.5 ) ) - vec2<f32>( 0.25, 0.0 ) ).x ) + ( nodeVar2 * ( ( a0 - vec2<f32>( 0.5, 0.5 ) ) - vec2<f32>( 0.25, 0.0 ) ).y ) ), ( ( nodeVar1 * ( ( a0 - vec2<f32>( 0.5, 0.5 ) ) - vec2<f32>( 0.25, 0.0 ) ).y ) - ( nodeVar2 * ( ( a0 - vec2<f32>( 0.5, 0.5 ) ) - vec2<f32>( 0.25, 0.0 ) ).x ) ) ) / vec2<f32>( 2.0, 3.0 ) ) + vec2<f32>( 0.5, 0.5 ) );

}


fn t_heighttonormal ( a0 : f32, a1 : vec2<f32> ) -> vec3<f32> {

	var nodeVar0 : vec2<f32>;
	var nodeVar1 : vec2<f32>;
	var nodeVar2 : vec2<f32>;
	var nodeVar3 : vec3<f32>;
	var nodeVar4 : vec3<f32>;

	nodeVar0 = vec2<f32>( dpdx( a1.x ), - dpdy( a1.x ) );
	nodeVar1 = vec2<f32>( dpdx( a1.y ), - dpdy( a1.y ) );
	nodeVar2 = ( ( vec2<f32>( dpdx( a0 ), - dpdy( a0 ) ) * vec2<f32>( 2.0 ) ) * vec2<f32>( 0.0625 ) );
	nodeVar3 = cross( vec3<f32>( nodeVar0.x, nodeVar1.x, nodeVar2.x ), vec3<f32>( nodeVar0.y, nodeVar1.y, nodeVar2.y ) );
	nodeVar4 = mix( nodeVar3, vec3<f32>( 0.0, 0.0, 1.0 ), f32( ( dot( nodeVar3, nodeVar3 ) < 1e-12 ) ) );

	return ( ( normalize( mix( nodeVar4, ( nodeVar4 * vec3<f32>( -1.0 ) ), f32( ( nodeVar4.z < 0.0 ) ) ) ) * vec3<f32>( 0.5 ) ) + vec3<f32>( 0.5 ) );

}




@fragment
fn main( @location( 0 ) positionLocal : vec3<f32>,
	@location( 1 ) nodeVarying3 : vec2<f32> ) -> OutputStruct {

	// flow
	// code

	DiffuseColor = vec4<f32>( vec3<f32>( ( ( ( ( ( ( ( ( ( ( ( ( ( ( ( ( ( ( ( ( ( ( ( ( ( ( ( ( ( ( ( ( ( ( ( ( ( ( ( ( ( ( ( ( ( ( ( ( ( ( ( ( ( ( ( ( ( ( ( ( ( ( ( 0.0 + t_noise_float_2( nodeVarying3 ) ) + t_noise_float_3( positionLocal ) ) + t_noise_vec3_2( nodeVarying3 ).x ) + t_noise_vec3_3( positionLocal ).x ) + t_noise_vec4_2( nodeVarying3 ).x ) + t_noise_vec4_3( positionLocal ).x ) + t_cell_noise_float_2( nodeVarying3 ) ) + t_cell_noise_float_3( positionLocal ) ) + t_cell_noise_vec3_2( nodeVarying3 ).x ) + t_cell_noise_vec3_3( positionLocal ).x ) + t_worley_noise_float_2( nodeVarying3 ) ) + t_worley_noise_float_3( positionLocal ) ) + t_worley_noise_float_2d( nodeVarying3 ) ) + t_worley_noise_float_3d( positionLocal ) ) + t_worley_noise_vec2_2( nodeVarying3 ).x ) + t_worley_noise_vec2_3( positionLocal ).x ) + t_worley_noise_vec3_2( nodeVarying3 ).x ) + t_worley_noise_vec3_3( positionLocal ).x ) + t_worley_noise_vec3_style_2( nodeVarying3 ).x ) + t_worley_noise_vec3_style_3( positionLocal ).x ) + t_unifiednoise2d( nodeVarying3 ) ) + t_unifiednoise3d( positionLocal ) ) + t_fractal_noise_float_2d( nodeVarying3 ) ) + t_fractal_noise_float( positionLocal ) ) + t_fractal_noise_vec2( positionLocal ).x ) + t_fractal_noise_vec3( positionLocal ).x ) + t_fractal_noise_vec4( positionLocal ).x ) + t_hsvtorgb( positionLocal ).x ) + t_rgbtohsv( positionLocal ).x ) + t_srgb_texture_to_lin_rec709( positionLocal ).x ) + t_rotate2d( nodeVarying3, positionLocal.x ).x ) + t_rotate3d( positionLocal, positionLocal.x ).x ) + t_rotate3d_axis( positionLocal, positionLocal.x ).x ) + t_aastep( positionLocal.x, positionLocal.x ) ) + t_ramplr( positionLocal, positionLocal, nodeVarying3 ).x ) + t_ramptb( positionLocal, positionLocal ).x ) + t_ramp4( positionLocal, positionLocal, positionLocal, positionLocal, nodeVarying3 ).x ) + t_splitlr( positionLocal, positionLocal, positionLocal.x, nodeVarying3 ).x ) + t_splittb( positionLocal, positionLocal, positionLocal.x ).x ) + t_transform_uv( positionLocal.x, positionLocal.x, nodeVarying3 ).x ) + t_transform_uv_default(  ).x ) + t_safepower( positionLocal.x, positionLocal.x ) ) + t_contrast( positionLocal.x ) ) + t_contrast_args( positionLocal.x, positionLocal.x, positionLocal.x ) ) + t_smoothstep( positionLocal.x, positionLocal.x, positionLocal.x ) ) + t_add( positionLocal, positionLocal ).x ) + t_subtract( positionLocal, positionLocal ).x ) + t_multiply( positionLocal, positionLocal.x ).x ) + t_divide( positionLocal, positionLocal.x ).x ) + t_modulo( positionLocal, positionLocal.x ).x ) + t_modulo_default( positionLocal.x ) ) + t_power( positionLocal.x, positionLocal.x ) ) + t_atan2( positionLocal.x, positionLocal.x ) ) + t_timer(  ) ) + t_invert( positionLocal.x ) ) + t_ifgreater( positionLocal.x, positionLocal.x, positionLocal, positionLocal ).x ) + t_ifgreatereq( positionLocal.x, positionLocal.x, positionLocal, positionLocal ).x ) + t_ifequal( positionLocal.x, positionLocal.x, positionLocal.x, positionLocal.x ) ) + t_separate( positionLocal ) ) + t_place2d( nodeVarying3, positionLocal.x ).x ) + t_place2d_trs( nodeVarying3, positionLocal.x ).x ) + t_heighttonormal( positionLocal.x, nodeVarying3 ).x ) * 0.001 ) ), 1.0 );
	DiffuseColor.w = ( DiffuseColor.w * object.nodeUniform1 );
	DiffuseColor.w = 1.0;
	nodeVar0 = max( vec4<f32>( DiffuseColor.xyz, DiffuseColor.w ), vec4<f32>( 0.0 ) );
	Output = nodeVar0;

	// result

	output.color = nodeVar0;

	return output;

}
