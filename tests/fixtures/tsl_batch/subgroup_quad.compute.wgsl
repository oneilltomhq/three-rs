// Three.js r187dev - Node System

// directives
enable subgroups;

// system
var<private> instanceIndex : u32;

// locals


// structs


// uniforms

struct NodeBuffer_2670Struct {
	value : array< f32 >
};
@binding( 0 ) @group( 0 )
var<storage, read_write> NodeBuffer_2670 : NodeBuffer_2670Struct;

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


	// flow -> subgroup_quad
	workgroupBarrier();
	NodeBuffer_2670.value[ instanceIndex ] = ( ( quadSwapX( f32( instanceIndex ) ) + quadSwapY( f32( instanceIndex ) ) ) + quadSwapDiagonal( f32( instanceIndex ) ) );

	

}
