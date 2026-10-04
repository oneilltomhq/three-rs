// Three.js r187dev - Node System

// global
diagnostic( off, derivative_uniformity );


// directives


// structs

struct OutputStruct {
	@location( 0 ) color: vec4<f32>
};
var<private> output : OutputStruct;

// uniforms
@binding( 0 ) @group( 0 ) var nodeUniform0_sampler : sampler;
@binding( 1 ) @group( 0 ) var nodeUniform0 : texture_2d<f32>;

struct objectStruct {
	nodeUniform1 : vec2<f32>,
	nodeUniform2 : vec2<f32>
};
@binding( 2 ) @group( 0 )
var<uniform> object : objectStruct;

// vars
var<private> nodeVar0 : vec4<f32>;
var<private> nodeVar1 : vec4<f32>;
var<private> nodeVar2 : f32;
var<private> nodeVar3 : f32;
var<private> nodeVar4 : vec4<f32>;
var<private> nodeVar5 : vec2<f32>;
var<private> nodeVar6 : vec4<f32>;
var<private> nodeVar7 : f32;
var<private> nodeVar8 : vec4<f32>;
var<private> nodeVar9 : f32;
var<private> nodeVar10 : vec2<f32>;
var<private> nodeVar11 : vec4<f32>;
var<private> nodeVar12 : f32;
var<private> nodeVar13 : vec4<f32>;
var<private> nodeVar14 : f32;
var<private> nodeVar15 : vec2<f32>;
var<private> nodeVar16 : vec4<f32>;
var<private> nodeVar17 : f32;
var<private> nodeVar18 : vec4<f32>;
var<private> nodeVar19 : f32;
var<private> nodeVar20 : vec2<f32>;
var<private> nodeVar21 : vec4<f32>;
var<private> nodeVar22 : f32;
var<private> nodeVar23 : vec4<f32>;
var<private> nodeVar24 : f32;
var<private> nodeVar25 : vec2<f32>;
var<private> nodeVar26 : vec4<f32>;
var<private> nodeVar27 : f32;
var<private> nodeVar28 : vec4<f32>;
var<private> nodeVar29 : f32;
var<private> nodeVar30 : vec2<f32>;
var<private> nodeVar31 : vec4<f32>;
var<private> nodeVar32 : f32;
var<private> nodeVar33 : vec4<f32>;
var<private> nodeVar34 : f32;
var<private> nodeVar35 : vec2<f32>;
var<private> nodeVar36 : vec4<f32>;
var<private> nodeVar37 : f32;
var<private> nodeVar38 : vec4<f32>;
var<private> nodeVar39 : f32;
var<private> nodeVar40 : vec2<f32>;
var<private> nodeVar41 : vec4<f32>;
var<private> nodeVar42 : f32;
var<private> nodeVar43 : vec4<f32>;
var<private> nodeVar44 : f32;
var<private> nodeVar45 : vec2<f32>;
var<private> nodeVar46 : vec4<f32>;
var<private> nodeVar47 : f32;
var<private> nodeVar48 : vec4<f32>;
var<private> nodeVar49 : f32;
var<private> nodeVar50 : vec2<f32>;
var<private> nodeVar51 : vec4<f32>;
var<private> nodeVar52 : f32;
var<private> nodeVar53 : vec4<f32>;
var<private> nodeVar54 : f32;

// codes


@fragment
fn main( @location( 0 ) nodeVarying0 : vec2<f32> ) -> OutputStruct {

	// flow
	// code

	nodeVar0 = textureSample( nodeUniform0, nodeUniform0_sampler, nodeVarying0 );
	nodeVar1 = nodeVar0;
	nodeVar2 = dot( nodeVar1.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) );
	nodeVar3 = 0.1088018181818182;
	nodeVar4 = ( nodeVar1 * vec4<f32>( 0.1088018181818182 ) );
	const nodeConst0 = ( -0.5 / 0.010000000000000002 );
	let nodeConst1 = ( vec2<f32>( 1.0, 1.0 ) * object.nodeUniform1 );
	nodeVar5 = ( nodeConst1 * ( object.nodeUniform2 * vec2<f32>( 1.0 ) ) );
	nodeVar6 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 + nodeVar5 ) );
	let nodeVar6 = nodeVar6;
	let nodeConst2 = abs( ( dot( nodeVar6.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) ) - nodeVar2 ) );
	nodeVar7 = exp( ( ( nodeConst2 * nodeConst2 ) * nodeConst0 ) );
	nodeVar8 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 - nodeVar5 ) );
	let nodeVar8 = nodeVar8;
	let nodeConst3 = abs( ( dot( nodeVar8.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) ) - nodeVar2 ) );
	nodeVar9 = exp( ( ( nodeConst3 * nodeConst3 ) * nodeConst0 ) );
	let nodeConst4 = ( 0.10482978744722705 * nodeVar7 );
	nodeVar4 = ( nodeVar4 + ( nodeVar6 * vec4<f32>( nodeConst4 ) ) );
	let nodeConst5 = ( 0.10482978744722705 * nodeVar9 );
	nodeVar4 = ( nodeVar4 + ( nodeVar8 * vec4<f32>( nodeConst5 ) ) );
	nodeVar3 = ( nodeVar3 + nodeConst4 );
	nodeVar3 = ( nodeVar3 + nodeConst5 );
	nodeVar10 = ( nodeConst1 * ( object.nodeUniform2 * vec2<f32>( 2.0 ) ) );
	nodeVar11 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 + nodeVar10 ) );
	let nodeVar11 = nodeVar11;
	let nodeConst6 = abs( ( dot( nodeVar11.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) ) - nodeVar2 ) );
	nodeVar12 = exp( ( ( nodeConst6 * nodeConst6 ) * nodeConst0 ) );
	nodeVar13 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 - nodeVar10 ) );
	let nodeVar13 = nodeVar13;
	let nodeConst7 = abs( ( dot( nodeVar13.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) ) - nodeVar2 ) );
	nodeVar14 = exp( ( ( nodeConst7 * nodeConst7 ) * nodeConst0 ) );
	let nodeConst8 = ( 0.09376275556297102 * nodeVar12 );
	nodeVar4 = ( nodeVar4 + ( nodeVar11 * vec4<f32>( nodeConst8 ) ) );
	let nodeConst9 = ( 0.09376275556297102 * nodeVar14 );
	nodeVar4 = ( nodeVar4 + ( nodeVar13 * vec4<f32>( nodeConst9 ) ) );
	nodeVar3 = ( nodeVar3 + nodeConst8 );
	nodeVar3 = ( nodeVar3 + nodeConst9 );
	nodeVar15 = ( nodeConst1 * ( object.nodeUniform2 * vec2<f32>( 3.0 ) ) );
	nodeVar16 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 + nodeVar15 ) );
	let nodeVar16 = nodeVar16;
	let nodeConst10 = abs( ( dot( nodeVar16.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) ) - nodeVar2 ) );
	nodeVar17 = exp( ( ( nodeConst10 * nodeConst10 ) * nodeConst0 ) );
	nodeVar18 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 - nodeVar15 ) );
	let nodeVar18 = nodeVar18;
	let nodeConst11 = abs( ( dot( nodeVar18.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) ) - nodeVar2 ) );
	nodeVar19 = exp( ( ( nodeConst11 * nodeConst11 ) * nodeConst0 ) );
	let nodeConst12 = ( 0.07785260050049682 * nodeVar17 );
	nodeVar4 = ( nodeVar4 + ( nodeVar16 * vec4<f32>( nodeConst12 ) ) );
	let nodeConst13 = ( 0.07785260050049682 * nodeVar19 );
	nodeVar4 = ( nodeVar4 + ( nodeVar18 * vec4<f32>( nodeConst13 ) ) );
	nodeVar3 = ( nodeVar3 + nodeConst12 );
	nodeVar3 = ( nodeVar3 + nodeConst13 );
	nodeVar20 = ( nodeConst1 * ( object.nodeUniform2 * vec2<f32>( 4.0 ) ) );
	nodeVar21 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 + nodeVar20 ) );
	let nodeVar21 = nodeVar21;
	let nodeConst14 = abs( ( dot( nodeVar21.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) ) - nodeVar2 ) );
	nodeVar22 = exp( ( ( nodeConst14 * nodeConst14 ) * nodeConst0 ) );
	nodeVar23 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 - nodeVar20 ) );
	let nodeVar23 = nodeVar23;
	let nodeConst15 = abs( ( dot( nodeVar23.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) ) - nodeVar2 ) );
	nodeVar24 = exp( ( ( nodeConst15 * nodeConst15 ) * nodeConst0 ) );
	let nodeConst16 = ( 0.060008530264881475 * nodeVar22 );
	nodeVar4 = ( nodeVar4 + ( nodeVar21 * vec4<f32>( nodeConst16 ) ) );
	let nodeConst17 = ( 0.060008530264881475 * nodeVar24 );
	nodeVar4 = ( nodeVar4 + ( nodeVar23 * vec4<f32>( nodeConst17 ) ) );
	nodeVar3 = ( nodeVar3 + nodeConst16 );
	nodeVar3 = ( nodeVar3 + nodeConst17 );
	nodeVar25 = ( nodeConst1 * ( object.nodeUniform2 * vec2<f32>( 5.0 ) ) );
	nodeVar26 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 + nodeVar25 ) );
	let nodeVar26 = nodeVar26;
	let nodeConst18 = abs( ( dot( nodeVar26.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) ) - nodeVar2 ) );
	nodeVar27 = exp( ( ( nodeConst18 * nodeConst18 ) * nodeConst0 ) );
	nodeVar28 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 - nodeVar25 ) );
	let nodeVar28 = nodeVar28;
	let nodeConst19 = abs( ( dot( nodeVar28.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) ) - nodeVar2 ) );
	nodeVar29 = exp( ( ( nodeConst19 * nodeConst19 ) * nodeConst0 ) );
	let nodeConst20 = ( 0.042938805724061835 * nodeVar27 );
	nodeVar4 = ( nodeVar4 + ( nodeVar26 * vec4<f32>( nodeConst20 ) ) );
	let nodeConst21 = ( 0.042938805724061835 * nodeVar29 );
	nodeVar4 = ( nodeVar4 + ( nodeVar28 * vec4<f32>( nodeConst21 ) ) );
	nodeVar3 = ( nodeVar3 + nodeConst20 );
	nodeVar3 = ( nodeVar3 + nodeConst21 );
	nodeVar30 = ( nodeConst1 * ( object.nodeUniform2 * vec2<f32>( 6.0 ) ) );
	nodeVar31 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 + nodeVar30 ) );
	let nodeVar31 = nodeVar31;
	let nodeConst22 = abs( ( dot( nodeVar31.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) ) - nodeVar2 ) );
	nodeVar32 = exp( ( ( nodeConst22 * nodeConst22 ) * nodeConst0 ) );
	nodeVar33 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 - nodeVar30 ) );
	let nodeVar33 = nodeVar33;
	let nodeConst23 = abs( ( dot( nodeVar33.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) ) - nodeVar2 ) );
	nodeVar34 = exp( ( ( nodeConst23 * nodeConst23 ) * nodeConst0 ) );
	let nodeConst24 = ( 0.02852226671021147 * nodeVar32 );
	nodeVar4 = ( nodeVar4 + ( nodeVar31 * vec4<f32>( nodeConst24 ) ) );
	let nodeConst25 = ( 0.02852226671021147 * nodeVar34 );
	nodeVar4 = ( nodeVar4 + ( nodeVar33 * vec4<f32>( nodeConst25 ) ) );
	nodeVar3 = ( nodeVar3 + nodeConst24 );
	nodeVar3 = ( nodeVar3 + nodeConst25 );
	nodeVar35 = ( nodeConst1 * ( object.nodeUniform2 * vec2<f32>( 7.0 ) ) );
	nodeVar36 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 + nodeVar35 ) );
	let nodeVar36 = nodeVar36;
	let nodeConst26 = abs( ( dot( nodeVar36.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) ) - nodeVar2 ) );
	nodeVar37 = exp( ( ( nodeConst26 * nodeConst26 ) * nodeConst0 ) );
	nodeVar38 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 - nodeVar35 ) );
	let nodeVar38 = nodeVar38;
	let nodeConst27 = abs( ( dot( nodeVar38.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) ) - nodeVar2 ) );
	nodeVar39 = exp( ( ( nodeConst27 * nodeConst27 ) * nodeConst0 ) );
	let nodeConst28 = ( 0.017587949779416395 * nodeVar37 );
	nodeVar4 = ( nodeVar4 + ( nodeVar36 * vec4<f32>( nodeConst28 ) ) );
	let nodeConst29 = ( 0.017587949779416395 * nodeVar39 );
	nodeVar4 = ( nodeVar4 + ( nodeVar38 * vec4<f32>( nodeConst29 ) ) );
	nodeVar3 = ( nodeVar3 + nodeConst28 );
	nodeVar3 = ( nodeVar3 + nodeConst29 );
	nodeVar40 = ( nodeConst1 * ( object.nodeUniform2 * vec2<f32>( 8.0 ) ) );
	nodeVar41 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 + nodeVar40 ) );
	let nodeVar41 = nodeVar41;
	let nodeConst30 = abs( ( dot( nodeVar41.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) ) - nodeVar2 ) );
	nodeVar42 = exp( ( ( nodeConst30 * nodeConst30 ) * nodeConst0 ) );
	nodeVar43 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 - nodeVar40 ) );
	let nodeVar43 = nodeVar43;
	let nodeConst31 = abs( ( dot( nodeVar43.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) ) - nodeVar2 ) );
	nodeVar44 = exp( ( ( nodeConst31 * nodeConst31 ) * nodeConst0 ) );
	let nodeConst32 = ( 0.010068006836002057 * nodeVar42 );
	nodeVar4 = ( nodeVar4 + ( nodeVar41 * vec4<f32>( nodeConst32 ) ) );
	let nodeConst33 = ( 0.010068006836002057 * nodeVar44 );
	nodeVar4 = ( nodeVar4 + ( nodeVar43 * vec4<f32>( nodeConst33 ) ) );
	nodeVar3 = ( nodeVar3 + nodeConst32 );
	nodeVar3 = ( nodeVar3 + nodeConst33 );
	nodeVar45 = ( nodeConst1 * ( object.nodeUniform2 * vec2<f32>( 9.0 ) ) );
	nodeVar46 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 + nodeVar45 ) );
	let nodeVar46 = nodeVar46;
	let nodeConst34 = abs( ( dot( nodeVar46.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) ) - nodeVar2 ) );
	nodeVar47 = exp( ( ( nodeConst34 * nodeConst34 ) * nodeConst0 ) );
	nodeVar48 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 - nodeVar45 ) );
	let nodeVar48 = nodeVar48;
	let nodeConst35 = abs( ( dot( nodeVar48.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) ) - nodeVar2 ) );
	nodeVar49 = exp( ( ( nodeConst35 * nodeConst35 ) * nodeConst0 ) );
	let nodeConst36 = ( 0.005350186131820889 * nodeVar47 );
	nodeVar4 = ( nodeVar4 + ( nodeVar46 * vec4<f32>( nodeConst36 ) ) );
	let nodeConst37 = ( 0.005350186131820889 * nodeVar49 );
	nodeVar4 = ( nodeVar4 + ( nodeVar48 * vec4<f32>( nodeConst37 ) ) );
	nodeVar3 = ( nodeVar3 + nodeConst36 );
	nodeVar3 = ( nodeVar3 + nodeConst37 );
	nodeVar50 = ( nodeConst1 * ( object.nodeUniform2 * vec2<f32>( 10.0 ) ) );
	nodeVar51 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 + nodeVar50 ) );
	let nodeVar51 = nodeVar51;
	let nodeConst38 = abs( ( dot( nodeVar51.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) ) - nodeVar2 ) );
	nodeVar52 = exp( ( ( nodeConst38 * nodeConst38 ) * nodeConst0 ) );
	nodeVar53 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 - nodeVar50 ) );
	let nodeVar53 = nodeVar53;
	let nodeConst39 = abs( ( dot( nodeVar53.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) ) - nodeVar2 ) );
	nodeVar54 = exp( ( ( nodeConst39 * nodeConst39 ) * nodeConst0 ) );
	let nodeConst40 = ( 0.002639315969304918 * nodeVar52 );
	nodeVar4 = ( nodeVar4 + ( nodeVar51 * vec4<f32>( nodeConst40 ) ) );
	let nodeConst41 = ( 0.002639315969304918 * nodeVar54 );
	nodeVar4 = ( nodeVar4 + ( nodeVar53 * vec4<f32>( nodeConst41 ) ) );
	nodeVar3 = ( nodeVar3 + nodeConst40 );
	nodeVar3 = ( nodeVar3 + nodeConst41 );

	// result

	output.color = ( nodeVar4 / vec4<f32>( max( nodeVar3, 0.0001 ) ) );

	return output;

}
