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
@binding( 8 ) @group( 0 ) var nodeUniform15_sampler : sampler;
@binding( 9 ) @group( 0 ) var nodeUniform15 : texture_2d<f32>;
@binding( 10 ) @group( 0 ) var nodeUniform16_sampler : sampler;
@binding( 11 ) @group( 0 ) var nodeUniform16 : texture_2d<f32>;
@binding( 12 ) @group( 0 ) var nodeUniform18 : texture_2d<f32>;
@binding( 13 ) @group( 0 ) var nodeUniform21_sampler : sampler;
@binding( 14 ) @group( 0 ) var nodeUniform21 : texture_2d<f32>;
@binding( 15 ) @group( 0 ) var nodeUniform23_sampler : sampler;
@binding( 16 ) @group( 0 ) var nodeUniform23 : texture_2d<f32>;

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
	nodeUniform17 : f32,
	nodeUniform19 : vec2<f32>,
	nodeUniform20 : f32,
	nodeUniform22 : mat3x3<f32>,
	nodeUniform24 : mat3x3<f32>,
	nodeUniform25 : f32,
	nodeUniform26 : f32,
	nodeUniform27 : f32,
	nodeUniform28 : f32
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
var<private> nodeVar136 : f32;
var<private> nodeVar137 : f32;
var<private> nodeVar138 : vec2<f32>;
var<private> nodeVar139 : f32;
var<private> nodeVar140 : f32;
var<private> nodeVar141 : vec2<f32>;
var<private> nodeVar142 : vec2<f32>;
var<private> nodeVar143 : f32;
var<private> nodeVar144 : f32;
var<private> nodeVar145 : f32;
var<private> nodeVar146 : f32;
var<private> nodeVar147 : vec3<f32>;
var<private> nodeVar148 : f32;
var<private> nodeVar149 : vec2<f32>;
var<private> nodeVar150 : vec3<f32>;
var<private> nodeVar151 : f32;
var<private> nodeVar152 : f32;
var<private> nodeVar153 : vec4<f32>;
var<private> nodeVar154 : vec3<f32>;
var<private> nodeVar155 : f32;
var<private> nodeVar156 : f32;
var<private> nodeVar157 : f32;
var<private> nodeVar158 : vec3<f32>;
var<private> nodeVar159 : vec3<f32>;
var<private> nodeVar160 : f32;
var<private> nodeVar161 : vec4<f32>;
var<private> nodeVar162 : vec4<f32>;
var<private> nodeVar163 : vec4<f32>;
var<private> nodeVar164 : vec4<f32>;
var<private> nodeVar165 : vec4<f32>;
var<private> nodeVar166 : vec3<f32>;
var<private> nodeVar167 : vec3<f32>;
var<private> nodeVar168 : vec3<f32>;
var<private> nodeVar169 : vec3<f32>;
var<private> nodeVar170 : f32;
var<private> nodeVar171 : vec3<f32>;
var<private> nodeVar172 : vec3<f32>;
var<private> nodeVar173 : f32;
var<private> nodeVar174 : f32;
var<private> nodeVar175 : f32;
var<private> nodeVar176 : vec4<f32>;
var<private> nodeVar177 : vec2<u32>;
var<private> nodeVar178 : f32;
var<private> nodeVar179 : f32;
var<private> nodeVar180 : f32;
var<private> nodeVar181 : vec3<f32>;
var<private> nodeVar182 : f32;
var<private> nodeVar183 : f32;
var<private> nodeVar184 : f32;
var<private> nodeVar185 : f32;
var<private> nodeVar186 : f32;
var<private> nodeVar187 : f32;
var<private> nodeVar188 : f32;
var<private> nodeVar189 : f32;
var<private> nodeVar190 : f32;
var<private> nodeVar191 : f32;
var<private> nodeVar192 : f32;
var<private> nodeVar193 : f32;
var<private> nodeVar194 : f32;
var<private> nodeVar195 : f32;
var<private> nodeVar196 : f32;
var<private> nodeVar197 : vec3<f32>;
var<private> nodeVar198 : vec4<f32>;
var<private> nodeVar199 : f32;
var<private> nodeVar200 : f32;
var<private> nodeVar201 : f32;
var<private> nodeVar202 : f32;
var<private> nodeVar203 : f32;
var<private> nodeVar204 : f32;
var<private> nodeVar205 : f32;
var<private> nodeVar206 : vec4<f32>;
var<private> nodeVar207 : f32;
var<private> nodeVar208 : f32;
var<private> nodeVar209 : f32;
var<private> nodeVar210 : f32;
var<private> nodeVar211 : f32;
var<private> nodeVar212 : f32;
var<private> nodeVar213 : f32;
var<private> nodeVar214 : f32;
var<private> nodeVar215 : f32;
var<private> nodeVar216 : f32;
var<private> nodeVar217 : vec3<f32>;
var<private> nodeVar218 : f32;
var<private> nodeVar219 : f32;
var<private> nodeVar220 : vec3<f32>;
var<private> nodeVar221 : vec3<f32>;
var<private> nodeVar222 : vec3<f32>;
var<private> nodeVar223 : vec3<f32>;
var<private> nodeVar224 : f32;
var<private> nodeVar225 : vec3<f32>;
var<private> nodeVar226 : vec3<f32>;
var<private> nodeVar227 : f32;
var<private> nodeVar228 : f32;
var<private> nodeVar229 : f32;
var<private> nodeVar230 : vec4<f32>;
var<private> nodeVar231 : vec2<u32>;
var<private> nodeVar232 : f32;
var<private> nodeVar233 : f32;
var<private> nodeVar234 : f32;
var<private> nodeVar235 : vec3<f32>;
var<private> nodeVar236 : f32;
var<private> nodeVar237 : f32;
var<private> nodeVar238 : f32;
var<private> nodeVar239 : f32;
var<private> nodeVar240 : f32;
var<private> nodeVar241 : f32;
var<private> nodeVar242 : f32;
var<private> nodeVar243 : f32;
var<private> nodeVar244 : f32;
var<private> nodeVar245 : f32;
var<private> nodeVar246 : f32;
var<private> nodeVar247 : f32;
var<private> nodeVar248 : f32;
var<private> nodeVar249 : f32;
var<private> nodeVar250 : f32;
var<private> nodeVar251 : vec3<f32>;
var<private> nodeVar252 : vec4<f32>;
var<private> nodeVar253 : f32;
var<private> nodeVar254 : f32;
var<private> nodeVar255 : f32;
var<private> nodeVar256 : f32;
var<private> nodeVar257 : f32;
var<private> nodeVar258 : f32;
var<private> nodeVar259 : f32;
var<private> nodeVar260 : vec4<f32>;
var<private> nodeVar261 : f32;
var<private> nodeVar262 : f32;
var<private> nodeVar263 : f32;
var<private> nodeVar264 : f32;
var<private> nodeVar265 : f32;
var<private> nodeVar266 : f32;
var<private> nodeVar267 : f32;
var<private> nodeVar268 : f32;
var<private> nodeVar269 : f32;
var<private> nodeVar270 : f32;
var<private> nodeVar271 : vec3<f32>;
var<private> nodeVar272 : f32;
var<private> nodeVar273 : f32;
var<private> nodeVar274 : f32;
var<private> nodeVar275 : vec3<f32>;
var<private> nodeVar276 : vec3<f32>;

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

fn equirectDirPdf ( direction : vec3<f32> ) -> f32 {

	var nodeVar0 : f32;

	let nodeConst0 = sin( ( vec2<f32>( ( ( atan2( direction.z, direction.x ) * 0.15915494309189535 ) + 0.5 ), ( ( asin( clamp( direction.y, -1.0, 1.0 ) ) * 0.3183098861837907 ) + 0.5 ) ).y * 3.141592653589793 ) );

	if ( ( abs( nodeConst0 ) < 0.000001 ) ) {

		nodeVar0 = 0.0;

	} else {

		nodeVar0 = ( 1.0 / ( 19.739208802178716 * nodeConst0 ) );

	}


	return nodeVar0;

}


fn misPowerHeuristic ( pdfA : f32, pdfB : f32 ) -> f32 {

	

	let nodeConst0 = ( pdfA * pdfA );

	return ( nodeConst0 / ( nodeConst0 + ( pdfB * pdfB ) ) );

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
	nodeVar136 = 0.0;
	nodeVar137 = 0.0;
	nodeVar138 = vec2<f32>( 0.0, 0.0 );
	nodeVar139 = 0.0;

	for ( var i : i32 = 1; i < nodeConst8; i ++ ) {

		let nodeConst11 = f32( i );
		nodeVar140 = max( pow( ( ( nodeConst11 + ( nodeVar11.z - 0.5 ) ) / f32( nodeConst8 ) ), 3.0 ), ( nodeConst11 / nodeVar127 ) );
		nodeVar141 = ( nodeVar122 + ( nodeVar130 * vec2<f32>( ( nodeVar140 * f32( nodeConst8 ) ) ) ) );

		if ( ( ( ( ( nodeVar141.x < 0.0 ) || ( nodeVar141.x > object.nodeUniform4.x ) ) || ( nodeVar141.y < 0.0 ) ) || ( nodeVar141.y > object.nodeUniform4.y ) ) ) {

			break;
			

		}

		nodeVar142 = ( nodeVar141 * nodeVar131 );
		nodeVar143 = textureLoad( nodeUniform0, vec2<u32>( clamp( floor( tsl_coord_clampS_clampT_2d( nodeVar142 ) * vec2<f32>( nodeVar2 ) ), vec2<f32>( 0 ), vec2<f32>( nodeVar2 - vec2<u32>( 1, 1 ) ) ) ), u32( 0 ) );
		nodeVar144 = nodeVar143;
		nodeVar145 = ( ( object.nodeUniform9 * object.nodeUniform12 ) / ( ( ( object.nodeUniform12 - object.nodeUniform9 ) * nodeVar144 ) - object.nodeUniform12 ) );
		nodeVar146 = ( 1.0 / ( nodeConst9 + ( nodeVar140 * ( nodeConst10 - nodeConst9 ) ) ) );

		if ( ( nodeVar146 <= nodeVar145 ) ) {

			let nodeConst12 = ( object.nodeUniform1 * vec4<f32>( vec3<f32>( ( ( vec2<f32>( nodeVar142.x, ( 1.0 - nodeVar142.y ) ) * vec2<f32>( 2.0 ) ) - vec2<f32>( 1.0 ) ), nodeVar144 ), 1.0 ) );
			nodeVar147 = ( nodeConst12.xyz / vec3<f32>( nodeConst12.w ) );
			nodeVar148 = ( length( cross( ( nodeVar147 - nodeVar4 ), ( nodeVar147 - nodeVar121 ) ) ) / length( ( nodeVar121 - nodeVar4 ) ) );
			nodeVar149 = ( nodeVar142 + nodeVar132 );
			let nodeConst13 = ( object.nodeUniform1 * vec4<f32>( vec3<f32>( ( ( vec2<f32>( nodeVar149.x, ( 1.0 - nodeVar149.y ) ) * vec2<f32>( 2.0 ) ) - vec2<f32>( 1.0 ) ), nodeVar144 ), 1.0 ) );
			nodeVar150 = ( nodeConst13.xyz / vec3<f32>( nodeConst13.w ) );
			nodeVar151 = ( ( nodeVar150.x - nodeVar147.x ) * 3.0 );
			nodeVar152 = max( nodeVar151, object.nodeUniform13 );

			if ( ( nodeVar148 <= nodeVar152 ) ) {

				nodeVar153 = textureSample( nodeUniform3, nodeUniform3_sampler, nodeVar142 );
				nodeVar154 = normalize( ( ( nodeVar153.xyz * vec3<f32>( 2.0 ) ) - vec3<f32>( 1.0 ) ) );
				nodeVar135 = true;
				nodeVar138 = nodeVar142;
				nodeVar139 = nodeVar144;
				let nodeConst14 = ( f32( i ) - 1.0 );
				nodeVar136 = max( pow( ( ( nodeConst14 + ( nodeVar11.z - 0.5 ) ) / f32( nodeConst8 ) ), 3.0 ), ( nodeConst14 / nodeVar127 ) );
				nodeVar137 = nodeVar140;
				break;
				

			}

			

		}


	}


	if ( nodeVar135 ) {


		for ( var i : i32 = 0; i < 8; i ++ ) {

			nodeVar155 = ( ( nodeVar136 + nodeVar137 ) * 0.5 );
			nodeVar156 = textureLoad( nodeUniform0, vec2<u32>( clamp( floor( tsl_coord_clampS_clampT_2d( ( ( nodeVar122 + ( nodeVar130 * vec2<f32>( ( nodeVar155 * f32( nodeConst8 ) ) ) ) ) * nodeVar131 ) ) * vec2<f32>( nodeVar2 ) ), vec2<f32>( 0 ), vec2<f32>( nodeVar2 - vec2<u32>( 1, 1 ) ) ) ), u32( 0 ) );

			if ( ( ( 1.0 / ( nodeConst9 + ( nodeVar155 * ( nodeConst10 - nodeConst9 ) ) ) ) <= ( ( object.nodeUniform9 * object.nodeUniform12 ) / ( ( ( object.nodeUniform12 - object.nodeUniform9 ) * nodeVar156 ) - object.nodeUniform12 ) ) ) ) {

				nodeVar137 = nodeVar155;
				

			} else {

				nodeVar136 = nodeVar155;
				

			}


		}

		nodeVar138 = ( ( nodeVar122 + ( nodeVar130 * vec2<f32>( ( nodeVar137 * f32( nodeConst8 ) ) ) ) ) * nodeVar131 );
		nodeVar157 = textureLoad( nodeUniform0, vec2<u32>( clamp( floor( tsl_coord_clampS_clampT_2d( nodeVar138 ) * vec2<f32>( nodeVar2 ) ), vec2<f32>( 0 ), vec2<f32>( nodeVar2 - vec2<u32>( 1, 1 ) ) ) ), u32( 0 ) );
		nodeVar139 = nodeVar157;
		let nodeConst15 = ( object.nodeUniform1 * vec4<f32>( vec3<f32>( ( ( vec2<f32>( nodeVar138.x, ( 1.0 - nodeVar138.y ) ) * vec2<f32>( 2.0 ) ) - vec2<f32>( 1.0 ) ), nodeVar139 ), 1.0 ) );
		nodeVar158 = ( nodeConst15.xyz / vec3<f32>( nodeConst15.w ) );

		if ( ( 0.0 <= object.nodeUniform8 ) ) {

			nodeVar159 = ( object.nodeUniform2 * vec4<f32>( nodeVar158, 1.0 ) ).xyz;
			nodeVar160 = ( distance( nodeVar5, nodeVar159 ) * nodeVar119 );
			nodeVar161 = textureSample( nodeUniform14, nodeUniform14_sampler, nodeVar138 );
			nodeVar162 = nodeVar161;
			nodeVar163 = textureSample( nodeUniform16, nodeUniform16_sampler, nodeVar138 );
			nodeVar164 = textureSample( nodeUniform15, nodeUniform15_sampler, ( nodeVar138 - nodeVar163.xy ) );
			nodeVar165 = nodeVar164;
			nodeVar166 = ( nodeVar162.xyz + ( nodeVar165.xyz * vec3<f32>( ( 1.0 - nodeVar165.w ) ) ) );
			nodeVar162.x = nodeVar166[ 0 ];
			nodeVar162.y = nodeVar166[ 1 ];
			nodeVar162.z = nodeVar166[ 2 ];
			let nodeConst16 = computeScreenBorderFactor( nodeVar138, ( object.nodeUniform17 * ( 1.0 - min( ( nodeConst4 / 0.25 ), 1.0 ) ) ) );

			if ( ( nodeConst16 < 1.0 ) ) {

				nodeVar167 = vec3<f32>( 0.0, 0.0, 0.0 );
				nodeVar168 = normalize( ( object.nodeUniform2 * vec4<f32>( nodeVar7, 0.0 ) ).xyz );
				nodeVar169 = normalize( ( object.nodeUniform2 * vec4<f32>( nodeVar9, 0.0 ) ).xyz );
				nodeVar170 = max( 0.0, dot( nodeVar168, nodeVar169 ) );
				nodeVar171 = normalize( ( object.nodeUniform2 * vec4<f32>( nodeVar117, 0.0 ) ).xyz );
				nodeVar172 = normalize( ( nodeVar169 + nodeVar171 ) );
				nodeVar173 = max( 0.0, dot( nodeVar168, nodeVar171 ) );
				nodeVar174 = max( 0.0, dot( nodeVar168, nodeVar172 ) );
				nodeVar175 = max( 0.0, dot( nodeVar169, nodeVar172 ) );
				nodeVar177 = textureDimensions( nodeUniform18, u32( 0.0 ) );
				nodeVar176 = textureLoad( nodeUniform18, vec2<u32>( clamp( floor( tsl_coord_repeatS_clampT_2d( vec2<f32>( ( ( atan2( nodeVar171.z, nodeVar171.x ) * 0.15915494309189535 ) + 0.5 ), ( ( asin( clamp( nodeVar171.y, -1.0, 1.0 ) ) * 0.3183098861837907 ) + 0.5 ) ) ) * vec2<f32>( nodeVar177 ) ), vec2<f32>( 0 ), vec2<f32>( nodeVar177 - vec2<u32>( 1, 1 ) ) ) ), u32( 0.0 ) );
				nodeVar178 = ( 1.0 - nodeVar175 );
				nodeVar179 = ( nodeVar178 * nodeVar178 );
				nodeVar180 = ( ( nodeVar179 * nodeVar179 ) * nodeVar178 );
				nodeVar181 = ( nodeVar66.f0 + ( ( vec3<f32>( 1.0, 1.0, 1.0 ) - nodeVar66.f0 ) * vec3<f32>( nodeVar180 ) ) );
				nodeVar182 = ( nodeVar66.alpha * nodeVar66.alpha );
				nodeVar183 = ( nodeVar170 * nodeVar170 );
				nodeVar184 = ( ( 2.0 * nodeVar170 ) / ( nodeVar170 + sqrt( ( nodeVar182 + ( ( 1.0 - nodeVar182 ) * nodeVar183 ) ) ) ) );
				nodeVar185 = ( nodeVar66.alpha * nodeVar66.alpha );
				nodeVar186 = ( nodeVar173 * nodeVar173 );
				nodeVar187 = ( ( 2.0 * nodeVar173 ) / ( nodeVar173 + sqrt( ( nodeVar185 + ( ( 1.0 - nodeVar185 ) * nodeVar186 ) ) ) ) );
				nodeVar188 = ( nodeVar184 * nodeVar187 );
				nodeVar189 = ( nodeVar66.alpha * nodeVar66.alpha );
				nodeVar190 = ( nodeVar170 * nodeVar170 );
				nodeVar191 = ( nodeVar66.alpha * nodeVar66.alpha );
				nodeVar192 = ( nodeVar174 * nodeVar174 );
				nodeVar193 = ( ( nodeVar192 * ( nodeVar191 - 1.0 ) ) + 1.0 );
				nodeVar194 = ( nodeVar191 / ( 3.141592653589793 * pow( nodeVar193, 2.0 ) ) );
				nodeVar195 = ( nodeVar66.alpha * nodeVar66.alpha );
				nodeVar196 = ( nodeVar170 * nodeVar170 );
				nodeVar197 = ( ( nodeVar176.xyz * ( ( nodeVar181 * vec3<f32>( nodeVar188 ) ) / vec3<f32>( max( ( ( 2.0 * nodeVar170 ) / ( nodeVar170 + sqrt( ( nodeVar189 + ( ( 1.0 - nodeVar189 ) * nodeVar190 ) ) ) ) ), 0.0001 ) ) ) ) * vec3<f32>( misPowerHeuristic( max( ( ( nodeVar194 * ( ( 2.0 * nodeVar170 ) / ( nodeVar170 + sqrt( ( nodeVar195 + ( ( 1.0 - nodeVar195 ) * nodeVar196 ) ) ) ) ) ) / max( 0.000001, ( 4.0 * nodeVar170 ) ) ), 1e-8 ), max( ( ( ( object.nodeUniform19.x * object.nodeUniform19.y ) * ( dot( nodeVar176.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) ) / object.nodeUniform20 ) ) * equirectDirPdf( nodeVar171 ) ), 1e-8 ) ) ) );

				if ( ( nodeVar66.alpha > 0.01 ) ) {

					nodeVar198 = vec4<f32>( 0.0, 0.0, 0.0, 1.0 );
					let nodeConst17 = ( i32( object.nodeUniform5 ) + 59 );
					let nodeConst18 = tsl_mod_vec2( ( floor( ( nodeVar0 * object.nodeUniform4 ) ) + floor( ( fract( vec2<f32>( ( f32( nodeConst17 ) * 0.7548776662 ), ( f32( nodeConst17 ) * 0.569840291 ) ) ) * vec2<f32>( 32.0 ) ) ) ), vec2<f32>( 32.0 ) );
					let nodeConst19 = ( ( ( nodeConst18.x * 0.7548776662466927 ) + ( nodeConst18.y * 0.5698402909980532 ) ) + 59.0 );
					nodeVar198 = vec4<f32>( fract( ( ( nodeConst19 * 1.324717957244746 ) * 0.7548776662466927 ) ), fract( ( ( nodeConst19 * 2.649435914489492 ) * 0.5698402909980532 ) ), fract( ( ( nodeConst19 * 3.974153871734238 ) * 0.419875421 ) ), fract( ( ( nodeConst19 * 5.298871828978984 ) * 0.43015970900194667 ) ) );
					let nodeConst20 = vec2<f32>( nodeVar198.z, nodeVar198.w );
					nodeVar199 = textureSample( nodeUniform23, nodeUniform23_sampler, ( object.nodeUniform24 * vec3<f32>( vec2<f32>( nodeConst20.x, 0.0 ), 1.0 ) ).xy ).x;
					nodeVar200 = textureSample( nodeUniform21, nodeUniform21_sampler, ( object.nodeUniform22 * vec3<f32>( vec2<f32>( nodeConst20.y, nodeVar199 ), 1.0 ) ).xy ).x;
					let nodeConst21 = vec2<f32>( nodeVar200, nodeVar199 );
					let nodeConst22 = vec2<f32>( ( ( atan2( vec3<f32>( nodeConst21, 0.0 ).z, nodeConst21.x ) * 0.15915494309189535 ) + 0.5 ), ( ( asin( clamp( nodeConst21.y, -1.0, 1.0 ) ) * 0.3183098861837907 ) + 0.5 ) );
					let nodeConst23 = max( 0.0, dot( nodeVar168, vec3<f32>( nodeConst22, 0.0 ) ) );

					if ( ( nodeConst23 > 0.001 ) ) {

						nodeVar201 = ( nodeVar66.alpha * nodeVar66.alpha );
						let nodeConst24 = normalize( ( nodeVar169 + vec3<f32>( nodeConst22, 0.0 ) ) );
						let nodeConst25 = max( 0.0, dot( nodeVar168, nodeConst24 ) );
						nodeVar202 = ( nodeConst25 * nodeConst25 );
						nodeVar203 = ( ( nodeVar202 * ( nodeVar201 - 1.0 ) ) + 1.0 );
						nodeVar204 = ( nodeVar201 / ( 3.141592653589793 * pow( nodeVar203, 2.0 ) ) );
						nodeVar205 = nodeVar204;
						nodeVar206 = textureLoad( nodeUniform18, vec2<u32>( clamp( floor( tsl_coord_repeatS_clampT_2d( nodeConst21 ) * vec2<f32>( nodeVar177 ) ), vec2<f32>( 0 ), vec2<f32>( nodeVar177 - vec2<u32>( 1, 1 ) ) ) ), u32( 0.0 ) );
						nodeVar207 = ( nodeVar66.alpha * nodeVar66.alpha );
						nodeVar208 = ( nodeVar170 * nodeVar170 );
						nodeVar209 = ( ( 2.0 * nodeVar170 ) / ( nodeVar170 + sqrt( ( nodeVar207 + ( ( 1.0 - nodeVar207 ) * nodeVar208 ) ) ) ) );
						nodeVar210 = ( nodeVar66.alpha * nodeVar66.alpha );
						nodeVar211 = ( nodeConst23 * nodeConst23 );
						nodeVar212 = ( ( 2.0 * nodeConst23 ) / ( nodeConst23 + sqrt( ( nodeVar210 + ( ( 1.0 - nodeVar210 ) * nodeVar211 ) ) ) ) );
						nodeVar213 = ( nodeVar209 * nodeVar212 );
						nodeVar214 = ( 1.0 - max( 0.0, dot( nodeVar169, nodeConst24 ) ) );
						nodeVar215 = ( nodeVar214 * nodeVar214 );
						nodeVar216 = ( ( nodeVar215 * nodeVar215 ) * nodeVar214 );
						nodeVar217 = ( nodeVar66.f0 + ( ( vec3<f32>( 1.0, 1.0, 1.0 ) - nodeVar66.f0 ) * vec3<f32>( nodeVar216 ) ) );
						let nodeConst26 = max( ( ( ( object.nodeUniform19.x * object.nodeUniform19.y ) * ( dot( nodeVar206.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) ) / object.nodeUniform20 ) ) * equirectDirPdf( vec3<f32>( nodeConst22, 0.0 ) ) ), 1e-8 );
						nodeVar218 = ( nodeVar66.alpha * nodeVar66.alpha );
						nodeVar219 = ( nodeVar170 * nodeVar170 );
						nodeVar197 = ( nodeVar197 + ( ( ( ( ( nodeVar206.xyz * vec3<f32>( ( ( nodeVar205 * nodeVar213 ) / max( 0.000001, ( ( 4.0 * nodeConst23 ) * nodeVar170 ) ) ) ) ) * nodeVar217 ) * vec3<f32>( nodeConst23 ) ) / vec3<f32>( nodeConst26 ) ) * vec3<f32>( misPowerHeuristic( nodeConst26, max( ( ( nodeVar205 * ( ( 2.0 * nodeVar170 ) / ( nodeVar170 + sqrt( ( nodeVar218 + ( ( 1.0 - nodeVar218 ) * nodeVar219 ) ) ) ) ) ) / max( 0.000001, ( 4.0 * nodeVar170 ) ) ), 1e-8 ) ) ) ) );
						

					}

					

				}

				nodeVar167 = ( nodeVar197 * vec3<f32>( object.nodeUniform25 ) );
				nodeVar220 = mix( ( nodeVar167 * vec3<f32>( object.nodeUniform26 ) ), nodeVar162.xyz, nodeConst16 );
				nodeVar162.x = nodeVar220[ 0 ];
				nodeVar162.y = nodeVar220[ 1 ];
				nodeVar162.z = nodeVar220[ 2 ];
				

			}

			nodeVar134 = 1.0;
			nodeVar133 = vec4<f32>( ( nodeVar162.xyz * nodeVar118 ), nodeVar160 );
			

		}

		

	}


	if ( ( nodeVar134 == 0.0 ) ) {

		nodeVar221 = vec3<f32>( 0.0, 0.0, 0.0 );
		nodeVar222 = normalize( ( object.nodeUniform2 * vec4<f32>( nodeVar7, 0.0 ) ).xyz );
		nodeVar223 = normalize( ( object.nodeUniform2 * vec4<f32>( nodeVar9, 0.0 ) ).xyz );
		nodeVar224 = max( 0.0, dot( nodeVar222, nodeVar223 ) );
		nodeVar225 = normalize( ( object.nodeUniform2 * vec4<f32>( nodeVar117, 0.0 ) ).xyz );
		nodeVar226 = normalize( ( nodeVar223 + nodeVar225 ) );
		nodeVar227 = max( 0.0, dot( nodeVar222, nodeVar225 ) );
		nodeVar228 = max( 0.0, dot( nodeVar222, nodeVar226 ) );
		nodeVar229 = max( 0.0, dot( nodeVar223, nodeVar226 ) );
		nodeVar231 = textureDimensions( nodeUniform18, u32( 0.0 ) );
		nodeVar230 = textureLoad( nodeUniform18, vec2<u32>( clamp( floor( tsl_coord_repeatS_clampT_2d( vec2<f32>( ( ( atan2( nodeVar225.z, nodeVar225.x ) * 0.15915494309189535 ) + 0.5 ), ( ( asin( clamp( nodeVar225.y, -1.0, 1.0 ) ) * 0.3183098861837907 ) + 0.5 ) ) ) * vec2<f32>( nodeVar231 ) ), vec2<f32>( 0 ), vec2<f32>( nodeVar231 - vec2<u32>( 1, 1 ) ) ) ), u32( 0.0 ) );
		nodeVar232 = ( 1.0 - nodeVar229 );
		nodeVar233 = ( nodeVar232 * nodeVar232 );
		nodeVar234 = ( ( nodeVar233 * nodeVar233 ) * nodeVar232 );
		nodeVar235 = ( nodeVar66.f0 + ( ( vec3<f32>( 1.0, 1.0, 1.0 ) - nodeVar66.f0 ) * vec3<f32>( nodeVar234 ) ) );
		nodeVar236 = ( nodeVar66.alpha * nodeVar66.alpha );
		nodeVar237 = ( nodeVar224 * nodeVar224 );
		nodeVar238 = ( ( 2.0 * nodeVar224 ) / ( nodeVar224 + sqrt( ( nodeVar236 + ( ( 1.0 - nodeVar236 ) * nodeVar237 ) ) ) ) );
		nodeVar239 = ( nodeVar66.alpha * nodeVar66.alpha );
		nodeVar240 = ( nodeVar227 * nodeVar227 );
		nodeVar241 = ( ( 2.0 * nodeVar227 ) / ( nodeVar227 + sqrt( ( nodeVar239 + ( ( 1.0 - nodeVar239 ) * nodeVar240 ) ) ) ) );
		nodeVar242 = ( nodeVar238 * nodeVar241 );
		nodeVar243 = ( nodeVar66.alpha * nodeVar66.alpha );
		nodeVar244 = ( nodeVar224 * nodeVar224 );
		nodeVar245 = ( nodeVar66.alpha * nodeVar66.alpha );
		nodeVar246 = ( nodeVar228 * nodeVar228 );
		nodeVar247 = ( ( nodeVar246 * ( nodeVar245 - 1.0 ) ) + 1.0 );
		nodeVar248 = ( nodeVar245 / ( 3.141592653589793 * pow( nodeVar247, 2.0 ) ) );
		nodeVar249 = ( nodeVar66.alpha * nodeVar66.alpha );
		nodeVar250 = ( nodeVar224 * nodeVar224 );
		nodeVar251 = ( ( nodeVar230.xyz * ( ( nodeVar235 * vec3<f32>( nodeVar242 ) ) / vec3<f32>( max( ( ( 2.0 * nodeVar224 ) / ( nodeVar224 + sqrt( ( nodeVar243 + ( ( 1.0 - nodeVar243 ) * nodeVar244 ) ) ) ) ), 0.0001 ) ) ) ) * vec3<f32>( misPowerHeuristic( max( ( ( nodeVar248 * ( ( 2.0 * nodeVar224 ) / ( nodeVar224 + sqrt( ( nodeVar249 + ( ( 1.0 - nodeVar249 ) * nodeVar250 ) ) ) ) ) ) / max( 0.000001, ( 4.0 * nodeVar224 ) ) ), 1e-8 ), max( ( ( ( object.nodeUniform19.x * object.nodeUniform19.y ) * ( dot( nodeVar230.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) ) / object.nodeUniform20 ) ) * equirectDirPdf( nodeVar225 ) ), 1e-8 ) ) ) );

		if ( ( nodeVar66.alpha > 0.01 ) ) {

			nodeVar252 = vec4<f32>( 0.0, 0.0, 0.0, 1.0 );
			let nodeConst27 = ( i32( object.nodeUniform5 ) + 59 );
			let nodeConst28 = tsl_mod_vec2( ( floor( ( nodeVar0 * object.nodeUniform4 ) ) + floor( ( fract( vec2<f32>( ( f32( nodeConst27 ) * 0.7548776662 ), ( f32( nodeConst27 ) * 0.569840291 ) ) ) * vec2<f32>( 32.0 ) ) ) ), vec2<f32>( 32.0 ) );
			let nodeConst29 = ( ( ( nodeConst28.x * 0.7548776662466927 ) + ( nodeConst28.y * 0.5698402909980532 ) ) + 59.0 );
			nodeVar252 = vec4<f32>( fract( ( ( nodeConst29 * 1.324717957244746 ) * 0.7548776662466927 ) ), fract( ( ( nodeConst29 * 2.649435914489492 ) * 0.5698402909980532 ) ), fract( ( ( nodeConst29 * 3.974153871734238 ) * 0.419875421 ) ), fract( ( ( nodeConst29 * 5.298871828978984 ) * 0.43015970900194667 ) ) );
			let nodeConst30 = vec2<f32>( nodeVar252.z, nodeVar252.w );
			nodeVar253 = textureSample( nodeUniform23, nodeUniform23_sampler, ( object.nodeUniform24 * vec3<f32>( vec2<f32>( nodeConst30.x, 0.0 ), 1.0 ) ).xy ).x;
			nodeVar254 = textureSample( nodeUniform21, nodeUniform21_sampler, ( object.nodeUniform22 * vec3<f32>( vec2<f32>( nodeConst30.y, nodeVar253 ), 1.0 ) ).xy ).x;
			let nodeConst31 = vec2<f32>( nodeVar254, nodeVar253 );
			let nodeConst32 = vec2<f32>( ( ( atan2( vec3<f32>( nodeConst31, 0.0 ).z, nodeConst31.x ) * 0.15915494309189535 ) + 0.5 ), ( ( asin( clamp( nodeConst31.y, -1.0, 1.0 ) ) * 0.3183098861837907 ) + 0.5 ) );
			let nodeConst33 = max( 0.0, dot( nodeVar222, vec3<f32>( nodeConst32, 0.0 ) ) );

			if ( ( nodeConst33 > 0.001 ) ) {

				nodeVar255 = ( nodeVar66.alpha * nodeVar66.alpha );
				let nodeConst34 = normalize( ( nodeVar223 + vec3<f32>( nodeConst32, 0.0 ) ) );
				let nodeConst35 = max( 0.0, dot( nodeVar222, nodeConst34 ) );
				nodeVar256 = ( nodeConst35 * nodeConst35 );
				nodeVar257 = ( ( nodeVar256 * ( nodeVar255 - 1.0 ) ) + 1.0 );
				nodeVar258 = ( nodeVar255 / ( 3.141592653589793 * pow( nodeVar257, 2.0 ) ) );
				nodeVar259 = nodeVar258;
				nodeVar260 = textureLoad( nodeUniform18, vec2<u32>( clamp( floor( tsl_coord_repeatS_clampT_2d( nodeConst31 ) * vec2<f32>( nodeVar231 ) ), vec2<f32>( 0 ), vec2<f32>( nodeVar231 - vec2<u32>( 1, 1 ) ) ) ), u32( 0.0 ) );
				nodeVar261 = ( nodeVar66.alpha * nodeVar66.alpha );
				nodeVar262 = ( nodeVar224 * nodeVar224 );
				nodeVar263 = ( ( 2.0 * nodeVar224 ) / ( nodeVar224 + sqrt( ( nodeVar261 + ( ( 1.0 - nodeVar261 ) * nodeVar262 ) ) ) ) );
				nodeVar264 = ( nodeVar66.alpha * nodeVar66.alpha );
				nodeVar265 = ( nodeConst33 * nodeConst33 );
				nodeVar266 = ( ( 2.0 * nodeConst33 ) / ( nodeConst33 + sqrt( ( nodeVar264 + ( ( 1.0 - nodeVar264 ) * nodeVar265 ) ) ) ) );
				nodeVar267 = ( nodeVar263 * nodeVar266 );
				nodeVar268 = ( 1.0 - max( 0.0, dot( nodeVar223, nodeConst34 ) ) );
				nodeVar269 = ( nodeVar268 * nodeVar268 );
				nodeVar270 = ( ( nodeVar269 * nodeVar269 ) * nodeVar268 );
				nodeVar271 = ( nodeVar66.f0 + ( ( vec3<f32>( 1.0, 1.0, 1.0 ) - nodeVar66.f0 ) * vec3<f32>( nodeVar270 ) ) );
				let nodeConst36 = max( ( ( ( object.nodeUniform19.x * object.nodeUniform19.y ) * ( dot( nodeVar260.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) ) / object.nodeUniform20 ) ) * equirectDirPdf( vec3<f32>( nodeConst32, 0.0 ) ) ), 1e-8 );
				nodeVar272 = ( nodeVar66.alpha * nodeVar66.alpha );
				nodeVar273 = ( nodeVar224 * nodeVar224 );
				nodeVar251 = ( nodeVar251 + ( ( ( ( ( nodeVar260.xyz * vec3<f32>( ( ( nodeVar259 * nodeVar267 ) / max( 0.000001, ( ( 4.0 * nodeConst33 ) * nodeVar224 ) ) ) ) ) * nodeVar271 ) * vec3<f32>( nodeConst33 ) ) / vec3<f32>( nodeConst36 ) ) * vec3<f32>( misPowerHeuristic( nodeConst36, max( ( ( nodeVar259 * ( ( 2.0 * nodeVar224 ) / ( nodeVar224 + sqrt( ( nodeVar272 + ( ( 1.0 - nodeVar272 ) * nodeVar273 ) ) ) ) ) ) / max( 0.000001, ( 4.0 * nodeVar224 ) ) ), 1e-8 ) ) ) ) );
				

			}

			

		}

		nodeVar221 = ( nodeVar251 * vec3<f32>( object.nodeUniform25 ) );
		nodeVar133 = vec4<f32>( ( nodeVar221 * vec3<f32>( object.nodeUniform26 ) ), 10000.0 );
		

	}

	nodeVar274 = max( dot( nodeVar133.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) ), 0.0001 );
	nodeVar275 = ( nodeVar133.xyz * vec3<f32>( min( ( object.nodeUniform27 / nodeVar274 ), 1.0 ) ) );
	nodeVar133.x = nodeVar275[ 0 ];
	nodeVar133.y = nodeVar275[ 1 ];
	nodeVar133.z = nodeVar275[ 2 ];
	nodeVar276 = ( nodeVar133.xyz * vec3<f32>( object.nodeUniform28 ) );
	nodeVar133.x = nodeVar276[ 0 ];
	nodeVar133.y = nodeVar276[ 1 ];
	nodeVar133.z = nodeVar276[ 2 ];

	// result

	output.color = max( nodeVar133, vec4<f32>( 0.0 ) );

	return output;

}
