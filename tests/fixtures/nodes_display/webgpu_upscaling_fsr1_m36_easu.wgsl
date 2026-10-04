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

// vars
var<private> nodeVar0 : vec2<f32>;
var<private> nodeVar1 : f32;
var<private> nodeVar2 : vec4<f32>;
var<private> nodeVar3 : vec4<f32>;
var<private> nodeVar4 : vec4<f32>;
var<private> nodeVar5 : vec4<f32>;
var<private> nodeVar6 : vec4<f32>;
var<private> nodeVar7 : vec4<f32>;
var<private> nodeVar8 : vec4<f32>;
var<private> nodeVar9 : vec4<f32>;
var<private> nodeVar10 : vec4<f32>;
var<private> nodeVar11 : vec4<f32>;
var<private> nodeVar12 : vec4<f32>;
var<private> nodeVar13 : vec4<f32>;
var<private> nodeVar14 : f32;
var<private> nodeVar15 : f32;
var<private> nodeVar16 : vec4<f32>;
var<private> nodeVar17 : f32;

// codes


@fragment
fn main( @location( 0 ) nodeVarying0 : vec2<f32> ) -> OutputStruct {

	// flow
	// code

	let nodeConst0 = ( ( nodeVarying0 * vec2<f32>( textureDimensions( nodeUniform0, 0 ) ) ) - vec2<f32>( 0.5 ) );
	let nodeConst1 = floor( nodeConst0 );
	let nodeConst2 = fract( nodeConst0 );
	let nodeConst3 = vec2<i32>( i32( nodeConst1.x ), i32( nodeConst1.y ) );
	nodeVar0 = vec2<f32>( 0.0, 0.0 );
	nodeVar1 = 0.0;
	let nodeConst4 = ( ( 1.0 - nodeConst2.x ) * ( 1.0 - nodeConst2.y ) );
	let nodeConst5 = ( nodeConst2.x * ( 1.0 - nodeConst2.y ) );
	let nodeConst6 = ( ( 1.0 - nodeConst2.x ) * nodeConst2.y );
	let nodeConst7 = ( nodeConst2.x * nodeConst2.y );
	nodeVar2 = textureLoad( nodeUniform0, ( nodeConst3 + vec2<i32>( 1, 0 ) ), u32( 0u ) );
	let nodeConst8 = ( ( ( nodeVar2.x * 0.5 ) + nodeVar2.y ) + ( nodeVar2.z * 0.5 ) );
	nodeVar3 = textureLoad( nodeUniform0, ( nodeConst3 + vec2<i32>( 0, 0 ) ), u32( 0u ) );
	let nodeConst9 = ( ( ( nodeVar3.x * 0.5 ) + nodeVar3.y ) + ( nodeVar3.z * 0.5 ) );
	let nodeConst10 = ( nodeConst8 - nodeConst9 );
	nodeVar4 = textureLoad( nodeUniform0, ( nodeConst3 + vec2<i32>( -1, 0 ) ), u32( 0u ) );
	let nodeConst11 = ( ( ( nodeVar4.x * 0.5 ) + nodeVar4.y ) + ( nodeVar4.z * 0.5 ) );
	let nodeConst12 = ( nodeConst9 - nodeConst11 );
	let nodeConst13 = ( nodeConst8 - nodeConst11 );
	let nodeConst14 = max( abs( nodeConst10 ), abs( nodeConst12 ) );
	let nodeConst15 = clamp( ( abs( nodeConst13 ) / max( nodeConst14, 0.0000152587890625 ) ), 0.0, 1.0 );
	nodeVar0.x = ( nodeVar0.x + ( nodeConst13 * nodeConst4 ) );
	nodeVar1 = ( nodeVar1 + ( ( nodeConst15 * nodeConst15 ) * nodeConst4 ) );
	nodeVar5 = textureLoad( nodeUniform0, ( nodeConst3 + vec2<i32>( 0, 1 ) ), u32( 0u ) );
	let nodeConst16 = ( ( ( nodeVar5.x * 0.5 ) + nodeVar5.y ) + ( nodeVar5.z * 0.5 ) );
	let nodeConst17 = ( nodeConst16 - nodeConst9 );
	nodeVar6 = textureLoad( nodeUniform0, ( nodeConst3 + vec2<i32>( 0, -1 ) ), u32( 0u ) );
	let nodeConst18 = ( ( ( nodeVar6.x * 0.5 ) + nodeVar6.y ) + ( nodeVar6.z * 0.5 ) );
	let nodeConst19 = ( nodeConst9 - nodeConst18 );
	let nodeConst20 = ( nodeConst16 - nodeConst18 );
	let nodeConst21 = max( abs( nodeConst17 ), abs( nodeConst19 ) );
	let nodeConst22 = clamp( ( abs( nodeConst20 ) / max( nodeConst21, 0.0000152587890625 ) ), 0.0, 1.0 );
	nodeVar0.y = ( nodeVar0.y + ( nodeConst20 * nodeConst4 ) );
	nodeVar1 = ( nodeVar1 + ( ( nodeConst22 * nodeConst22 ) * nodeConst4 ) );
	nodeVar7 = textureLoad( nodeUniform0, ( nodeConst3 + vec2<i32>( 2, 0 ) ), u32( 0u ) );
	let nodeConst23 = ( ( ( nodeVar7.x * 0.5 ) + nodeVar7.y ) + ( nodeVar7.z * 0.5 ) );
	let nodeConst24 = ( nodeConst23 - nodeConst8 );
	let nodeConst25 = ( nodeConst8 - nodeConst9 );
	let nodeConst26 = ( nodeConst23 - nodeConst9 );
	let nodeConst27 = max( abs( nodeConst24 ), abs( nodeConst25 ) );
	let nodeConst28 = clamp( ( abs( nodeConst26 ) / max( nodeConst27, 0.0000152587890625 ) ), 0.0, 1.0 );
	nodeVar0.x = ( nodeVar0.x + ( nodeConst26 * nodeConst5 ) );
	nodeVar1 = ( nodeVar1 + ( ( nodeConst28 * nodeConst28 ) * nodeConst5 ) );
	nodeVar8 = textureLoad( nodeUniform0, ( nodeConst3 + vec2<i32>( 1, 1 ) ), u32( 0u ) );
	let nodeConst29 = ( ( ( nodeVar8.x * 0.5 ) + nodeVar8.y ) + ( nodeVar8.z * 0.5 ) );
	let nodeConst30 = ( nodeConst29 - nodeConst8 );
	nodeVar9 = textureLoad( nodeUniform0, ( nodeConst3 + vec2<i32>( 1, -1 ) ), u32( 0u ) );
	let nodeConst31 = ( ( ( nodeVar9.x * 0.5 ) + nodeVar9.y ) + ( nodeVar9.z * 0.5 ) );
	let nodeConst32 = ( nodeConst8 - nodeConst31 );
	let nodeConst33 = ( nodeConst29 - nodeConst31 );
	let nodeConst34 = max( abs( nodeConst30 ), abs( nodeConst32 ) );
	let nodeConst35 = clamp( ( abs( nodeConst33 ) / max( nodeConst34, 0.0000152587890625 ) ), 0.0, 1.0 );
	nodeVar0.y = ( nodeVar0.y + ( nodeConst33 * nodeConst5 ) );
	nodeVar1 = ( nodeVar1 + ( ( nodeConst35 * nodeConst35 ) * nodeConst5 ) );
	let nodeConst36 = ( nodeConst29 - nodeConst16 );
	nodeVar10 = textureLoad( nodeUniform0, ( nodeConst3 + vec2<i32>( -1, 1 ) ), u32( 0u ) );
	let nodeConst37 = ( ( ( nodeVar10.x * 0.5 ) + nodeVar10.y ) + ( nodeVar10.z * 0.5 ) );
	let nodeConst38 = ( nodeConst16 - nodeConst37 );
	let nodeConst39 = ( nodeConst29 - nodeConst37 );
	let nodeConst40 = max( abs( nodeConst36 ), abs( nodeConst38 ) );
	let nodeConst41 = clamp( ( abs( nodeConst39 ) / max( nodeConst40, 0.0000152587890625 ) ), 0.0, 1.0 );
	nodeVar0.x = ( nodeVar0.x + ( nodeConst39 * nodeConst6 ) );
	nodeVar1 = ( nodeVar1 + ( ( nodeConst41 * nodeConst41 ) * nodeConst6 ) );
	nodeVar11 = textureLoad( nodeUniform0, ( nodeConst3 + vec2<i32>( 0, 2 ) ), u32( 0u ) );
	let nodeConst42 = ( ( ( nodeVar11.x * 0.5 ) + nodeVar11.y ) + ( nodeVar11.z * 0.5 ) );
	let nodeConst43 = ( nodeConst42 - nodeConst16 );
	let nodeConst44 = ( nodeConst16 - nodeConst9 );
	let nodeConst45 = ( nodeConst42 - nodeConst9 );
	let nodeConst46 = max( abs( nodeConst43 ), abs( nodeConst44 ) );
	let nodeConst47 = clamp( ( abs( nodeConst45 ) / max( nodeConst46, 0.0000152587890625 ) ), 0.0, 1.0 );
	nodeVar0.y = ( nodeVar0.y + ( nodeConst45 * nodeConst6 ) );
	nodeVar1 = ( nodeVar1 + ( ( nodeConst47 * nodeConst47 ) * nodeConst6 ) );
	nodeVar12 = textureLoad( nodeUniform0, ( nodeConst3 + vec2<i32>( 2, 1 ) ), u32( 0u ) );
	let nodeConst48 = ( ( ( nodeVar12.x * 0.5 ) + nodeVar12.y ) + ( nodeVar12.z * 0.5 ) );
	let nodeConst49 = ( nodeConst48 - nodeConst29 );
	let nodeConst50 = ( nodeConst29 - nodeConst16 );
	let nodeConst51 = ( nodeConst48 - nodeConst16 );
	let nodeConst52 = max( abs( nodeConst49 ), abs( nodeConst50 ) );
	let nodeConst53 = clamp( ( abs( nodeConst51 ) / max( nodeConst52, 0.0000152587890625 ) ), 0.0, 1.0 );
	nodeVar0.x = ( nodeVar0.x + ( nodeConst51 * nodeConst7 ) );
	nodeVar1 = ( nodeVar1 + ( ( nodeConst53 * nodeConst53 ) * nodeConst7 ) );
	nodeVar13 = textureLoad( nodeUniform0, ( nodeConst3 + vec2<i32>( 1, 2 ) ), u32( 0u ) );
	let nodeConst54 = ( ( ( nodeVar13.x * 0.5 ) + nodeVar13.y ) + ( nodeVar13.z * 0.5 ) );
	let nodeConst55 = ( nodeConst54 - nodeConst29 );
	let nodeConst56 = ( nodeConst29 - nodeConst8 );
	let nodeConst57 = ( nodeConst54 - nodeConst8 );
	let nodeConst58 = max( abs( nodeConst55 ), abs( nodeConst56 ) );
	let nodeConst59 = clamp( ( abs( nodeConst57 ) / max( nodeConst58, 0.0000152587890625 ) ), 0.0, 1.0 );
	nodeVar0.y = ( nodeVar0.y + ( nodeConst57 * nodeConst7 ) );
	nodeVar1 = ( nodeVar1 + ( ( nodeConst59 * nodeConst59 ) * nodeConst7 ) );
	let nodeConst60 = ( ( nodeVar0.x * nodeVar0.x ) + ( nodeVar0.y * nodeVar0.y ) );
	let nodeConst61 = ( nodeConst60 < 0.000030517578125 );
	let nodeConst62 = ( 1.0 / sqrt( max( nodeConst60, 0.000030517578125 ) ) );

	if ( nodeConst61 ) {

		nodeVar14 = 1.0;

	} else {

		nodeVar14 = nodeVar0.x;

	}

	nodeVar0.x = nodeVar14;

	if ( nodeConst61 ) {

		nodeVar15 = 1.0;

	} else {

		nodeVar15 = nodeConst62;

	}

	nodeVar0 = ( nodeVar0 * vec2<f32>( nodeVar15 ) );
	nodeVar1 = ( nodeVar1 * 0.5 );
	nodeVar1 = ( nodeVar1 * nodeVar1 );
	let nodeConst63 = ( ( ( nodeVar0.x * nodeVar0.x ) + ( nodeVar0.y * nodeVar0.y ) ) / max( abs( nodeVar0.x ), abs( nodeVar0.y ) ) );
	let nodeConst64 = vec2<f32>( ( 1.0 + ( ( nodeConst63 - 1.0 ) * nodeVar1 ) ), ( 1.0 - ( nodeVar1 * 0.5 ) ) );
	let nodeConst65 = ( 0.5 + ( -0.29000000000000004 * nodeVar1 ) );
	let nodeConst66 = ( 1.0 / nodeConst65 );
	nodeVar16 = vec4<f32>( 0.0, 0.0, 0.0, 0.0 );
	nodeVar17 = 0.0;
	let nodeConst67 = ( vec2<f32>( 0.0, -1.0 ) - nodeConst2 );
	let nodeConst68 = ( ( nodeConst67.x * nodeVar0.x ) + ( nodeConst67.y * nodeVar0.y ) );
	let nodeConst69 = ( ( - ( nodeConst67.x * nodeVar0.y ) ) + ( nodeConst67.y * nodeVar0.x ) );
	let nodeConst70 = ( nodeConst68 * nodeConst64.x );
	let nodeConst71 = ( nodeConst69 * nodeConst64.y );
	let nodeConst72 = min( ( ( nodeConst70 * nodeConst70 ) + ( nodeConst71 * nodeConst71 ) ), nodeConst66 );
	let nodeConst73 = ( ( nodeConst72 * 0.4 ) - 1.0 );
	let nodeConst74 = ( ( nodeConst72 * nodeConst65 ) - 1.0 );
	let nodeConst75 = ( ( ( ( nodeConst73 * nodeConst73 ) * 1.5625 ) - 0.5625 ) * ( nodeConst74 * nodeConst74 ) );
	nodeVar16 = ( nodeVar16 + ( nodeVar6 * vec4<f32>( nodeConst75 ) ) );
	nodeVar17 = ( nodeVar17 + nodeConst75 );
	let nodeConst76 = ( vec2<f32>( 1.0, -1.0 ) - nodeConst2 );
	let nodeConst77 = ( ( nodeConst76.x * nodeVar0.x ) + ( nodeConst76.y * nodeVar0.y ) );
	let nodeConst78 = ( ( - ( nodeConst76.x * nodeVar0.y ) ) + ( nodeConst76.y * nodeVar0.x ) );
	let nodeConst79 = ( nodeConst77 * nodeConst64.x );
	let nodeConst80 = ( nodeConst78 * nodeConst64.y );
	let nodeConst81 = min( ( ( nodeConst79 * nodeConst79 ) + ( nodeConst80 * nodeConst80 ) ), nodeConst66 );
	let nodeConst82 = ( ( nodeConst81 * 0.4 ) - 1.0 );
	let nodeConst83 = ( ( nodeConst81 * nodeConst65 ) - 1.0 );
	let nodeConst84 = ( ( ( ( nodeConst82 * nodeConst82 ) * 1.5625 ) - 0.5625 ) * ( nodeConst83 * nodeConst83 ) );
	nodeVar16 = ( nodeVar16 + ( nodeVar9 * vec4<f32>( nodeConst84 ) ) );
	nodeVar17 = ( nodeVar17 + nodeConst84 );
	let nodeConst85 = ( vec2<f32>( -1.0, 0.0 ) - nodeConst2 );
	let nodeConst86 = ( ( nodeConst85.x * nodeVar0.x ) + ( nodeConst85.y * nodeVar0.y ) );
	let nodeConst87 = ( ( - ( nodeConst85.x * nodeVar0.y ) ) + ( nodeConst85.y * nodeVar0.x ) );
	let nodeConst88 = ( nodeConst86 * nodeConst64.x );
	let nodeConst89 = ( nodeConst87 * nodeConst64.y );
	let nodeConst90 = min( ( ( nodeConst88 * nodeConst88 ) + ( nodeConst89 * nodeConst89 ) ), nodeConst66 );
	let nodeConst91 = ( ( nodeConst90 * 0.4 ) - 1.0 );
	let nodeConst92 = ( ( nodeConst90 * nodeConst65 ) - 1.0 );
	let nodeConst93 = ( ( ( ( nodeConst91 * nodeConst91 ) * 1.5625 ) - 0.5625 ) * ( nodeConst92 * nodeConst92 ) );
	nodeVar16 = ( nodeVar16 + ( nodeVar4 * vec4<f32>( nodeConst93 ) ) );
	nodeVar17 = ( nodeVar17 + nodeConst93 );
	let nodeConst94 = ( vec2<f32>( 0.0, 0.0 ) - nodeConst2 );
	let nodeConst95 = ( ( nodeConst94.x * nodeVar0.x ) + ( nodeConst94.y * nodeVar0.y ) );
	let nodeConst96 = ( ( - ( nodeConst94.x * nodeVar0.y ) ) + ( nodeConst94.y * nodeVar0.x ) );
	let nodeConst97 = ( nodeConst95 * nodeConst64.x );
	let nodeConst98 = ( nodeConst96 * nodeConst64.y );
	let nodeConst99 = min( ( ( nodeConst97 * nodeConst97 ) + ( nodeConst98 * nodeConst98 ) ), nodeConst66 );
	let nodeConst100 = ( ( nodeConst99 * 0.4 ) - 1.0 );
	let nodeConst101 = ( ( nodeConst99 * nodeConst65 ) - 1.0 );
	let nodeConst102 = ( ( ( ( nodeConst100 * nodeConst100 ) * 1.5625 ) - 0.5625 ) * ( nodeConst101 * nodeConst101 ) );
	nodeVar16 = ( nodeVar16 + ( nodeVar3 * vec4<f32>( nodeConst102 ) ) );
	nodeVar17 = ( nodeVar17 + nodeConst102 );
	let nodeConst103 = ( vec2<f32>( 1.0, 0.0 ) - nodeConst2 );
	let nodeConst104 = ( ( nodeConst103.x * nodeVar0.x ) + ( nodeConst103.y * nodeVar0.y ) );
	let nodeConst105 = ( ( - ( nodeConst103.x * nodeVar0.y ) ) + ( nodeConst103.y * nodeVar0.x ) );
	let nodeConst106 = ( nodeConst104 * nodeConst64.x );
	let nodeConst107 = ( nodeConst105 * nodeConst64.y );
	let nodeConst108 = min( ( ( nodeConst106 * nodeConst106 ) + ( nodeConst107 * nodeConst107 ) ), nodeConst66 );
	let nodeConst109 = ( ( nodeConst108 * 0.4 ) - 1.0 );
	let nodeConst110 = ( ( nodeConst108 * nodeConst65 ) - 1.0 );
	let nodeConst111 = ( ( ( ( nodeConst109 * nodeConst109 ) * 1.5625 ) - 0.5625 ) * ( nodeConst110 * nodeConst110 ) );
	nodeVar16 = ( nodeVar16 + ( nodeVar2 * vec4<f32>( nodeConst111 ) ) );
	nodeVar17 = ( nodeVar17 + nodeConst111 );
	let nodeConst112 = ( vec2<f32>( 2.0, 0.0 ) - nodeConst2 );
	let nodeConst113 = ( ( nodeConst112.x * nodeVar0.x ) + ( nodeConst112.y * nodeVar0.y ) );
	let nodeConst114 = ( ( - ( nodeConst112.x * nodeVar0.y ) ) + ( nodeConst112.y * nodeVar0.x ) );
	let nodeConst115 = ( nodeConst113 * nodeConst64.x );
	let nodeConst116 = ( nodeConst114 * nodeConst64.y );
	let nodeConst117 = min( ( ( nodeConst115 * nodeConst115 ) + ( nodeConst116 * nodeConst116 ) ), nodeConst66 );
	let nodeConst118 = ( ( nodeConst117 * 0.4 ) - 1.0 );
	let nodeConst119 = ( ( nodeConst117 * nodeConst65 ) - 1.0 );
	let nodeConst120 = ( ( ( ( nodeConst118 * nodeConst118 ) * 1.5625 ) - 0.5625 ) * ( nodeConst119 * nodeConst119 ) );
	nodeVar16 = ( nodeVar16 + ( nodeVar7 * vec4<f32>( nodeConst120 ) ) );
	nodeVar17 = ( nodeVar17 + nodeConst120 );
	let nodeConst121 = ( vec2<f32>( -1.0, 1.0 ) - nodeConst2 );
	let nodeConst122 = ( ( nodeConst121.x * nodeVar0.x ) + ( nodeConst121.y * nodeVar0.y ) );
	let nodeConst123 = ( ( - ( nodeConst121.x * nodeVar0.y ) ) + ( nodeConst121.y * nodeVar0.x ) );
	let nodeConst124 = ( nodeConst122 * nodeConst64.x );
	let nodeConst125 = ( nodeConst123 * nodeConst64.y );
	let nodeConst126 = min( ( ( nodeConst124 * nodeConst124 ) + ( nodeConst125 * nodeConst125 ) ), nodeConst66 );
	let nodeConst127 = ( ( nodeConst126 * 0.4 ) - 1.0 );
	let nodeConst128 = ( ( nodeConst126 * nodeConst65 ) - 1.0 );
	let nodeConst129 = ( ( ( ( nodeConst127 * nodeConst127 ) * 1.5625 ) - 0.5625 ) * ( nodeConst128 * nodeConst128 ) );
	nodeVar16 = ( nodeVar16 + ( nodeVar10 * vec4<f32>( nodeConst129 ) ) );
	nodeVar17 = ( nodeVar17 + nodeConst129 );
	let nodeConst130 = ( vec2<f32>( 0.0, 1.0 ) - nodeConst2 );
	let nodeConst131 = ( ( nodeConst130.x * nodeVar0.x ) + ( nodeConst130.y * nodeVar0.y ) );
	let nodeConst132 = ( ( - ( nodeConst130.x * nodeVar0.y ) ) + ( nodeConst130.y * nodeVar0.x ) );
	let nodeConst133 = ( nodeConst131 * nodeConst64.x );
	let nodeConst134 = ( nodeConst132 * nodeConst64.y );
	let nodeConst135 = min( ( ( nodeConst133 * nodeConst133 ) + ( nodeConst134 * nodeConst134 ) ), nodeConst66 );
	let nodeConst136 = ( ( nodeConst135 * 0.4 ) - 1.0 );
	let nodeConst137 = ( ( nodeConst135 * nodeConst65 ) - 1.0 );
	let nodeConst138 = ( ( ( ( nodeConst136 * nodeConst136 ) * 1.5625 ) - 0.5625 ) * ( nodeConst137 * nodeConst137 ) );
	nodeVar16 = ( nodeVar16 + ( nodeVar5 * vec4<f32>( nodeConst138 ) ) );
	nodeVar17 = ( nodeVar17 + nodeConst138 );
	let nodeConst139 = ( vec2<f32>( 1.0, 1.0 ) - nodeConst2 );
	let nodeConst140 = ( ( nodeConst139.x * nodeVar0.x ) + ( nodeConst139.y * nodeVar0.y ) );
	let nodeConst141 = ( ( - ( nodeConst139.x * nodeVar0.y ) ) + ( nodeConst139.y * nodeVar0.x ) );
	let nodeConst142 = ( nodeConst140 * nodeConst64.x );
	let nodeConst143 = ( nodeConst141 * nodeConst64.y );
	let nodeConst144 = min( ( ( nodeConst142 * nodeConst142 ) + ( nodeConst143 * nodeConst143 ) ), nodeConst66 );
	let nodeConst145 = ( ( nodeConst144 * 0.4 ) - 1.0 );
	let nodeConst146 = ( ( nodeConst144 * nodeConst65 ) - 1.0 );
	let nodeConst147 = ( ( ( ( nodeConst145 * nodeConst145 ) * 1.5625 ) - 0.5625 ) * ( nodeConst146 * nodeConst146 ) );
	nodeVar16 = ( nodeVar16 + ( nodeVar8 * vec4<f32>( nodeConst147 ) ) );
	nodeVar17 = ( nodeVar17 + nodeConst147 );
	let nodeConst148 = ( vec2<f32>( 2.0, 1.0 ) - nodeConst2 );
	let nodeConst149 = ( ( nodeConst148.x * nodeVar0.x ) + ( nodeConst148.y * nodeVar0.y ) );
	let nodeConst150 = ( ( - ( nodeConst148.x * nodeVar0.y ) ) + ( nodeConst148.y * nodeVar0.x ) );
	let nodeConst151 = ( nodeConst149 * nodeConst64.x );
	let nodeConst152 = ( nodeConst150 * nodeConst64.y );
	let nodeConst153 = min( ( ( nodeConst151 * nodeConst151 ) + ( nodeConst152 * nodeConst152 ) ), nodeConst66 );
	let nodeConst154 = ( ( nodeConst153 * 0.4 ) - 1.0 );
	let nodeConst155 = ( ( nodeConst153 * nodeConst65 ) - 1.0 );
	let nodeConst156 = ( ( ( ( nodeConst154 * nodeConst154 ) * 1.5625 ) - 0.5625 ) * ( nodeConst155 * nodeConst155 ) );
	nodeVar16 = ( nodeVar16 + ( nodeVar12 * vec4<f32>( nodeConst156 ) ) );
	nodeVar17 = ( nodeVar17 + nodeConst156 );
	let nodeConst157 = ( vec2<f32>( 0.0, 2.0 ) - nodeConst2 );
	let nodeConst158 = ( ( nodeConst157.x * nodeVar0.x ) + ( nodeConst157.y * nodeVar0.y ) );
	let nodeConst159 = ( ( - ( nodeConst157.x * nodeVar0.y ) ) + ( nodeConst157.y * nodeVar0.x ) );
	let nodeConst160 = ( nodeConst158 * nodeConst64.x );
	let nodeConst161 = ( nodeConst159 * nodeConst64.y );
	let nodeConst162 = min( ( ( nodeConst160 * nodeConst160 ) + ( nodeConst161 * nodeConst161 ) ), nodeConst66 );
	let nodeConst163 = ( ( nodeConst162 * 0.4 ) - 1.0 );
	let nodeConst164 = ( ( nodeConst162 * nodeConst65 ) - 1.0 );
	let nodeConst165 = ( ( ( ( nodeConst163 * nodeConst163 ) * 1.5625 ) - 0.5625 ) * ( nodeConst164 * nodeConst164 ) );
	nodeVar16 = ( nodeVar16 + ( nodeVar11 * vec4<f32>( nodeConst165 ) ) );
	nodeVar17 = ( nodeVar17 + nodeConst165 );
	let nodeConst166 = ( vec2<f32>( 1.0, 2.0 ) - nodeConst2 );
	let nodeConst167 = ( ( nodeConst166.x * nodeVar0.x ) + ( nodeConst166.y * nodeVar0.y ) );
	let nodeConst168 = ( ( - ( nodeConst166.x * nodeVar0.y ) ) + ( nodeConst166.y * nodeVar0.x ) );
	let nodeConst169 = ( nodeConst167 * nodeConst64.x );
	let nodeConst170 = ( nodeConst168 * nodeConst64.y );
	let nodeConst171 = min( ( ( nodeConst169 * nodeConst169 ) + ( nodeConst170 * nodeConst170 ) ), nodeConst66 );
	let nodeConst172 = ( ( nodeConst171 * 0.4 ) - 1.0 );
	let nodeConst173 = ( ( nodeConst171 * nodeConst65 ) - 1.0 );
	let nodeConst174 = ( ( ( ( nodeConst172 * nodeConst172 ) * 1.5625 ) - 0.5625 ) * ( nodeConst173 * nodeConst173 ) );
	nodeVar16 = ( nodeVar16 + ( nodeVar13 * vec4<f32>( nodeConst174 ) ) );
	nodeVar17 = ( nodeVar17 + nodeConst174 );
	nodeVar16 = ( nodeVar16 / vec4<f32>( nodeVar17 ) );
	let nodeConst175 = min( min( nodeVar3, nodeVar2 ), min( nodeVar5, nodeVar8 ) );
	let nodeConst176 = max( max( nodeVar3, nodeVar2 ), max( nodeVar5, nodeVar8 ) );

	// result

	output.color = clamp( nodeVar16, nodeConst175, nodeConst176 );

	return output;

}
