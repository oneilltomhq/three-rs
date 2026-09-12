// Three.js r186 - Node System

// directives
enable subgroups;

// system
var<private> instanceIndex : u32;

// locals


// structs


// uniforms

struct NodeBuffer_992Struct {
	value : array< vec2<f32> >
};
@binding( 0 ) @group( 0 )
var<storage, read_write> NodeBuffer_992 : NodeBuffer_992Struct;

struct NodeBuffer_993Struct {
	value : array< vec2<f32> >
};
@binding( 1 ) @group( 0 )
var<storage, read_write> NodeBuffer_993 : NodeBuffer_993Struct;

struct objectStruct {
	nodeUniform2 : vec2<f32>,
	nodeUniform3 : vec2<f32>,
	nodeUniform4 : u32
};
@binding( 2 ) @group( 0 )
var<uniform> object : objectStruct;

// vars
var<private> nodeVar0 : vec2<f32>;
var<private> nodeVar1 : f32;
var<private> nodeVar2 : f32;
var<private> nodeVar3 : vec2<f32>;

// codes


@compute @workgroup_size( 64, 1, 1 )
fn main( @builtin( global_invocation_id ) globalId : vec3<u32>,
	@builtin( workgroup_id ) workgroupId : vec3<u32>,
	@builtin( local_invocation_id ) localId : vec3<u32>,
	@builtin( num_workgroups ) numWorkgroups : vec3<u32>,
	@builtin( subgroup_size ) subgroupSize : u32 ) {

	// local vars
	

	// system
	instanceIndex = globalId.x
		+ globalId.y * ( 64 * numWorkgroups.x )
		+ globalId.z * ( 64 * numWorkgroups.x ) * ( 1 * numWorkgroups.y );

	// flow
	// code


	// flow -> Update Particles
	if ( instanceIndex >= object.nodeUniform4 ) { return; }

	nodeVar0 = ( NodeBuffer_992.value[ instanceIndex ] + NodeBuffer_993.value[ instanceIndex ] );

	if ( ( abs( nodeVar0.x ) >= object.nodeUniform2.x ) ) {

		nodeVar1 = ( - NodeBuffer_993.value[ instanceIndex ].x );

	} else {

		nodeVar1 = NodeBuffer_993.value[ instanceIndex ].x;

	}

	NodeBuffer_993.value[ instanceIndex ].x = nodeVar1;

	if ( ( abs( nodeVar0.y ) >= object.nodeUniform2.y ) ) {

		nodeVar2 = ( - NodeBuffer_993.value[ instanceIndex ].y );

	} else {

		nodeVar2 = NodeBuffer_993.value[ instanceIndex ].y;

	}

	NodeBuffer_993.value[ instanceIndex ].y = nodeVar2;
	nodeVar0 = max( min( nodeVar0, object.nodeUniform2 ), ( - object.nodeUniform2 ) );

	if ( ( length( ( object.nodeUniform3 - nodeVar0 ) ) <= 0.1 ) ) {

		nodeVar3 = vec2<f32>( 0.0, 0.0 );

	} else {

		nodeVar3 = nodeVar0;

	}

	NodeBuffer_992.value[ instanceIndex ] = nodeVar3;

	

}
