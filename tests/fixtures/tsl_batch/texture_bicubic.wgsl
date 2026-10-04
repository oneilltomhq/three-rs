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

struct objectStruct {
	nodeUniform1 : f32,
	nodeUniform4 : mat4x4<f32>
};
@binding( 2 ) @group( 1 )
var<uniform> object : objectStruct;

// vars
var<private> nodeVar0 : vec4<f32>;
var<private> nodeVar1 : vec4<f32>;
var<private> nodeVar2 : vec4<f32>;
var<private> nodeVar3 : vec4<f32>;
var<private> nodeVar4 : vec4<f32>;
var<private> nodeVar5 : vec4<f32>;
var<private> nodeVar6 : vec4<f32>;
var<private> nodeVar7 : vec4<f32>;

// codes


@fragment
fn main( @location( 0 ) nodeVarying4 : vec2<f32> ) -> OutputStruct {

	// flow
	// code

	let nodeConst0 = ( 0.5 * object.nodeUniform1 );
	let nodeConst1 = vec4<f32>( vec2<f32>( textureDimensions( nodeUniform0, i32( nodeConst0 ) ) ), vec2<f32>( textureDimensions( nodeUniform0, i32( ( nodeConst0 + 1.0 ) ) ) ) );
	let nodeConst2 = ( ( vec4<f32>( vec3<f32>( nodeVarying4, 0.0 ), 1.0 ).xyxy * nodeConst1 ) + vec4<f32>( 0.5 ) );
	let nodeConst3 = fract( nodeConst2 );
	let nodeConst4 = ( vec4<f32>( 0.16666666666666666 ) * ( ( nodeConst3 * ( nodeConst3 * ( ( vec4<f32>( 3.0 ) * nodeConst3 ) - vec4<f32>( 6.0 ) ) ) ) + vec4<f32>( 4.0 ) ) );
	let nodeConst5 = ( ( vec4<f32>( 0.16666666666666666 ) * ( ( nodeConst3 * ( ( nodeConst3 * ( ( - nodeConst3 ) + vec4<f32>( 3.0 ) ) ) - vec4<f32>( 3.0 ) ) ) + vec4<f32>( 1.0 ) ) ) + nodeConst4 );
	let nodeConst6 = floor( nodeConst2 );
	let nodeConst7 = ( vec4<f32>( 1.0 ) / nodeConst1 );
	let nodeConst8 = ( ( ( nodeConst6 + ( vec4<f32>( -1.0 ) + ( nodeConst4 / nodeConst5 ) ) ) - vec4<f32>( 0.5 ) ) * nodeConst7 );
	let nodeConst9 = floor( nodeConst0 );
	nodeVar0 = textureSampleLevel( nodeUniform0, nodeUniform0_sampler, nodeConst8.xy, nodeConst9 );
	let nodeConst10 = ( vec4<f32>( 0.16666666666666666 ) * pow( nodeConst3, vec4<f32>( 3.0 ) ) );
	let nodeConst11 = ( ( vec4<f32>( 0.16666666666666666 ) * ( ( nodeConst3 * ( ( nodeConst3 * ( ( vec4<f32>( -3.0 ) * nodeConst3 ) + vec4<f32>( 3.0 ) ) ) + vec4<f32>( 3.0 ) ) ) + vec4<f32>( 1.0 ) ) ) + nodeConst10 );
	let nodeConst12 = ( ( ( nodeConst6 + ( vec4<f32>( 1.0 ) + ( nodeConst10 / nodeConst11 ) ) ) - vec4<f32>( 0.5 ) ) * nodeConst7 );
	nodeVar1 = textureSampleLevel( nodeUniform0, nodeUniform0_sampler, vec2<f32>( nodeConst12.xy.x, nodeConst8.xy.y ), nodeConst9 );
	nodeVar2 = textureSampleLevel( nodeUniform0, nodeUniform0_sampler, vec2<f32>( nodeConst8.xy.x, nodeConst12.xy.y ), nodeConst9 );
	nodeVar3 = textureSampleLevel( nodeUniform0, nodeUniform0_sampler, nodeConst12.xy, nodeConst9 );
	let nodeConst13 = ceil( nodeConst0 );
	nodeVar4 = textureSampleLevel( nodeUniform0, nodeUniform0_sampler, nodeConst8.zw, nodeConst13 );
	nodeVar5 = textureSampleLevel( nodeUniform0, nodeUniform0_sampler, vec2<f32>( nodeConst12.zw.x, nodeConst8.zw.y ), nodeConst13 );
	nodeVar6 = textureSampleLevel( nodeUniform0, nodeUniform0_sampler, vec2<f32>( nodeConst8.zw.x, nodeConst12.zw.y ), nodeConst13 );
	nodeVar7 = textureSampleLevel( nodeUniform0, nodeUniform0_sampler, nodeConst12.zw, nodeConst13 );

	// result

	output.color = mix( ( ( vec4<f32>( nodeConst5.xy.y ) * ( ( vec4<f32>( nodeConst5.xy.x ) * nodeVar0 ) + ( vec4<f32>( nodeConst11.xy.x ) * nodeVar1 ) ) ) + ( vec4<f32>( nodeConst11.xy.y ) * ( ( vec4<f32>( nodeConst5.xy.x ) * nodeVar2 ) + ( vec4<f32>( nodeConst11.xy.x ) * nodeVar3 ) ) ) ), ( ( vec4<f32>( nodeConst5.zw.y ) * ( ( vec4<f32>( nodeConst5.zw.x ) * nodeVar4 ) + ( vec4<f32>( nodeConst11.zw.x ) * nodeVar5 ) ) ) + ( vec4<f32>( nodeConst11.zw.y ) * ( ( vec4<f32>( nodeConst5.zw.x ) * nodeVar6 ) + ( vec4<f32>( nodeConst11.zw.x ) * nodeVar7 ) ) ) ), fract( nodeConst0 ) );

	return output;

}
