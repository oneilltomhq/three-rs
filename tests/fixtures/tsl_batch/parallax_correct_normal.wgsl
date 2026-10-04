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

struct renderStruct {
	cameraProjectionMatrix : mat4x4<f32>,
	cameraViewMatrix : mat4x4<f32>
};
@binding( 0 ) @group( 0 )
var<uniform> render : renderStruct;

// vars
var<private> normalViewGeometry : vec3<f32>;
var<private> normalView : vec3<f32>;
var<private> normalWorld : vec3<f32>;
var<private> nodeVar0 : vec3<f32>;
var<private> nodeVar1 : vec3<f32>;
var<private> nodeVar2 : vec3<f32>;
var<private> nodeVar3 : vec3<f32>;
var<private> nodeVar4 : f32;
var<private> nodeVar5 : f32;
var<private> nodeVar6 : f32;
var<private> nodeVar7 : f32;
var<private> nodeVar8 : vec3<f32>;

// codes


@fragment
fn main( @location( 0 ) v_normalViewGeometry : vec3<f32>,
	@location( 1 ) v_positionWorld : vec3<f32> ) -> OutputStruct {

	// flow
	// code

	normalViewGeometry = normalize( v_normalViewGeometry );
	normalView = normalViewGeometry;
	normalWorld = normalize( ( vec4<f32>( normalView, 0.0 ) * render.cameraViewMatrix ).xyz );
	nodeVar0 = normalize( normalWorld );
	nodeVar1 = ( ( ( ( vec3<f32>( 200.0, 100.0, 100.0 ) * vec3<f32>( 0.5 ) ) + vec3<f32>( 0.0, -50.0, 0.0 ) ) - v_positionWorld ) / nodeVar0 );
	nodeVar2 = ( ( ( ( vec3<f32>( 200.0, 100.0, 100.0 ) * vec3<f32>( -0.5 ) ) + vec3<f32>( 0.0, -50.0, 0.0 ) ) - v_positionWorld ) / nodeVar0 );
	nodeVar3 = vec3<f32>( 0.0, 0.0, 0.0 );

	if ( ( nodeVar0.x > 0.0 ) ) {

		nodeVar4 = nodeVar1.x;

	} else {

		nodeVar4 = nodeVar2.x;

	}

	nodeVar3.x = nodeVar4;

	if ( ( nodeVar0.y > 0.0 ) ) {

		nodeVar5 = nodeVar1.y;

	} else {

		nodeVar5 = nodeVar2.y;

	}

	nodeVar3.y = nodeVar5;

	if ( ( nodeVar0.z > 0.0 ) ) {

		nodeVar6 = nodeVar1.z;

	} else {

		nodeVar6 = nodeVar2.z;

	}

	nodeVar3.z = nodeVar6;
	nodeVar7 = min( min( nodeVar3.x, nodeVar3.y ), nodeVar3.z );
	nodeVar8 = ( v_positionWorld + ( nodeVar0 * vec3<f32>( nodeVar7 ) ) );

	// result

	output.color = vec4<f32>( ( nodeVar8 - vec3<f32>( 0.0, -50.0, 0.0 ) ), 1.0 );

	return output;

}
