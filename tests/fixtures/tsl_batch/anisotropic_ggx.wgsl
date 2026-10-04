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


// codes
fn D_GGX_Anisotropic ( alphaT : f32, alphaB : f32, dotNH : f32, dotTH : f32, dotBH : f32 ) -> f32 {

	

	let nodeConst0 = ( alphaT * alphaB );
	let nodeConst1 = vec3<f32>( ( alphaB * dotTH ), ( alphaT * dotBH ), ( nodeConst0 * dotNH ) );
	let nodeConst2 = ( nodeConst0 / dot( nodeConst1, nodeConst1 ) );

	return ( 0.3183098861837907 * ( nodeConst0 * ( nodeConst2 * nodeConst2 ) ) );

}


fn V_GGX_SmithCorrelated_Anisotropic ( alphaT : f32, alphaB : f32, dotTV : f32, dotBV : f32, dotTL : f32, dotBL : f32, dotNV : f32, dotNL : f32 ) -> f32 {

	


	return ( 0.5 / max( ( ( dotNL * length( vec3<f32>( ( alphaT * dotTV ), ( alphaB * dotBV ), dotNV ) ) ) + ( dotNV * length( vec3<f32>( ( alphaT * dotTL ), ( alphaB * dotBL ), dotNL ) ) ) ), 0.000001 ) );

}




@fragment
fn main( @location( 0 ) nodeVarying4 : vec2<f32> ) -> OutputStruct {

	// flow
	// code


	// result

	output.color = vec4<f32>( D_GGX_Anisotropic( nodeVarying4.x, nodeVarying4.y, ( nodeVarying4.x * 0.5 ), ( nodeVarying4.y * 0.25 ), ( nodeVarying4.x * nodeVarying4.y ) ), V_GGX_SmithCorrelated_Anisotropic( nodeVarying4.x, nodeVarying4.y, 0.5, 0.25, ( nodeVarying4.x * 0.5 ), ( nodeVarying4.y * 0.5 ), nodeVarying4.x, nodeVarying4.y ), 0.0, 1.0 );

	return output;

}
