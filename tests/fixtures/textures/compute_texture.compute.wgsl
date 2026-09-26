// Three.js r187dev - Node System

// directives
enable subgroups;

// system
var<private> instanceIndex : u32;

// locals


// structs


// uniforms
@binding( 0 ) @group( 0 ) var nodeUniform0 : texture_storage_2d<rgba8unorm, write>;

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

	let nodeConst0 = ( instanceIndex % 512u );
	let nodeConst1 = ( instanceIndex / 512u );
	let nodeConst2 = ( f32( nodeConst0 ) / 50.0 );
	let nodeConst3 = ( f32( nodeConst1 ) / 50.0 );
	let nodeConst4 = ( ( ( sin( nodeConst2 ) + sin( nodeConst3 ) ) + sin( ( nodeConst2 + nodeConst3 ) ) ) + sin( ( sqrt( ( ( nodeConst2 * nodeConst2 ) + ( nodeConst3 * nodeConst3 ) ) ) + 5.0 ) ) );
	textureStore( nodeUniform0, vec2<u32>( nodeConst0, nodeConst1 ), vec4<f32>( sin( nodeConst4 ), sin( ( nodeConst4 + 3.141592653589793 ) ), sin( ( ( nodeConst4 + 3.141592653589793 ) - 0.5 ) ), 1.0 ) );

	

}
