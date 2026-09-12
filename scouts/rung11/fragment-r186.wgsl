// Three.js r186 - Node System

// global
diagnostic( off, derivative_uniformity );


// structs

struct OutputStruct {
	@location( 0 ) color: vec4<f32>
};
var<private> output : OutputStruct;

// uniforms

struct objectStruct {
	nodeUniform3 : vec3<f32>,
	nodeUniform4 : f32,
	nodeUniform6 : mat3x3<f32>,
	nodeUniform8 : mat4x4<f32>
};
@binding( 0 ) @group( 1 )
var<uniform> object : objectStruct;

// vars
var<private> DiffuseColor : vec4<f32>;
var<private> Output : vec4<f32>;
var<private> normalViewGeometry : vec3<f32>;
var<private> normalView : vec3<f32>;

// codes


@fragment
fn main( @location( 0 ) v_normalViewGeometry : vec3<f32>,
	@location( 1 ) @interpolate(flat, either) vBatchIndirectId : u32,
	@location( 2 ) vBatchColor : vec4<f32> ) -> OutputStruct {

	// flow
	// code

	DiffuseColor = ( vBatchColor * vec4<f32>( object.nodeUniform3, 1.0 ) );
	DiffuseColor.w = ( DiffuseColor.w * object.nodeUniform4 );
	DiffuseColor.w = 1.0;
	Output = max( vec4<f32>( DiffuseColor.xyz, DiffuseColor.w ), vec4<f32>( 0.0 ) );
	normalViewGeometry = normalize( v_normalViewGeometry );
	normalView = normalViewGeometry;

	// result

	output.color = vec4<f32>( ( DiffuseColor * vec4<f32>( ( ( ( normalView * vec3<f32>( 0.5 ) ) + vec3<f32>( 0.5 ) ).y + 0.5 ) ) ).xyz, DiffuseColor.w );

	return output;

}
