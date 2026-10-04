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

struct NodeBuffer_1045Struct {
	value : array< vec4<f32>, 1 >
};
@binding( 0 ) @group( 0 )
var<uniform> NodeBuffer_1045 : NodeBuffer_1045Struct;

struct NodeBuffer_1049Struct {
	value : array< vec4<f32>, 2 >
};
@binding( 1 ) @group( 0 )
var<uniform> NodeBuffer_1049 : NodeBuffer_1049Struct;

struct objectStruct {
	nodeUniform0 : vec3<f32>,
	nodeUniform1 : f32,
	nodeUniform3 : mat4x4<f32>
};
@binding( 0 ) @group( 1 )
var<uniform> object : objectStruct;

// vars
var<private> DiffuseColor : vec4<f32>;
var<private> distanceToPlane : f32;
var<private> distanceToGradient : f32;
var<private> clipOpacity : f32;
var<private> intersectionClipOpacity : f32;
var<private> Output : vec4<f32>;

// codes


@fragment
fn main( @location( 0 ) v_positionView : vec3<f32> ) -> OutputStruct {

	// flow
	// code

	DiffuseColor = vec4<f32>( object.nodeUniform0, 1.0 );
	DiffuseColor.w = ( DiffuseColor.w * object.nodeUniform1 );
	distanceToPlane = 0.0;
	distanceToGradient = 0.0;
	clipOpacity = 1.0;

	for ( var i : i32 = 0; i < 1; i ++ ) {

		distanceToPlane = ( ( - dot( v_positionView, NodeBuffer_1045.value[ i ].xyz ) ) + NodeBuffer_1045.value[ i ].w );
		distanceToGradient = ( fwidth( distanceToPlane ) / 2.0 );
		clipOpacity = ( clipOpacity * smoothstep( ( - distanceToGradient ), distanceToGradient, distanceToPlane ) );

	}

	intersectionClipOpacity = 1.0;

	for ( var i : i32 = 0; i < 2; i ++ ) {

		distanceToPlane = ( ( - dot( v_positionView, NodeBuffer_1049.value[ i ].xyz ) ) + NodeBuffer_1049.value[ i ].w );
		distanceToGradient = ( fwidth( distanceToPlane ) / 2.0 );
		intersectionClipOpacity = ( intersectionClipOpacity * ( 1.0 - smoothstep( ( - distanceToGradient ), distanceToGradient, distanceToPlane ) ) );

	}

	clipOpacity = ( clipOpacity * ( 1.0 - intersectionClipOpacity ) );
	DiffuseColor.w = ( DiffuseColor.w * clipOpacity );

	if ( ( DiffuseColor.w == 0.0 ) ) {

		discard;
		

	}

	let nodeConst0 = max( vec4<f32>( DiffuseColor.xyz, DiffuseColor.w ), vec4<f32>( 0.0 ) );
	Output = nodeConst0;

	// result

	output.color = nodeConst0;

	return output;

}
