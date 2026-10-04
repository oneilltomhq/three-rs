// Three.js r187dev - Node System

// directives
enable subgroups;

// system
var<private> instanceIndex : u32;

// locals


// structs


// uniforms

struct NodeBuffer_2565Struct {
	value : array< u32 >
};
@binding( 0 ) @group( 0 )
var<storage, read_write> NodeBuffer_2565 : NodeBuffer_2565Struct;

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


	// flow -> subgroup_bits
	workgroupBarrier();
	NodeBuffer_2565.value[ instanceIndex ] = ( ( ( ( ( ( subgroupAnd( instanceIndex ) + subgroupOr( instanceIndex ) ) + subgroupXor( instanceIndex ) ) + u32( subgroupAll( ( f32( instanceIndex ) < 32.0 ) ) ) ) + u32( subgroupAny( ( f32( instanceIndex ) == 3.0 ) ) ) ) + u32( subgroupElect() ) ) + subgroupBallot( ( f32( instanceIndex ) > 7.0 ) ).x );

	

}
