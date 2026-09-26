// Three.js r187dev - Node System

// directives
enable subgroups;

// system
var<private> instanceIndex : u32;

// locals


// structs

struct DrawBuffer {
	vertexCount : u32,
	instanceCount : atomic< u32 >,
	firstVertex : u32,
	firstInstance : u32,
	offset : u32
};


// uniforms
@binding( 0 ) @group( 0 ) var<storage, read_write> NodeBuffer_897 : DrawBuffer;

struct objectStruct {
	nodeUniform1 : u32
};
@binding( 1 ) @group( 0 )
var<uniform> object : objectStruct;

// vars


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

	if ( instanceIndex >= object.nodeUniform1 ) { return; }

	NodeBuffer_897.vertexCount = 3u;
	atomicStore( &NodeBuffer_897.instanceCount, 0u );
	NodeBuffer_897.firstVertex = 0u;
	NodeBuffer_897.firstInstance = 0u;
	NodeBuffer_897.offset = 0u;

	

}
