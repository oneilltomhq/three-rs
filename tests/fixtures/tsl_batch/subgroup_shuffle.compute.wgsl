// Three.js r187dev - Node System

// directives
enable subgroups;

// system
var<private> instanceIndex : u32;

// locals


// structs


// uniforms

struct NodeBuffer_2619Struct {
	value : array< f32 >
};
@binding( 0 ) @group( 0 )
var<storage, read_write> NodeBuffer_2619 : NodeBuffer_2619Struct;

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


	// flow -> subgroup_shuffle
	workgroupBarrier();
	NodeBuffer_2619.value[ instanceIndex ] = ( ( ( ( ( subgroupBroadcastFirst( f32( instanceIndex ) ) + subgroupBroadcast( f32( instanceIndex ), 3 ) ) + f32( subgroupShuffle( i32( f32( instanceIndex ) ), i32( ( instanceIndex ^ 1u ) ) ) ) ) + subgroupShuffleXor( f32( instanceIndex ), 2u ) ) + subgroupShuffleUp( f32( instanceIndex ), 1u ) ) + subgroupShuffleDown( f32( instanceIndex ), 1u ) );

	

}
