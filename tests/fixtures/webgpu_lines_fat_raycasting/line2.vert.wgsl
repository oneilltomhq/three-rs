// Three.js r187dev - Node System

// directives


// structs


// uniforms

struct renderStruct {
	cameraWorldMatrix : mat4x4<f32>,
	cameraProjectionMatrixInverse : mat4x4<f32>,
	cameraViewMatrix : mat4x4<f32>,
	cameraProjectionMatrix : mat4x4<f32>,
	nodeUniform6 : vec4<f32>
};
@binding( 0 ) @group( 0 )
var<uniform> render : renderStruct;

struct objectStruct {
	nodeUniform1 : mat4x4<f32>,
	nodeUniform5 : mat4x4<f32>,
	nodeUniform7 : f32,
	nodeUniform8 : vec3<f32>,
	nodeUniform9 : f32,
	nodeUniform11 : f32
};
@binding( 0 ) @group( 1 )
var<uniform> object : objectStruct;

// varyings

struct VaryingsStruct {
	@location( 0 ) worldStart : vec3<f32>,
	@location( 1 ) worldEnd : vec3<f32>,
	@location( 2 ) worldPos : vec4<f32>,
	@location( 3 ) nodeVarying7 : vec3<f32>,
	@location( 4 ) nodeVarying8 : vec3<f32>,
	@location( 5 ) nodeVarying9 : vec3<f32>,
	@builtin( position ) builtinClipSpace : vec4<f32>
};
var<private> varyings : VaryingsStruct;

// vars
var<private> modelViewMatrix : mat4x4<f32>;
var<private> start : vec4<f32>;
var<private> end : vec4<f32>;
var<private> nodeVar0 : vec2<f32>;
var<private> nodeVar1 : vec4<f32>;
var<private> nodeVar2 : vec4<f32>;
var<private> nodeVar3 : vec3<f32>;
var<private> nodeVar4 : vec3<f32>;
var<private> nodeVar5 : vec3<f32>;
var<private> nodeVar6 : vec3<f32>;
var<private> nodeVar7 : vec3<f32>;
var<private> nodeVar8 : vec3<f32>;
var<private> nodeVar9 : vec3<f32>;
var<private> positionLocal : vec3<f32>;
var<private> v_modelViewProjection : vec4<f32>;
var<private> v_positionView : vec3<f32>;
var<private> VERTEX_v_modelViewProjection : vec4<f32>;

// codes
fn fn2 ( start : vec4<f32>, end : vec4<f32> ) -> f32 {

	var nodeVar0 : f32;


	if ( ( render.cameraProjectionMatrix[ 2u ][ 2u ] > 0.0 ) ) {

		nodeVar0 = ( ( - render.cameraProjectionMatrix[ 3u ][ 2u ] ) / ( render.cameraProjectionMatrix[ 2u ][ 2u ] + 1.0 ) );

	} else {

		nodeVar0 = ( ( render.cameraProjectionMatrix[ 3u ][ 2u ] * -0.5 ) / render.cameraProjectionMatrix[ 2u ][ 2u ] );

	}


	return ( ( nodeVar0 - start.z ) / ( end.z - start.z ) );

}




@vertex
fn main( @location( 0 ) position : vec3<f32>,
	@location( 1 ) instanceStart : vec3<f32>,
	@location( 2 ) instanceEnd : vec3<f32>,
	@location( 3 ) instanceColorStart : vec3<f32>,
	@location( 4 ) instanceColorEnd : vec3<f32> ) -> VaryingsStruct {

	// flow
	// code

	positionLocal = position;
	modelViewMatrix = ( render.cameraViewMatrix * object.nodeUniform5 );
	start = ( modelViewMatrix * vec4<f32>( instanceStart, 1.0 ) );
	end = ( modelViewMatrix * vec4<f32>( instanceEnd, 1.0 ) );
	varyings.worldStart = start.xyz;
	varyings.worldEnd = end.xyz;
	let nodeConst0 = ( render.cameraProjectionMatrix[ 2u ][ 3u ] == -1.0 );

	if ( nodeConst0 ) {


		if ( ( ( start.z < 0.0 ) && ( end.z > 0.0 ) ) ) {

			end = vec4<f32>( mix( start.xyz, end.xyz, fn2( start, end ) ), end.w );
			

		} else {


			if ( ( ( end.z < 0.0 ) && ( start.z >= 0.0 ) ) ) {

				start = vec4<f32>( mix( end.xyz, start.xyz, fn2( end, start ) ), start.w );
				

			}

			

		}

		

	}

	let nodeConst1 = ( render.cameraProjectionMatrix * end );
	let nodeConst2 = ( nodeConst1.xyz / vec3<f32>( nodeConst1.w ) );
	let nodeConst3 = ( render.cameraProjectionMatrix * start );
	let nodeConst4 = ( nodeConst3.xyz / vec3<f32>( nodeConst3.w ) );
	nodeVar0 = ( nodeConst2.xy - nodeConst4.xy );
	nodeVar0.x = ( nodeVar0.x * ( render.nodeUniform6.z / render.nodeUniform6.w ) );
	nodeVar0 = normalize( nodeVar0 );
	nodeVar1 = vec4<f32>( 0.0, 0.0, 0.0, 1.0 );

	if ( ( position.y < 0.5 ) ) {

		nodeVar2 = start;

	} else {

		nodeVar2 = end;

	}

	varyings.worldPos = nodeVar2;

	if ( ( position.x < 0.0 ) ) {


		if ( nodeConst0 ) {

			nodeVar4 = normalize( mix( start.xyz, end.xyz, 0.5 ) );

		} else {

			nodeVar4 = vec3<f32>( 0.0, 0.0, -1.0 );

		}

		nodeVar3 = ( normalize( cross( normalize( ( end.xyz - start.xyz ) ), nodeVar4 ) ) * vec3<f32>( ( object.nodeUniform7 * 0.5 ) ) );

	} else {


		if ( nodeConst0 ) {

			nodeVar5 = normalize( mix( start.xyz, end.xyz, 0.5 ) );

		} else {

			nodeVar5 = vec3<f32>( 0.0, 0.0, -1.0 );

		}

		nodeVar3 = ( - ( normalize( cross( normalize( ( end.xyz - start.xyz ) ), nodeVar5 ) ) * vec3<f32>( ( object.nodeUniform7 * 0.5 ) ) ) );

	}

	varyings.worldPos = ( varyings.worldPos + vec4<f32>( nodeVar3, 0.0 ) );

	if ( ( position.y < 0.5 ) ) {

		nodeVar6 = ( - ( normalize( ( end.xyz - start.xyz ) ) * vec3<f32>( ( object.nodeUniform7 * 0.5 ) ) ) );

	} else {

		nodeVar6 = ( normalize( ( end.xyz - start.xyz ) ) * vec3<f32>( ( object.nodeUniform7 * 0.5 ) ) );

	}

	varyings.worldPos = ( varyings.worldPos + vec4<f32>( nodeVar6, 0.0 ) );
	let nodeConst5 = normalize( ( end.xyz - start.xyz ) );

	if ( nodeConst0 ) {

		nodeVar7 = normalize( mix( start.xyz, end.xyz, 0.5 ) );

	} else {

		nodeVar7 = vec3<f32>( 0.0, 0.0, -1.0 );

	}

	let nodeConst6 = cross( nodeConst5, normalize( cross( nodeConst5, nodeVar7 ) ) );
	let nodeConst7 = ( object.nodeUniform7 * 0.5 );
	varyings.worldPos = ( varyings.worldPos + vec4<f32>( ( nodeConst6 * vec3<f32>( nodeConst7 ) ), 0.0 ) );

	if ( ( ( position.y > 1.0 ) || ( position.y < 0.0 ) ) ) {

		varyings.worldPos = ( varyings.worldPos - vec4<f32>( ( ( nodeConst6 * vec3<f32>( 2.0 ) ) * vec3<f32>( nodeConst7 ) ), 0.0 ) );
		

	}

	nodeVar1 = ( render.cameraProjectionMatrix * varyings.worldPos );
	nodeVar8 = vec3<f32>( 0.0, 0.0, 0.0 );

	if ( ( position.y < 0.5 ) ) {

		nodeVar9 = nodeConst4;

	} else {

		nodeVar9 = nodeConst2;

	}

	nodeVar8 = nodeVar9;
	nodeVar1.z = ( nodeVar8.z * nodeVar1.w );
	let nodeConst8 = ( ( ( object.nodeUniform1 * render.cameraWorldMatrix ) * render.cameraProjectionMatrixInverse ) * nodeVar1 );
	positionLocal = ( nodeConst8.xyz / vec3<f32>( nodeConst8.w ) );
	varyings.nodeVarying7 = position;
	varyings.nodeVarying8 = instanceColorStart;
	varyings.nodeVarying9 = instanceColorEnd;
	v_positionView = ( modelViewMatrix * vec4<f32>( positionLocal, 1.0 ) ).xyz;
	let VERTEX_nodeConst16 = ( render.cameraProjectionMatrix * vec4<f32>( v_positionView, 1.0 ) );
	VERTEX_v_modelViewProjection = VERTEX_nodeConst16;

	// result

	varyings.builtinClipSpace = VERTEX_v_modelViewProjection;

	return varyings;

}
