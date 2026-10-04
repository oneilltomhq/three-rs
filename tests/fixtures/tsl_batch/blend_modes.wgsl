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
fn blendBurn ( base : vec3<f32>, blend : vec3<f32> ) -> vec3<f32> {

	


	return ( vec3<f32>( 1.0 ) - min( vec3<f32>( 1.0 ), ( ( vec3<f32>( 1.0 ) - base ) / blend ) ) );

}


fn blendDodge ( base : vec3<f32>, blend : vec3<f32> ) -> vec3<f32> {

	


	return min( ( base / ( vec3<f32>( 1.0 ) - blend ) ), vec3<f32>( 1.0 ) );

}


fn blendScreen ( base : vec3<f32>, blend : vec3<f32> ) -> vec3<f32> {

	


	return ( vec3<f32>( 1.0 ) - ( ( vec3<f32>( 1.0 ) - base ) * ( vec3<f32>( 1.0 ) - blend ) ) );

}


fn blendColor ( base : vec4<f32>, blend : vec4<f32> ) -> vec4<f32> {

	

	let nodeConst0 = ( blend.w + ( base.w * ( 1.0 - blend.w ) ) );

	return vec4<f32>( ( ( ( blend.xyz * vec3<f32>( blend.w ) ) + ( ( base.xyz * vec3<f32>( base.w ) ) * vec3<f32>( ( 1.0 - blend.w ) ) ) ) / vec3<f32>( nodeConst0 ) ), nodeConst0 );

}




@fragment
fn main( @location( 0 ) nodeVarying4 : vec2<f32> ) -> OutputStruct {

	// flow
	// code

	let nodeConst0 = vec3<f32>( nodeVarying4, 0.5 );
	let nodeConst1 = vec3<f32>( nodeVarying4.y, 0.25, nodeVarying4.x );

	// result

	output.color = ( vec4<f32>( ( ( blendBurn( nodeConst0, nodeConst1 ) + blendDodge( nodeConst0, nodeConst1 ) ) + blendScreen( nodeConst0, nodeConst1 ) ), 1.0 ) + blendColor( vec4<f32>( nodeConst0, nodeVarying4.x ), vec4<f32>( nodeConst1, nodeVarying4.y ) ) );

	return output;

}
