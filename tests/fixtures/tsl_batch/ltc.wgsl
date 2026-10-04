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

struct objectStruct {
	nodeUniform0 : mat4x4<f32>,
	nodeUniform3 : mat4x4<f32>
};
@binding( 0 ) @group( 1 )
var<uniform> object : objectStruct;

// vars


// codes
fn LTC_EdgeVectorFormFactor ( v1 : vec3<f32>, v2 : vec3<f32> ) -> vec3<f32> {

	var nodeVar0 : f32;
	var nodeVar1 : f32;
	var nodeVar2 : f32;
	var nodeVar3 : f32;

	let nodeConst0 = dot( v1, v2 );
	nodeVar0 = abs( nodeConst0 );
	nodeVar1 = ( ( ( ( nodeVar0 * 0.0145206 ) + 0.4965155 ) * nodeVar0 ) + 0.8543985 );
	nodeVar2 = ( ( ( nodeVar0 + 4.1616724 ) * nodeVar0 ) + 3.417594 );

	if ( ( nodeConst0 > 0.0 ) ) {

		nodeVar3 = ( nodeVar1 / nodeVar2 );

	} else {

		nodeVar3 = ( ( inverseSqrt( max( ( 1.0 - ( nodeConst0 * nodeConst0 ) ), 1e-7 ) ) * 0.5 ) - ( nodeVar1 / nodeVar2 ) );

	}


	return ( cross( v1, v2 ) * vec3<f32>( nodeVar3 ) );

}


fn LTC_ClippedSphereFormFactor ( f : vec3<f32> ) -> f32 {

	

	let nodeConst0 = length( f );

	return max( ( ( ( nodeConst0 * nodeConst0 ) + f.z ) / ( nodeConst0 + 1.0 ) ), 0.0 );

}


fn LTC_Evaluate ( N : vec3<f32>, V : vec3<f32>, P : vec3<f32>, mInv : mat3x3<f32>, p0 : vec3<f32>, p1 : vec3<f32>, p2 : vec3<f32>, p3 : vec3<f32> ) -> vec3<f32> {

	var nodeVar0 : vec3<f32>;
	var nodeVar1 : vec3<f32>;
	var nodeVar2 : vec3<f32>;
	var nodeVar3 : mat3x3<f32>;
	var nodeVar4 : vec3<f32>;
	var nodeVar5 : vec3<f32>;
	var nodeVar6 : vec3<f32>;
	var nodeVar7 : vec3<f32>;
	var nodeVar8 : vec3<f32>;

	nodeVar0 = ( p1 - p0 );
	nodeVar1 = ( p3 - p0 );
	nodeVar2 = vec3<f32>( 0.0, 0.0, 0.0 );

	if ( ( dot( cross( nodeVar0, nodeVar1 ), ( P - p0 ) ) >= 0.0 ) ) {

		let nodeConst0 = normalize( ( V - ( N * vec3<f32>( dot( V, N ) ) ) ) );
		nodeVar3 = ( mInv * transpose( mat3x3<f32>( nodeConst0, ( - cross( N, nodeConst0 ) ), N ) ) );
		nodeVar4 = normalize( ( nodeVar3 * ( p0 - P ) ) );
		nodeVar5 = normalize( ( nodeVar3 * ( p1 - P ) ) );
		nodeVar6 = normalize( ( nodeVar3 * ( p2 - P ) ) );
		nodeVar7 = normalize( ( nodeVar3 * ( p3 - P ) ) );
		nodeVar8 = vec3<f32>( 0.0, 0.0, 0.0 );
		nodeVar8 = ( nodeVar8 + LTC_EdgeVectorFormFactor( nodeVar4, nodeVar5 ) );
		nodeVar8 = ( nodeVar8 + LTC_EdgeVectorFormFactor( nodeVar5, nodeVar6 ) );
		nodeVar8 = ( nodeVar8 + LTC_EdgeVectorFormFactor( nodeVar6, nodeVar7 ) );
		nodeVar8 = ( nodeVar8 + LTC_EdgeVectorFormFactor( nodeVar7, nodeVar4 ) );
		nodeVar2 = vec3<f32>( LTC_ClippedSphereFormFactor( nodeVar8 ) );
		

	}


	return nodeVar2;

}


fn LTC_Evaluate_Volume ( P : vec3<f32>, p0 : vec3<f32>, p1 : vec3<f32>, p2 : vec3<f32>, p3 : vec3<f32> ) -> vec3<f32> {

	var nodeVar0 : vec3<f32>;
	var nodeVar1 : vec3<f32>;
	var nodeVar2 : vec3<f32>;
	var nodeVar3 : vec3<f32>;
	var nodeVar4 : vec3<f32>;
	var nodeVar5 : vec3<f32>;
	var nodeVar6 : vec3<f32>;
	var nodeVar7 : vec3<f32>;

	nodeVar0 = ( p1 - p0 );
	nodeVar1 = ( p3 - p0 );
	nodeVar2 = vec3<f32>( 0.0, 0.0, 0.0 );

	if ( ( dot( cross( nodeVar0, nodeVar1 ), ( P - p0 ) ) >= 0.0 ) ) {

		nodeVar3 = normalize( ( p0 - P ) );
		nodeVar4 = normalize( ( p1 - P ) );
		nodeVar5 = normalize( ( p2 - P ) );
		nodeVar6 = normalize( ( p3 - P ) );
		nodeVar7 = vec3<f32>( 0.0, 0.0, 0.0 );
		nodeVar7 = ( nodeVar7 + LTC_EdgeVectorFormFactor( nodeVar3, nodeVar4 ) );
		nodeVar7 = ( nodeVar7 + LTC_EdgeVectorFormFactor( nodeVar4, nodeVar5 ) );
		nodeVar7 = ( nodeVar7 + LTC_EdgeVectorFormFactor( nodeVar5, nodeVar6 ) );
		nodeVar7 = ( nodeVar7 + LTC_EdgeVectorFormFactor( nodeVar6, nodeVar3 ) );
		nodeVar2 = vec3<f32>( LTC_ClippedSphereFormFactor( abs( nodeVar7 ) ) );
		

	}


	return nodeVar2;

}


fn LTC_Uv ( N : vec3<f32>, V : vec3<f32>, roughness : f32 ) -> vec2<f32> {

	

	var nodeVar0 : vec2<f32> = vec2<f32>( roughness, sqrt( ( 1.0 - clamp( dot( N, V ), 0.0, 1.0 ) ) ) );
	nodeVar0 = ( ( nodeVar0 * vec2<f32>( 0.984375 ) ) + vec2<f32>( 0.0078125 ) );

	return nodeVar0;

}




@fragment
fn main( @location( 0 ) nodeVarying4 : vec2<f32> ) -> OutputStruct {

	// flow
	// code

	let nodeConst0 = normalize( vec3<f32>( nodeVarying4, 1.0 ) );
	let nodeConst1 = vec3<f32>( nodeVarying4, 0.0 );

	// result

	output.color = ( vec4<f32>( ( LTC_Evaluate( vec3<f32>( 0.0, 0.0, 1.0 ), nodeConst0, nodeConst1, mat3x3<f32>( object.nodeUniform0[ 0 ].xyz, object.nodeUniform0[ 1 ].xyz, object.nodeUniform0[ 2 ].xyz ), vec3<f32>( -1.0, -1.0, 2.0 ), vec3<f32>( 1.0, -1.0, 2.0 ), vec3<f32>( 1.0, 1.0, 2.0 ), vec3<f32>( -1.0, 1.0, 2.0 ) ) + LTC_Evaluate_Volume( nodeConst1, vec3<f32>( -1.0, -1.0, 2.0 ), vec3<f32>( 1.0, -1.0, 2.0 ), vec3<f32>( 1.0, 1.0, 2.0 ), vec3<f32>( -1.0, 1.0, 2.0 ) ) ), 1.0 ) + vec4<f32>( LTC_Uv( vec3<f32>( 0.0, 0.0, 1.0 ), nodeConst0, nodeVarying4.x ), 0.0, 0.0 ) );

	return output;

}
