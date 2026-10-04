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

struct objectStruct {
	nodeUniform0 : mat4x4<f32>,
	nodeUniform2 : f32
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
var<private> nodeVar7 : vec4<f32>;
var<private> nodeVar8 : vec2<u32>;
var<private> nodeVar9 : f32;
var<private> nodeVar10 : f32;
var<private> nodeVar11 : f32;
var<private> nodeVar12 : vec3<f32>;
var<private> nodeVar13 : f32;
var<private> nodeVar14 : f32;
var<private> nodeVar15 : f32;
var<private> nodeVar16 : f32;
var<private> nodeVar17 : f32;
var<private> nodeVar18 : f32;
var<private> nodeVar19 : f32;
var<private> nodeVar20 : f32;
var<private> nodeVar21 : f32;

// codes
fn tsl_repeatWrapping_float( coord: f32 ) -> f32 { return fract( coord ); }
fn tsl_clampWrapping_float( coord: f32 ) -> f32 { return clamp( coord, 0.0, 1.0 ); }
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

	nodeVar0 = normalize( ( object.nodeUniform0 * vec4<f32>( normalize( vec3<f32>( ( nodeVarying0.yx - vec2<f32>( 0.5 ) ), 1.0 ) ), 0.0 ) ).xyz );
	nodeVar1 = normalize( ( object.nodeUniform0 * vec4<f32>( vec3<f32>( 0.0, 0.0, 1.0 ), 0.0 ) ).xyz );
	nodeVar2 = max( 0.0, dot( nodeVar0, nodeVar1 ) );
	nodeVar3 = normalize( ( object.nodeUniform0 * vec4<f32>( normalize( vec3<f32>( ( nodeVarying0 - vec2<f32>( 0.5 ) ), -1.0 ) ), 0.0 ) ).xyz );
	nodeVar4 = normalize( ( nodeVar1 + nodeVar3 ) );
	nodeVar5 = max( 0.0, dot( nodeVar0, nodeVar3 ) );
	nodeVar6 = max( 0.0, dot( nodeVar1, nodeVar4 ) );
	nodeVar8 = textureDimensions( nodeUniform1, u32( 0.0 ) );
	nodeVar7 = textureLoad( nodeUniform1, vec2<u32>( clamp( floor( tsl_coord_repeatS_clampT_2d( vec2<f32>( ( ( atan2( nodeVar3.z, nodeVar3.x ) * 0.15915494309189535 ) + 0.5 ), ( ( asin( clamp( nodeVar3.y, -1.0, 1.0 ) ) * 0.3183098861837907 ) + 0.5 ) ) ) * vec2<f32>( nodeVar8 ) ), vec2<f32>( 0 ), vec2<f32>( nodeVar8 - vec2<u32>( 1, 1 ) ) ) ), u32( 0.0 ) );
	nodeVar9 = ( 1.0 - nodeVar6 );
	nodeVar10 = ( nodeVar9 * nodeVar9 );
	nodeVar11 = ( ( nodeVar10 * nodeVar10 ) * nodeVar9 );
	nodeVar12 = ( vec3<f32>( 0.04, 0.04, 0.04 ) + ( ( vec3<f32>( 1.0, 1.0, 1.0 ) - vec3<f32>( 0.04, 0.04, 0.04 ) ) * vec3<f32>( nodeVar11 ) ) );
	let nodeConst0 = ( nodeVarying0.x * nodeVarying0.x );
	nodeVar13 = ( nodeConst0 * nodeConst0 );
	nodeVar14 = ( nodeVar2 * nodeVar2 );
	nodeVar15 = ( ( 2.0 * nodeVar2 ) / ( nodeVar2 + sqrt( ( nodeVar13 + ( ( 1.0 - nodeVar13 ) * nodeVar14 ) ) ) ) );
	nodeVar16 = ( nodeConst0 * nodeConst0 );
	nodeVar17 = ( nodeVar5 * nodeVar5 );
	nodeVar18 = ( ( 2.0 * nodeVar5 ) / ( nodeVar5 + sqrt( ( nodeVar16 + ( ( 1.0 - nodeVar16 ) * nodeVar17 ) ) ) ) );
	nodeVar19 = ( nodeVar15 * nodeVar18 );
	nodeVar20 = ( nodeConst0 * nodeConst0 );
	nodeVar21 = ( nodeVar2 * nodeVar2 );

	// result

	output.color = vec4<f32>( ( ( nodeVar7.xyz * ( ( nodeVar12 * vec3<f32>( nodeVar19 ) ) / vec3<f32>( max( ( ( 2.0 * nodeVar2 ) / ( nodeVar2 + sqrt( ( nodeVar20 + ( ( 1.0 - nodeVar20 ) * nodeVar21 ) ) ) ) ), 0.0001 ) ) ) ) * vec3<f32>( object.nodeUniform2 ) ), 1.0 );

	return output;

}
