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
@binding( 0 ) @group( 1 ) var nodeUniform0_sampler : sampler;
@binding( 1 ) @group( 1 ) var nodeUniform0 : texture_2d<f32>;

struct renderStruct {
	cameraProjectionMatrix : mat4x4<f32>,
	cameraViewMatrix : mat4x4<f32>
};
@binding( 0 ) @group( 0 )
var<uniform> render : renderStruct;

// vars
var<private> nodeVar0 : vec4<f32>;
var<private> normalViewGeometry : vec3<f32>;
var<private> normalView : vec3<f32>;
var<private> normalWorld : vec3<f32>;
var<private> nodeVar1 : vec4<f32>;
var<private> nodeVar2 : vec4<f32>;

// codes


@fragment
fn main( @location( 0 ) v_positionWorld : vec3<f32>,
	@location( 1 ) v_normalViewGeometry : vec3<f32> ) -> OutputStruct {

	// flow
	// code

	nodeVar0 = textureSample( nodeUniform0, nodeUniform0_sampler, ( v_positionWorld.yz * vec2<f32>( 2.0 ) ) );
	normalViewGeometry = normalize( v_normalViewGeometry );
	normalView = normalViewGeometry;
	normalWorld = normalize( ( vec4<f32>( normalView, 0.0 ) * render.cameraViewMatrix ).xyz );
	let nodeConst0 = normalize( abs( normalWorld ) );
	let nodeConst1 = ( nodeConst0 / vec3<f32>( dot( nodeConst0, vec3<f32>( 1.0, 1.0, 1.0 ) ) ) );
	nodeVar1 = textureSample( nodeUniform0, nodeUniform0_sampler, ( v_positionWorld.zx * vec2<f32>( 2.0 ) ) );
	nodeVar2 = textureSample( nodeUniform0, nodeUniform0_sampler, ( v_positionWorld.xy * vec2<f32>( 2.0 ) ) );

	// result

	output.color = ( ( ( nodeVar0 * vec4<f32>( nodeConst1.x ) ) + ( nodeVar1 * vec4<f32>( nodeConst1.y ) ) ) + ( nodeVar2 * vec4<f32>( nodeConst1.z ) ) );

	return output;

}
