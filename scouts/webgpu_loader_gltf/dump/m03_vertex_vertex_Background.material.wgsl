// Three.js r186 - Node System

// directives


// structs


// uniforms

struct renderStruct {
	nodeUniform5 : f32,
	nodeUniform6 : f32,
	nodeUniform2 : mat4x4<f32>,
	cameraProjectionMatrix : mat4x4<f32>,
	cameraViewMatrix : mat4x4<f32>
};
@binding( 0 ) @group( 0 )
var<uniform> render : renderStruct;

struct objectStruct {
	nodeUniform1 : mat4x4<f32>,
	nodeUniform4 : mat3x3<f32>,
	nodeUniform7 : f32,
	nodeUniform9 : mat4x4<f32>
};
@binding( 2 ) @group( 1 )
var<uniform> object : objectStruct;

// varyings

struct VaryingsStruct {
	@location( 0 ) v_normalWorldGeometry : vec3<f32>,
	@builtin( position ) builtinClipSpace : vec4<f32>
};
var<private> varyings : VaryingsStruct;

// vars
var<private> normalLocal : vec3<f32>;
var<private> normalViewGeometry : vec3<f32>;
var<private> modelViewMatrix : mat4x4<f32>;
var<private> nodeVar3 : vec3<f32>;
var<private> nodeVar4 : vec4<f32>;
var<private> nodeVar5 : vec4<f32>;
var<private> positionLocal : vec3<f32>;
var<private> v_normalViewGeometry : vec3<f32>;

// codes


@vertex
fn main( @location( 0 ) normal : vec3<f32>,
	@location( 1 ) position : vec3<f32> ) -> VaryingsStruct {

	// flow
	// code

	normalLocal = normal;
	v_normalViewGeometry = normalize( ( render.cameraViewMatrix * vec4<f32>( ( object.nodeUniform4 * normalLocal ), 0.0 ) ).xyz );
	normalViewGeometry = normalize( v_normalViewGeometry );
	varyings.v_normalWorldGeometry = normalize( ( vec4<f32>( normalViewGeometry, 0.0 ) * render.cameraViewMatrix ).xyz );
	modelViewMatrix = ( render.cameraViewMatrix * object.nodeUniform9 );

	if ( ( render.cameraProjectionMatrix[ 3u ][ 3u ] == 1.0 ) ) {

		positionLocal = position;
		nodeVar3 = ( positionLocal * vec3<f32>( ( ( 1.0 / render.cameraProjectionMatrix[ 1u ][ 1u ] ) * 3.0 ) ) );

	} else {

		positionLocal = position;
		nodeVar3 = positionLocal;

	}

	nodeVar4 = ( render.cameraProjectionMatrix * vec4<f32>( ( modelViewMatrix * vec4<f32>( nodeVar3, 0.0 ) ).xyz, 1.0 ) );
	nodeVar5 = vec4<f32>( nodeVar4.x, nodeVar4.y, nodeVar4.w, nodeVar4.w );

	// result

	varyings.builtinClipSpace = nodeVar5;

	return varyings;

}
