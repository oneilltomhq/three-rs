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
@binding( 1 ) @group( 0 ) var nodeUniform1 : texture_2d<f32>;
@binding( 2 ) @group( 0 ) var nodeUniform4_sampler : sampler;
@binding( 3 ) @group( 0 ) var nodeUniform4 : texture_2d<f32>;
@binding( 4 ) @group( 0 ) var nodeUniform6_sampler : sampler;
@binding( 5 ) @group( 0 ) var nodeUniform6 : texture_2d<f32>;

struct objectStruct {
	nodeUniform0 : mat4x4<f32>,
	nodeUniform2 : vec2<f32>,
	nodeUniform3 : f32,
	nodeUniform5 : mat3x3<f32>,
	nodeUniform7 : mat3x3<f32>,
	nodeUniform8 : f32
};
@binding( 0 ) @group( 0 )
var<uniform> object : objectStruct;

// vars
var<private> nodeVar0 : vec3<f32>;
var<private> nodeVar1 : vec3<f32>;
var<private> nodeVar2 : f32;
var<private> nodeVar3 : vec3<f32>;
var<private> nodeVar4 : vec3<f32>;
var<private> nodeVar5 : f32;
var<private> nodeVar6 : f32;
var<private> nodeVar7 : f32;
var<private> nodeVar8 : vec4<f32>;
var<private> nodeVar9 : vec2<u32>;
var<private> nodeVar10 : f32;
var<private> nodeVar11 : f32;
var<private> nodeVar12 : f32;
var<private> nodeVar13 : vec3<f32>;
var<private> nodeVar14 : f32;
var<private> nodeVar15 : f32;
var<private> nodeVar16 : f32;
var<private> nodeVar17 : f32;
var<private> nodeVar18 : f32;
var<private> nodeVar19 : f32;
var<private> nodeVar20 : f32;
var<private> nodeVar21 : f32;
var<private> nodeVar22 : f32;
var<private> nodeVar23 : f32;
var<private> nodeVar24 : f32;
var<private> nodeVar25 : f32;
var<private> nodeVar26 : f32;
var<private> nodeVar27 : f32;
var<private> nodeVar28 : f32;
var<private> nodeVar29 : vec3<f32>;
var<private> nodeVar30 : f32;
var<private> nodeVar31 : f32;
var<private> nodeVar32 : f32;
var<private> nodeVar33 : f32;
var<private> nodeVar34 : f32;
var<private> nodeVar35 : f32;
var<private> nodeVar36 : f32;
var<private> nodeVar37 : vec4<f32>;
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
var<private> nodeVar48 : vec3<f32>;
var<private> nodeVar49 : f32;
var<private> nodeVar50 : f32;

// codes
fn tsl_repeatWrapping_float( coord: f32 ) -> f32 { return fract( coord ); }
fn tsl_clampWrapping_float( coord: f32 ) -> f32 { return clamp( coord, 0.0, 1.0 ); }
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

	nodeVar0 = normalize( ( object.nodeUniform0 * vec4<f32>( normalize( vec3<f32>( ( nodeVarying0.yx - vec2<f32>( 0.5 ) ), 1.0 ) ), 0.0 ) ).xyz );
	nodeVar1 = normalize( ( object.nodeUniform0 * vec4<f32>( vec3<f32>( 0.0, 0.0, 1.0 ), 0.0 ) ).xyz );
	nodeVar2 = max( 0.0, dot( nodeVar0, nodeVar1 ) );
	nodeVar3 = normalize( ( object.nodeUniform0 * vec4<f32>( normalize( vec3<f32>( ( nodeVarying0 - vec2<f32>( 0.5 ) ), -1.0 ) ), 0.0 ) ).xyz );
	nodeVar4 = normalize( ( nodeVar1 + nodeVar3 ) );
	nodeVar5 = max( 0.0, dot( nodeVar0, nodeVar3 ) );
	nodeVar6 = max( 0.0, dot( nodeVar0, nodeVar4 ) );
	nodeVar7 = max( 0.0, dot( nodeVar1, nodeVar4 ) );
	nodeVar9 = textureDimensions( nodeUniform1, u32( 0.0 ) );
	nodeVar8 = textureLoad( nodeUniform1, vec2<u32>( clamp( floor( tsl_coord_repeatS_clampT_2d( vec2<f32>( ( ( atan2( nodeVar3.z, nodeVar3.x ) * 0.15915494309189535 ) + 0.5 ), ( ( asin( clamp( nodeVar3.y, -1.0, 1.0 ) ) * 0.3183098861837907 ) + 0.5 ) ) ) * vec2<f32>( nodeVar9 ) ), vec2<f32>( 0 ), vec2<f32>( nodeVar9 - vec2<u32>( 1, 1 ) ) ) ), u32( 0.0 ) );
	nodeVar10 = ( 1.0 - nodeVar7 );
	nodeVar11 = ( nodeVar10 * nodeVar10 );
	nodeVar12 = ( ( nodeVar11 * nodeVar11 ) * nodeVar10 );
	nodeVar13 = ( vec3<f32>( 0.04, 0.04, 0.04 ) + ( ( vec3<f32>( 1.0, 1.0, 1.0 ) - vec3<f32>( 0.04, 0.04, 0.04 ) ) * vec3<f32>( nodeVar12 ) ) );
	let nodeConst0 = ( nodeVarying0.x * nodeVarying0.x );
	nodeVar14 = ( nodeConst0 * nodeConst0 );
	nodeVar15 = ( nodeVar2 * nodeVar2 );
	nodeVar16 = ( ( 2.0 * nodeVar2 ) / ( nodeVar2 + sqrt( ( nodeVar14 + ( ( 1.0 - nodeVar14 ) * nodeVar15 ) ) ) ) );
	nodeVar17 = ( nodeConst0 * nodeConst0 );
	nodeVar18 = ( nodeVar5 * nodeVar5 );
	nodeVar19 = ( ( 2.0 * nodeVar5 ) / ( nodeVar5 + sqrt( ( nodeVar17 + ( ( 1.0 - nodeVar17 ) * nodeVar18 ) ) ) ) );
	nodeVar20 = ( nodeVar16 * nodeVar19 );
	nodeVar21 = ( nodeConst0 * nodeConst0 );
	nodeVar22 = ( nodeVar2 * nodeVar2 );
	nodeVar23 = ( nodeConst0 * nodeConst0 );
	nodeVar24 = ( nodeVar6 * nodeVar6 );
	nodeVar25 = ( ( nodeVar24 * ( nodeVar23 - 1.0 ) ) + 1.0 );
	nodeVar26 = ( nodeVar23 / ( 3.141592653589793 * pow( nodeVar25, 2.0 ) ) );
	nodeVar27 = ( nodeConst0 * nodeConst0 );
	nodeVar28 = ( nodeVar2 * nodeVar2 );
	nodeVar29 = ( ( nodeVar8.xyz * ( ( nodeVar13 * vec3<f32>( nodeVar20 ) ) / vec3<f32>( max( ( ( 2.0 * nodeVar2 ) / ( nodeVar2 + sqrt( ( nodeVar21 + ( ( 1.0 - nodeVar21 ) * nodeVar22 ) ) ) ) ), 0.0001 ) ) ) ) * vec3<f32>( misPowerHeuristic( max( ( ( nodeVar26 * ( ( 2.0 * nodeVar2 ) / ( nodeVar2 + sqrt( ( nodeVar27 + ( ( 1.0 - nodeVar27 ) * nodeVar28 ) ) ) ) ) ) / max( 0.000001, ( 4.0 * nodeVar2 ) ) ), 1e-8 ), max( ( ( ( object.nodeUniform2.x * object.nodeUniform2.y ) * ( dot( nodeVar8.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) ) / object.nodeUniform3 ) ) * equirectDirPdf( nodeVar3 ) ), 1e-8 ) ) ) );

	if ( ( nodeConst0 > 0.01 ) ) {

		let nodeConst1 = vec4<f32>( nodeVarying0, nodeVarying0.yx );
		let nodeConst2 = vec2<f32>( nodeConst1.z, nodeConst1.w );
		nodeVar30 = textureSample( nodeUniform6, nodeUniform6_sampler, ( object.nodeUniform7 * vec3<f32>( vec2<f32>( nodeConst2.x, 0.0 ), 1.0 ) ).xy ).x;
		nodeVar31 = textureSample( nodeUniform4, nodeUniform4_sampler, ( object.nodeUniform5 * vec3<f32>( vec2<f32>( nodeConst2.y, nodeVar30 ), 1.0 ) ).xy ).x;
		let nodeConst3 = vec2<f32>( nodeVar31, nodeVar30 );
		let nodeConst4 = vec2<f32>( ( ( atan2( vec3<f32>( nodeConst3, 0.0 ).z, nodeConst3.x ) * 0.15915494309189535 ) + 0.5 ), ( ( asin( clamp( nodeConst3.y, -1.0, 1.0 ) ) * 0.3183098861837907 ) + 0.5 ) );
		let nodeConst5 = max( 0.0, dot( nodeVar0, vec3<f32>( nodeConst4, 0.0 ) ) );

		if ( ( nodeConst5 > 0.001 ) ) {

			nodeVar32 = ( nodeConst0 * nodeConst0 );
			let nodeConst6 = normalize( ( nodeVar1 + vec3<f32>( nodeConst4, 0.0 ) ) );
			let nodeConst7 = max( 0.0, dot( nodeVar0, nodeConst6 ) );
			nodeVar33 = ( nodeConst7 * nodeConst7 );
			nodeVar34 = ( ( nodeVar33 * ( nodeVar32 - 1.0 ) ) + 1.0 );
			nodeVar35 = ( nodeVar32 / ( 3.141592653589793 * pow( nodeVar34, 2.0 ) ) );
			nodeVar36 = nodeVar35;
			nodeVar37 = textureLoad( nodeUniform1, vec2<u32>( clamp( floor( tsl_coord_repeatS_clampT_2d( nodeConst3 ) * vec2<f32>( nodeVar9 ) ), vec2<f32>( 0 ), vec2<f32>( nodeVar9 - vec2<u32>( 1, 1 ) ) ) ), u32( 0.0 ) );
			nodeVar38 = ( nodeConst0 * nodeConst0 );
			nodeVar39 = ( nodeVar2 * nodeVar2 );
			nodeVar40 = ( ( 2.0 * nodeVar2 ) / ( nodeVar2 + sqrt( ( nodeVar38 + ( ( 1.0 - nodeVar38 ) * nodeVar39 ) ) ) ) );
			nodeVar41 = ( nodeConst0 * nodeConst0 );
			nodeVar42 = ( nodeConst5 * nodeConst5 );
			nodeVar43 = ( ( 2.0 * nodeConst5 ) / ( nodeConst5 + sqrt( ( nodeVar41 + ( ( 1.0 - nodeVar41 ) * nodeVar42 ) ) ) ) );
			nodeVar44 = ( nodeVar40 * nodeVar43 );
			nodeVar45 = ( 1.0 - max( 0.0, dot( nodeVar1, nodeConst6 ) ) );
			nodeVar46 = ( nodeVar45 * nodeVar45 );
			nodeVar47 = ( ( nodeVar46 * nodeVar46 ) * nodeVar45 );
			nodeVar48 = ( vec3<f32>( 0.04, 0.04, 0.04 ) + ( ( vec3<f32>( 1.0, 1.0, 1.0 ) - vec3<f32>( 0.04, 0.04, 0.04 ) ) * vec3<f32>( nodeVar47 ) ) );
			let nodeConst8 = max( ( ( ( object.nodeUniform2.x * object.nodeUniform2.y ) * ( dot( nodeVar37.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) ) / object.nodeUniform3 ) ) * equirectDirPdf( vec3<f32>( nodeConst4, 0.0 ) ) ), 1e-8 );
			nodeVar49 = ( nodeConst0 * nodeConst0 );
			nodeVar50 = ( nodeVar2 * nodeVar2 );
			nodeVar29 = ( nodeVar29 + ( ( ( ( ( nodeVar37.xyz * vec3<f32>( ( ( nodeVar36 * nodeVar44 ) / max( 0.000001, ( ( 4.0 * nodeConst5 ) * nodeVar2 ) ) ) ) ) * nodeVar48 ) * vec3<f32>( nodeConst5 ) ) / vec3<f32>( nodeConst8 ) ) * vec3<f32>( misPowerHeuristic( nodeConst8, max( ( ( nodeVar36 * ( ( 2.0 * nodeVar2 ) / ( nodeVar2 + sqrt( ( nodeVar49 + ( ( 1.0 - nodeVar49 ) * nodeVar50 ) ) ) ) ) ) / max( 0.000001, ( 4.0 * nodeVar2 ) ) ), 1e-8 ) ) ) ) );
			

		}

		

	}


	// result

	output.color = vec4<f32>( ( nodeVar29 * vec3<f32>( object.nodeUniform8 ) ), 1.0 );

	return output;

}
