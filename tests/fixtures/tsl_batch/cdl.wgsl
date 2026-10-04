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


@fragment
fn main( @location( 0 ) nodeVarying4 : vec2<f32> ) -> OutputStruct {

	// flow
	// code

	let nodeConst0 = vec4<f32>( nodeVarying4, 0.5, 1.0 );
	var nodeVar0 : vec3<f32> = max( ( ( nodeConst0.xyz * vec3<f32>( 1.1, 1.0, 0.9 ) ) + vec3<f32>( 0.1, 0.1, 0.1 ) ), vec3<f32>( 0.0 ) );

	if ( ( nodeVar0.x > 0.0 ) ) {

		nodeVar0.x = pow( nodeVar0, vec3<f32>( 1.2, 1.2, 1.2 ) ).x;
		

	}


	if ( ( nodeVar0.y > 0.0 ) ) {

		nodeVar0.y = pow( nodeVar0, vec3<f32>( 1.2, 1.2, 1.2 ) ).y;
		

	}


	if ( ( nodeVar0.z > 0.0 ) ) {

		nodeVar0.z = pow( nodeVar0, vec3<f32>( 1.2, 1.2, 1.2 ) ).z;
		

	}

	let nodeConst1 = dot( nodeConst0.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) );
	nodeVar0 = max( ( vec3<f32>( nodeConst1 ) + ( ( nodeVar0 - vec3<f32>( nodeConst1 ) ) * vec3<f32>( 0.9 ) ) ), vec3<f32>( 0.0 ) );
	let nodeConst2 = vec4<f32>( nodeVarying4.y, nodeVarying4.x, 0.25, 1.0 );
	var nodeVar1 : vec3<f32> = max( ( ( nodeConst2.xyz * vec3<f32>( 1.0, 1.0, 1.0 ) ) + vec3<f32>( 0.0, 0.0, 0.0 ) ), vec3<f32>( 0.0 ) );

	if ( ( nodeVar1.x > 0.0 ) ) {

		nodeVar1.x = pow( nodeVar1, vec3<f32>( 1.0, 1.0, 1.0 ) ).x;
		

	}


	if ( ( nodeVar1.y > 0.0 ) ) {

		nodeVar1.y = pow( nodeVar1, vec3<f32>( 1.0, 1.0, 1.0 ) ).y;
		

	}


	if ( ( nodeVar1.z > 0.0 ) ) {

		nodeVar1.z = pow( nodeVar1, vec3<f32>( 1.0, 1.0, 1.0 ) ).z;
		

	}

	let nodeConst3 = dot( nodeConst2.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) );
	nodeVar1 = max( ( vec3<f32>( nodeConst3 ) + ( ( nodeVar1 - vec3<f32>( nodeConst3 ) ) * vec3<f32>( 1.0 ) ) ), vec3<f32>( 0.0 ) );

	// result

	output.color = ( vec4<f32>( nodeVar0, nodeConst0.w ) + vec4<f32>( nodeVar1, nodeConst2.w ) );

	return output;

}
