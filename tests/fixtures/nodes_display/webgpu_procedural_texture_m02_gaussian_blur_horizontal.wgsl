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
	nodeUniform1 : f32,
	nodeUniform2 : vec2<f32>
};
@binding( 2 ) @group( 0 )
var<uniform> object : objectStruct;

// vars
var<private> nodeVar0 : vec4<f32>;
var<private> nodeVar1 : vec4<f32>;
var<private> nodeVar2 : vec2<f32>;
var<private> nodeVar3 : vec4<f32>;
var<private> nodeVar4 : vec4<f32>;
var<private> nodeVar5 : vec2<f32>;
var<private> nodeVar6 : vec4<f32>;
var<private> nodeVar7 : vec4<f32>;
var<private> nodeVar8 : vec2<f32>;
var<private> nodeVar9 : vec4<f32>;
var<private> nodeVar10 : vec4<f32>;
var<private> nodeVar11 : vec2<f32>;
var<private> nodeVar12 : vec4<f32>;
var<private> nodeVar13 : vec4<f32>;
var<private> nodeVar14 : vec2<f32>;
var<private> nodeVar15 : vec4<f32>;
var<private> nodeVar16 : vec4<f32>;
var<private> nodeVar17 : vec2<f32>;
var<private> nodeVar18 : vec4<f32>;
var<private> nodeVar19 : vec4<f32>;
var<private> nodeVar20 : vec2<f32>;
var<private> nodeVar21 : vec4<f32>;
var<private> nodeVar22 : vec4<f32>;
var<private> nodeVar23 : vec2<f32>;
var<private> nodeVar24 : vec4<f32>;
var<private> nodeVar25 : vec4<f32>;
var<private> nodeVar26 : vec2<f32>;
var<private> nodeVar27 : vec4<f32>;
var<private> nodeVar28 : vec4<f32>;
var<private> nodeVar29 : vec2<f32>;
var<private> nodeVar30 : vec4<f32>;
var<private> nodeVar31 : vec4<f32>;
var<private> nodeVar32 : vec2<f32>;
var<private> nodeVar33 : vec4<f32>;
var<private> nodeVar34 : vec4<f32>;
var<private> nodeVar35 : vec2<f32>;
var<private> nodeVar36 : vec4<f32>;
var<private> nodeVar37 : vec4<f32>;
var<private> nodeVar38 : vec2<f32>;
var<private> nodeVar39 : vec4<f32>;
var<private> nodeVar40 : vec4<f32>;
var<private> nodeVar41 : vec2<f32>;
var<private> nodeVar42 : vec4<f32>;
var<private> nodeVar43 : vec4<f32>;
var<private> nodeVar44 : vec2<f32>;
var<private> nodeVar45 : vec4<f32>;
var<private> nodeVar46 : vec4<f32>;
var<private> nodeVar47 : vec2<f32>;
var<private> nodeVar48 : vec4<f32>;
var<private> nodeVar49 : vec4<f32>;
var<private> nodeVar50 : vec2<f32>;
var<private> nodeVar51 : vec4<f32>;
var<private> nodeVar52 : vec4<f32>;
var<private> nodeVar53 : vec2<f32>;
var<private> nodeVar54 : vec4<f32>;
var<private> nodeVar55 : vec4<f32>;
var<private> nodeVar56 : vec2<f32>;
var<private> nodeVar57 : vec4<f32>;
var<private> nodeVar58 : vec4<f32>;
var<private> nodeVar59 : vec2<f32>;
var<private> nodeVar60 : vec4<f32>;
var<private> nodeVar61 : vec4<f32>;
var<private> nodeVar62 : vec2<f32>;
var<private> nodeVar63 : vec4<f32>;
var<private> nodeVar64 : vec4<f32>;
var<private> nodeVar65 : vec2<f32>;
var<private> nodeVar66 : vec4<f32>;
var<private> nodeVar67 : vec4<f32>;
var<private> nodeVar68 : vec2<f32>;
var<private> nodeVar69 : vec4<f32>;
var<private> nodeVar70 : vec4<f32>;
var<private> nodeVar71 : vec2<f32>;
var<private> nodeVar72 : vec4<f32>;
var<private> nodeVar73 : vec4<f32>;
var<private> nodeVar74 : vec2<f32>;
var<private> nodeVar75 : vec4<f32>;
var<private> nodeVar76 : vec4<f32>;
var<private> nodeVar77 : vec2<f32>;
var<private> nodeVar78 : vec4<f32>;
var<private> nodeVar79 : vec4<f32>;
var<private> nodeVar80 : vec2<f32>;
var<private> nodeVar81 : vec4<f32>;
var<private> nodeVar82 : vec4<f32>;
var<private> nodeVar83 : vec2<f32>;
var<private> nodeVar84 : vec4<f32>;
var<private> nodeVar85 : vec4<f32>;
var<private> nodeVar86 : vec2<f32>;
var<private> nodeVar87 : vec4<f32>;
var<private> nodeVar88 : vec4<f32>;
var<private> nodeVar89 : vec2<f32>;
var<private> nodeVar90 : vec4<f32>;
var<private> nodeVar91 : vec4<f32>;
var<private> nodeVar92 : vec2<f32>;
var<private> nodeVar93 : vec4<f32>;
var<private> nodeVar94 : vec4<f32>;
var<private> nodeVar95 : vec2<f32>;
var<private> nodeVar96 : vec4<f32>;
var<private> nodeVar97 : vec4<f32>;
var<private> nodeVar98 : vec2<f32>;
var<private> nodeVar99 : vec4<f32>;
var<private> nodeVar100 : vec4<f32>;
var<private> nodeVar101 : vec2<f32>;
var<private> nodeVar102 : vec4<f32>;
var<private> nodeVar103 : vec4<f32>;
var<private> nodeVar104 : vec2<f32>;
var<private> nodeVar105 : vec4<f32>;
var<private> nodeVar106 : vec4<f32>;
var<private> nodeVar107 : vec2<f32>;
var<private> nodeVar108 : vec4<f32>;
var<private> nodeVar109 : vec4<f32>;
var<private> nodeVar110 : vec2<f32>;
var<private> nodeVar111 : vec4<f32>;
var<private> nodeVar112 : vec4<f32>;
var<private> nodeVar113 : vec2<f32>;
var<private> nodeVar114 : vec4<f32>;
var<private> nodeVar115 : vec4<f32>;
var<private> nodeVar116 : vec2<f32>;
var<private> nodeVar117 : vec4<f32>;
var<private> nodeVar118 : vec4<f32>;
var<private> nodeVar119 : vec2<f32>;
var<private> nodeVar120 : vec4<f32>;
var<private> nodeVar121 : vec4<f32>;
var<private> nodeVar122 : vec2<f32>;
var<private> nodeVar123 : vec4<f32>;
var<private> nodeVar124 : vec4<f32>;
var<private> nodeVar125 : vec2<f32>;
var<private> nodeVar126 : vec4<f32>;
var<private> nodeVar127 : vec4<f32>;

// codes


@fragment
fn main( @location( 0 ) nodeVarying0 : vec2<f32> ) -> OutputStruct {

	// flow
	// code

	nodeVar0 = textureSample( nodeUniform0, nodeUniform0_sampler, nodeVarying0 );
	nodeVar1 = ( nodeVar0 * vec4<f32>( 0.027917486897724192 ) );
	let nodeConst0 = ( vec2<f32>( object.nodeUniform1 ) * vec2<f32>( 1.0, 0.0 ) );
	nodeVar2 = ( nodeConst0 * ( object.nodeUniform2 * vec2<f32>( 1.0 ) ) );
	nodeVar3 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 + nodeVar2 ) );
	nodeVar4 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 - nodeVar2 ) );
	nodeVar1 = ( nodeVar1 + ( ( nodeVar3 + nodeVar4 ) * vec4<f32>( 0.027849625383014897 ) ) );
	nodeVar5 = ( nodeConst0 * ( object.nodeUniform2 * vec2<f32>( 2.0 ) ) );
	nodeVar6 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 + nodeVar5 ) );
	nodeVar7 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 - nodeVar5 ) );
	nodeVar1 = ( nodeVar1 + ( ( nodeVar6 + nodeVar7 ) * vec4<f32>( 0.027647028978021935 ) ) );
	nodeVar8 = ( nodeConst0 * ( object.nodeUniform2 * vec2<f32>( 3.0 ) ) );
	nodeVar9 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 + nodeVar8 ) );
	nodeVar10 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 - nodeVar8 ) );
	nodeVar1 = ( nodeVar1 + ( ( nodeVar9 + nodeVar10 ) * vec4<f32>( 0.027312638158275185 ) ) );
	nodeVar11 = ( nodeConst0 * ( object.nodeUniform2 * vec2<f32>( 4.0 ) ) );
	nodeVar12 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 + nodeVar11 ) );
	nodeVar13 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 - nodeVar11 ) );
	nodeVar1 = ( nodeVar1 + ( ( nodeVar12 + nodeVar13 ) * vec4<f32>( 0.026851274720893416 ) ) );
	nodeVar14 = ( nodeConst0 * ( object.nodeUniform2 * vec2<f32>( 5.0 ) ) );
	nodeVar15 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 + nodeVar14 ) );
	nodeVar16 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 - nodeVar14 ) );
	nodeVar1 = ( nodeVar1 + ( ( nodeVar15 + nodeVar16 ) * vec4<f32>( 0.02626952609405497 ) ) );
	nodeVar17 = ( nodeConst0 * ( object.nodeUniform2 * vec2<f32>( 6.0 ) ) );
	nodeVar18 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 + nodeVar17 ) );
	nodeVar19 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 - nodeVar17 ) );
	nodeVar1 = ( nodeVar1 + ( ( nodeVar18 + nodeVar19 ) * vec4<f32>( 0.025575588850468962 ) ) );
	nodeVar20 = ( nodeConst0 * ( object.nodeUniform2 * vec2<f32>( 7.0 ) ) );
	nodeVar21 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 + nodeVar20 ) );
	nodeVar22 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 - nodeVar20 ) );
	nodeVar1 = ( nodeVar1 + ( ( nodeVar21 + nodeVar22 ) * vec4<f32>( 0.024779076620079426 ) ) );
	nodeVar23 = ( nodeConst0 * ( object.nodeUniform2 * vec2<f32>( 8.0 ) ) );
	nodeVar24 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 + nodeVar23 ) );
	nodeVar25 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 - nodeVar23 ) );
	nodeVar1 = ( nodeVar1 + ( ( nodeVar24 + nodeVar25 ) * vec4<f32>( 0.02389079869137512 ) ) );
	nodeVar26 = ( nodeConst0 * ( object.nodeUniform2 * vec2<f32>( 9.0 ) ) );
	nodeVar27 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 + nodeVar26 ) );
	nodeVar28 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 - nodeVar26 ) );
	nodeVar1 = ( nodeVar1 + ( ( nodeVar27 + nodeVar28 ) * vec4<f32>( 0.022922516420596662 ) ) );
	nodeVar29 = ( nodeConst0 * ( object.nodeUniform2 * vec2<f32>( 10.0 ) ) );
	nodeVar30 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 + nodeVar29 ) );
	nodeVar31 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 - nodeVar29 ) );
	nodeVar1 = ( nodeVar1 + ( ( nodeVar30 + nodeVar31 ) * vec4<f32>( 0.02188668510451569 ) ) );
	nodeVar32 = ( nodeConst0 * ( object.nodeUniform2 * vec2<f32>( 11.0 ) ) );
	nodeVar33 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 + nodeVar32 ) );
	nodeVar34 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 - nodeVar32 ) );
	nodeVar1 = ( nodeVar1 + ( ( nodeVar33 + nodeVar34 ) * vec4<f32>( 0.02079618920074556 ) ) );
	nodeVar35 = ( nodeConst0 * ( object.nodeUniform2 * vec2<f32>( 12.0 ) ) );
	nodeVar36 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 + nodeVar35 ) );
	nodeVar37 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 - nodeVar35 ) );
	nodeVar1 = ( nodeVar1 + ( ( nodeVar36 + nodeVar37 ) * vec4<f32>( 0.019664078700254686 ) ) );
	nodeVar38 = ( nodeConst0 * ( object.nodeUniform2 * vec2<f32>( 13.0 ) ) );
	nodeVar39 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 + nodeVar38 ) );
	nodeVar40 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 - nodeVar38 ) );
	nodeVar1 = ( nodeVar1 + ( ( nodeVar39 + nodeVar40 ) * vec4<f32>( 0.018503314084979513 ) ) );
	nodeVar41 = ( nodeConst0 * ( object.nodeUniform2 * vec2<f32>( 14.0 ) ) );
	nodeVar42 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 + nodeVar41 ) );
	nodeVar43 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 - nodeVar41 ) );
	nodeVar1 = ( nodeVar1 + ( ( nodeVar42 + nodeVar43 ) * vec4<f32>( 0.017326526667477818 ) ) );
	nodeVar44 = ( nodeConst0 * ( object.nodeUniform2 * vec2<f32>( 15.0 ) ) );
	nodeVar45 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 + nodeVar44 ) );
	nodeVar46 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 - nodeVar44 ) );
	nodeVar1 = ( nodeVar1 + ( ( nodeVar45 + nodeVar46 ) * vec4<f32>( 0.01614580024891989 ) ) );
	nodeVar47 = ( nodeConst0 * ( object.nodeUniform2 * vec2<f32>( 16.0 ) ) );
	nodeVar48 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 + nodeVar47 ) );
	nodeVar49 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 - nodeVar47 ) );
	nodeVar1 = ( nodeVar1 + ( ( nodeVar48 + nodeVar49 ) * vec4<f32>( 0.014972478994488754 ) ) );
	nodeVar50 = ( nodeConst0 * ( object.nodeUniform2 * vec2<f32>( 17.0 ) ) );
	nodeVar51 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 + nodeVar50 ) );
	nodeVar52 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 - nodeVar50 ) );
	nodeVar1 = ( nodeVar1 + ( ( nodeVar51 + nodeVar52 ) * vec4<f32>( 0.013817005265239128 ) ) );
	nodeVar53 = ( nodeConst0 * ( object.nodeUniform2 * vec2<f32>( 18.0 ) ) );
	nodeVar54 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 + nodeVar53 ) );
	nodeVar55 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 - nodeVar53 ) );
	nodeVar1 = ( nodeVar1 + ( ( nodeVar54 + nodeVar55 ) * vec4<f32>( 0.012688789919021358 ) ) );
	nodeVar56 = ( nodeConst0 * ( object.nodeUniform2 * vec2<f32>( 19.0 ) ) );
	nodeVar57 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 + nodeVar56 ) );
	nodeVar58 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 - nodeVar56 ) );
	nodeVar1 = ( nodeVar1 + ( ( nodeVar57 + nodeVar58 ) * vec4<f32>( 0.011596116356118253 ) ) );
	nodeVar59 = ( nodeConst0 * ( object.nodeUniform2 * vec2<f32>( 20.0 ) ) );
	nodeVar60 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 + nodeVar59 ) );
	nodeVar61 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 - nodeVar59 ) );
	nodeVar1 = ( nodeVar1 + ( ( nodeVar60 + nodeVar61 ) * vec4<f32>( 0.010546078390370368 ) ) );
	nodeVar62 = ( nodeConst0 * ( object.nodeUniform2 * vec2<f32>( 21.0 ) ) );
	nodeVar63 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 + nodeVar62 ) );
	nodeVar64 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 - nodeVar62 ) );
	nodeVar1 = ( nodeVar1 + ( ( nodeVar63 + nodeVar64 ) * vec4<f32>( 0.009544550920616164 ) ) );
	nodeVar65 = ( nodeConst0 * ( object.nodeUniform2 * vec2<f32>( 22.0 ) ) );
	nodeVar66 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 + nodeVar65 ) );
	nodeVar67 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 - nodeVar65 ) );
	nodeVar1 = ( nodeVar1 + ( ( nodeVar66 + nodeVar67 ) * vec4<f32>( 0.008596191399359158 ) ) );
	nodeVar68 = ( nodeConst0 * ( object.nodeUniform2 * vec2<f32>( 23.0 ) ) );
	nodeVar69 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 + nodeVar68 ) );
	nodeVar70 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 - nodeVar68 ) );
	nodeVar1 = ( nodeVar1 + ( ( nodeVar69 + nodeVar70 ) * vec4<f32>( 0.007704469275701113 ) ) );
	nodeVar71 = ( nodeConst0 * ( object.nodeUniform2 * vec2<f32>( 24.0 ) ) );
	nodeVar72 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 + nodeVar71 ) );
	nodeVar73 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 - nodeVar71 ) );
	nodeVar1 = ( nodeVar1 + ( ( nodeVar72 + nodeVar73 ) * vec4<f32>( 0.006871719947884998 ) ) );
	nodeVar74 = ( nodeConst0 * ( object.nodeUniform2 * vec2<f32>( 25.0 ) ) );
	nodeVar75 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 + nodeVar74 ) );
	nodeVar76 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 - nodeVar74 ) );
	nodeVar1 = ( nodeVar1 + ( ( nodeVar75 + nodeVar76 ) * vec4<f32>( 0.006099219307388703 ) ) );
	nodeVar77 = ( nodeConst0 * ( object.nodeUniform2 * vec2<f32>( 26.0 ) ) );
	nodeVar78 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 + nodeVar77 ) );
	nodeVar79 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 - nodeVar77 ) );
	nodeVar1 = ( nodeVar1 + ( ( nodeVar78 + nodeVar79 ) * vec4<f32>( 0.00538727469190783 ) ) );
	nodeVar80 = ( nodeConst0 * ( object.nodeUniform2 * vec2<f32>( 27.0 ) ) );
	nodeVar81 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 + nodeVar80 ) );
	nodeVar82 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 - nodeVar80 ) );
	nodeVar1 = ( nodeVar1 + ( ( nodeVar81 + nodeVar82 ) * vec4<f32>( 0.004735327980566895 ) ) );
	nodeVar83 = ( nodeConst0 * ( object.nodeUniform2 * vec2<f32>( 28.0 ) ) );
	nodeVar84 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 + nodeVar83 ) );
	nodeVar85 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 - nodeVar83 ) );
	nodeVar1 = ( nodeVar1 + ( ( nodeVar84 + nodeVar85 ) * vec4<f32>( 0.004142066645696151 ) ) );
	nodeVar86 = ( nodeConst0 * ( object.nodeUniform2 * vec2<f32>( 29.0 ) ) );
	nodeVar87 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 + nodeVar86 ) );
	nodeVar88 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 - nodeVar86 ) );
	nodeVar1 = ( nodeVar1 + ( ( nodeVar87 + nodeVar88 ) * vec4<f32>( 0.0036055388000524225 ) ) );
	nodeVar89 = ( nodeConst0 * ( object.nodeUniform2 * vec2<f32>( 30.0 ) ) );
	nodeVar90 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 + nodeVar89 ) );
	nodeVar91 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 - nodeVar89 ) );
	nodeVar1 = ( nodeVar1 + ( ( nodeVar90 + nodeVar91 ) * vec4<f32>( 0.0031232686208646195 ) ) );
	nodeVar92 = ( nodeConst0 * ( object.nodeUniform2 * vec2<f32>( 31.0 ) ) );
	nodeVar93 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 + nodeVar92 ) );
	nodeVar94 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 - nodeVar92 ) );
	nodeVar1 = ( nodeVar1 + ( ( nodeVar93 + nodeVar94 ) * vec4<f32>( 0.002692368964583971 ) ) );
	nodeVar95 = ( nodeConst0 * ( object.nodeUniform2 * vec2<f32>( 32.0 ) ) );
	nodeVar96 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 + nodeVar95 ) );
	nodeVar97 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 - nodeVar95 ) );
	nodeVar1 = ( nodeVar1 + ( ( nodeVar96 + nodeVar97 ) * vec4<f32>( 0.002309648480047827 ) ) );
	nodeVar98 = ( nodeConst0 * ( object.nodeUniform2 * vec2<f32>( 33.0 ) ) );
	nodeVar99 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 + nodeVar98 ) );
	nodeVar100 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 - nodeVar98 ) );
	nodeVar1 = ( nodeVar1 + ( ( nodeVar99 + nodeVar100 ) * vec4<f32>( 0.0019717110550623745 ) ) );
	nodeVar101 = ( nodeConst0 * ( object.nodeUniform2 * vec2<f32>( 34.0 ) ) );
	nodeVar102 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 + nodeVar101 ) );
	nodeVar103 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 - nodeVar101 ) );
	nodeVar1 = ( nodeVar1 + ( ( nodeVar102 + nodeVar103 ) * vec4<f32>( 0.0016750459663994136 ) ) );
	nodeVar104 = ( nodeConst0 * ( object.nodeUniform2 * vec2<f32>( 35.0 ) ) );
	nodeVar105 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 + nodeVar104 ) );
	nodeVar106 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 - nodeVar104 ) );
	nodeVar1 = ( nodeVar1 + ( ( nodeVar105 + nodeVar106 ) * vec4<f32>( 0.0014161076231992609 ) ) );
	nodeVar107 = ( nodeConst0 * ( object.nodeUniform2 * vec2<f32>( 36.0 ) ) );
	nodeVar108 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 + nodeVar107 ) );
	nodeVar109 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 - nodeVar107 ) );
	nodeVar1 = ( nodeVar1 + ( ( nodeVar108 + nodeVar109 ) * vec4<f32>( 0.0011913842798807258 ) ) );
	nodeVar110 = ( nodeConst0 * ( object.nodeUniform2 * vec2<f32>( 37.0 ) ) );
	nodeVar111 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 + nodeVar110 ) );
	nodeVar112 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 - nodeVar110 ) );
	nodeVar1 = ( nodeVar1 + ( ( nodeVar111 + nodeVar112 ) * vec4<f32>( 0.000997455532177249 ) ) );
	nodeVar113 = ( nodeConst0 * ( object.nodeUniform2 * vec2<f32>( 38.0 ) ) );
	nodeVar114 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 + nodeVar113 ) );
	nodeVar115 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 - nodeVar113 ) );
	nodeVar1 = ( nodeVar1 + ( ( nodeVar114 + nodeVar115 ) * vec4<f32>( 0.0008310387884561469 ) ) );
	nodeVar116 = ( nodeConst0 * ( object.nodeUniform2 * vec2<f32>( 39.0 ) ) );
	nodeVar117 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 + nodeVar116 ) );
	nodeVar118 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 - nodeVar116 ) );
	nodeVar1 = ( nodeVar1 + ( ( nodeVar117 + nodeVar118 ) * vec4<f32>( 0.0006890252218306068 ) ) );
	nodeVar119 = ( nodeConst0 * ( object.nodeUniform2 * vec2<f32>( 40.0 ) ) );
	nodeVar120 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 + nodeVar119 ) );
	nodeVar121 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 - nodeVar119 ) );
	nodeVar1 = ( nodeVar1 + ( ( nodeVar120 + nodeVar121 ) * vec4<f32>( 0.000568505954389495 ) ) );
	nodeVar122 = ( nodeConst0 * ( object.nodeUniform2 * vec2<f32>( 41.0 ) ) );
	nodeVar123 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 + nodeVar122 ) );
	nodeVar124 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 - nodeVar122 ) );
	nodeVar1 = ( nodeVar1 + ( ( nodeVar123 + nodeVar124 ) * vec4<f32>( 0.0004667894041623962 ) ) );
	nodeVar125 = ( nodeConst0 * ( object.nodeUniform2 * vec2<f32>( 42.0 ) ) );
	nodeVar126 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 + nodeVar125 ) );
	nodeVar127 = textureSample( nodeUniform0, nodeUniform0_sampler, ( nodeVarying0 - nodeVar125 ) );
	nodeVar1 = ( nodeVar1 + ( ( nodeVar126 + nodeVar127 ) * vec4<f32>( 0.0003814108419989991 ) ) );

	// result

	output.color = nodeVar1;

	return output;

}
