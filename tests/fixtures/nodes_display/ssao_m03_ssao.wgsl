// Three.js r187dev - Node System

// global
diagnostic( off, derivative_uniformity );


// directives


// structs

struct OutputStruct {
	@location( 0 ) color: f32
};
var<private> output : OutputStruct;

// uniforms
@binding( 0 ) @group( 0 ) var nodeUniform0 : texture_depth_2d;
@binding( 2 ) @group( 0 ) var nodeUniform2_sampler : sampler;
@binding( 3 ) @group( 0 ) var nodeUniform2 : texture_2d<f32>;

struct objectStruct {
	nodeUniform1 : mat4x4<f32>,
	nodeUniform3 : mat4x4<f32>,
	nodeUniform4 : f32,
	nodeUniform5 : f32,
	nodeUniform6 : f32,
	nodeUniform7 : f32
};
@binding( 1 ) @group( 0 )
var<uniform> object : objectStruct;

// vars
var<private> nodeVar0 : f32;
var<private> nodeVar1 : vec2<u32>;
var<private> nodeVar2 : f32;
var<private> nodeVar3 : vec3<f32>;
var<private> nodeVar4 : vec4<f32>;
var<private> nodeVar5 : vec3<f32>;
var<private> nodeVar6 : f32;
var<private> nodeVar7 : vec4<f32>;
var<private> nodeVar8 : f32;
var<private> nodeVar9 : f32;
var<private> nodeVar10 : vec3<f32>;
var<private> nodeVar11 : f32;

// codes
fn tsl_clampWrapping_float( coord: f32 ) -> f32 { return clamp( coord, 0.0, 1.0 ); }
fn tsl_coord_clampS_clampT_2d( coord : vec2f ) -> vec2f {

	return vec2f(
		tsl_clampWrapping_float( coord.x ),
		tsl_clampWrapping_float( coord.y )
	);

}

fn interleavedGradientNoise ( position : vec2<f32> ) -> f32 {

	


	return fract( ( 52.9829189 * fract( dot( position, vec2<f32>( 0.06711056, 0.00583715 ) ) ) ) );

}


fn vogelDiskSample ( sampleIndex : i32, samplesCount : i32, phi : f32 ) -> vec2<f32> {

	

	let nodeConst0 = ( ( f32( sampleIndex ) * 2.399963229728653 ) + phi );

	return ( vec2<f32>( cos( nodeConst0 ), sin( nodeConst0 ) ) * vec2<f32>( sqrt( ( ( f32( sampleIndex ) + 0.5 ) / f32( samplesCount ) ) ) ) );

}


fn getScreenPositionFromClip ( clipPosition : vec4<f32> ) -> vec2<f32> {

	var nodeVar0 : vec2<f32>;

	nodeVar0 = ( ( ( clipPosition.xy / vec2<f32>( clipPosition.w ) ) * vec2<f32>( 0.5 ) ) + vec2<f32>( 0.5 ) );

	return vec2<f32>( nodeVar0.x, ( 1.0 - nodeVar0.y ) );

}




@fragment
fn main( @location( 0 ) nodeVarying0 : vec2<f32>,
	@builtin( position ) fragCoord : vec4<f32> ) -> OutputStruct {

	// flow
	// code

	nodeVar1 = textureDimensions( nodeUniform0, u32( 0 ) );
	nodeVar0 = textureLoad( nodeUniform0, vec2<u32>( clamp( floor( tsl_coord_clampS_clampT_2d( nodeVarying0 ) * vec2<f32>( nodeVar1 ) ), vec2<f32>( 0 ), vec2<f32>( nodeVar1 - vec2<u32>( 1, 1 ) ) ) ), u32( 0 ) );
	nodeVar2 = nodeVar0;

	if ( ( nodeVar2 >= 1.0 ) ) {

		discard;
		

	}

	let nodeConst0 = ( object.nodeUniform1 * vec4<f32>( vec3<f32>( ( ( vec2<f32>( nodeVarying0.x, ( 1.0 - nodeVarying0.y ) ) * vec2<f32>( 2.0 ) ) - vec2<f32>( 1.0 ) ), nodeVar2 ), 1.0 ) );
	nodeVar3 = ( nodeConst0.xyz / vec3<f32>( nodeConst0.w ) );
	nodeVar4 = textureSample( nodeUniform2, nodeUniform2_sampler, nodeVarying0 );
	nodeVar5 = normalize( ( ( nodeVar4 * vec4<f32>( 2.0 ) ) - vec4<f32>( 1.0 ) ).xyz );
	nodeVar6 = ( interleavedGradientNoise( fragCoord.xy ) * 6.283185307179586 );
	nodeVar7 = ( object.nodeUniform3 * vec4<f32>( nodeVar3, 1.0 ) );
	nodeVar8 = 0.0;

	for ( var i : i32 = 0; i < i32( object.nodeUniform4 ); i ++ ) {

		let nodeConst1 = getScreenPositionFromClip( ( nodeVar7 + ( object.nodeUniform3 * vec4<f32>( ( vogelDiskSample( i, i32( object.nodeUniform4 ), nodeVar6 ) * vec2<f32>( object.nodeUniform5 ) ), 0.0, 0.0 ) ) ) );
		nodeVar9 = textureLoad( nodeUniform0, vec2<u32>( clamp( floor( tsl_coord_clampS_clampT_2d( nodeConst1 ) * vec2<f32>( nodeVar1 ) ), vec2<f32>( 0 ), vec2<f32>( nodeVar1 - vec2<u32>( 1, 1 ) ) ) ), u32( 0 ) );
		let nodeConst2 = ( object.nodeUniform1 * vec4<f32>( vec3<f32>( ( ( vec2<f32>( nodeConst1.x, ( 1.0 - nodeConst1.y ) ) * vec2<f32>( 2.0 ) ) - vec2<f32>( 1.0 ) ), nodeVar9 ), 1.0 ) );
		nodeVar10 = ( ( nodeConst2.xyz / vec3<f32>( nodeConst2.w ) ) - nodeVar3 );
		nodeVar11 = length( nodeVar10 );
		nodeVar8 = ( nodeVar8 + ( max( ( ( dot( nodeVar10, nodeVar5 ) / max( nodeVar11, 0.0001 ) ) - object.nodeUniform6 ), 0.0 ) * ( object.nodeUniform5 / ( object.nodeUniform5 + nodeVar11 ) ) ) );

	}


	// result

	output.color = clamp( ( 1.0 - ( ( nodeVar8 / object.nodeUniform4 ) * object.nodeUniform7 ) ), 0.0, 1.0 );

	return output;

}
