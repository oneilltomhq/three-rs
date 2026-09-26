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
@binding( 1 ) @group( 1 ) var nodeUniform3 : texture_2d<f32>;

struct objectStruct {
	nodeUniform0 : vec3<f32>,
	nodeUniform1 : vec3<f32>,
	nodeUniform7 : mat4x4<f32>
};
@binding( 0 ) @group( 1 )
var<uniform> object : objectStruct;

struct renderStruct {
	cameraProjectionMatrix : mat4x4<f32>,
	cameraViewMatrix : mat4x4<f32>,
	nodeUniform2 : vec3<f32>,
	nodeUniform4 : vec2<f32>
};
@binding( 0 ) @group( 0 )
var<uniform> render : renderStruct;

// vars
var<private> DiffuseColor : vec4<f32>;
var<private> Output : vec4<f32>;
var<private> irradiance : vec3<f32>;
var<private> indirectDiffuse : vec3<f32>;
var<private> ambientOcclusion : f32;
var<private> totalDiffuse : vec3<f32>;
var<private> directDiffuse : vec3<f32>;
var<private> nodeVar1 : vec4<f32>;
var<private> nodeVar2 : vec2<u32>;
var<private> totalSpecular : vec3<f32>;
var<private> directSpecular : vec3<f32>;
var<private> indirectSpecular : vec3<f32>;
var<private> outgoingLight : vec3<f32>;

// codes
fn tsl_mod_float( x : f32, y : f32 ) -> f32 { return x - y * floor( x / y ); }
fn tsl_clampWrapping_float( coord: f32 ) -> f32 { return clamp( coord, 0.0, 1.0 ); }
fn tsl_coord_clampS_clampT_2d( coord : vec2f ) -> vec2f {

	return vec2f(
		tsl_clampWrapping_float( coord.x ),
		tsl_clampWrapping_float( coord.y )
	);

}



@fragment
fn main( @location( 0 ) nodeVarying4 : vec2<f32>,
	@builtin( position ) fragCoord : vec4<f32> ) -> OutputStruct {

	// flow
	// code

	DiffuseColor = vec4<f32>( object.nodeUniform0, 1.0 );
	let nodeConst0 = ( ( ( nodeVarying4 * vec2<f32>( 3.0 ) ) * object.nodeUniform1.xy ) * vec2<f32>( 2.0 ) );
	let nodeConst1 = sign( tsl_mod_float( ( floor( nodeConst0.x ) + floor( nodeConst0.y ) ), 2.0 ) );
	DiffuseColor.w = ( DiffuseColor.w * nodeConst1 );
	irradiance = vec3<f32>( 0.0, 0.0, 0.0 );
	irradiance = ( irradiance + render.nodeUniform2 );
	indirectDiffuse = vec3<f32>( 0.0, 0.0, 0.0 );
	indirectDiffuse = vec4<f32>( 0.0, 0.0, 0.0, 0.0 ).xyz;
	indirectDiffuse = ( vec4<f32>( indirectDiffuse, 1.0 ) + vec4<f32>( 1.0, 1.0, 1.0, 0.0 ) ).xyz;
	ambientOcclusion = 1.0;
	indirectDiffuse = ( indirectDiffuse * vec3<f32>( ambientOcclusion ) );
	indirectDiffuse = ( indirectDiffuse * DiffuseColor.xyz );
	directDiffuse = vec3<f32>( 0.0, 0.0, 0.0 );
	var nodeVar0 : vec4<f32> = vec4<f32>( 0.0, 0.0, 0.0, 0.0 );

	for ( var i : f32 = 0.0; i < 45.0; i += 1. ) {

		let nodeConst2 = ( ( i / 45.0 ) * 6.283185307179586 );
		nodeVar2 = textureDimensions( nodeUniform3, u32( 0 ) );
		nodeVar1 = textureLoad( nodeUniform3, vec2<u32>( clamp( floor( tsl_coord_clampS_clampT_2d( ( ( fragCoord.xy / render.nodeUniform4 ) + ( ( vec2<f32>( cos( nodeConst2 ), sin( nodeConst2 ) ) * vec2<f32>( ( fract( ( sin( tsl_mod_float( dot( vec2<f32>( i, ( ( fragCoord.xy / render.nodeUniform4 ).x + ( fragCoord.xy / render.nodeUniform4 ).y ) ), vec2<f32>( 12.9898, 78.233 ) ), 3.141592653589793 ) ) * 43758.5453 ) ) + 0.05 ) ) ) * vec2<f32>( 0.05 ) ) ) ) * vec2<f32>( nodeVar2 ) ), vec2<f32>( 0 ), vec2<f32>( nodeVar2 - vec2<u32>( 1, 1 ) ) ) ), u32( 0 ) );
		nodeVar0 = ( nodeVar0 + nodeVar1 );

	}

	nodeVar0 = ( nodeVar0 / vec4<f32>( 45.0 ) );
	totalDiffuse = mix( vec4<f32>( ( directDiffuse + indirectDiffuse ), 1.0 ), nodeVar0, nodeConst1 ).xyz;
	directSpecular = vec3<f32>( 0.0, 0.0, 0.0 );
	indirectSpecular = vec3<f32>( 0.0, 0.0, 0.0 );
	totalSpecular = ( directSpecular + indirectSpecular );
	outgoingLight = ( totalDiffuse + totalSpecular );
	let nodeConst3 = max( vec4<f32>( outgoingLight, DiffuseColor.w ), vec4<f32>( 0.0 ) );
	Output = nodeConst3;

	// result

	output.color = nodeConst3;

	return output;

}
