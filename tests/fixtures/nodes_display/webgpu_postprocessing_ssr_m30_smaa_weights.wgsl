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
@binding( 0 ) @group( 0 ) var nodeUniform0_sampler : sampler;
@binding( 1 ) @group( 0 ) var nodeUniform0 : texture_2d<f32>;
@binding( 3 ) @group( 0 ) var nodeUniform4 : texture_2d<f32>;
@binding( 4 ) @group( 0 ) var nodeUniform6_sampler : sampler;
@binding( 5 ) @group( 0 ) var nodeUniform6 : texture_2d<f32>;

struct objectStruct {
	nodeUniform1 : mat3x3<f32>,
	nodeUniform2 : vec2<f32>,
	nodeUniform3 : vec2<f32>,
	nodeUniform5 : mat3x3<f32>,
	nodeUniform7 : mat3x3<f32>
};
@binding( 2 ) @group( 0 )
var<uniform> object : objectStruct;

// vars
var<private> nodeVar0 : vec4<f32>;
var<private> nodeVar1 : vec4<f32>;
var<private> nodeVar2 : vec4<f32>;
var<private> nodeVar3 : vec2<f32>;
var<private> nodeVar4 : vec2<f32>;
var<private> nodeVar5 : vec2<f32>;
var<private> nodeVar6 : vec2<f32>;
var<private> nodeVar7 : vec2<f32>;
var<private> nodeVar8 : vec4<f32>;
var<private> nodeVar9 : vec2<f32>;
var<private> nodeVar10 : vec4<f32>;
var<private> nodeVar11 : vec2<u32>;
var<private> nodeVar12 : vec4<f32>;
var<private> nodeVar13 : f32;
var<private> nodeVar14 : vec2<f32>;
var<private> nodeVar15 : vec2<f32>;
var<private> nodeVar16 : vec2<f32>;
var<private> nodeVar17 : vec4<f32>;
var<private> nodeVar18 : vec2<f32>;
var<private> nodeVar19 : vec4<f32>;
var<private> nodeVar20 : vec4<f32>;
var<private> nodeVar21 : f32;
var<private> nodeVar23 : vec4<f32>;
var<private> nodeVar24 : vec2<f32>;
var<private> nodeVar25 : vec2<f32>;
var<private> nodeVar26 : vec2<f32>;
var<private> nodeVar27 : vec2<f32>;
var<private> nodeVar28 : vec2<f32>;
var<private> nodeVar29 : vec4<f32>;
var<private> nodeVar30 : vec2<f32>;
var<private> nodeVar31 : vec4<f32>;
var<private> nodeVar32 : vec2<u32>;
var<private> nodeVar33 : vec4<f32>;
var<private> nodeVar34 : f32;
var<private> nodeVar35 : vec2<f32>;
var<private> nodeVar36 : vec2<f32>;
var<private> nodeVar37 : vec2<f32>;
var<private> nodeVar38 : vec4<f32>;
var<private> nodeVar39 : vec2<f32>;
var<private> nodeVar40 : vec4<f32>;
var<private> nodeVar41 : vec4<f32>;
var<private> nodeVar42 : f32;
var<private> nodeVar44 : vec4<f32>;
var<private> nodeVar45 : vec2<f32>;

// codes
fn tsl_clampWrapping_float( coord: f32 ) -> f32 { return clamp( coord, 0.0, 1.0 ); }
fn tsl_coord_clampS_clampT_2d( coord : vec2f ) -> vec2f {

	return vec2f(
		tsl_clampWrapping_float( coord.x ),
		tsl_clampWrapping_float( coord.y )
	);

}



@fragment
fn main( @location( 0 ) nodeVarying0 : vec4<f32>,
	@location( 1 ) nodeVarying1 : vec4<f32>,
	@location( 2 ) nodeVarying2 : vec4<f32>,
	@location( 3 ) nodeVarying3 : vec2<f32>,
	@location( 4 ) nodeVarying4 : vec2<f32> ) -> OutputStruct {

	// flow
	// code

	nodeVar0 = vec4<f32>( 0.0, 0.0, 0.0, 0.0 );
	nodeVar1 = vec4<f32>( 0.0, 0.0, 0.0, 0.0 );
	nodeVar2 = textureSample( nodeUniform0, nodeUniform0_sampler, ( object.nodeUniform1 * vec3<f32>( nodeVarying4, 1.0 ) ).xy );
	nodeVar3 = nodeVar2.xy;

	if ( ( nodeVar3.y > 0.0 ) ) {

		nodeVar4 = vec2<f32>( 0.0, 0.0 );
		nodeVar5 = vec2<f32>( 0.0, 0.0 );
		nodeVar6 = vec2<f32>( 0.0, 1.0 );
		nodeVar7 = nodeVarying0.xy;

		for ( var i : i32 = 0; i < 8; i ++ ) {

			nodeVar8 = textureSample( nodeUniform0, nodeUniform0_sampler, ( object.nodeUniform1 * vec3<f32>( nodeVar7, 1.0 ) ).xy );
			nodeVar6 = nodeVar8.xy;
			nodeVar7 = ( nodeVar7 - ( vec2<f32>( 2.0, 0.0 ) * object.nodeUniform3 ) );

			if ( ( ( nodeVar7.x <= nodeVarying1.x ) || ( ( nodeVar6.y <= 0.8281 ) || ( nodeVar6.x != 0.0 ) ) ) ) {

				break;
				

			}


		}

		nodeVar7.x = ( nodeVar7.x + ( 0.25 * object.nodeUniform3.x ) );
		nodeVar7.x = ( nodeVar7.x + object.nodeUniform3.x );
		nodeVar7.x = ( nodeVar7.x + ( 2.0 * object.nodeUniform3.x ) );
		nodeVar9 = nodeVar6;
		nodeVar9.x = ( 0.0 + ( nodeVar9.x * 0.5 ) );
		nodeVar11 = textureDimensions( nodeUniform4, u32( 0 ) );
		nodeVar10 = textureLoad( nodeUniform4, vec2<u32>( clamp( floor( tsl_coord_clampS_clampT_2d( ( object.nodeUniform5 * vec3<f32>( nodeVar9, 1.0 ) ).xy ) * vec2<f32>( nodeVar11 ) ), vec2<f32>( 0 ), vec2<f32>( nodeVar11 - vec2<u32>( 1, 1 ) ) ) ), u32( 0 ) );
		nodeVar7.x = ( nodeVar7.x - ( object.nodeUniform3.x * ( vec4<f32>( 255.0 ) * nodeVar10 ).x ) );
		nodeVar5.x = nodeVar7.x;
		nodeVar5.y = nodeVarying2.y;
		nodeVar4.x = nodeVar5.x;
		nodeVar12 = textureSample( nodeUniform0, nodeUniform0_sampler, ( object.nodeUniform1 * vec3<f32>( nodeVar5, 1.0 ) ).xy );
		nodeVar13 = nodeVar12.x;
		nodeVar14 = vec2<f32>( 0.0, 0.0 );
		nodeVar15 = vec2<f32>( 0.0, 1.0 );
		nodeVar16 = nodeVarying0.zw;

		for ( var i : i32 = 0; i < 8; i ++ ) {

			nodeVar17 = textureSample( nodeUniform0, nodeUniform0_sampler, ( object.nodeUniform1 * vec3<f32>( nodeVar16, 1.0 ) ).xy );
			nodeVar15 = nodeVar17.xy;
			nodeVar16 = ( nodeVar16 + ( vec2<f32>( 2.0, 0.0 ) * object.nodeUniform3 ) );

			if ( ( ( nodeVar16.x >= nodeVarying1.y ) || ( ( nodeVar15.y <= 0.8281 ) || ( nodeVar15.x != 0.0 ) ) ) ) {

				break;
				

			}


		}

		nodeVar16.x = ( nodeVar16.x - ( 0.25 * object.nodeUniform3.x ) );
		nodeVar16.x = ( nodeVar16.x - object.nodeUniform3.x );
		nodeVar16.x = ( nodeVar16.x - ( 2.0 * object.nodeUniform3.x ) );
		nodeVar18 = nodeVar15;
		nodeVar18.x = ( 0.5 + ( nodeVar18.x * 0.5 ) );
		nodeVar19 = textureLoad( nodeUniform4, vec2<u32>( clamp( floor( tsl_coord_clampS_clampT_2d( ( object.nodeUniform5 * vec3<f32>( nodeVar18, 1.0 ) ).xy ) * vec2<f32>( nodeVar11 ) ), vec2<f32>( 0 ), vec2<f32>( nodeVar11 - vec2<u32>( 1, 1 ) ) ) ), u32( 0 ) );
		nodeVar16.x = ( nodeVar16.x + ( object.nodeUniform3.x * ( vec4<f32>( 255.0 ) * nodeVar19 ).x ) );
		nodeVar14.x = nodeVar16.x;
		nodeVar14.y = nodeVarying2.y;
		nodeVar4.y = nodeVar14.x;
		nodeVar20 = textureSample( nodeUniform0, nodeUniform0_sampler, ( object.nodeUniform1 * vec3<f32>( ( nodeVar14 + ( vec2<f32>( 1.0, 0.0 ) * object.nodeUniform3 ) ), 1.0 ) ).xy );
		nodeVar21 = nodeVar20.x;
		nodeVar0.x = nodeVar21;
		var nodeVar22 : vec2<f32> = ( ( vec2<f32>( 0.00625, 0.0017857142857142857 ) * ( ( vec2<f32>( 16.0 ) * round( ( vec2<f32>( 4.0 ) * vec2<f32>( nodeVar13, nodeVar21 ) ) ) ) + sqrt( abs( ( ( nodeVar4 / vec2<f32>( object.nodeUniform3.x ) ) - vec2<f32>( nodeVarying3.x ) ) ) ) ) ) + ( vec2<f32>( 0.5 ) * vec2<f32>( 0.00625, 0.0017857142857142857 ) ) );
		nodeVar22.y = ( nodeVar22.y + ( 0.14285714285714285 * nodeVar1.y ) );
		nodeVar23 = textureSample( nodeUniform6, nodeUniform6_sampler, ( object.nodeUniform7 * vec3<f32>( nodeVar22, 1.0 ) ).xy );
		nodeVar24 = nodeVar23.xy;
		nodeVar0.x = nodeVar24[ 0 ];
		nodeVar0.y = nodeVar24[ 1 ];
		

	}


	if ( ( nodeVar3.x > 0.0 ) ) {

		nodeVar25 = vec2<f32>( 0.0, 0.0 );
		nodeVar26 = vec2<f32>( 0.0, 0.0 );
		nodeVar27 = vec2<f32>( 1.0, 0.0 );
		nodeVar28 = nodeVarying2.xy;

		for ( var i : i32 = 0; i < 8; i ++ ) {

			nodeVar29 = textureSample( nodeUniform0, nodeUniform0_sampler, ( object.nodeUniform1 * vec3<f32>( nodeVar28, 1.0 ) ).xy );
			nodeVar27 = nodeVar29.xy;
			nodeVar28 = ( nodeVar28 + ( vec2<f32>( 0.0, -2.0 ) * object.nodeUniform3 ) );

			if ( ( ( nodeVar28.y <= nodeVarying1.z ) || ( ( nodeVar27.x <= 0.8281 ) || ( nodeVar27.y != 0.0 ) ) ) ) {

				break;
				

			}


		}

		nodeVar28.y = ( nodeVar28.y + ( 0.25 * object.nodeUniform3.y ) );
		nodeVar28.y = ( nodeVar28.y + object.nodeUniform3.y );
		nodeVar28.y = ( nodeVar28.y + ( 2.0 * object.nodeUniform3.y ) );
		nodeVar30 = nodeVar27.yx;
		nodeVar30.x = ( 0.0 + ( nodeVar30.x * 0.5 ) );
		nodeVar32 = textureDimensions( nodeUniform4, u32( 0 ) );
		nodeVar31 = textureLoad( nodeUniform4, vec2<u32>( clamp( floor( tsl_coord_clampS_clampT_2d( ( object.nodeUniform5 * vec3<f32>( nodeVar30, 1.0 ) ).xy ) * vec2<f32>( nodeVar32 ) ), vec2<f32>( 0 ), vec2<f32>( nodeVar32 - vec2<u32>( 1, 1 ) ) ) ), u32( 0 ) );
		nodeVar28.y = ( nodeVar28.y - ( object.nodeUniform3.y * ( vec4<f32>( 255.0 ) * nodeVar31 ).x ) );
		nodeVar26.y = nodeVar28.y;
		nodeVar26.x = nodeVarying0.x;
		nodeVar25.x = nodeVar26.y;
		nodeVar33 = textureSample( nodeUniform0, nodeUniform0_sampler, ( object.nodeUniform1 * vec3<f32>( nodeVar26, 1.0 ) ).xy );
		nodeVar34 = nodeVar33.y;
		nodeVar35 = vec2<f32>( 0.0, 0.0 );
		nodeVar36 = vec2<f32>( 1.0, 0.0 );
		nodeVar37 = nodeVarying2.zw;

		for ( var i : i32 = 0; i < 8; i ++ ) {

			nodeVar38 = textureSample( nodeUniform0, nodeUniform0_sampler, ( object.nodeUniform1 * vec3<f32>( nodeVar37, 1.0 ) ).xy );
			nodeVar36 = nodeVar38.xy;
			nodeVar37 = ( nodeVar37 - ( vec2<f32>( 0.0, -2.0 ) * object.nodeUniform3 ) );

			if ( ( ( nodeVar37.y >= nodeVarying1.w ) || ( ( nodeVar36.x <= 0.8281 ) || ( nodeVar36.y != 0.0 ) ) ) ) {

				break;
				

			}


		}

		nodeVar37.y = ( nodeVar37.y - ( 0.25 * object.nodeUniform3.y ) );
		nodeVar37.y = ( nodeVar37.y - object.nodeUniform3.y );
		nodeVar37.y = ( nodeVar37.y - ( 2.0 * object.nodeUniform3.y ) );
		nodeVar39 = nodeVar36.yx;
		nodeVar39.x = ( 0.5 + ( nodeVar39.x * 0.5 ) );
		nodeVar40 = textureLoad( nodeUniform4, vec2<u32>( clamp( floor( tsl_coord_clampS_clampT_2d( ( object.nodeUniform5 * vec3<f32>( nodeVar39, 1.0 ) ).xy ) * vec2<f32>( nodeVar32 ) ), vec2<f32>( 0 ), vec2<f32>( nodeVar32 - vec2<u32>( 1, 1 ) ) ) ), u32( 0 ) );
		nodeVar37.y = ( nodeVar37.y + ( object.nodeUniform3.y * ( vec4<f32>( 255.0 ) * nodeVar40 ).x ) );
		nodeVar35.y = nodeVar37.y;
		nodeVar35.x = nodeVarying0.x;
		nodeVar25.y = nodeVar35.y;
		nodeVar41 = textureSample( nodeUniform0, nodeUniform0_sampler, ( object.nodeUniform1 * vec3<f32>( ( nodeVar35 + ( vec2<f32>( 0.0, 1.0 ) * object.nodeUniform3 ) ), 1.0 ) ).xy );
		nodeVar42 = nodeVar41.y;
		var nodeVar43 : vec2<f32> = ( ( vec2<f32>( 0.00625, 0.0017857142857142857 ) * ( ( vec2<f32>( 16.0 ) * round( ( vec2<f32>( 4.0 ) * vec2<f32>( nodeVar34, nodeVar42 ) ) ) ) + sqrt( abs( ( ( nodeVar25 / vec2<f32>( object.nodeUniform3.y ) ) - vec2<f32>( nodeVarying3.y ) ) ) ) ) ) + ( vec2<f32>( 0.5 ) * vec2<f32>( 0.00625, 0.0017857142857142857 ) ) );
		nodeVar43.y = ( nodeVar43.y + ( 0.14285714285714285 * nodeVar1.x ) );
		nodeVar44 = textureSample( nodeUniform6, nodeUniform6_sampler, ( object.nodeUniform7 * vec3<f32>( nodeVar43, 1.0 ) ).xy );
		nodeVar45 = nodeVar44.xy;
		nodeVar0.z = nodeVar45[ 0 ];
		nodeVar0.w = nodeVar45[ 1 ];
		

	}


	// result

	output.color = nodeVar0;

	return output;

}
