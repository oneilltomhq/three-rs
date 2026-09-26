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
@binding( 0 ) @group( 0 ) var nodeUniform0 : texture_2d<f32>;
@binding( 2 ) @group( 0 ) var nodeUniform3 : texture_depth_2d;
@binding( 3 ) @group( 0 ) var nodeUniform4 : texture_2d<f32>;

struct objectStruct {
	nodeUniform1 : f32,
	nodeUniform2 : f32
};
@binding( 1 ) @group( 0 )
var<uniform> object : objectStruct;

// vars
var<private> nodeVar0 : f32;
var<private> nodeVar1 : f32;
var<private> nodeVar2 : f32;
var<private> nodeVar3 : f32;
var<private> depth : f32;
var<private> nodeVar4 : f32;
var<private> nodeVar5 : vec2<u32>;
var<private> normal : vec3<f32>;
var<private> nodeVar6 : vec4<f32>;
var<private> nodeVar7 : vec2<u32>;
var<private> nodeVar8 : f32;
var<private> nodeVar9 : f32;
var<private> nodeVar10 : f32;
var<private> nodeVar11 : f32;
var<private> diff : f32;
var<private> dei : f32;
var<private> nodeVar12 : vec4<f32>;
var<private> nodeVar13 : vec2<u32>;
var<private> indicator : f32;
var<private> nodeVar14 : vec4<f32>;
var<private> nodeVar15 : vec4<f32>;
var<private> nodeVar16 : vec4<f32>;
var<private> nei : f32;
var<private> nodeVar17 : vec4<f32>;
var<private> nodeVar18 : vec2<u32>;
var<private> nodeVar19 : f32;

// codes
fn tsl_clampWrapping_float( coord: f32 ) -> f32 { return clamp( coord, 0.0, 1.0 ); }
fn tsl_coord_clampS_clampT_2d( coord : vec2f ) -> vec2f {

	return vec2f(
		tsl_clampWrapping_float( coord.x ),
		tsl_clampWrapping_float( coord.y )
	);

}

fn fn1 ( color : vec4<f32> ) -> vec4<f32> {

	var nodeVar0 : vec4<f32>;


	if ( ( color.w == 0.0 ) ) {

		nodeVar0 = vec4<f32>( 0.0, 0.0, 0.0, 0.0 );

	} else {

		nodeVar0 = vec4<f32>( ( color.xyz / vec3<f32>( color.w ) ), color.w );

	}


	return nodeVar0;

}


fn sRGBTransferOETF ( color : vec3<f32> ) -> vec3<f32> {

	


	return mix( ( ( pow( color, vec3<f32>( 0.41666 ) ) * vec3<f32>( 1.055 ) ) - vec3<f32>( 0.055 ) ), ( color * vec3<f32>( 12.92 ) ), vec3<f32>( ( color <= vec3<f32>( 0.0031308 ) ) ) );

}


fn fn0 ( color : vec4<f32> ) -> vec4<f32> {

	


	return vec4<f32>( ( color.xyz * vec3<f32>( color.w ) ), color.w );

}




@fragment
fn main( @location( 0 ) nodeVarying0 : vec2<f32> ) -> OutputStruct {

	// flow
	// code

	nodeVar0 = 0.0;
	nodeVar1 = 0.0;
	nodeVar2 = 0.0;
	nodeVar3 = 0.0;
	let nodeConst0 = ( vec2<f32>( 1.0, 1.0 ) / vec2<f32>( textureDimensions( nodeUniform0, 0 ) ) );

	if ( ( ( object.nodeUniform1 > 0.0 ) || ( object.nodeUniform2 > 0.0 ) ) ) {

		nodeVar5 = textureDimensions( nodeUniform3, u32( 0 ) );
		nodeVar4 = textureLoad( nodeUniform3, vec2<u32>( clamp( floor( tsl_coord_clampS_clampT_2d( nodeVarying0 ) * vec2<f32>( nodeVar5 ) ), vec2<f32>( 0 ), vec2<f32>( nodeVar5 - vec2<u32>( 1, 1 ) ) ) ), u32( 0 ) );
		depth = nodeVar4;
		nodeVar7 = textureDimensions( nodeUniform4, u32( 0 ) );
		nodeVar6 = textureLoad( nodeUniform4, vec2<u32>( clamp( floor( tsl_coord_clampS_clampT_2d( nodeVarying0 ) * vec2<f32>( nodeVar7 ) ), vec2<f32>( 0 ), vec2<f32>( nodeVar7 - vec2<u32>( 1, 1 ) ) ) ), u32( 0 ) );
		normal = normalize( nodeVar6.xyz );
		nodeVar8 = textureLoad( nodeUniform3, vec2<u32>( clamp( floor( tsl_coord_clampS_clampT_2d( ( nodeVarying0 + ( vec2<f32>( 1.0, 0.0 ) * nodeConst0 ) ) ) * vec2<f32>( nodeVar5 ) ), vec2<f32>( 0 ), vec2<f32>( nodeVar5 - vec2<u32>( 1, 1 ) ) ) ), u32( 0 ) );
		nodeVar0 = nodeVar8;
		nodeVar9 = textureLoad( nodeUniform3, vec2<u32>( clamp( floor( tsl_coord_clampS_clampT_2d( ( nodeVarying0 + ( vec2<f32>( -1.0, 0.0 ) * nodeConst0 ) ) ) * vec2<f32>( nodeVar5 ) ), vec2<f32>( 0 ), vec2<f32>( nodeVar5 - vec2<u32>( 1, 1 ) ) ) ), u32( 0 ) );
		nodeVar1 = nodeVar9;
		nodeVar10 = textureLoad( nodeUniform3, vec2<u32>( clamp( floor( tsl_coord_clampS_clampT_2d( ( nodeVarying0 + ( vec2<f32>( 0.0, 1.0 ) * nodeConst0 ) ) ) * vec2<f32>( nodeVar5 ) ), vec2<f32>( 0 ), vec2<f32>( nodeVar5 - vec2<u32>( 1, 1 ) ) ) ), u32( 0 ) );
		nodeVar2 = nodeVar10;
		nodeVar11 = textureLoad( nodeUniform3, vec2<u32>( clamp( floor( tsl_coord_clampS_clampT_2d( ( nodeVarying0 + ( vec2<f32>( 0.0, -1.0 ) * nodeConst0 ) ) ) * vec2<f32>( nodeVar5 ) ), vec2<f32>( 0 ), vec2<f32>( nodeVar5 - vec2<u32>( 1, 1 ) ) ) ), u32( 0 ) );
		nodeVar3 = nodeVar11;
		

	}


	if ( ( object.nodeUniform1 > 0.0 ) ) {

		diff = ( diff + clamp( ( nodeVar0 - depth ), 0.0, 1.0 ) );
		diff = ( diff + clamp( ( nodeVar1 - depth ), 0.0, 1.0 ) );
		diff = ( diff + clamp( ( nodeVar2 - depth ), 0.0, 1.0 ) );
		diff = ( diff + clamp( ( nodeVar3 - depth ), 0.0, 1.0 ) );
		dei = ( floor( ( smoothstep( 0.01, 0.02, diff ) * 2.0 ) ) / 2.0 );
		

	}


	if ( ( ( object.nodeUniform2 > 0.0 ) && ( length( normal ) > 0.0 ) ) ) {

		let nodeConst1 = ( nodeVar3 - depth );
		nodeVar13 = textureDimensions( nodeUniform4, u32( 0 ) );
		nodeVar12 = textureLoad( nodeUniform4, vec2<u32>( clamp( floor( tsl_coord_clampS_clampT_2d( ( nodeVarying0 + ( vec2<f32>( 0.0, -1.0 ) * nodeConst0 ) ) ) * vec2<f32>( nodeVar13 ) ), vec2<f32>( 0 ), vec2<f32>( nodeVar13 - vec2<u32>( 1, 1 ) ) ) ), u32( 0 ) );
		let nodeConst2 = normalize( nodeVar12.xyz );
		let nodeConst3 = dot( ( normal - nodeConst2 ), vec3<f32>( 1.0, 1.0, 1.0 ) );
		let nodeConst4 = clamp( smoothstep( -0.01, 0.01, nodeConst3 ), 0.0, 1.0 );
		let nodeConst5 = clamp( sign( ( ( nodeConst1 * 0.25 ) + 0.0025 ) ), 0.0, 1.0 );
		indicator = ( indicator + ( ( ( 1.0 - dot( normal, nodeConst2 ) ) * nodeConst5 ) * nodeConst4 ) );
		let nodeConst6 = ( nodeVar2 - depth );
		nodeVar14 = textureLoad( nodeUniform4, vec2<u32>( clamp( floor( tsl_coord_clampS_clampT_2d( ( nodeVarying0 + ( vec2<f32>( 0.0, 1.0 ) * nodeConst0 ) ) ) * vec2<f32>( nodeVar13 ) ), vec2<f32>( 0 ), vec2<f32>( nodeVar13 - vec2<u32>( 1, 1 ) ) ) ), u32( 0 ) );
		let nodeConst7 = normalize( nodeVar14.xyz );
		let nodeConst8 = dot( ( normal - nodeConst7 ), vec3<f32>( 1.0, 1.0, 1.0 ) );
		let nodeConst9 = clamp( smoothstep( -0.01, 0.01, nodeConst8 ), 0.0, 1.0 );
		let nodeConst10 = clamp( sign( ( ( nodeConst6 * 0.25 ) + 0.0025 ) ), 0.0, 1.0 );
		indicator = ( indicator + ( ( ( 1.0 - dot( normal, nodeConst7 ) ) * nodeConst10 ) * nodeConst9 ) );
		let nodeConst11 = ( nodeVar1 - depth );
		nodeVar15 = textureLoad( nodeUniform4, vec2<u32>( clamp( floor( tsl_coord_clampS_clampT_2d( ( nodeVarying0 + ( vec2<f32>( -1.0, 0.0 ) * nodeConst0 ) ) ) * vec2<f32>( nodeVar13 ) ), vec2<f32>( 0 ), vec2<f32>( nodeVar13 - vec2<u32>( 1, 1 ) ) ) ), u32( 0 ) );
		let nodeConst12 = normalize( nodeVar15.xyz );
		let nodeConst13 = dot( ( normal - nodeConst12 ), vec3<f32>( 1.0, 1.0, 1.0 ) );
		let nodeConst14 = clamp( smoothstep( -0.01, 0.01, nodeConst13 ), 0.0, 1.0 );
		let nodeConst15 = clamp( sign( ( ( nodeConst11 * 0.25 ) + 0.0025 ) ), 0.0, 1.0 );
		indicator = ( indicator + ( ( ( 1.0 - dot( normal, nodeConst12 ) ) * nodeConst15 ) * nodeConst14 ) );
		let nodeConst16 = ( nodeVar0 - depth );
		nodeVar16 = textureLoad( nodeUniform4, vec2<u32>( clamp( floor( tsl_coord_clampS_clampT_2d( ( nodeVarying0 + ( vec2<f32>( 1.0, 0.0 ) * nodeConst0 ) ) ) * vec2<f32>( nodeVar13 ) ), vec2<f32>( 0 ), vec2<f32>( nodeVar13 - vec2<u32>( 1, 1 ) ) ) ), u32( 0 ) );
		let nodeConst17 = normalize( nodeVar16.xyz );
		let nodeConst18 = dot( ( normal - nodeConst17 ), vec3<f32>( 1.0, 1.0, 1.0 ) );
		let nodeConst19 = clamp( smoothstep( -0.01, 0.01, nodeConst18 ), 0.0, 1.0 );
		let nodeConst20 = clamp( sign( ( ( nodeConst16 * 0.25 ) + 0.0025 ) ), 0.0, 1.0 );
		indicator = ( indicator + ( ( ( 1.0 - dot( normal, nodeConst17 ) ) * nodeConst20 ) * nodeConst19 ) );
		nei = step( 0.1, indicator );
		

	}

	nodeVar18 = textureDimensions( nodeUniform0, u32( 0 ) );
	nodeVar17 = textureLoad( nodeUniform0, vec2<u32>( clamp( floor( tsl_coord_clampS_clampT_2d( nodeVarying0 ) * vec2<f32>( nodeVar18 ) ), vec2<f32>( 0 ), vec2<f32>( nodeVar18 - vec2<u32>( 1, 1 ) ) ) ), u32( 0 ) );
	let nodeVar17 = nodeVar17;

	if ( ( dei > 0.0 ) ) {

		nodeVar19 = ( 1.0 - ( dei * object.nodeUniform1 ) );

	} else {

		nodeVar19 = ( ( nei * object.nodeUniform2 ) + 1.0 );

	}

	let nodeConst21 = fn1( vec4<f32>( vec4<f32>( ( nodeVar17 * vec4<f32>( nodeVar19 ) ).xyz, nodeVar17.w ).xyz, clamp( vec4<f32>( ( nodeVar17 * nodeVar19 ).xyz, nodeVar17.w ).w, 0.0, 1.0 ) ) );

	// result

	output.color = fn0( vec4<f32>( sRGBTransferOETF( nodeConst21.xyz ), nodeConst21.w ) );

	return output;

}
