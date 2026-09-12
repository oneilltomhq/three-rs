// Hand-written stand-in for the WGSL that `Background.update()` builds for a
// `scene.background` that is a `CubeTexture` (rung 4 replaces this file with the
// ported TSL path). `renderers/common/Background.js`:
//
//   backgroundNode  = cubeMapNode( cubeTexture( background ) )     (NodeManager.updateBackground)
//   colorNode       = vec4( backgroundNode ).mul( backgroundIntensity ).context( {
//                         getUV: () => backgroundRotation.mul( normalWorldGeometry ),
//                         getTextureLevel: () => backgroundBlurriness } )
//   vertexNode      = cameraProjectionMatrix * vec4( ( modelViewMatrix * vec4( positionLocal, 0 ) ).xyz, 1 )
//                     with z forced to w so the skybox sits on the far plane
//   mesh            = Mesh( SphereGeometry( 1, 32, 32 ), material )
//   material.side   = BackSide, depthTest = false, depthWrite = false
//
// The uv still goes through `CubeTextureNode.setupUV()`, hence the extra
// `materialEnvRotation` multiply and the x negation. `getTextureLevel` makes the
// sample an explicit-level one: `textureSampleLevel( …, backgroundBlurriness )`,
// i.e. mip 0 for the default blurriness of 0.

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
@group(0) @binding(2) var backgroundMap : texture_cube<f32>;
@group(0) @binding(3) var backgroundMap_sampler : sampler;

struct VaryingsStruct {
	@location( 0 ) v_normalWorldGeometry : vec3<f32>,
	@builtin( position ) builtinClipSpace : vec4<f32>,
};

@vertex
fn main_vertex( @location( 0 ) normal : vec3<f32>,
	@location( 1 ) position : vec3<f32> ) -> VaryingsStruct {

	var varyings : VaryingsStruct;

	let normalLocal = normal;
	let v_normalViewGeometry = normalize( ( render.cameraViewMatrix * vec4<f32>( ( object.normalMatrix * normalLocal ), 0.0 ) ).xyz );
	let normalViewGeometry = normalize( v_normalViewGeometry );
	// `transformNormalByInverseViewMatrix`: a row-vector multiply, i.e. the
	// transpose of the view matrix.
	varyings.v_normalWorldGeometry = normalize( ( vec4<f32>( normalViewGeometry, 0.0 ) * render.cameraViewMatrix ).xyz );

	let modelViewMatrix = ( render.cameraViewMatrix * object.worldMatrix );

	var scaledPosition : vec3<f32>;

	if ( ( render.cameraProjectionMatrix[ 3u ][ 3u ] == 1.0 ) ) {

		// orthographic: scale the skybox sphere past the viewing box
		scaledPosition = ( position * vec3<f32>( ( ( 1.0 / render.cameraProjectionMatrix[ 1u ][ 1u ] ) * 3.0 ) ) );

	} else {

		scaledPosition = position;

	}

	let viewProj = ( render.cameraProjectionMatrix * vec4<f32>( ( modelViewMatrix * vec4<f32>( scaledPosition, 0.0 ) ).xyz, 1.0 ) );
	varyings.builtinClipSpace = vec4<f32>( viewProj.x, viewProj.y, viewProj.w, viewProj.w );

	return varyings;

}

@fragment
fn main_fragment( @location( 0 ) v_normalWorldGeometry : vec3<f32> ) -> @location(0) vec4<f32> {

	let normalWorldGeometry = normalize( v_normalWorldGeometry );
	let rotated = ( object.envRotation * ( render.backgroundRotation * vec4<f32>( normalWorldGeometry, 1.0 ) ) );
	let sampled = textureSampleLevel( backgroundMap, backgroundMap_sampler, vec3<f32>( ( - rotated.x ), rotated.yz ), render.backgroundBlurriness );

	var DiffuseColor = ( sampled * vec4<f32>( render.backgroundIntensity ) );
	DiffuseColor.w = ( DiffuseColor.w * object.opacity );
	DiffuseColor.w = 1.0;

	return max( vec4<f32>( DiffuseColor.xyz, DiffuseColor.w ), vec4<f32>( 0.0 ) );

}
