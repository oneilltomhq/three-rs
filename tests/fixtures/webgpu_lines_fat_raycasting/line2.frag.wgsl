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
	nodeUniform1 : mat4x4<f32>,
	nodeUniform5 : mat4x4<f32>,
	nodeUniform7 : f32,
	nodeUniform8 : vec3<f32>,
	nodeUniform9 : f32,
	nodeUniform11 : f32
};
@binding( 0 ) @group( 1 )
var<uniform> object : objectStruct;

struct renderStruct {
	cameraWorldMatrix : mat4x4<f32>,
	cameraProjectionMatrixInverse : mat4x4<f32>,
	cameraViewMatrix : mat4x4<f32>,
	cameraProjectionMatrix : mat4x4<f32>,
	nodeUniform6 : vec4<f32>
};
@binding( 0 ) @group( 0 )
var<uniform> render : renderStruct;

// vars
var<private> DiffuseColor : vec4<f32>;
var<private> alpha : f32;
var<private> nodeVar10 : f32;
var<private> nodeVar11 : vec3<f32>;
var<private> nodeVar12 : vec3<f32>;
var<private> Output : vec4<f32>;

// codes
fn fn3 ( p1 : vec3<f32>, p2 : vec3<f32>, p3 : vec3<f32>, p4 : vec3<f32> ) -> vec2<f32> {

	

	let nodeConst0 = ( p1 - p3 );
	let nodeConst1 = ( p4 - p3 );
	let nodeConst2 = dot( nodeConst0, nodeConst1 );
	let nodeConst3 = ( p2 - p1 );
	let nodeConst4 = dot( nodeConst1, nodeConst3 );
	let nodeConst5 = dot( nodeConst1, nodeConst1 );
	let nodeConst6 = clamp( ( ( ( nodeConst2 * nodeConst4 ) - ( dot( nodeConst0, nodeConst3 ) * nodeConst5 ) ) / ( ( dot( nodeConst3, nodeConst3 ) * nodeConst5 ) - ( nodeConst4 * nodeConst4 ) ) ), 0.0, 1.0 );

	return vec2<f32>( nodeConst6, clamp( ( ( nodeConst2 + ( nodeConst4 * nodeConst6 ) ) / nodeConst5 ), 0.0, 1.0 ) );

}




@fragment
fn main( @location( 0 ) worldStart : vec3<f32>,
	@location( 1 ) worldEnd : vec3<f32>,
	@location( 2 ) worldPos : vec4<f32>,
	@location( 3 ) nodeVarying7 : vec3<f32>,
	@location( 4 ) nodeVarying8 : vec3<f32>,
	@location( 5 ) nodeVarying9 : vec3<f32> ) -> OutputStruct {

	// flow
	// code

	DiffuseColor = vec4<f32>( object.nodeUniform8, 1.0 );
	DiffuseColor.w = ( DiffuseColor.w * object.nodeUniform9 );
	alpha = 1.0;
	nodeVar10 = 0.0;
	let nodeConst9 = ( render.cameraProjectionMatrix[ 2u ][ 3u ] != -1.0 );

	if ( nodeConst9 ) {

		let nodeConst10 = ( worldEnd.xy - worldStart.xy );
		nodeVar10 = length( ( ( worldStart.xy + ( nodeConst10 * vec2<f32>( clamp( ( dot( ( worldPos.xy - worldStart.xy ), nodeConst10 ) / dot( nodeConst10, nodeConst10 ) ), 0.0, 1.0 ) ) ) ) - worldPos.xy ) );
		

	} else {

		let nodeConst11 = ( normalize( worldPos.xyz ) * vec3<f32>( 100000.0 ) );
		let nodeConst12 = fn3( worldStart, worldEnd, vec3<f32>( 0.0, 0.0, 0.0 ), nodeConst11 );
		nodeVar10 = length( ( ( worldStart + ( ( worldEnd - worldStart ) * vec3<f32>( nodeConst12.x ) ) ) - ( nodeConst11 * vec3<f32>( nodeConst12.y ) ) ) );
		

	}

	let nodeConst13 = ( nodeVar10 / object.nodeUniform11 );
	let nodeConst14 = fwidth( nodeConst13 );
	alpha = ( 1.0 - smoothstep( ( ( - nodeConst14 ) + 0.5 ), ( nodeConst14 + 0.5 ), nodeConst13 ) );
	DiffuseColor.w = ( DiffuseColor.w * alpha );

	if ( ( nodeVarying7.y < 0.5 ) ) {

		nodeVar11 = nodeVarying8;

	} else {

		nodeVar11 = nodeVarying9;

	}

	nodeVar12 = ( DiffuseColor.xyz * nodeVar11 );
	DiffuseColor.x = nodeVar12[ 0 ];
	DiffuseColor.y = nodeVar12[ 1 ];
	DiffuseColor.z = nodeVar12[ 2 ];
	let nodeConst15 = max( vec4<f32>( DiffuseColor.xyz, DiffuseColor.w ), vec4<f32>( 0.0 ) );
	Output = nodeConst15;

	// result

	output.color = nodeConst15;

	return output;

}
