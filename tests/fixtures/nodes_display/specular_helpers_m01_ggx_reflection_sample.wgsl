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


// vars
var<private> nodeVar0 : f32;
var<private> nodeVar1 : f32;
var<private> nodeVar2 : f32;
var<private> nodeVar3 : vec3<f32>;
var<private> nodeVar4 : vec3<f32>;
var<private> nodeVar5 : vec3<f32>;
var<private> nodeVar6 : vec3<f32>;
var<private> nodeVar7 : vec3<f32>;
var<private> nodeVar8 : vec3<f32>;
var<private> nodeVar9 : vec3<f32>;
var<private> nodeVar10 : vec3<f32>;
var<private> nodeVar11 : vec3<f32>;
var<private> nodeVar12 : f32;
var<private> nodeVar13 : f32;
var<private> nodeVar14 : f32;
var<private> nodeVar15 : f32;
var<private> nodeVar16 : vec3<f32>;
var<private> nodeVar17 : f32;
var<private> nodeVar18 : f32;
var<private> nodeVar19 : f32;
var<private> nodeVar20 : vec3<f32>;
var<private> nodeVar21 : vec3<f32>;
var<private> nodeVar22 : f32;
var<private> nodeVar23 : f32;
var<private> nodeVar24 : f32;
var<private> nodeVar25 : f32;
var<private> nodeVar26 : f32;
var<private> nodeVar27 : f32;
var<private> nodeVar28 : f32;
var<private> nodeVar29 : f32;
var<private> nodeVar30 : f32;
var<private> nodeVar31 : f32;
var<private> nodeVar32 : f32;
var<private> nodeVar33 : f32;
var<private> nodeVar34 : f32;
var<private> nodeVar35 : f32;
var<private> nodeVar36 : f32;
var<private> nodeVar37 : f32;
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
var<private> nodeVar49 : StructType0;
var<private> nodeVar50 : StructType0;

// codes
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




@fragment
fn main( @location( 0 ) nodeVarying0 : vec2<f32> ) -> OutputStruct {

	// flow
	// code

	nodeVar0 = max( ( nodeVarying0.x * nodeVarying0.x ), 0.001 );
	nodeVar1 = nodeVar0;
	nodeVar2 = nodeVar0;
	let nodeConst0 = normalize( vec3<f32>( ( nodeVarying0 - vec2<f32>( 0.5 ) ), 1.0 ) );
	nodeVar3 = cross( vec3<f32>( 0.0, 0.0, 1.0 ), nodeConst0 );
	nodeVar4 = normalize( nodeVar3 );

	if ( ( length( nodeVar4 ) < 0.001 ) ) {

		nodeVar4 = normalize( cross( vec3<f32>( 0.0, 1.0, 0.0 ), nodeConst0 ) );
		

	}

	nodeVar5 = normalize( cross( nodeConst0, nodeVar4 ) );
	let nodeConst1 = normalize( vec3<f32>( ( nodeVarying0.yx - vec2<f32>( 0.5 ) ), 1.0 ) );
	nodeVar6 = vec3<f32>( dot( nodeVar4, nodeConst1 ), dot( nodeVar5, nodeConst1 ), dot( nodeConst0, nodeConst1 ) );
	let nodeConst2 = vec4<f32>( nodeVarying0, nodeVarying0.yx );
	nodeVar7 = SampleGGXVNDF( nodeVar6, nodeVar1, nodeVar2, nodeConst2.x, nodeConst2.y );

	if ( ( nodeVar7.z < 0.0 ) ) {

		nodeVar7 = ( - nodeVar7 );
		

	}

	nodeVar8 = normalize( ( ( ( nodeVar4 * vec3<f32>( nodeVar7.x ) ) + ( nodeVar5 * vec3<f32>( nodeVar7.y ) ) ) + ( nodeConst0 * vec3<f32>( nodeVar7.z ) ) ) );
	nodeVar9 = normalize( reflect( ( - nodeConst1 ), nodeVar8 ) );
	nodeVar10 = nodeVar9;
	nodeVar11 = normalize( ( nodeConst1 + nodeVar10 ) );
	nodeVar12 = max( 0.0, dot( nodeConst0, nodeConst1 ) );
	nodeVar13 = max( 0.0, dot( nodeConst0, nodeVar10 ) );
	nodeVar14 = max( 0.0, dot( nodeConst0, nodeVar11 ) );
	nodeVar15 = max( 0.0, dot( nodeConst1, nodeVar11 ) );
	nodeVar16 = mix( vec3<f32>( 0.04, 0.04, 0.04 ), vec3<f32>( 0.9, 0.6, 0.3 ), nodeVarying0.y );
	nodeVar17 = ( 1.0 - nodeVar15 );
	nodeVar18 = ( nodeVar17 * nodeVar17 );
	nodeVar19 = ( ( nodeVar18 * nodeVar18 ) * nodeVar17 );
	nodeVar20 = ( nodeVar16 + ( ( vec3<f32>( 1.0, 1.0, 1.0 ) - nodeVar16 ) * vec3<f32>( nodeVar19 ) ) );
	nodeVar21 = nodeVar20;
	nodeVar22 = ( nodeVar1 * nodeVar1 );
	nodeVar23 = ( nodeVar14 * nodeVar14 );
	nodeVar24 = ( ( nodeVar23 * ( nodeVar22 - 1.0 ) ) + 1.0 );
	nodeVar25 = ( nodeVar22 / ( 3.141592653589793 * pow( nodeVar24, 2.0 ) ) );
	nodeVar26 = nodeVar25;
	nodeVar27 = ( nodeVar1 * nodeVar1 );
	nodeVar28 = max( 0.0, ( 1.0 - ( nodeVar12 * nodeVar12 ) ) );
	nodeVar29 = ( 1.0 + sqrt( nodeVar28 ) );
	nodeVar30 = ( nodeVar29 * nodeVar29 );
	nodeVar31 = ( ( ( 1.0 - nodeVar27 ) * nodeVar30 ) / ( nodeVar30 + ( ( nodeVar27 * nodeVar12 ) * nodeVar12 ) ) );
	nodeVar32 = sqrt( ( ( nodeVar27 * nodeVar28 ) + ( nodeVar12 * nodeVar12 ) ) );
	nodeVar33 = ( nodeVar26 / max( 0.000001, ( 2.0 * ( ( nodeVar31 * nodeVar12 ) + nodeVar32 ) ) ) );
	nodeVar34 = nodeVar33;
	nodeVar35 = ( nodeVar1 * nodeVar1 );
	nodeVar36 = max( ( 1.0 - ( nodeVar12 * nodeVar12 ) ), 0.0 );
	nodeVar37 = ( 1.0 + sqrt( nodeVar36 ) );
	nodeVar38 = ( nodeVar37 * nodeVar37 );
	nodeVar39 = ( ( ( 1.0 - nodeVar35 ) * nodeVar38 ) / ( nodeVar38 + ( ( nodeVar35 * nodeVar12 ) * nodeVar12 ) ) );
	nodeVar40 = sqrt( ( ( nodeVar35 * nodeVar36 ) + ( nodeVar12 * nodeVar12 ) ) );
	nodeVar41 = ( nodeVar1 * nodeVar1 );
	nodeVar42 = ( nodeVar12 * nodeVar12 );
	nodeVar43 = ( ( 2.0 * nodeVar12 ) / ( nodeVar12 + sqrt( ( nodeVar41 + ( ( 1.0 - nodeVar41 ) * nodeVar42 ) ) ) ) );
	nodeVar44 = ( nodeVar1 * nodeVar1 );
	nodeVar45 = ( nodeVar13 * nodeVar13 );
	nodeVar46 = ( ( 2.0 * nodeVar13 ) / ( nodeVar13 + sqrt( ( nodeVar44 + ( ( 1.0 - nodeVar44 ) * nodeVar45 ) ) ) ) );
	nodeVar47 = ( nodeVar43 * nodeVar46 );
	nodeVar48 = ( ( ( nodeVar21 * vec3<f32>( nodeVar47 ) ) * vec3<f32>( ( ( nodeVar39 * nodeVar12 ) + nodeVar40 ) ) ) / vec3<f32>( max( ( 2.0 * nodeVar12 ), 0.0001 ) ) );
	nodeVar49 = StructType0( nodeVar9, nodeVar48, nodeVar34, nodeVar12, nodeVar1, nodeVar16 );
	nodeVar50 = nodeVar49;

	// result

	output.color = vec4<f32>( ( ( nodeVar50.reflectDir + nodeVar50.sampleWeight ) + nodeVar50.f0 ), ( ( nodeVar50.pdf + nodeVar50.NdotV ) + nodeVar50.alpha ) );

	return output;

}
