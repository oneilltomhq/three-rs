// Three.js r186 - Node System

// directives
enable subgroups;

// system
var<private> instanceIndex : u32;

// locals


// structs


// uniforms

struct NodeBuffer_993Struct {
	value : array< vec2<f32> >
};
@binding( 0 ) @group( 0 )
var<storage, read_write> NodeBuffer_993 : NodeBuffer_993Struct;

struct objectStruct {
	nodeUniform1 : u32
};
@binding( 1 ) @group( 0 )
var<uniform> object : objectStruct;

// vars
var<private> nodeVar0 : f32;
var<private> nodeVar1 : f32;

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

	nodeVar0 = ( ( f32( instanceIndex ) * 0.005 ) * 6.283185307179586 );
	nodeVar1 = ( ( f32( instanceIndex ) * 1e-8 ) + 1e-7 );
	NodeBuffer_993.value[ instanceIndex ] = vec2<f32>( ( sin( nodeVar0 ) * nodeVar1 ), ( cos( nodeVar0 ) * nodeVar1 ) );

	

}
