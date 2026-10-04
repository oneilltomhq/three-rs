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
@binding( 0 ) @group( 0 ) var nodeUniform0 : texture_depth_2d;
@binding( 2 ) @group( 0 ) var nodeUniform2_sampler : sampler;
@binding( 3 ) @group( 0 ) var nodeUniform2 : texture_2d<f32>;
@binding( 4 ) @group( 0 ) var nodeUniform3 : texture_2d<f32>;

struct NodeBuffer_923Struct {
	value : array< vec4<f32>, 16 >
};
@binding( 5 ) @group( 0 )
var<uniform> NodeBuffer_923 : NodeBuffer_923Struct;

struct objectStruct {
	nodeUniform1 : mat4x4<f32>,
	nodeUniform4 : mat3x3<f32>,
	nodeUniform5 : vec2<f32>,
	nodeUniform6 : f32,
	nodeUniform8 : f32,
	nodeUniform9 : f32,
	nodeUniform10 : f32,
	nodeUniform11 : f32
};
@binding( 1 ) @group( 0 )
var<uniform> object : objectStruct;

// vars
var<private> nodeVar0 : f32;
var<private> nodeVar1 : vec2<u32>;
var<private> nodeVar2 : f32;
var<private> nodeVar3 : vec2<i32>;
var<private> nodeVar4 : f32;
var<private> nodeVar5 : f32;
var<private> nodeVar6 : f32;
var<private> nodeVar7 : f32;
var<private> nodeVar8 : f32;
var<private> nodeVar9 : f32;
var<private> nodeVar10 : f32;
var<private> nodeVar11 : f32;
var<private> nodeVar12 : f32;
var<private> nodeVar13 : f32;
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
var<private> nodeVar26 : vec3<f32>;
var<private> nodeVar27 : vec3<f32>;
var<private> nodeVar28 : vec3<f32>;
var<private> nodeVar29 : vec3<f32>;
var<private> nodeVar30 : vec4<f32>;
var<private> nodeVar31 : vec4<f32>;
var<private> nodeVar32 : vec4<f32>;
var<private> nodeVar33 : vec4<f32>;
var<private> nodeVar34 : vec2<u32>;
var<private> nodeVar35 : vec4<f32>;
var<private> nodeVar36 : f32;
var<private> nodeVar37 : vec3<f32>;
var<private> nodeVar38 : vec4<f32>;
var<private> nodeVar39 : vec4<f32>;
var<private> nodeVar40 : f32;
var<private> nodeVar41 : f32;
var<private> nodeVar42 : vec2<i32>;
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
var<private> nodeVar64 : f32;
var<private> nodeVar65 : vec3<f32>;
var<private> nodeVar66 : vec3<f32>;
var<private> nodeVar67 : vec3<f32>;
var<private> nodeVar68 : vec3<f32>;
var<private> nodeVar69 : vec3<f32>;
var<private> nodeVar70 : f32;
var<private> nodeVar71 : f32;
var<private> nodeVar72 : f32;
var<private> nodeVar73 : f32;
var<private> nodeVar74 : f32;

// codes
fn tsl_clampWrapping_float( coord: f32 ) -> f32 { return clamp( coord, 0.0, 1.0 ); }
fn tsl_coord_clampS_clampT_2d( coord : vec2f ) -> vec2f {

	return vec2f(
		tsl_clampWrapping_float( coord.x ),
		tsl_clampWrapping_float( coord.y )
	);

}

fn tsl_repeatWrapping_float( coord: f32 ) -> f32 { return fract( coord ); }
fn tsl_coord_repeatS_repeatT_2d( coord : vec2f ) -> vec2f {

	return vec2f(
		tsl_repeatWrapping_float( coord.x ),
		tsl_repeatWrapping_float( coord.y )
	);

}

fn tsl_mod_float( x : f32, y : f32 ) -> f32 { return x - y * floor( x / y ); }


@fragment
fn main( @location( 0 ) nodeVarying0 : vec2<f32> ) -> OutputStruct {

	// flow
	// code

	nodeVar1 = textureDimensions( nodeUniform0, u32( 0 ) );
	nodeVar0 = textureLoad( nodeUniform0, vec2<u32>( clamp( floor( tsl_coord_clampS_clampT_2d( nodeVarying0 ) * vec2<f32>( nodeVar1 ) ), vec2<f32>( 0 ), vec2<f32>( nodeVar1 - vec2<u32>( 1, 1 ) ) ) ), u32( 0 ) );
	nodeVar2 = nodeVar0;
	nodeVar3 = vec2<i32>( ( nodeVarying0 * vec2<f32>( textureDimensions( nodeUniform0, 0 ) ) ) );
	nodeVar4 = textureLoad( nodeUniform0, nodeVar3, u32( 0u ) );
	nodeVar5 = nodeVar4;
	nodeVar6 = textureLoad( nodeUniform0, ( nodeVar3 - vec2<i32>( 2, 0 ) ), u32( 0u ) );
	nodeVar7 = nodeVar6;
	nodeVar8 = textureLoad( nodeUniform0, ( nodeVar3 - vec2<i32>( 1, 0 ) ), u32( 0u ) );
	nodeVar9 = nodeVar8;
	nodeVar10 = textureLoad( nodeUniform0, ( nodeVar3 + vec2<i32>( 1, 0 ) ), u32( 0u ) );
	nodeVar11 = nodeVar10;
	nodeVar12 = textureLoad( nodeUniform0, ( nodeVar3 + vec2<i32>( 2, 0 ) ), u32( 0u ) );
	nodeVar13 = nodeVar12;
	nodeVar14 = textureLoad( nodeUniform0, ( nodeVar3 + vec2<i32>( 0, 2 ) ), u32( 0u ) );
	nodeVar15 = nodeVar14;
	nodeVar16 = textureLoad( nodeUniform0, ( nodeVar3 + vec2<i32>( 0, 1 ) ), u32( 0u ) );
	nodeVar17 = nodeVar16;
	nodeVar18 = textureLoad( nodeUniform0, ( nodeVar3 - vec2<i32>( 0, 1 ) ), u32( 0u ) );
	nodeVar19 = nodeVar18;
	nodeVar20 = textureLoad( nodeUniform0, ( nodeVar3 - vec2<i32>( 0, 2 ) ), u32( 0u ) );
	nodeVar21 = nodeVar20;
	nodeVar22 = abs( ( ( ( 2.0 * nodeVar9 ) - nodeVar7 ) - nodeVar5 ) );
	nodeVar23 = abs( ( ( ( 2.0 * nodeVar11 ) - nodeVar13 ) - nodeVar5 ) );
	nodeVar24 = abs( ( ( ( 2.0 * nodeVar17 ) - nodeVar15 ) - nodeVar5 ) );
	nodeVar25 = abs( ( ( ( 2.0 * nodeVar19 ) - nodeVar21 ) - nodeVar5 ) );
	let nodeConst0 = ( object.nodeUniform1 * vec4<f32>( vec3<f32>( ( ( vec2<f32>( nodeVarying0.x, ( 1.0 - nodeVarying0.y ) ) * vec2<f32>( 2.0 ) ) - vec2<f32>( 1.0 ) ), nodeVar5 ), 1.0 ) );
	nodeVar26 = ( nodeConst0.xyz / vec3<f32>( nodeConst0.w ) );

	if ( ( nodeVar22 < nodeVar23 ) ) {

		let nodeConst1 = ( nodeVarying0 - vec2<f32>( ( 1.0 / f32( textureDimensions( nodeUniform0, 0 ).x ) ), 0.0 ) );
		let nodeConst2 = ( object.nodeUniform1 * vec4<f32>( vec3<f32>( ( ( vec2<f32>( nodeConst1.x, ( 1.0 - nodeConst1.y ) ) * vec2<f32>( 2.0 ) ) - vec2<f32>( 1.0 ) ), nodeVar9 ), 1.0 ) );
		nodeVar27 = ( nodeVar26 - ( nodeConst2.xyz / vec3<f32>( nodeConst2.w ) ) );

	} else {

		let nodeConst3 = ( nodeVarying0 + vec2<f32>( ( 1.0 / f32( textureDimensions( nodeUniform0, 0 ).x ) ), 0.0 ) );
		let nodeConst4 = ( object.nodeUniform1 * vec4<f32>( vec3<f32>( ( ( vec2<f32>( nodeConst3.x, ( 1.0 - nodeConst3.y ) ) * vec2<f32>( 2.0 ) ) - vec2<f32>( 1.0 ) ), nodeVar11 ), 1.0 ) );
		nodeVar27 = ( ( - nodeVar26 ) + ( nodeConst4.xyz / vec3<f32>( nodeConst4.w ) ) );

	}


	if ( ( nodeVar24 < nodeVar25 ) ) {

		let nodeConst5 = ( nodeVarying0 + vec2<f32>( 0.0, ( 1.0 / f32( textureDimensions( nodeUniform0, 0 ).y ) ) ) );
		let nodeConst6 = ( object.nodeUniform1 * vec4<f32>( vec3<f32>( ( ( vec2<f32>( nodeConst5.x, ( 1.0 - nodeConst5.y ) ) * vec2<f32>( 2.0 ) ) - vec2<f32>( 1.0 ) ), nodeVar17 ), 1.0 ) );
		nodeVar28 = ( nodeVar26 - ( nodeConst6.xyz / vec3<f32>( nodeConst6.w ) ) );

	} else {

		let nodeConst7 = ( nodeVarying0 - vec2<f32>( 0.0, ( 1.0 / f32( textureDimensions( nodeUniform0, 0 ).y ) ) ) );
		let nodeConst8 = ( object.nodeUniform1 * vec4<f32>( vec3<f32>( ( ( vec2<f32>( nodeConst7.x, ( 1.0 - nodeConst7.y ) ) * vec2<f32>( 2.0 ) ) - vec2<f32>( 1.0 ) ), nodeVar19 ), 1.0 ) );
		nodeVar28 = ( ( - nodeVar26 ) + ( nodeConst8.xyz / vec3<f32>( nodeConst8.w ) ) );

	}

	nodeVar29 = normalize( cross( nodeVar27, nodeVar28 ) );
	nodeVar30 = textureSample( nodeUniform2, nodeUniform2_sampler, nodeVarying0 );
	nodeVar31 = nodeVar30;

	if ( ( ( nodeVar2 >= 1.0 ) || ( dot( nodeVar29, nodeVar29 ) == 0.0 ) ) ) {

		nodeVar32 = nodeVar31;
		

	} else {

		let nodeConst9 = ( object.nodeUniform1 * vec4<f32>( vec3<f32>( ( ( vec2<f32>( nodeVarying0.x, ( 1.0 - nodeVarying0.y ) ) * vec2<f32>( 2.0 ) ) - vec2<f32>( 1.0 ) ), nodeVar2 ), 1.0 ) );
		let nodeConst10 = ( nodeConst9.xyz / vec3<f32>( nodeConst9.w ) );
		nodeVar34 = textureDimensions( nodeUniform3, u32( 0 ) );
		nodeVar33 = textureLoad( nodeUniform3, vec2<u32>( clamp( floor( tsl_coord_repeatS_repeatT_2d( ( object.nodeUniform4 * vec3<f32>( ( vec2<f32>( nodeVarying0.x, ( 1.0 - nodeVarying0.y ) ) * ( object.nodeUniform5 / vec2<f32>( textureDimensions( nodeUniform3, 0 ) ) ) ), 1.0 ) ).xy ) * vec2<f32>( nodeVar34 ) ), vec2<f32>( 0 ), vec2<f32>( nodeVar34 - vec2<u32>( 1, 1 ) ) ) ), u32( 0 ) );
		nodeVar35 = nodeVar33;
		nodeVar36 = 1.0;
		nodeVar37 = nodeVar31.xyz;

		for ( var i : i32 = 0; i < 16; i ++ ) {

			let nodeConst11 = vec2<f32>( sin( nodeVar35[ u32( ( ( tsl_mod_float( object.nodeUniform6, 4.0 ) * 2.0 ) * 3.141592653589793 ) ) ] ), cos( nodeVar35[ u32( ( ( tsl_mod_float( object.nodeUniform6, 4.0 ) * 2.0 ) * 3.141592653589793 ) ) ] ) );
			let nodeConst12 = ( nodeVarying0 + ( ( mat2x2<f32>( nodeConst11.x, ( - nodeConst11.y ), nodeConst11.x, nodeConst11.y ) * ( NodeBuffer_923.value[ i ].xyz.xy * vec2<f32>( ( 1.0 + ( NodeBuffer_923.value[ i ].xyz.z * ( object.nodeUniform8 - 1.0 ) ) ) ) ) ) / object.nodeUniform5 ) );
			nodeVar38 = textureSample( nodeUniform2, nodeUniform2_sampler, nodeConst12 );
			nodeVar39 = nodeVar38;
			nodeVar40 = textureLoad( nodeUniform0, vec2<u32>( clamp( floor( tsl_coord_clampS_clampT_2d( nodeConst12 ) * vec2<f32>( nodeVar1 ) ), vec2<f32>( 0 ), vec2<f32>( nodeVar1 - vec2<u32>( 1, 1 ) ) ) ), u32( 0 ) );
			nodeVar41 = nodeVar40;
			nodeVar42 = vec2<i32>( ( nodeConst12 * vec2<f32>( textureDimensions( nodeUniform0, 0 ) ) ) );
			nodeVar43 = textureLoad( nodeUniform0, nodeVar42, u32( 0u ) );
			nodeVar44 = nodeVar43;
			nodeVar45 = textureLoad( nodeUniform0, ( nodeVar42 - vec2<i32>( 2, 0 ) ), u32( 0u ) );
			nodeVar46 = nodeVar45;
			nodeVar47 = textureLoad( nodeUniform0, ( nodeVar42 - vec2<i32>( 1, 0 ) ), u32( 0u ) );
			nodeVar48 = nodeVar47;
			nodeVar49 = textureLoad( nodeUniform0, ( nodeVar42 + vec2<i32>( 1, 0 ) ), u32( 0u ) );
			nodeVar50 = nodeVar49;
			nodeVar51 = textureLoad( nodeUniform0, ( nodeVar42 + vec2<i32>( 2, 0 ) ), u32( 0u ) );
			nodeVar52 = nodeVar51;
			nodeVar53 = textureLoad( nodeUniform0, ( nodeVar42 + vec2<i32>( 0, 2 ) ), u32( 0u ) );
			nodeVar54 = nodeVar53;
			nodeVar55 = textureLoad( nodeUniform0, ( nodeVar42 + vec2<i32>( 0, 1 ) ), u32( 0u ) );
			nodeVar56 = nodeVar55;
			nodeVar57 = textureLoad( nodeUniform0, ( nodeVar42 - vec2<i32>( 0, 1 ) ), u32( 0u ) );
			nodeVar58 = nodeVar57;
			nodeVar59 = textureLoad( nodeUniform0, ( nodeVar42 - vec2<i32>( 0, 2 ) ), u32( 0u ) );
			nodeVar60 = nodeVar59;
			nodeVar61 = abs( ( ( ( 2.0 * nodeVar48 ) - nodeVar46 ) - nodeVar44 ) );
			nodeVar62 = abs( ( ( ( 2.0 * nodeVar50 ) - nodeVar52 ) - nodeVar44 ) );
			nodeVar63 = abs( ( ( ( 2.0 * nodeVar56 ) - nodeVar54 ) - nodeVar44 ) );
			nodeVar64 = abs( ( ( ( 2.0 * nodeVar58 ) - nodeVar60 ) - nodeVar44 ) );
			let nodeConst13 = ( object.nodeUniform1 * vec4<f32>( vec3<f32>( ( ( vec2<f32>( nodeConst12.x, ( 1.0 - nodeConst12.y ) ) * vec2<f32>( 2.0 ) ) - vec2<f32>( 1.0 ) ), nodeVar44 ), 1.0 ) );
			nodeVar65 = ( nodeConst13.xyz / vec3<f32>( nodeConst13.w ) );

			if ( ( nodeVar61 < nodeVar62 ) ) {

				let nodeConst14 = ( nodeConst12 - vec2<f32>( ( 1.0 / f32( textureDimensions( nodeUniform0, 0 ).x ) ), 0.0 ) );
				let nodeConst15 = ( object.nodeUniform1 * vec4<f32>( vec3<f32>( ( ( vec2<f32>( nodeConst14.x, ( 1.0 - nodeConst14.y ) ) * vec2<f32>( 2.0 ) ) - vec2<f32>( 1.0 ) ), nodeVar48 ), 1.0 ) );
				nodeVar66 = ( nodeVar65 - ( nodeConst15.xyz / vec3<f32>( nodeConst15.w ) ) );

			} else {

				let nodeConst16 = ( nodeConst12 + vec2<f32>( ( 1.0 / f32( textureDimensions( nodeUniform0, 0 ).x ) ), 0.0 ) );
				let nodeConst17 = ( object.nodeUniform1 * vec4<f32>( vec3<f32>( ( ( vec2<f32>( nodeConst16.x, ( 1.0 - nodeConst16.y ) ) * vec2<f32>( 2.0 ) ) - vec2<f32>( 1.0 ) ), nodeVar50 ), 1.0 ) );
				nodeVar66 = ( ( - nodeVar65 ) + ( nodeConst17.xyz / vec3<f32>( nodeConst17.w ) ) );

			}


			if ( ( nodeVar63 < nodeVar64 ) ) {

				let nodeConst18 = ( nodeConst12 + vec2<f32>( 0.0, ( 1.0 / f32( textureDimensions( nodeUniform0, 0 ).y ) ) ) );
				let nodeConst19 = ( object.nodeUniform1 * vec4<f32>( vec3<f32>( ( ( vec2<f32>( nodeConst18.x, ( 1.0 - nodeConst18.y ) ) * vec2<f32>( 2.0 ) ) - vec2<f32>( 1.0 ) ), nodeVar56 ), 1.0 ) );
				nodeVar67 = ( nodeVar65 - ( nodeConst19.xyz / vec3<f32>( nodeConst19.w ) ) );

			} else {

				let nodeConst20 = ( nodeConst12 - vec2<f32>( 0.0, ( 1.0 / f32( textureDimensions( nodeUniform0, 0 ).y ) ) ) );
				let nodeConst21 = ( object.nodeUniform1 * vec4<f32>( vec3<f32>( ( ( vec2<f32>( nodeConst20.x, ( 1.0 - nodeConst20.y ) ) * vec2<f32>( 2.0 ) ) - vec2<f32>( 1.0 ) ), nodeVar58 ), 1.0 ) );
				nodeVar67 = ( ( - nodeVar65 ) + ( nodeConst21.xyz / vec3<f32>( nodeConst21.w ) ) );

			}

			nodeVar68 = normalize( cross( nodeVar66, nodeVar67 ) );
			let nodeConst22 = ( object.nodeUniform1 * vec4<f32>( vec3<f32>( ( ( vec2<f32>( nodeConst12.x, ( 1.0 - nodeConst12.y ) ) * vec2<f32>( 2.0 ) ) - vec2<f32>( 1.0 ) ), nodeVar41 ), 1.0 ) );
			nodeVar69 = ( nodeConst22.xyz / vec3<f32>( nodeConst22.w ) );
			nodeVar70 = dot( nodeVar29, nodeVar68 );
			nodeVar71 = pow( max( nodeVar70, 0.0 ), object.nodeUniform9 );
			nodeVar72 = abs( ( dot( nodeVar39.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) ) - dot( nodeVar31.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) ) ) );
			nodeVar73 = max( ( 1.0 - ( nodeVar72 / object.nodeUniform10 ) ), 0.0 );
			nodeVar74 = abs( dot( ( nodeConst10 - nodeVar69 ), nodeVar29 ) );
			let nodeConst23 = ( ( nodeVar73 * max( ( 1.0 - ( nodeVar74 / object.nodeUniform11 ) ), 0.0 ) ) * nodeVar71 );
			let nodeConst24 = vec4<f32>( ( nodeVar39.xyz * vec3<f32>( nodeConst23 ) ), nodeConst23 );
			nodeVar37 = ( nodeVar37 + nodeConst24.xyz );
			nodeVar36 = ( nodeVar36 + nodeConst24.w );

		}


		if ( ( nodeVar36 > 0.0 ) ) {

			nodeVar37 = ( nodeVar37 / vec3<f32>( nodeVar36 ) );
			

		}

		nodeVar32 = vec4<f32>( nodeVar37, nodeVar31.w );
		

	}


	// result

	output.color = nodeVar32;

	return output;

}
