// Three.js r187dev - Node System

// directives
enable subgroups;

// system
var<private> instanceIndex : u32;

// locals


// structs


// uniforms

struct NodeBuffer_2828Struct {
	value : array< vec3<f32> >
};
@binding( 0 ) @group( 0 )
var<storage, read_write> NodeBuffer_2828 : NodeBuffer_2828Struct;

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


	// flow -> attribute_array_compute
	if ( instanceIndex >= object.nodeUniform1 ) { return; }

	NodeBuffer_2828.value[ instanceIndex ] = vec3<f32>( f32( instanceIndex ), 0.0, 1.0 );

	

}
