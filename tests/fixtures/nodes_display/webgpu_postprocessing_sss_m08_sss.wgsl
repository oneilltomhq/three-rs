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
@binding( 0 ) @group( 1 ) var nodeUniform0 : texture_depth_2d;

struct objectStruct {
	nodeUniform1 : mat4x4<f32>,
	nodeUniform2 : mat4x4<f32>,
	nodeUniform5 : f32,
	nodeUniform6 : mat4x4<f32>,
	nodeUniform7 : vec2<f32>,
	nodeUniform8 : f32,
	nodeUniform9 : f32,
	nodeUniform10 : f32,
	nodeUniform11 : f32,
	nodeUniform12 : f32,
	nodeUniform13 : f32
};
@binding( 1 ) @group( 1 )
var<uniform> object : objectStruct;

struct renderStruct {
	nodeUniform3 : vec3<f32>,
	nodeUniform4 : vec3<f32>
};
@binding( 0 ) @group( 0 )
var<uniform> render : renderStruct;

// vars
var<private> nodeVar0 : f32;
var<private> nodeVar1 : vec2<u32>;
var<private> nodeVar2 : f32;
var<private> rayStartPosition : vec3<f32>;
var<private> nodeVar3 : vec2<f32>;
var<private> nodeVar4 : vec2<f32>;
var<private> nodeVar5 : vec2<f32>;
var<private> nodeVar6 : f32;
var<private> nodeVar7 : f32;
var<private> nodeVar8 : f32;
var<private> nodeVar9 : f32;
var<private> nodeVar10 : f32;
var<private> nodeVar11 : f32;
var<private> nodeVar12 : vec2<f32>;
var<private> nodeVar13 : f32;
var<private> nodeVar14 : f32;

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


fn tsl_mod_float( x : f32, y : f32 ) -> f32 { return x - y * floor( x / y ); }


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
	rayStartPosition = ( nodeConst0.xyz / vec3<f32>( nodeConst0.w ) );
	let rayDirection = normalize( ( object.nodeUniform2 * vec4<f32>( ( render.nodeUniform3 - render.nodeUniform4 ), 0.0 ) ).xyz );
	let rayEndPosition = ( rayStartPosition + ( rayDirection * vec3<f32>( object.nodeUniform5 ) ) );
	nodeVar3 = fragCoord.xy;
	let nodeConst1 = ( object.nodeUniform6 * vec4<f32>( rayEndPosition, 1.0 ) );
	nodeVar4 = ( ( ( nodeConst1.xy / vec2<f32>( nodeConst1.w ) ) * vec2<f32>( 0.5 ) ) + vec2<f32>( 0.5 ) );
	nodeVar5 = ( vec2<f32>( nodeVar4.x, ( 1.0 - nodeVar4.y ) ) * object.nodeUniform7 );
	nodeVar6 = length( ( nodeVar5 - nodeVar3 ) );
	nodeVar7 = ( nodeVar5.x - nodeVar3.x );
	nodeVar8 = ( nodeVar5.y - nodeVar3.y );
	let nodeConst2 = i32( ( max( abs( nodeVar7 ), abs( nodeVar8 ) ) * clamp( object.nodeUniform8, 0.0, 1.0 ) ) );
	nodeVar9 = ( nodeVar7 / f32( nodeConst2 ) );
	nodeVar10 = ( nodeVar8 / f32( nodeConst2 ) );
	let offset = ( fract( ( interleavedGradientNoise( fragCoord.xy ) + object.nodeUniform9 ) ) + fract( ( sin( tsl_mod_float( dot( ( nodeVarying0 + vec2<f32>( 2.0 ) ), vec2<f32>( 12.9898, 78.233 ) ), 3.141592653589793 ) ) * 43758.5453 ) ) );
	nodeVar11 = 0.0;

	for ( var i : i32 = 0; i < nodeConst2; i ++ ) {

		nodeVar12 = vec2<f32>( ( nodeVar3.x + ( nodeVar9 * ( f32( i ) + offset ) ) ), ( nodeVar3.y + ( nodeVar10 * ( f32( i ) + offset ) ) ) );

		if ( ( ( ( ( nodeVar12.x < 0.0 ) || ( nodeVar12.x > object.nodeUniform7.x ) ) || ( nodeVar12.y < 0.0 ) ) || ( nodeVar12.y > object.nodeUniform7.y ) ) ) {

			break;
			

		}

		nodeVar13 = textureLoad( nodeUniform0, vec2<u32>( clamp( floor( tsl_coord_clampS_clampT_2d( ( nodeVar12 / object.nodeUniform7 ) ) * vec2<f32>( nodeVar1 ) ), vec2<f32>( 0 ), vec2<f32>( nodeVar1 - vec2<u32>( 1, 1 ) ) ) ), u32( 0 ) );
		let nodeConst3 = nodeVar13;
		let fragmentViewZ = ( ( object.nodeUniform10 * object.nodeUniform11 ) / ( ( ( object.nodeUniform11 - object.nodeUniform10 ) * nodeConst3 ) - object.nodeUniform11 ) );
		nodeVar14 = ( length( ( nodeVar12 - nodeVar3 ) ) / nodeVar6 );
		let nodeConst4 = ( - ( mix( rayStartPosition, rayEndPosition, nodeVar14 ).z - fragmentViewZ ) );

		if ( ( ( nodeConst4 > 0.0 ) && ( nodeConst4 < object.nodeUniform12 ) ) ) {

			nodeVar11 = object.nodeUniform13;
			break;
			

		}


	}


	// result

	output.color = ( 1.0 - nodeVar11 );

	return output;

}
