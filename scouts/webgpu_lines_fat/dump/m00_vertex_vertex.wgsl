// Three.js r186 - Node System

// directives


// structs


// uniforms

struct renderStruct {
	nodeUniform8 : f32,
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
	nodeUniform9 : vec3<f32>,
	nodeUniform10 : f32
};
@binding( 0 ) @group( 1 )
var<uniform> object : objectStruct;

// varyings

struct VaryingsStruct {
	@location( 0 ) nodeVarying4 : vec2<f32>,
	@location( 1 ) nodeVarying5 : vec3<f32>,
	@location( 2 ) nodeVarying6 : vec3<f32>,
	@location( 3 ) nodeVarying7 : vec3<f32>,
	@builtin( position ) builtinClipSpace : vec4<f32>
};
var<private> varyings : VaryingsStruct;

// vars
var<private> modelViewMatrix : mat4x4<f32>;
var<private> start : vec4<f32>;
var<private> end : vec4<f32>;
var<private> nodeVar0 : vec4<f32>;
var<private> nodeVar1 : vec4<f32>;
var<private> nodeVar2 : vec2<f32>;
var<private> nodeVar3 : f32;
var<private> nodeVar4 : vec4<f32>;
var<private> offset : vec2<f32>;
var<private> nodeVar5 : vec2<f32>;
var<private> nodeVar6 : vec4<f32>;
var<private> nodeVar7 : vec4<f32>;
var<private> VERTEX_nodeVar12 : vec4<f32>;
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
	@location( 3 ) uv : vec2<f32>,
	@location( 4 ) instanceColorStart : vec3<f32>,
	@location( 5 ) instanceColorEnd : vec3<f32> ) -> VaryingsStruct {

	// flow
	// code

	positionLocal = position;
	modelViewMatrix = ( render.cameraViewMatrix * object.nodeUniform5 );
	start = ( modelViewMatrix * vec4<f32>( instanceStart, 1.0 ) );
	end = ( modelViewMatrix * vec4<f32>( instanceEnd, 1.0 ) );

	if ( ( render.cameraProjectionMatrix[ 2u ][ 3u ] == -1.0 ) ) {


		if ( ( ( start.z < 0.0 ) && ( end.z > 0.0 ) ) ) {

			end = vec4<f32>( mix( start.xyz, end.xyz, fn2( start, end ) ), end.w );
			

		} else {


			if ( ( ( end.z < 0.0 ) && ( start.z >= 0.0 ) ) ) {

				start = vec4<f32>( mix( end.xyz, start.xyz, fn2( end, start ) ), start.w );
				

			}

			

		}

		

	}

	nodeVar0 = ( render.cameraProjectionMatrix * end );
	nodeVar1 = ( render.cameraProjectionMatrix * start );
	nodeVar2 = ( ( nodeVar0.xyz / vec3<f32>( nodeVar0.w ) ).xy - ( nodeVar1.xyz / vec3<f32>( nodeVar1.w ) ).xy );
	nodeVar3 = ( render.nodeUniform6.z / render.nodeUniform6.w );
	nodeVar2.x = ( nodeVar2.x * nodeVar3 );
	nodeVar2 = normalize( nodeVar2 );
	nodeVar4 = vec4<f32>( 0.0, 0.0, 0.0, 1.0 );
	offset = vec2<f32>( nodeVar2.y, ( - nodeVar2.x ) );
	nodeVar2.x = ( nodeVar2.x / nodeVar3 );
	offset.x = ( offset.x / nodeVar3 );

	if ( ( position.x < 0.0 ) ) {

		nodeVar5 = ( - offset );

	} else {

		nodeVar5 = offset;

	}

	offset = nodeVar5;

	if ( ( position.y < 0.0 ) ) {

		offset = ( offset - nodeVar2 );
		

	} else {


		if ( ( position.y > 1.0 ) ) {

			offset = ( offset + nodeVar2 );
			

		}

		

	}

	offset = ( offset * vec2<f32>( object.nodeUniform7 ) );
	offset = ( offset / vec2<f32>( ( render.nodeUniform6.w / render.nodeUniform8 ) ) );

	if ( ( position.y < 0.5 ) ) {

		nodeVar6 = nodeVar1;

	} else {

		nodeVar6 = nodeVar0;

	}

	nodeVar4 = nodeVar6;
	offset = ( offset * vec2<f32>( nodeVar4.w ) );
	nodeVar4 = ( nodeVar4 + vec4<f32>( offset, 0.0, 0.0 ) );
	nodeVar7 = ( ( ( object.nodeUniform1 * render.cameraWorldMatrix ) * render.cameraProjectionMatrixInverse ) * nodeVar4 );
	positionLocal = ( nodeVar7.xyz / vec3<f32>( nodeVar7.w ) );
	varyings.nodeVarying4 = uv;
	varyings.nodeVarying5 = position;
	varyings.nodeVarying6 = instanceColorStart;
	varyings.nodeVarying7 = instanceColorEnd;
	v_positionView = ( modelViewMatrix * vec4<f32>( positionLocal, 1.0 ) ).xyz;
	VERTEX_nodeVar12 = ( render.cameraProjectionMatrix * vec4<f32>( v_positionView, 1.0 ) );
	VERTEX_v_modelViewProjection = VERTEX_nodeVar12;

	// result

	varyings.builtinClipSpace = VERTEX_v_modelViewProjection;

	return varyings;

}
