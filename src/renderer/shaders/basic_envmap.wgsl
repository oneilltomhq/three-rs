// Hand-written stand-in for the WGSL that `MeshBasicNodeMaterial` with an
// `envMap` generates through three.js' node system (rung 4 replaces this file
// with the ported TSL path). Flow, statement for statement:
//
//   MeshBasicNodeMaterial.setupEnvironment()  -> BasicEnvironmentNode( cubeTexture( envMap ) )
//   BasicEnvironmentNode.setup()              -> context.environment = cubeMapNode( envNode )
//   CubeTextureNode.getDefaultUV()            -> reflectVector   (CubeReflectionMapping)
//   ReflectVector.js                          -> reflectView.transformDirection( cameraWorldMatrix )
//   CubeTextureNode.setupUV()                 -> materialEnvRotation * uv, then x negated
//                                                (WebGPUCoordinateSystem: three's cube maps have
//                                                 pos-x / neg-x swapped relative to WebGPU)
//   BasicLightingModel.indirect()             -> indirectDiffuse = vec3( 1 ) * AO * diffuseColor.rgb
//   MeshBasicNodeMaterial.setupOutgoingLight()-> diffuseColor.rgb
//   BasicLightingModel.finish(), MultiplyOperation (MeshBasicMaterial.combine default):
//       outgoingLight = mix( outgoingLight, outgoingLight * env.rgb,
//                            materialSpecularStrength * materialReflectivity )
//
// No colour-space node on the texture: `WGSLNodeBuilder` leaves
// `needsToWorkingColorSpace()` false, and `WebGPUTextureUtils.getFormat()` picks
// `rgba8unorm-srgb` for an `UnsignedByteType` / `SRGBColorSpace` texture, so the
// GPU applies the sRGB transfer on sample and the result is already linear.

struct RenderUniforms {
	cameraProjectionMatrix : mat4x4<f32>,
	cameraViewMatrix : mat4x4<f32>,
	viewportSize : vec2<f32>,
	cameraWorldMatrix : mat4x4<f32>,
	backgroundRotation : mat4x4<f32>,
	backgroundBlurriness : f32,
	backgroundIntensity : f32,
};

struct ObjectUniforms {
	worldMatrix : mat4x4<f32>,
	normalMatrix : mat3x3<f32>,
	diffuse : vec3<f32>,
	opacity : f32,
	reflectivity : f32,
	envRotation : mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> render : RenderUniforms;
@group(0) @binding(1) var<uniform> object : ObjectUniforms;
@group(0) @binding(2) var envMap : texture_cube<f32>;
@group(0) @binding(3) var envMap_sampler : sampler;

struct VaryingsStruct {
	@location( 0 ) v_positionViewDirection : vec3<f32>,
	@location( 1 ) v_normalViewGeometry : vec3<f32>,
	@builtin( position ) builtinClipSpace : vec4<f32>,
};

@vertex
fn main_vertex( @location( 0 ) position : vec3<f32>,
	@location( 1 ) normal : vec3<f32> ) -> VaryingsStruct {

	var varyings : VaryingsStruct;

	let modelViewMatrix = render.cameraViewMatrix * object.worldMatrix;
	let positionLocal = position;
	let v_positionView = ( modelViewMatrix * vec4<f32>( positionLocal, 1.0 ) ).xyz;
	varyings.v_positionViewDirection = ( - v_positionView );
	let normalLocal = normal;
	varyings.v_normalViewGeometry = normalize( ( render.cameraViewMatrix * vec4<f32>( ( object.normalMatrix * normalLocal ), 0.0 ) ).xyz );
	varyings.builtinClipSpace = ( render.cameraProjectionMatrix * vec4<f32>( v_positionView, 1.0 ) );

	return varyings;

}

@fragment
fn main_fragment( @location( 0 ) v_positionViewDirection : vec3<f32>,
	@location( 1 ) v_normalViewGeometry : vec3<f32> ) -> @location(0) vec4<f32> {

	var DiffuseColor = vec4<f32>( object.diffuse, 1.0 );
	DiffuseColor.w = ( DiffuseColor.w * object.opacity );
	DiffuseColor.w = 1.0;

	var indirectDiffuse = vec3<f32>( 0.0, 0.0, 0.0 );
	indirectDiffuse = vec4<f32>( 0.0, 0.0, 0.0, 0.0 ).xyz;
	indirectDiffuse = ( vec4<f32>( indirectDiffuse, 1.0 ) + vec4<f32>( 1.0, 1.0, 1.0, 0.0 ) ).xyz;
	let ambientOcclusion = 1.0;
	indirectDiffuse = ( indirectDiffuse * vec3<f32>( ambientOcclusion ) );
	indirectDiffuse = ( indirectDiffuse * DiffuseColor.xyz );

	let directDiffuse = vec3<f32>( 0.0, 0.0, 0.0 );
	let totalDiffuse = ( directDiffuse + indirectDiffuse );

	let directSpecular = vec3<f32>( 0.0, 0.0, 0.0 );
	let indirectSpecular = vec3<f32>( 0.0, 0.0, 0.0 );
	let totalSpecular = ( directSpecular + indirectSpecular );

	var outgoingLight = ( totalDiffuse + totalSpecular );

	let positionViewDirection = normalize( v_positionViewDirection );
	let normalViewGeometry = normalize( v_normalViewGeometry );
	let normalView = normalViewGeometry;
	let reflectVector = normalize( ( render.cameraWorldMatrix * vec4<f32>( reflect( ( - positionViewDirection ), normalView ), 0.0 ) ).xyz );

	let rotated = ( object.envRotation * vec4<f32>( reflectVector, 1.0 ) );
	let cubeUV = vec3<f32>( ( - rotated.x ), rotated.yz );
	let env = textureSample( envMap, envMap_sampler, cubeUV );

	outgoingLight = mix( outgoingLight, ( outgoingLight * env.xyz ), ( 1.0 * object.reflectivity ) );

	return max( vec4<f32>( outgoingLight, DiffuseColor.w ), vec4<f32>( 0.0 ) );

}
