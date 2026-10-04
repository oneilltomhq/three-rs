// Three.js r187dev - Node System

// global
diagnostic( off, derivative_uniformity );


// directives


// structs

struct StructType0 {
	reflectDir : vec3<f32>,
	sampleWeight : vec3<f32>,
	pdf : f32,
	NdotV : f32,
	alpha : f32,
	f0 : vec3<f32>
};

struct OutputStruct {
	@location( 0 ) color: vec4<f32>
};
var<private> output : OutputStruct;

// uniforms
@binding( 0 ) @group( 0 ) var nodeUniform0 : texture_depth_2d;
@binding( 2 ) @group( 0 ) var nodeUniform3_sampler : sampler;
@binding( 3 ) @group( 0 ) var nodeUniform3 : texture_2d<f32>;
@binding( 4 ) @group( 0 ) var nodeUniform7_sampler : sampler;
@binding( 5 ) @group( 0 ) var nodeUniform7 : texture_2d<f32>;
@binding( 6 ) @group( 0 ) var nodeUniform14_sampler : sampler;
@binding( 7 ) @group( 0 ) var nodeUniform14 : texture_2d<f32>;
@binding( 8 ) @group( 0 ) var nodeUniform16 : texture_2d<f32>;

struct objectStruct {
	nodeUniform1 : mat4x4<f32>,
	nodeUniform2 : mat4x4<f32>,
	nodeUniform4 : vec2<f32>,
	nodeUniform5 : f32,
	nodeUniform6 : f32,
	nodeUniform8 : f32,
	nodeUniform9 : f32,
	nodeUniform10 : mat4x4<f32>,
	nodeUniform11 : f32,
	nodeUniform12 : f32,
	nodeUniform13 : f32,
	nodeUniform15 : f32,
	nodeUniform17 : f32,
	nodeUniform18 : f32,
	nodeUniform19 : f32,
	nodeUniform20 : f32
};
@binding( 1 ) @group( 0 )
var<uniform> object : objectStruct;

// vars
var<private> nodeVar0 : vec2<f32>;
var<private> nodeVar1 : f32;
var<private> nodeVar2 : vec2<u32>;
var<private> nodeVar3 : f32;
var<private> nodeVar4 : vec3<f32>;
var<private> nodeVar5 : vec3<f32>;
var<private> nodeVar6 : vec4<f32>;
var<private> nodeVar7 : vec3<f32>;
var<private> nodeVar8 : vec3<f32>;
var<private> nodeVar9 : vec3<f32>;
var<private> nodeVar10 : vec3<f32>;
var<private> nodeVar11 : vec4<f32>;
var<private> nodeVar12 : vec4<f32>;
var<private> nodeVar13 : vec4<f32>;
var<private> nodeVar14 : vec4<f32>;
var<private> nodeVar15 : f32;
var<private> nodeVar16 : f32;
var<private> nodeVar17 : f32;
var<private> nodeVar18 : vec3<f32>;
var<private> nodeVar19 : vec3<f32>;
var<private> nodeVar20 : vec3<f32>;
var<private> nodeVar21 : vec3<f32>;
var<private> nodeVar22 : vec3<f32>;
var<private> nodeVar23 : vec3<f32>;
var<private> nodeVar24 : vec3<f32>;
var<private> nodeVar25 : vec3<f32>;
var<private> nodeVar26 : vec3<f32>;
var<private> nodeVar27 : f32;
var<private> nodeVar28 : f32;
var<private> nodeVar29 : f32;
var<private> nodeVar30 : f32;
var<private> nodeVar31 : vec4<f32>;
var<private> nodeVar32 : vec3<f32>;
var<private> nodeVar33 : f32;
var<private> nodeVar34 : f32;
var<private> nodeVar35 : f32;
var<private> nodeVar36 : vec3<f32>;
var<private> nodeVar37 : vec3<f32>;
var<private> nodeVar38 : f32;
var<private> nodeVar39 : f32;
var<private> nodeVar40 : f32;
var<private> nodeVar41 : f32;
var<private> nodeVar42 : f32;
var<private> nodeVar43 : f32;
var<private> nodeVar44 : f32;
var<private> nodeVar45 : f32;
var<private> nodeVar46 : f32;
var<private> nodeVar47 : f32;
var<private> nodeVar48 : f32;
var<private> nodeVar49 : f32;
var<private> nodeVar50 : f32;
var<private> nodeVar51 : f32;
var<private> nodeVar52 : f32;
var<private> nodeVar53 : f32;
var<private> nodeVar54 : f32;
var<private> nodeVar55 : f32;
var<private> nodeVar56 : f32;
var<private> nodeVar57 : f32;
var<private> nodeVar58 : f32;
var<private> nodeVar59 : f32;
var<private> nodeVar60 : f32;
var<private> nodeVar61 : f32;
var<private> nodeVar62 : f32;
var<private> nodeVar63 : f32;
var<private> nodeVar64 : vec3<f32>;
var<private> nodeVar65 : StructType0;
var<private> nodeVar66 : StructType0;
var<private> nodeVar67 : f32;
var<private> nodeVar68 : f32;
var<private> nodeVar69 : f32;
var<private> nodeVar70 : vec3<f32>;
var<private> nodeVar71 : vec3<f32>;
var<private> nodeVar72 : vec3<f32>;
var<private> nodeVar73 : vec3<f32>;
var<private> nodeVar74 : vec3<f32>;
var<private> nodeVar75 : vec3<f32>;
var<private> nodeVar76 : vec3<f32>;
var<private> nodeVar77 : vec3<f32>;
var<private> nodeVar78 : vec3<f32>;
var<private> nodeVar79 : f32;
var<private> nodeVar80 : f32;
var<private> nodeVar81 : f32;
var<private> nodeVar82 : f32;
var<private> nodeVar83 : vec3<f32>;
var<private> nodeVar84 : f32;
var<private> nodeVar85 : f32;
var<private> nodeVar86 : f32;
var<private> nodeVar87 : vec3<f32>;
var<private> nodeVar88 : vec3<f32>;
var<private> nodeVar89 : f32;
var<private> nodeVar90 : f32;
var<private> nodeVar91 : f32;
var<private> nodeVar92 : f32;
var<private> nodeVar93 : f32;
var<private> nodeVar94 : f32;
var<private> nodeVar95 : f32;
var<private> nodeVar96 : f32;
var<private> nodeVar97 : f32;
var<private> nodeVar98 : f32;
var<private> nodeVar99 : f32;
var<private> nodeVar100 : f32;
var<private> nodeVar101 : f32;
var<private> nodeVar102 : f32;
var<private> nodeVar103 : f32;
var<private> nodeVar104 : f32;
var<private> nodeVar105 : f32;
var<private> nodeVar106 : f32;
var<private> nodeVar107 : f32;
var<private> nodeVar108 : f32;
var<private> nodeVar109 : f32;
var<private> nodeVar110 : f32;
var<private> nodeVar111 : f32;
var<private> nodeVar112 : f32;
var<private> nodeVar113 : f32;
var<private> nodeVar114 : f32;
var<private> nodeVar115 : vec3<f32>;
var<private> nodeVar116 : StructType0;
var<private> nodeVar117 : vec3<f32>;
var<private> nodeVar118 : vec3<f32>;
var<private> nodeVar119 : f32;
var<private> nodeVar120 : f32;
var<private> nodeVar121 : vec3<f32>;
var<private> nodeVar122 : vec2<f32>;
var<private> nodeVar123 : vec2<f32>;
var<private> nodeVar124 : vec2<f32>;
var<private> nodeVar125 : f32;
var<private> nodeVar126 : f32;
var<private> nodeVar127 : f32;
var<private> nodeVar128 : f32;
var<private> nodeVar129 : f32;
var<private> nodeVar130 : vec2<f32>;
var<private> nodeVar131 : vec2<f32>;
var<private> nodeVar132 : vec2<f32>;
var<private> nodeVar133 : vec4<f32>;
var<private> nodeVar134 : f32;
var<private> nodeVar135 : bool;
var<private> nodeVar136 : vec2<f32>;
var<private> nodeVar137 : f32;
var<private> nodeVar138 : f32;
var<private> nodeVar139 : vec2<f32>;
var<private> nodeVar140 : vec2<f32>;
var<private> nodeVar141 : f32;
var<private> nodeVar142 : f32;
var<private> nodeVar143 : f32;
var<private> nodeVar144 : f32;
var<private> nodeVar145 : vec3<f32>;
var<private> nodeVar146 : f32;
var<private> nodeVar147 : vec2<f32>;
var<private> nodeVar148 : vec3<f32>;
var<private> nodeVar149 : f32;
var<private> nodeVar150 : f32;
var<private> nodeVar151 : vec4<f32>;
var<private> nodeVar152 : vec3<f32>;
var<private> nodeVar153 : vec3<f32>;
var<private> nodeVar154 : vec3<f32>;
var<private> nodeVar155 : f32;
var<private> nodeVar156 : vec4<f32>;
var<private> nodeVar157 : vec4<f32>;
var<private> nodeVar158 : vec3<f32>;
var<private> nodeVar159 : vec3<f32>;
var<private> nodeVar160 : vec3<f32>;
var<private> nodeVar161 : vec3<f32>;
var<private> nodeVar162 : f32;
var<private> nodeVar163 : vec3<f32>;
var<private> nodeVar164 : vec3<f32>;
var<private> nodeVar165 : f32;
var<private> nodeVar166 : f32;
var<private> nodeVar167 : vec4<f32>;
var<private> nodeVar168 : vec2<u32>;
var<private> nodeVar169 : f32;
var<private> nodeVar170 : f32;
var<private> nodeVar171 : f32;
var<private> nodeVar172 : vec3<f32>;
var<private> nodeVar173 : f32;
var<private> nodeVar174 : f32;
var<private> nodeVar175 : f32;
var<private> nodeVar176 : f32;
var<private> nodeVar177 : f32;
var<private> nodeVar178 : f32;
var<private> nodeVar179 : f32;
var<private> nodeVar180 : f32;
var<private> nodeVar181 : f32;
var<private> nodeVar182 : vec3<f32>;
var<private> nodeVar183 : vec3<f32>;
var<private> nodeVar184 : vec3<f32>;
var<private> nodeVar185 : vec3<f32>;
var<private> nodeVar186 : f32;
var<private> nodeVar187 : vec3<f32>;
var<private> nodeVar188 : vec3<f32>;
var<private> nodeVar189 : f32;
var<private> nodeVar190 : f32;
var<private> nodeVar191 : vec4<f32>;
var<private> nodeVar192 : vec2<u32>;
var<private> nodeVar193 : f32;
var<private> nodeVar194 : f32;
var<private> nodeVar195 : f32;
var<private> nodeVar196 : vec3<f32>;
var<private> nodeVar197 : f32;
var<private> nodeVar198 : f32;
var<private> nodeVar199 : f32;
var<private> nodeVar200 : f32;
var<private> nodeVar201 : f32;
var<private> nodeVar202 : f32;
var<private> nodeVar203 : f32;
var<private> nodeVar204 : f32;
var<private> nodeVar205 : f32;
var<private> nodeVar206 : f32;
var<private> nodeVar207 : vec3<f32>;
var<private> nodeVar208 : vec3<f32>;

// codes
fn tsl_clampWrapping_float( coord: f32 ) -> f32 { return clamp( coord, 0.0, 1.0 ); }
fn tsl_coord_clampS_clampT_2d( coord : vec2f ) -> vec2f {

	return vec2f(
		tsl_clampWrapping_float( coord.x ),
		tsl_clampWrapping_float( coord.y )
	);

}

fn tsl_mod_vec2( x : vec2f, y : vec2f ) -> vec2f { return x - y * floor( x / y ); }
fn SampleGGXVNDF ( V : vec3<f32>, ax : f32, ay : f32, r1 : f32, r2 : f32 ) -> vec3<f32> {

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
	var nodeVar10 : vec3<f32>;
	var nodeVar11 : vec3<f32>;
	var nodeVar12 : vec3<f32>;

	nodeVar0 = normalize( vec3<f32>( ( ax * V.x ), ( ay * V.y ), V.z ) );
	nodeVar1 = min( ax, ay );
	nodeVar2 = ( 1.0 + length( V.xy ) );
	nodeVar3 = ( nodeVar1 * nodeVar1 );
	nodeVar4 = ( nodeVar2 * nodeVar2 );
	nodeVar5 = ( ( ( 1.0 - nodeVar3 ) * nodeVar4 ) / ( nodeVar4 + ( ( nodeVar3 * V.z ) * V.z ) ) );
	nodeVar6 = ( nodeVar0.z * nodeVar5 );
	nodeVar7 = ( 6.283185307179586 * r1 );
	nodeVar8 = ( ( ( 1.0 - r2 ) * ( 1.0 + nodeVar6 ) ) - nodeVar6 );
	nodeVar9 = sqrt( max( 0.0, ( 1.0 - ( nodeVar8 * nodeVar8 ) ) ) );
	nodeVar10 = vec3<f32>( ( nodeVar9 * cos( nodeVar7 ) ), ( nodeVar9 * sin( nodeVar7 ) ), nodeVar8 );
	nodeVar11 = ( nodeVar10 + nodeVar0 );
	nodeVar12 = normalize( vec3<f32>( ( ax * nodeVar11.x ), ( ay * nodeVar11.y ), max( 0.0, nodeVar11.z ) ) );

	return nodeVar12;

}


fn getSpecularDominantFactor ( NoV : f32, roughness : f32 ) -> f32 {

	

	let nodeConst0 = ( 0.298475 * log( ( 39.4115 - ( 39.0029 * roughness ) ) ) );

	return clamp( ( ( pow( ( 1.0 - NoV ), 10.8649 ) * ( 1.0 - nodeConst0 ) ) + nodeConst0 ), 0.0, 1.0 );

}


fn computeScreenBorderFactor ( uvCoord : vec2<f32>, borderWidth : f32 ) -> f32 {

	


	return pow( smoothstep( 0.0, 1.0, smoothstep( 0.0, max( borderWidth, 0.0001 ), min( min( uvCoord.x, ( 1.0 - uvCoord.x ) ), min( uvCoord.y, ( 1.0 - uvCoord.y ) ) ) ) ), 0.125 );

}


fn tsl_repeatWrapping_float( coord: f32 ) -> f32 { return fract( coord ); }
fn tsl_coord_repeatS_clampT_2d( coord : vec2f ) -> vec2f {

	return vec2f(
		tsl_repeatWrapping_float( coord.x ),
		tsl_clampWrapping_float( coord.y )
	);

}



@fragment
fn main( @location( 0 ) nodeVarying0 : vec2<f32> ) -> OutputStruct {

	// flow
	// code

	nodeVar0 = nodeVarying0;
	nodeVar2 = textureDimensions( nodeUniform0, u32( 0 ) );
	nodeVar1 = textureLoad( nodeUniform0, vec2<u32>( clamp( floor( tsl_coord_clampS_clampT_2d( nodeVar0 ) * vec2<f32>( nodeVar2 ) ), vec2<f32>( 0 ), vec2<f32>( nodeVar2 - vec2<u32>( 1, 1 ) ) ) ), u32( 0 ) );
	nodeVar3 = nodeVar1;

	if ( ( nodeVar3 >= 1.0 ) ) {

		discard;
		

	}

	let nodeConst0 = ( object.nodeUniform1 * vec4<f32>( vec3<f32>( ( ( vec2<f32>( nodeVar0.x, ( 1.0 - nodeVar0.y ) ) * vec2<f32>( 2.0 ) ) - vec2<f32>( 1.0 ) ), nodeVar3 ), 1.0 ) );
	nodeVar4 = ( nodeConst0.xyz / vec3<f32>( nodeConst0.w ) );
	nodeVar5 = ( object.nodeUniform2 * vec4<f32>( nodeVar4, 1.0 ) ).xyz;
	nodeVar6 = textureSample( nodeUniform3, nodeUniform3_sampler, nodeVarying0 );
	nodeVar7 = normalize( ( ( nodeVar6.xyz * vec3<f32>( 2.0 ) ) - vec3<f32>( 1.0 ) ) );
	nodeVar8 = normalize( nodeVar4 );
	nodeVar9 = normalize( ( - nodeVar8 ) );
	nodeVar10 = vec3<f32>( 1.0, 1.0, 1.0 );
	nodeVar11 = vec4<f32>( 0.0, 0.0, 0.0, 1.0 );
	let nodeConst1 = ( i32( object.nodeUniform5 ) + 47 );
	let nodeConst2 = tsl_mod_vec2( ( floor( ( nodeVarying0 * object.nodeUniform4 ) ) + floor( ( fract( vec2<f32>( ( f32( nodeConst1 ) * 0.7548776662 ), ( f32( nodeConst1 ) * 0.569840291 ) ) ) * vec2<f32>( 32.0 ) ) ) ), vec2<f32>( 32.0 ) );
	let nodeConst3 = ( ( ( nodeConst2.x * 0.7548776662466927 ) + ( nodeConst2.y * 0.5698402909980532 ) ) + 47.0 );
	nodeVar11 = vec4<f32>( fract( ( ( nodeConst3 * 1.324717957244746 ) * 0.7548776662466927 ) ), fract( ( ( nodeConst3 * 2.649435914489492 ) * 0.5698402909980532 ) ), fract( ( ( nodeConst3 * 3.974153871734238 ) * 0.419875421 ) ), fract( ( ( nodeConst3 * 5.298871828978984 ) * 0.43015970900194667 ) ) );
	nodeVar12 = nodeVar11;
	nodeVar12.y = mix( nodeVar12.y, 0.0, ( object.nodeUniform6 * sqrt( nodeVar12.w ) ) );
	nodeVar13 = textureSample( nodeUniform7, nodeUniform7_sampler, nodeVar0 );
	nodeVar10 = nodeVar13.xyz;
	nodeVar14 = textureSample( nodeUniform3, nodeUniform3_sampler, nodeVarying0 );
	let nodeVar14 = nodeVar14;
	let nodeConst4 = nodeVar14.w;
	nodeVar15 = max( ( nodeConst4 * nodeConst4 ), 0.001 );
	nodeVar16 = nodeVar15;
	nodeVar17 = nodeVar15;
	nodeVar18 = cross( vec3<f32>( 0.0, 0.0, 1.0 ), nodeVar7 );
	nodeVar19 = normalize( nodeVar18 );

	if ( ( length( nodeVar19 ) < 0.001 ) ) {

		nodeVar19 = normalize( cross( vec3<f32>( 0.0, 1.0, 0.0 ), nodeVar7 ) );
		

	}

	nodeVar20 = normalize( cross( nodeVar7, nodeVar19 ) );
	nodeVar21 = vec3<f32>( dot( nodeVar19, nodeVar9 ), dot( nodeVar20, nodeVar9 ), dot( nodeVar7, nodeVar9 ) );
	nodeVar22 = SampleGGXVNDF( nodeVar21, nodeVar16, nodeVar17, nodeVar12.x, nodeVar12.y );

	if ( ( nodeVar22.z < 0.0 ) ) {

		nodeVar22 = ( - nodeVar22 );
		

	}

	nodeVar23 = normalize( ( ( ( nodeVar19 * vec3<f32>( nodeVar22.x ) ) + ( nodeVar20 * vec3<f32>( nodeVar22.y ) ) ) + ( nodeVar7 * vec3<f32>( nodeVar22.z ) ) ) );
	nodeVar24 = normalize( reflect( ( - nodeVar9 ), nodeVar23 ) );
	nodeVar25 = nodeVar24;
	nodeVar26 = normalize( ( nodeVar9 + nodeVar25 ) );
	nodeVar27 = max( 0.0, dot( nodeVar7, nodeVar9 ) );
	nodeVar28 = max( 0.0, dot( nodeVar7, nodeVar25 ) );
	nodeVar29 = max( 0.0, dot( nodeVar7, nodeVar26 ) );
	nodeVar30 = max( 0.0, dot( nodeVar9, nodeVar26 ) );
	nodeVar31 = textureSample( nodeUniform7, nodeUniform7_sampler, nodeVarying0 );
	let nodeVar31 = nodeVar31;
	let nodeConst5 = nodeVar31.w;
	nodeVar32 = mix( vec3<f32>( 0.04, 0.04, 0.04 ), nodeVar10, nodeConst5 );
	nodeVar33 = ( 1.0 - nodeVar30 );
	nodeVar34 = ( nodeVar33 * nodeVar33 );
	nodeVar35 = ( ( nodeVar34 * nodeVar34 ) * nodeVar33 );
	nodeVar36 = ( nodeVar32 + ( ( vec3<f32>( 1.0, 1.0, 1.0 ) - nodeVar32 ) * vec3<f32>( nodeVar35 ) ) );
	nodeVar37 = nodeVar36;
	nodeVar38 = ( nodeVar16 * nodeVar16 );
	nodeVar39 = ( nodeVar29 * nodeVar29 );
	nodeVar40 = ( ( nodeVar39 * ( nodeVar38 - 1.0 ) ) + 1.0 );
	nodeVar41 = ( nodeVar38 / ( 3.141592653589793 * pow( nodeVar40, 2.0 ) ) );
	nodeVar42 = nodeVar41;
	nodeVar43 = ( nodeVar16 * nodeVar16 );
	nodeVar44 = max( 0.0, ( 1.0 - ( nodeVar27 * nodeVar27 ) ) );
	nodeVar45 = ( 1.0 + sqrt( nodeVar44 ) );
	nodeVar46 = ( nodeVar45 * nodeVar45 );
	nodeVar47 = ( ( ( 1.0 - nodeVar43 ) * nodeVar46 ) / ( nodeVar46 + ( ( nodeVar43 * nodeVar27 ) * nodeVar27 ) ) );
	nodeVar48 = sqrt( ( ( nodeVar43 * nodeVar44 ) + ( nodeVar27 * nodeVar27 ) ) );
	nodeVar49 = ( nodeVar42 / max( 0.000001, ( 2.0 * ( ( nodeVar47 * nodeVar27 ) + nodeVar48 ) ) ) );
	nodeVar50 = nodeVar49;
	nodeVar51 = ( nodeVar16 * nodeVar16 );
	nodeVar52 = max( ( 1.0 - ( nodeVar27 * nodeVar27 ) ), 0.0 );
	nodeVar53 = ( 1.0 + sqrt( nodeVar52 ) );
	nodeVar54 = ( nodeVar53 * nodeVar53 );
	nodeVar55 = ( ( ( 1.0 - nodeVar51 ) * nodeVar54 ) / ( nodeVar54 + ( ( nodeVar51 * nodeVar27 ) * nodeVar27 ) ) );
	nodeVar56 = sqrt( ( ( nodeVar51 * nodeVar52 ) + ( nodeVar27 * nodeVar27 ) ) );
	nodeVar57 = ( nodeVar16 * nodeVar16 );
	nodeVar58 = ( nodeVar27 * nodeVar27 );
	nodeVar59 = ( ( 2.0 * nodeVar27 ) / ( nodeVar27 + sqrt( ( nodeVar57 + ( ( 1.0 - nodeVar57 ) * nodeVar58 ) ) ) ) );
	nodeVar60 = ( nodeVar16 * nodeVar16 );
	nodeVar61 = ( nodeVar28 * nodeVar28 );
	nodeVar62 = ( ( 2.0 * nodeVar28 ) / ( nodeVar28 + sqrt( ( nodeVar60 + ( ( 1.0 - nodeVar60 ) * nodeVar61 ) ) ) ) );
	nodeVar63 = ( nodeVar59 * nodeVar62 );
	nodeVar64 = ( ( ( nodeVar37 * vec3<f32>( nodeVar63 ) ) * vec3<f32>( ( ( nodeVar55 * nodeVar27 ) + nodeVar56 ) ) ) / vec3<f32>( max( ( 2.0 * nodeVar27 ), 0.0001 ) ) );
	nodeVar65 = StructType0( nodeVar24, nodeVar64, nodeVar50, nodeVar27, nodeVar16, nodeVar32 );
	nodeVar66 = nodeVar65;

	if ( ( dot( nodeVar66.reflectDir, nodeVar7 ) < 0.0 ) ) {

		nodeVar67 = max( ( nodeConst4 * nodeConst4 ), 0.001 );
		nodeVar68 = nodeVar67;
		nodeVar69 = nodeVar67;
		nodeVar70 = cross( vec3<f32>( 0.0, 0.0, 1.0 ), nodeVar7 );
		nodeVar71 = normalize( nodeVar70 );

		if ( ( length( nodeVar71 ) < 0.001 ) ) {

			nodeVar71 = normalize( cross( vec3<f32>( 0.0, 1.0, 0.0 ), nodeVar7 ) );
			

		}

		nodeVar72 = normalize( cross( nodeVar7, nodeVar71 ) );
		nodeVar73 = vec3<f32>( dot( nodeVar71, nodeVar9 ), dot( nodeVar72, nodeVar9 ), dot( nodeVar7, nodeVar9 ) );
		let nodeConst6 = fract( ( nodeVar12 + ( nodeVar12 * vec4<f32>( 7.0 ) ) ) );
		nodeVar74 = SampleGGXVNDF( nodeVar73, nodeVar68, nodeVar69, nodeConst6.x, nodeConst6.y );

		if ( ( nodeVar74.z < 0.0 ) ) {

			nodeVar74 = ( - nodeVar74 );
			

		}

		nodeVar75 = normalize( ( ( ( nodeVar71 * vec3<f32>( nodeVar74.x ) ) + ( nodeVar72 * vec3<f32>( nodeVar74.y ) ) ) + ( nodeVar7 * vec3<f32>( nodeVar74.z ) ) ) );
		nodeVar76 = normalize( reflect( ( - nodeVar9 ), nodeVar75 ) );
		nodeVar77 = nodeVar76;
		nodeVar78 = normalize( ( nodeVar9 + nodeVar77 ) );
		nodeVar79 = max( 0.0, dot( nodeVar7, nodeVar9 ) );
		nodeVar80 = max( 0.0, dot( nodeVar7, nodeVar77 ) );
		nodeVar81 = max( 0.0, dot( nodeVar7, nodeVar78 ) );
		nodeVar82 = max( 0.0, dot( nodeVar9, nodeVar78 ) );
		nodeVar83 = mix( vec3<f32>( 0.04, 0.04, 0.04 ), nodeVar10, nodeConst5 );
		nodeVar84 = ( 1.0 - nodeVar82 );
		nodeVar85 = ( nodeVar84 * nodeVar84 );
		nodeVar86 = ( ( nodeVar85 * nodeVar85 ) * nodeVar84 );
		nodeVar87 = ( nodeVar83 + ( ( vec3<f32>( 1.0, 1.0, 1.0 ) - nodeVar83 ) * vec3<f32>( nodeVar86 ) ) );
		nodeVar88 = nodeVar87;
		nodeVar89 = ( nodeVar68 * nodeVar68 );
		nodeVar90 = ( nodeVar81 * nodeVar81 );
		nodeVar91 = ( ( nodeVar90 * ( nodeVar89 - 1.0 ) ) + 1.0 );
		nodeVar92 = ( nodeVar89 / ( 3.141592653589793 * pow( nodeVar91, 2.0 ) ) );
		nodeVar93 = nodeVar92;
		nodeVar94 = ( nodeVar68 * nodeVar68 );
		nodeVar95 = max( 0.0, ( 1.0 - ( nodeVar79 * nodeVar79 ) ) );
		nodeVar96 = ( 1.0 + sqrt( nodeVar95 ) );
		nodeVar97 = ( nodeVar96 * nodeVar96 );
		nodeVar98 = ( ( ( 1.0 - nodeVar94 ) * nodeVar97 ) / ( nodeVar97 + ( ( nodeVar94 * nodeVar79 ) * nodeVar79 ) ) );
		nodeVar99 = sqrt( ( ( nodeVar94 * nodeVar95 ) + ( nodeVar79 * nodeVar79 ) ) );
		nodeVar100 = ( nodeVar93 / max( 0.000001, ( 2.0 * ( ( nodeVar98 * nodeVar79 ) + nodeVar99 ) ) ) );
		nodeVar101 = nodeVar100;
		nodeVar102 = ( nodeVar68 * nodeVar68 );
		nodeVar103 = max( ( 1.0 - ( nodeVar79 * nodeVar79 ) ), 0.0 );
		nodeVar104 = ( 1.0 + sqrt( nodeVar103 ) );
		nodeVar105 = ( nodeVar104 * nodeVar104 );
		nodeVar106 = ( ( ( 1.0 - nodeVar102 ) * nodeVar105 ) / ( nodeVar105 + ( ( nodeVar102 * nodeVar79 ) * nodeVar79 ) ) );
		nodeVar107 = sqrt( ( ( nodeVar102 * nodeVar103 ) + ( nodeVar79 * nodeVar79 ) ) );
		nodeVar108 = ( nodeVar68 * nodeVar68 );
		nodeVar109 = ( nodeVar79 * nodeVar79 );
		nodeVar110 = ( ( 2.0 * nodeVar79 ) / ( nodeVar79 + sqrt( ( nodeVar108 + ( ( 1.0 - nodeVar108 ) * nodeVar109 ) ) ) ) );
		nodeVar111 = ( nodeVar68 * nodeVar68 );
		nodeVar112 = ( nodeVar80 * nodeVar80 );
		nodeVar113 = ( ( 2.0 * nodeVar80 ) / ( nodeVar80 + sqrt( ( nodeVar111 + ( ( 1.0 - nodeVar111 ) * nodeVar112 ) ) ) ) );
		nodeVar114 = ( nodeVar110 * nodeVar113 );
		nodeVar115 = ( ( ( nodeVar88 * vec3<f32>( nodeVar114 ) ) * vec3<f32>( ( ( nodeVar106 * nodeVar79 ) + nodeVar107 ) ) ) / vec3<f32>( max( ( 2.0 * nodeVar79 ), 0.0001 ) ) );
		nodeVar116 = StructType0( nodeVar76, nodeVar115, nodeVar101, nodeVar79, nodeVar68, nodeVar83 );
		nodeVar66 = nodeVar116;
		

	}

	nodeVar117 = nodeVar66.reflectDir;
	nodeVar118 = nodeVar66.sampleWeight;
	nodeVar119 = getSpecularDominantFactor( nodeVar66.NdotV, nodeConst4 );
	nodeVar120 = ( object.nodeUniform8 / dot( ( - nodeVar8 ), nodeVar7 ) );
	nodeVar121 = ( nodeVar4 + ( nodeVar117 * vec3<f32>( nodeVar120 ) ) );

	if ( ( nodeVar121.z > ( - object.nodeUniform9 ) ) ) {

		nodeVar121 = ( nodeVar4 + ( nodeVar117 * vec3<f32>( ( ( ( - object.nodeUniform9 ) - nodeVar4.z ) / nodeVar117.z ) ) ) );
		

	}

	nodeVar122 = ( nodeVar0 * object.nodeUniform4 );
	let nodeConst7 = ( object.nodeUniform10 * vec4<f32>( nodeVar121, 1.0 ) );
	nodeVar123 = ( ( ( nodeConst7.xy / vec2<f32>( nodeConst7.w ) ) * vec2<f32>( 0.5 ) ) + vec2<f32>( 0.5 ) );
	nodeVar124 = ( vec2<f32>( nodeVar123.x, ( 1.0 - nodeVar123.y ) ) * object.nodeUniform4 );
	nodeVar125 = ( nodeVar124.x - nodeVar122.x );
	nodeVar126 = ( nodeVar124.y - nodeVar122.y );
	nodeVar127 = max( max( abs( nodeVar125 ), abs( nodeVar126 ) ), 1.0 );
	let nodeConst8 = i32( max( ( clamp( object.nodeUniform11, 0.0, 1.0 ) * 64.0 ), 1.0 ) );
	nodeVar128 = ( nodeVar125 / f32( nodeConst8 ) );
	nodeVar129 = ( nodeVar126 / f32( nodeConst8 ) );
	nodeVar130 = vec2<f32>( nodeVar128, nodeVar129 );
	nodeVar131 = ( vec2<f32>( 1.0, 1.0 ) / object.nodeUniform4 );
	nodeVar132 = vec2<f32>( nodeVar131.x, 0.0 );
	nodeVar133 = vec4<f32>( 0.0, 0.0, 0.0, 0.0 );
	nodeVar134 = 0.0;
	let nodeConst9 = ( 1.0 / nodeVar4.z );
	let nodeConst10 = ( 1.0 / nodeVar121.z );
	nodeVar135 = false;
	nodeVar136 = vec2<f32>( 0.0, 0.0 );
	nodeVar137 = 0.0;

	for ( var i : i32 = 1; i < nodeConst8; i ++ ) {

		let nodeConst11 = f32( i );
		nodeVar138 = max( pow( ( ( nodeConst11 + ( nodeVar11.z - 0.5 ) ) / f32( nodeConst8 ) ), 2.0 ), ( nodeConst11 / nodeVar127 ) );
		nodeVar139 = ( nodeVar122 + ( nodeVar130 * vec2<f32>( ( nodeVar138 * f32( nodeConst8 ) ) ) ) );

		if ( ( ( ( ( nodeVar139.x < 0.0 ) || ( nodeVar139.x > object.nodeUniform4.x ) ) || ( nodeVar139.y < 0.0 ) ) || ( nodeVar139.y > object.nodeUniform4.y ) ) ) {

			break;
			

		}

		nodeVar140 = ( nodeVar139 * nodeVar131 );
		nodeVar141 = textureLoad( nodeUniform0, vec2<u32>( clamp( floor( tsl_coord_clampS_clampT_2d( nodeVar140 ) * vec2<f32>( nodeVar2 ) ), vec2<f32>( 0 ), vec2<f32>( nodeVar2 - vec2<u32>( 1, 1 ) ) ) ), u32( 0 ) );
		nodeVar142 = nodeVar141;
		nodeVar143 = ( ( object.nodeUniform9 * object.nodeUniform12 ) / ( ( ( object.nodeUniform12 - object.nodeUniform9 ) * nodeVar142 ) - object.nodeUniform12 ) );
		nodeVar144 = ( 1.0 / ( nodeConst9 + ( nodeVar138 * ( nodeConst10 - nodeConst9 ) ) ) );

		if ( ( nodeVar144 <= nodeVar143 ) ) {

			let nodeConst12 = ( object.nodeUniform1 * vec4<f32>( vec3<f32>( ( ( vec2<f32>( nodeVar140.x, ( 1.0 - nodeVar140.y ) ) * vec2<f32>( 2.0 ) ) - vec2<f32>( 1.0 ) ), nodeVar142 ), 1.0 ) );
			nodeVar145 = ( nodeConst12.xyz / vec3<f32>( nodeConst12.w ) );
			nodeVar146 = ( length( cross( ( nodeVar145 - nodeVar4 ), ( nodeVar145 - nodeVar121 ) ) ) / length( ( nodeVar121 - nodeVar4 ) ) );
			nodeVar147 = ( nodeVar140 + nodeVar132 );
			let nodeConst13 = ( object.nodeUniform1 * vec4<f32>( vec3<f32>( ( ( vec2<f32>( nodeVar147.x, ( 1.0 - nodeVar147.y ) ) * vec2<f32>( 2.0 ) ) - vec2<f32>( 1.0 ) ), nodeVar142 ), 1.0 ) );
			nodeVar148 = ( nodeConst13.xyz / vec3<f32>( nodeConst13.w ) );
			nodeVar149 = ( ( nodeVar148.x - nodeVar145.x ) * 3.0 );
			nodeVar150 = max( nodeVar149, object.nodeUniform13 );

			if ( ( nodeVar146 <= nodeVar150 ) ) {

				nodeVar151 = textureSample( nodeUniform3, nodeUniform3_sampler, nodeVar140 );
				nodeVar152 = normalize( ( ( nodeVar151.xyz * vec3<f32>( 2.0 ) ) - vec3<f32>( 1.0 ) ) );
				nodeVar135 = true;
				nodeVar136 = nodeVar140;
				nodeVar137 = nodeVar142;
				break;
				

			}

			

		}


	}


	if ( nodeVar135 ) {

		let nodeConst14 = ( object.nodeUniform1 * vec4<f32>( vec3<f32>( ( ( vec2<f32>( nodeVar136.x, ( 1.0 - nodeVar136.y ) ) * vec2<f32>( 2.0 ) ) - vec2<f32>( 1.0 ) ), nodeVar137 ), 1.0 ) );
		nodeVar153 = ( nodeConst14.xyz / vec3<f32>( nodeConst14.w ) );

		if ( ( 0.0 <= object.nodeUniform8 ) ) {

			nodeVar154 = ( object.nodeUniform2 * vec4<f32>( nodeVar153, 1.0 ) ).xyz;
			nodeVar155 = ( distance( nodeVar5, nodeVar154 ) * nodeVar119 );
			nodeVar156 = textureSample( nodeUniform14, nodeUniform14_sampler, nodeVar136 );
			nodeVar157 = nodeVar156;
			nodeVar158 = nodeVar157.xyz;
			nodeVar157.x = nodeVar158[ 0 ];
			nodeVar157.y = nodeVar158[ 1 ];
			nodeVar157.z = nodeVar158[ 2 ];
			let nodeConst15 = computeScreenBorderFactor( nodeVar136, ( object.nodeUniform15 * ( 1.0 - min( ( nodeConst4 / 0.25 ), 1.0 ) ) ) );

			if ( ( nodeConst15 < 1.0 ) ) {

				nodeVar159 = vec3<f32>( 0.0, 0.0, 0.0 );
				nodeVar160 = normalize( ( object.nodeUniform2 * vec4<f32>( nodeVar7, 0.0 ) ).xyz );
				nodeVar161 = normalize( ( object.nodeUniform2 * vec4<f32>( nodeVar9, 0.0 ) ).xyz );
				nodeVar162 = max( 0.0, dot( nodeVar160, nodeVar161 ) );
				nodeVar163 = normalize( ( object.nodeUniform2 * vec4<f32>( nodeVar117, 0.0 ) ).xyz );
				nodeVar164 = normalize( ( nodeVar161 + nodeVar163 ) );
				nodeVar165 = max( 0.0, dot( nodeVar160, nodeVar163 ) );
				nodeVar166 = max( 0.0, dot( nodeVar161, nodeVar164 ) );
				nodeVar168 = textureDimensions( nodeUniform16, u32( 0.0 ) );
				nodeVar167 = textureLoad( nodeUniform16, vec2<u32>( clamp( floor( tsl_coord_repeatS_clampT_2d( vec2<f32>( ( ( atan2( nodeVar163.z, nodeVar163.x ) * 0.15915494309189535 ) + 0.5 ), ( ( asin( clamp( nodeVar163.y, -1.0, 1.0 ) ) * 0.3183098861837907 ) + 0.5 ) ) ) * vec2<f32>( nodeVar168 ) ), vec2<f32>( 0 ), vec2<f32>( nodeVar168 - vec2<u32>( 1, 1 ) ) ) ), u32( 0.0 ) );
				nodeVar169 = ( 1.0 - nodeVar166 );
				nodeVar170 = ( nodeVar169 * nodeVar169 );
				nodeVar171 = ( ( nodeVar170 * nodeVar170 ) * nodeVar169 );
				nodeVar172 = ( nodeVar66.f0 + ( ( vec3<f32>( 1.0, 1.0, 1.0 ) - nodeVar66.f0 ) * vec3<f32>( nodeVar171 ) ) );
				nodeVar173 = ( nodeVar66.alpha * nodeVar66.alpha );
				nodeVar174 = ( nodeVar162 * nodeVar162 );
				nodeVar175 = ( ( 2.0 * nodeVar162 ) / ( nodeVar162 + sqrt( ( nodeVar173 + ( ( 1.0 - nodeVar173 ) * nodeVar174 ) ) ) ) );
				nodeVar176 = ( nodeVar66.alpha * nodeVar66.alpha );
				nodeVar177 = ( nodeVar165 * nodeVar165 );
				nodeVar178 = ( ( 2.0 * nodeVar165 ) / ( nodeVar165 + sqrt( ( nodeVar176 + ( ( 1.0 - nodeVar176 ) * nodeVar177 ) ) ) ) );
				nodeVar179 = ( nodeVar175 * nodeVar178 );
				nodeVar180 = ( nodeVar66.alpha * nodeVar66.alpha );
				nodeVar181 = ( nodeVar162 * nodeVar162 );
				nodeVar159 = ( ( nodeVar167.xyz * ( ( nodeVar172 * vec3<f32>( nodeVar179 ) ) / vec3<f32>( max( ( ( 2.0 * nodeVar162 ) / ( nodeVar162 + sqrt( ( nodeVar180 + ( ( 1.0 - nodeVar180 ) * nodeVar181 ) ) ) ) ), 0.0001 ) ) ) ) * vec3<f32>( object.nodeUniform17 ) );
				nodeVar182 = mix( ( nodeVar159 * vec3<f32>( object.nodeUniform18 ) ), nodeVar157.xyz, nodeConst15 );
				nodeVar157.x = nodeVar182[ 0 ];
				nodeVar157.y = nodeVar182[ 1 ];
				nodeVar157.z = nodeVar182[ 2 ];
				

			}

			nodeVar134 = 1.0;
			nodeVar133 = vec4<f32>( ( nodeVar157.xyz * nodeVar118 ), nodeVar155 );
			

		}

		

	}


	if ( ( nodeVar134 == 0.0 ) ) {

		nodeVar183 = vec3<f32>( 0.0, 0.0, 0.0 );
		nodeVar184 = normalize( ( object.nodeUniform2 * vec4<f32>( nodeVar7, 0.0 ) ).xyz );
		nodeVar185 = normalize( ( object.nodeUniform2 * vec4<f32>( nodeVar9, 0.0 ) ).xyz );
		nodeVar186 = max( 0.0, dot( nodeVar184, nodeVar185 ) );
		nodeVar187 = normalize( ( object.nodeUniform2 * vec4<f32>( nodeVar117, 0.0 ) ).xyz );
		nodeVar188 = normalize( ( nodeVar185 + nodeVar187 ) );
		nodeVar189 = max( 0.0, dot( nodeVar184, nodeVar187 ) );
		nodeVar190 = max( 0.0, dot( nodeVar185, nodeVar188 ) );
		nodeVar192 = textureDimensions( nodeUniform16, u32( 0.0 ) );
		nodeVar191 = textureLoad( nodeUniform16, vec2<u32>( clamp( floor( tsl_coord_repeatS_clampT_2d( vec2<f32>( ( ( atan2( nodeVar187.z, nodeVar187.x ) * 0.15915494309189535 ) + 0.5 ), ( ( asin( clamp( nodeVar187.y, -1.0, 1.0 ) ) * 0.3183098861837907 ) + 0.5 ) ) ) * vec2<f32>( nodeVar192 ) ), vec2<f32>( 0 ), vec2<f32>( nodeVar192 - vec2<u32>( 1, 1 ) ) ) ), u32( 0.0 ) );
		nodeVar193 = ( 1.0 - nodeVar190 );
		nodeVar194 = ( nodeVar193 * nodeVar193 );
		nodeVar195 = ( ( nodeVar194 * nodeVar194 ) * nodeVar193 );
		nodeVar196 = ( nodeVar66.f0 + ( ( vec3<f32>( 1.0, 1.0, 1.0 ) - nodeVar66.f0 ) * vec3<f32>( nodeVar195 ) ) );
		nodeVar197 = ( nodeVar66.alpha * nodeVar66.alpha );
		nodeVar198 = ( nodeVar186 * nodeVar186 );
		nodeVar199 = ( ( 2.0 * nodeVar186 ) / ( nodeVar186 + sqrt( ( nodeVar197 + ( ( 1.0 - nodeVar197 ) * nodeVar198 ) ) ) ) );
		nodeVar200 = ( nodeVar66.alpha * nodeVar66.alpha );
		nodeVar201 = ( nodeVar189 * nodeVar189 );
		nodeVar202 = ( ( 2.0 * nodeVar189 ) / ( nodeVar189 + sqrt( ( nodeVar200 + ( ( 1.0 - nodeVar200 ) * nodeVar201 ) ) ) ) );
		nodeVar203 = ( nodeVar199 * nodeVar202 );
		nodeVar204 = ( nodeVar66.alpha * nodeVar66.alpha );
		nodeVar205 = ( nodeVar186 * nodeVar186 );
		nodeVar183 = ( ( nodeVar191.xyz * ( ( nodeVar196 * vec3<f32>( nodeVar203 ) ) / vec3<f32>( max( ( ( 2.0 * nodeVar186 ) / ( nodeVar186 + sqrt( ( nodeVar204 + ( ( 1.0 - nodeVar204 ) * nodeVar205 ) ) ) ) ), 0.0001 ) ) ) ) * vec3<f32>( object.nodeUniform17 ) );
		nodeVar133 = vec4<f32>( ( nodeVar183 * vec3<f32>( object.nodeUniform18 ) ), 10000.0 );
		

	}

	nodeVar206 = max( dot( nodeVar133.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) ), 0.0001 );
	nodeVar207 = ( nodeVar133.xyz * vec3<f32>( min( ( object.nodeUniform19 / nodeVar206 ), 1.0 ) ) );
	nodeVar133.x = nodeVar207[ 0 ];
	nodeVar133.y = nodeVar207[ 1 ];
	nodeVar133.z = nodeVar207[ 2 ];
	nodeVar208 = ( nodeVar133.xyz * vec3<f32>( object.nodeUniform20 ) );
	nodeVar133.x = nodeVar208[ 0 ];
	nodeVar133.y = nodeVar208[ 1 ];
	nodeVar133.z = nodeVar208[ 2 ];

	// result

	output.color = max( nodeVar133, vec4<f32>( 0.0 ) );

	return output;

}
