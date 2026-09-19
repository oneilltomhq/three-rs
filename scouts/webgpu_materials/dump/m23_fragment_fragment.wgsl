// Three.js r186 - Node System

// global
diagnostic( off, derivative_uniformity );


// structs

struct OutputStruct {
	@location( 0 ) color: vec4<f32>
};
var<private> output : OutputStruct;

// uniforms
@binding( 0 ) @group( 1 ) var nodeUniform0_sampler : sampler;
@binding( 1 ) @group( 1 ) var nodeUniform0 : texture_2d<f32>;

struct renderStruct {
	nodeUniform1 : f32,
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
var<private> nodeVar1 : f32;
var<private> nodeVar2 : vec4<f32>;
var<private> nodeVar3 : f32;
var<private> nodeVar4 : vec4<f32>;
var<private> nodeVar5 : vec4<f32>;
var<private> nodeVar6 : vec4<f32>;
var<private> Output : vec4<f32>;
var<private> nodeVar7 : vec4<f32>;

// codes


@fragment
fn main( @location( 0 ) nodeVarying4 : vec2<f32> ) -> OutputStruct {

	// flow
	// code


	for ( var i : i32 = 0; i < 10; i ++ ) {

		nodeVar0 = vec4<f32>( 0.0, 0.0, 0.0, 1.0 );
		nodeVar1 = ( ( ( ( sin( ( ( render.nodeUniform1 + 0.75 ) * 6.283185307179586 ) ) * 0.5 ) + 0.5 ) * 0.09 ) * f32( i ) );
		nodeVar2 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying4 + vec2<f32>( nodeVar1, 0.0 ) ) );
		nodeVar0 = ( nodeVar0 + nodeVar2 );
		nodeVar3 = ( - nodeVar1 );
		nodeVar4 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying4 + vec2<f32>( nodeVar3, 0.0 ) ) );
		nodeVar0 = ( nodeVar0 + nodeVar4 );
		nodeVar5 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying4 + vec2<f32>( 0.0, nodeVar1 ) ) );
		nodeVar0 = ( nodeVar0 + nodeVar5 );
		nodeVar6 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying4 + vec2<f32>( 0.0, nodeVar3 ) ) );
		nodeVar0 = ( nodeVar0 + nodeVar6 );

	}

	DiffuseColor = vec4<f32>(  );
	DiffuseColor.w = ( DiffuseColor.w * object.nodeUniform2 );
	DiffuseColor.w = 1.0;
	nodeVar7 = max( vec4<f32>( DiffuseColor.xyz, DiffuseColor.w ), vec4<f32>( 0.0 ) );
	Output = nodeVar7;

	// result

	output.color = nodeVar7;

	return output;

}
