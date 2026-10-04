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
@binding( 0 ) @group( 1 ) var nodeUniform0 : texture_depth_2d;

struct renderStruct {
	cameraProjectionMatrix : mat4x4<f32>,
	cameraViewMatrix : mat4x4<f32>,
	cameraProjectionMatrixInverse : mat4x4<f32>
};
@binding( 0 ) @group( 0 )
var<uniform> render : renderStruct;

// vars
var<private> nodeVar0 : vec2<i32>;
var<private> nodeVar1 : f32;
var<private> nodeVar2 : f32;
var<private> nodeVar3 : f32;
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
var<private> nodeVar23 : vec3<f32>;
var<private> nodeVar24 : vec3<f32>;
var<private> nodeVar25 : vec3<f32>;

// codes


@fragment
fn main( @location( 0 ) nodeVarying4 : vec2<f32> ) -> OutputStruct {

	// flow
	// code

	nodeVar0 = vec2<i32>( ( nodeVarying4 * vec2<f32>( textureDimensions( nodeUniform0, 0 ) ) ) );
	nodeVar1 = textureLoad( nodeUniform0, nodeVar0, u32( 0u ) );
	nodeVar2 = nodeVar1;
	nodeVar3 = textureLoad( nodeUniform0, ( nodeVar0 - vec2<i32>( 2, 0 ) ), u32( 0u ) );
	nodeVar4 = nodeVar3;
	nodeVar5 = textureLoad( nodeUniform0, ( nodeVar0 - vec2<i32>( 1, 0 ) ), u32( 0u ) );
	nodeVar6 = nodeVar5;
	nodeVar7 = textureLoad( nodeUniform0, ( nodeVar0 + vec2<i32>( 1, 0 ) ), u32( 0u ) );
	nodeVar8 = nodeVar7;
	nodeVar9 = textureLoad( nodeUniform0, ( nodeVar0 + vec2<i32>( 2, 0 ) ), u32( 0u ) );
	nodeVar10 = nodeVar9;
	nodeVar11 = textureLoad( nodeUniform0, ( nodeVar0 + vec2<i32>( 0, 2 ) ), u32( 0u ) );
	nodeVar12 = nodeVar11;
	nodeVar13 = textureLoad( nodeUniform0, ( nodeVar0 + vec2<i32>( 0, 1 ) ), u32( 0u ) );
	nodeVar14 = nodeVar13;
	nodeVar15 = textureLoad( nodeUniform0, ( nodeVar0 - vec2<i32>( 0, 1 ) ), u32( 0u ) );
	nodeVar16 = nodeVar15;
	nodeVar17 = textureLoad( nodeUniform0, ( nodeVar0 - vec2<i32>( 0, 2 ) ), u32( 0u ) );
	nodeVar18 = nodeVar17;
	nodeVar19 = abs( ( ( ( 2.0 * nodeVar6 ) - nodeVar4 ) - nodeVar2 ) );
	nodeVar20 = abs( ( ( ( 2.0 * nodeVar8 ) - nodeVar10 ) - nodeVar2 ) );
	nodeVar21 = abs( ( ( ( 2.0 * nodeVar14 ) - nodeVar12 ) - nodeVar2 ) );
	nodeVar22 = abs( ( ( ( 2.0 * nodeVar16 ) - nodeVar18 ) - nodeVar2 ) );
	let nodeConst0 = ( render.cameraProjectionMatrixInverse * vec4<f32>( vec3<f32>( ( ( vec2<f32>( nodeVarying4.x, ( 1.0 - nodeVarying4.y ) ) * vec2<f32>( 2.0 ) ) - vec2<f32>( 1.0 ) ), nodeVar2 ), 1.0 ) );
	nodeVar23 = ( nodeConst0.xyz / vec3<f32>( nodeConst0.w ) );

	if ( ( nodeVar19 < nodeVar20 ) ) {

		let nodeConst1 = ( nodeVarying4 - vec2<f32>( ( 1.0 / f32( textureDimensions( nodeUniform0, 0 ).x ) ), 0.0 ) );
		let nodeConst2 = ( render.cameraProjectionMatrixInverse * vec4<f32>( vec3<f32>( ( ( vec2<f32>( nodeConst1.x, ( 1.0 - nodeConst1.y ) ) * vec2<f32>( 2.0 ) ) - vec2<f32>( 1.0 ) ), nodeVar6 ), 1.0 ) );
		nodeVar24 = ( nodeVar23 - ( nodeConst2.xyz / vec3<f32>( nodeConst2.w ) ) );

	} else {

		let nodeConst3 = ( nodeVarying4 + vec2<f32>( ( 1.0 / f32( textureDimensions( nodeUniform0, 0 ).x ) ), 0.0 ) );
		let nodeConst4 = ( render.cameraProjectionMatrixInverse * vec4<f32>( vec3<f32>( ( ( vec2<f32>( nodeConst3.x, ( 1.0 - nodeConst3.y ) ) * vec2<f32>( 2.0 ) ) - vec2<f32>( 1.0 ) ), nodeVar8 ), 1.0 ) );
		nodeVar24 = ( ( - nodeVar23 ) + ( nodeConst4.xyz / vec3<f32>( nodeConst4.w ) ) );

	}


	if ( ( nodeVar21 < nodeVar22 ) ) {

		let nodeConst5 = ( nodeVarying4 + vec2<f32>( 0.0, ( 1.0 / f32( textureDimensions( nodeUniform0, 0 ).y ) ) ) );
		let nodeConst6 = ( render.cameraProjectionMatrixInverse * vec4<f32>( vec3<f32>( ( ( vec2<f32>( nodeConst5.x, ( 1.0 - nodeConst5.y ) ) * vec2<f32>( 2.0 ) ) - vec2<f32>( 1.0 ) ), nodeVar14 ), 1.0 ) );
		nodeVar25 = ( nodeVar23 - ( nodeConst6.xyz / vec3<f32>( nodeConst6.w ) ) );

	} else {

		let nodeConst7 = ( nodeVarying4 - vec2<f32>( 0.0, ( 1.0 / f32( textureDimensions( nodeUniform0, 0 ).y ) ) ) );
		let nodeConst8 = ( render.cameraProjectionMatrixInverse * vec4<f32>( vec3<f32>( ( ( vec2<f32>( nodeConst7.x, ( 1.0 - nodeConst7.y ) ) * vec2<f32>( 2.0 ) ) - vec2<f32>( 1.0 ) ), nodeVar16 ), 1.0 ) );
		nodeVar25 = ( ( - nodeVar23 ) + ( nodeConst8.xyz / vec3<f32>( nodeConst8.w ) ) );

	}


	// result

	output.color = vec4<f32>( normalize( cross( nodeVar24, nodeVar25 ) ), 1.0 );

	return output;

}
