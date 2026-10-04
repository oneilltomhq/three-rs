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


// vars
var<private> nodeVar0 : f32;
var<private> nodeVar1 : f32;
var<private> nodeVar2 : f32;
var<private> nodeVar3 : vec3<f32>;
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

// codes
fn equirectUvToDir ( uv : vec2<f32> ) -> vec3<f32> {

	

	let nodeConst0 = ( ( uv.y - 0.5 ) * 3.141592653589793 );
	let nodeConst1 = cos( nodeConst0 );
	let nodeConst2 = ( ( uv.x * 6.283185307179586 ) - 3.141592653589793 );

	return normalize( vec3<f32>( ( nodeConst1 * cos( nodeConst2 ) ), sin( nodeConst0 ), ( nodeConst1 * sin( nodeConst2 ) ) ) );

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


fn getSpecularDominantFactor ( NoV : f32, roughness : f32 ) -> f32 {

	

	let nodeConst0 = ( 0.298475 * log( ( 39.4115 - ( 39.0029 * roughness ) ) ) );

	return clamp( ( ( pow( ( 1.0 - NoV ), 10.8649 ) * ( 1.0 - nodeConst0 ) ) + nodeConst0 ), 0.0, 1.0 );

}




@fragment
fn main( @location( 0 ) nodeVarying0 : vec2<f32> ) -> OutputStruct {

	// flow
	// code

	let nodeConst0 = equirectUvToDir( nodeVarying0 );
	nodeVar0 = ( 1.0 - nodeVarying0.y );
	nodeVar1 = ( nodeVar0 * nodeVar0 );
	nodeVar2 = ( ( nodeVar1 * nodeVar1 ) * nodeVar0 );
	nodeVar3 = ( vec3<f32>( 0.04, 0.04, 0.04 ) + ( ( vec3<f32>( 1.0, 1.0, 1.0 ) - vec3<f32>( 0.04, 0.04, 0.04 ) ) * vec3<f32>( nodeVar2 ) ) );
	nodeVar4 = ( nodeVarying0.x * nodeVarying0.x );
	nodeVar5 = ( nodeVarying0.y * nodeVarying0.y );
	nodeVar6 = ( ( nodeVar5 * ( nodeVar4 - 1.0 ) ) + 1.0 );
	nodeVar7 = ( nodeVar4 / ( 3.141592653589793 * pow( nodeVar6, 2.0 ) ) );
	nodeVar8 = ( nodeVarying0.x * nodeVarying0.x );
	nodeVar9 = ( nodeVarying0.y * nodeVarying0.y );
	nodeVar10 = ( 0.5 * 0.5 );
	nodeVar11 = ( nodeVarying0.y * nodeVarying0.y );
	nodeVar12 = ( ( 2.0 * nodeVarying0.y ) / ( nodeVarying0.y + sqrt( ( nodeVar10 + ( ( 1.0 - nodeVar10 ) * nodeVar11 ) ) ) ) );
	nodeVar13 = ( 0.5 * 0.5 );
	nodeVar14 = ( nodeVarying0.x * nodeVarying0.x );
	nodeVar15 = ( ( 2.0 * nodeVarying0.x ) / ( nodeVarying0.x + sqrt( ( nodeVar13 + ( ( 1.0 - nodeVar13 ) * nodeVar14 ) ) ) ) );
	nodeVar16 = ( nodeVar12 * nodeVar15 );

	// result

	output.color = vec4<f32>( ( ( nodeConst0 * vec3<f32>( misPowerHeuristic( equirectDirPdf( nodeConst0 ), nodeVarying0.x ) ) ) + nodeVar3 ), ( ( ( nodeVar7 + ( ( 2.0 * nodeVarying0.y ) / ( nodeVarying0.y + sqrt( ( nodeVar8 + ( ( 1.0 - nodeVar8 ) * nodeVar9 ) ) ) ) ) ) + nodeVar16 ) + getSpecularDominantFactor( nodeVarying0.y, nodeVarying0.x ) ) );

	return output;

}
