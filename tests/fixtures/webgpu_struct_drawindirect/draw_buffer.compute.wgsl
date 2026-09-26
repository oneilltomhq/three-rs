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
@binding( 0 ) @group( 1 ) var<storage, read_write> NodeBuffer_897 : DrawBuffer;

struct renderStruct {
	nodeUniform0 : f32
};
@binding( 0 ) @group( 0 )
var<uniform> render : renderStruct;

struct objectStruct {
	nodeUniform2 : u32
};
@binding( 1 ) @group( 1 )
var<uniform> object : objectStruct;

// vars
var<private> instanceCount : f32;

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

	if ( instanceIndex >= object.nodeUniform2 ) { return; }

	instanceCount = max( ( pow( ( sin( ( render.nodeUniform0 * 0.5 ) ) + 1.0 ), 4.0 ) * 100000.0 ), 100.0 );
	atomicStore( &NodeBuffer_897.instanceCount, u32( instanceCount ) );

	

}
