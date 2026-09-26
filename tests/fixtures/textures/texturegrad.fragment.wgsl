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
@binding( 0 ) @group( 1 ) var nodeUniform1_sampler : sampler;
@binding( 1 ) @group( 1 ) var nodeUniform1 : texture_2d<f32>;

struct renderStruct {
	nodeUniform0 : f32,
	cameraProjectionMatrix : mat4x4<f32>,
	cameraViewMatrix : mat4x4<f32>
};
@binding( 0 ) @group( 0 )
var<uniform> render : renderStruct;

struct objectStruct {
	nodeUniform2 : f32,
	nodeUniform5 : mat4x4<f32>
};
@binding( 2 ) @group( 1 )
var<uniform> object : objectStruct;

// vars
var<private> DiffuseColor : vec4<f32>;
var<private> nodeVar0 : vec4<f32>;
var<private> nodeVar1 : vec2<f32>;
var<private> nodeVar2 : vec2<f32>;
var<private> nodeVar3 : vec4<f32>;
var<private> nodeVar4 : vec4<f32>;
var<private> nodeVar5 : vec4<f32>;
var<private> nodeVar6 : vec4<f32>;
var<private> Output : vec4<f32>;

// codes


@fragment
fn main( @location( 0 ) nodeVarying4 : vec2<f32> ) -> OutputStruct {

	// flow
	// code

	nodeVar0 = vec4<f32>( 1.0, 1.0, 1.0, 1.0 );
	nodeVar1 = nodeVarying4;
	let nodeConst0 = pow( ( ( 0.0625 - cos( ( ( nodeVar1.x * 20.0 ) + render.nodeUniform0 ) ) ) * 0.0625 ), 2.0 );
	nodeVar2 = vec2<f32>( nodeConst0 );

	if ( ( nodeVar1.y > 0.5 ) ) {

		nodeVar2 = vec2<f32>( 0.0 );
		

	}

	let nodeConst1 = ( nodeVar1 + ( vec2<f32>( nodeConst0, nodeConst0 ) * vec2<f32>( 0.5 ) ) );
	nodeVar3 = textureSampleGrad( nodeUniform1, nodeUniform1_sampler, nodeConst1, nodeVar2, nodeVar2 );
	let nodeConst2 = ( nodeVar1 + ( vec2<f32>( nodeConst0, ( - nodeConst0 ) ) * vec2<f32>( 0.5 ) ) );
	nodeVar4 = textureSampleGrad( nodeUniform1, nodeUniform1_sampler, nodeConst2, nodeVar2, nodeVar2 );
	let nodeConst3 = ( nodeVar1 + ( vec2<f32>( ( - nodeConst0 ), nodeConst0 ) * vec2<f32>( 0.5 ) ) );
	nodeVar5 = textureSampleGrad( nodeUniform1, nodeUniform1_sampler, nodeConst3, nodeVar2, nodeVar2 );
	let nodeConst4 = ( nodeVar1 + ( vec2<f32>( ( - nodeConst0 ), ( - nodeConst0 ) ) * vec2<f32>( 0.5 ) ) );
	nodeVar6 = textureSampleGrad( nodeUniform1, nodeUniform1_sampler, nodeConst4, nodeVar2, nodeVar2 );
	nodeVar0 = ( ( ( ( nodeVar3 * vec4<f32>( 0.25 ) ) + ( nodeVar4 * vec4<f32>( 0.25 ) ) ) + ( nodeVar5 * vec4<f32>( 0.25 ) ) ) + ( nodeVar6 * vec4<f32>( 0.25 ) ) );

	if ( ( ( nodeVar1.y > 0.497 ) && ( nodeVar1.y < 0.503 ) ) ) {

		nodeVar0 = vec4<f32>( 1.0 );
		

	}

	DiffuseColor = nodeVar0;
	DiffuseColor.w = ( DiffuseColor.w * object.nodeUniform2 );
	DiffuseColor.w = 1.0;
	let nodeConst5 = max( vec4<f32>( DiffuseColor.xyz, DiffuseColor.w ), vec4<f32>( 0.0 ) );
	Output = nodeConst5;

	// result

	output.color = nodeConst5;

	return output;

}
