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

struct renderStruct {
	nodeUniform7 : f32,
	cameraProjectionMatrix : mat4x4<f32>,
	cameraViewMatrix : mat4x4<f32>,
	cameraPosition : vec3<f32>
};
@binding( 0 ) @group( 0 )
var<uniform> render : renderStruct;

struct objectStruct {
	nodeUniform0 : mat4x4<f32>,
	nodeUniform2 : f32,
	nodeUniform3 : u32,
	nodeUniform4 : f32,
	nodeUniform5 : f32,
	nodeUniform6 : f32,
	nodeUniform8 : f32,
	nodeUniform9 : f32,
	nodeUniform10 : f32,
	nodeUniform11 : vec3<f32>,
	nodeUniform12 : f32,
	nodeUniform13 : f32,
	nodeUniform14 : f32
};
@binding( 0 ) @group( 1 )
var<uniform> object : objectStruct;

// vars
var<private> DiffuseColor : vec4<f32>;
var<private> nodeVar1 : bool;
var<private> nodeVar2 : vec3<f32>;
var<private> nodeVar3 : vec2<f32>;
var<private> nodeVar4 : vec2<f32>;
var<private> nodeVar5 : f32;
var<private> nodeVar6 : f32;
var<private> nodeVar7 : vec3<f32>;
var<private> nodeVar8 : vec3<f32>;
var<private> nodeVar9 : vec3<f32>;
var<private> nodeVar10 : vec3<f32>;
var<private> nodeVar11 : f32;
var<private> nodeVar12 : vec3<f32>;
var<private> nodeVar13 : vec3<f32>;
var<private> nodeVar14 : vec3<f32>;
var<private> nodeVar15 : vec3<f32>;
var<private> nodeVar16 : f32;
var<private> nodeVar17 : f32;
var<private> nodeVar18 : vec3<f32>;
var<private> nodeVar19 : f32;
var<private> nodeVar20 : f32;
var<private> nodeVar21 : vec3<f32>;
var<private> nodeVar22 : f32;
var<private> Output : vec4<f32>;

// codes


@fragment
fn main( @location( 0 ) v_positionWorld : vec3<f32>,
	@location( 1 ) nodeVarying5 : f32,
	@location( 2 ) nodeVarying6 : vec3<f32>,
	@location( 3 ) nodeVarying7 : vec3<f32>,
	@location( 4 ) nodeVarying8 : vec3<f32> ) -> OutputStruct {

	// flow
	// code

	let nodeConst0 = normalize( ( v_positionWorld - render.cameraPosition ) );
	let nodeConst1 = dot( nodeConst0, nodeVarying7 );
	let nodeConst2 = ( nodeVarying6 * vec3<f32>( ( 0.05968310365946075 * ( 1.0 + pow( ( ( nodeConst1 * 0.5 ) + 0.5 ), 2.0 ) ) ) ) );
	let nodeConst3 = pow( object.nodeUniform2, 2.0 );
	let nodeConst4 = ( nodeVarying8 * vec3<f32>( ( ( 0.07957747154594767 * ( 1.0 - nodeConst3 ) ) * ( 1.0 / pow( ( ( 1.0 - ( ( 2.0 * object.nodeUniform2 ) * nodeConst1 ) ) + nodeConst3 ), 1.5 ) ) ) ) );
	let nodeConst5 = acos( max( 0.0, nodeConst0.y ) );
	let nodeConst6 = ( 1.0 / ( cos( nodeConst5 ) + ( 0.15 * pow( ( 93.885 - ( ( nodeConst5 * 180.0 ) / 3.141592653589793 ) ), -1.253 ) ) ) );
	let nodeConst7 = exp( ( - ( ( nodeVarying6 * vec3<f32>( ( 8400.0 * nodeConst6 ) ) ) + ( nodeVarying8 * vec3<f32>( ( 1250.0 * nodeConst6 ) ) ) ) ) );
	var nodeVar0 : vec3<f32> = pow( ( ( vec3<f32>( nodeVarying5 ) * ( ( nodeConst2 + nodeConst4 ) / ( nodeVarying6 + nodeVarying8 ) ) ) * ( vec3<f32>( 1.0 ) - nodeConst7 ) ), vec3<f32>( 1.5, 1.5, 1.5 ) );
	nodeVar0 = ( nodeVar0 * mix( vec3<f32>( 1.0, 1.0, 1.0 ), pow( ( ( vec3<f32>( nodeVarying5 ) * ( ( nodeConst2 + nodeConst4 ) / ( nodeVarying6 + nodeVarying8 ) ) ) * nodeConst7 ), vec3<f32>( 0.5, 0.5, 0.5 ) ), clamp( pow( ( 1.0 - nodeVarying7.y ), 5.0 ), 0.0, 1.0 ) ) );
	let nodeConst8 = ( vec3<f32>( 0.1, 0.1, 0.1 ) * nodeConst7 );
	nodeVar1 = bool( object.nodeUniform3 );
	let nodeConst9 = ( ( min( ( vec3<f32>( nodeVarying5 ) * nodeConst7 ), vec3<f32>( 80.0 ) ) * vec3<f32>( 760.0 ) ) * vec3<f32>( ( clamp( ( ( nodeConst1 - 0.9999566769464484 ) * 50000.0 ), 0.0, 1.0 ) * f32( nodeVar1 ) ) ) );
	nodeVar2 = ( ( ( ( nodeVar0 + nodeConst8 ) * vec3<f32>( 0.04 ) ) + nodeConst9 ) + vec3<f32>( 0.0, 0.0003, 0.00075 ) );

	if ( ( ( nodeConst0.y > 0.0 ) && ( object.nodeUniform4 > 0.0 ) ) ) {

		nodeVar3 = ( nodeConst0.xz / vec2<f32>( ( nodeConst0.y * mix( 1.0, 0.1, object.nodeUniform5 ) ) ) );
		nodeVar3 = ( nodeVar3 * vec2<f32>( object.nodeUniform6 ) );
		nodeVar3 = ( nodeVar3 + vec2<f32>( ( render.nodeUniform7 * object.nodeUniform8 ) ) );
		nodeVar4 = ( nodeVar3 * vec2<f32>( 1000.0 ) );
		nodeVar5 = 0.0;
		nodeVar6 = 1.0;

		for ( var i : i32 = 0; i < 4; i ++ ) {

			let nodeConst10 = floor( nodeVar4 );
			nodeVar7 = fract( ( vec3<f32>( nodeConst10, 0.0 ).xyx * vec3<f32>( 0.1031, 0.103, 0.0973 ) ) );
			nodeVar7 = ( nodeVar7 + vec3<f32>( dot( nodeVar7, ( nodeVar7.yzx + vec3<f32>( 33.33 ) ) ) ) );
			let nodeConst11 = fract( nodeVar4 );
			nodeVar8 = fract( ( vec3<f32>( ( nodeConst10 + vec2<f32>( 1.0, 0.0 ) ), 0.0 ).xyx * vec3<f32>( 0.1031, 0.103, 0.0973 ) ) );
			nodeVar8 = ( nodeVar8 + vec3<f32>( dot( nodeVar8, ( nodeVar8.yzx + vec3<f32>( 33.33 ) ) ) ) );
			let nodeConst12 = ( ( ( nodeConst11 * nodeConst11 ) * nodeConst11 ) * ( ( nodeConst11 * ( ( nodeConst11 * vec2<f32>( 6.0 ) ) - vec2<f32>( 15.0 ) ) ) + vec2<f32>( 10.0 ) ) );
			nodeVar9 = fract( ( vec3<f32>( ( nodeConst10 + vec2<f32>( 0.0, 1.0 ) ), 0.0 ).xyx * vec3<f32>( 0.1031, 0.103, 0.0973 ) ) );
			nodeVar9 = ( nodeVar9 + vec3<f32>( dot( nodeVar9, ( nodeVar9.yzx + vec3<f32>( 33.33 ) ) ) ) );
			nodeVar10 = fract( ( vec3<f32>( ( nodeConst10 + vec2<f32>( 1.0, 1.0 ) ), 0.0 ).xyx * vec3<f32>( 0.1031, 0.103, 0.0973 ) ) );
			nodeVar10 = ( nodeVar10 + vec3<f32>( dot( nodeVar10, ( nodeVar10.yzx + vec3<f32>( 33.33 ) ) ) ) );
			nodeVar5 = ( nodeVar5 + ( nodeVar6 * ( mix( mix( dot( ( ( fract( ( ( nodeVar7.xx + nodeVar7.yz ) * nodeVar7.zy ) ) * vec2<f32>( 2.0 ) ) - vec2<f32>( 1.0 ) ), nodeConst11 ), dot( ( ( fract( ( ( nodeVar8.xx + nodeVar8.yz ) * nodeVar8.zy ) ) * vec2<f32>( 2.0 ) ) - vec2<f32>( 1.0 ) ), ( nodeConst11 - vec2<f32>( 1.0, 0.0 ) ) ), nodeConst12.x ), mix( dot( ( ( fract( ( ( nodeVar9.xx + nodeVar9.yz ) * nodeVar9.zy ) ) * vec2<f32>( 2.0 ) ) - vec2<f32>( 1.0 ) ), ( nodeConst11 - vec2<f32>( 0.0, 1.0 ) ) ), dot( ( ( fract( ( ( nodeVar10.xx + nodeVar10.yz ) * nodeVar10.zy ) ) * vec2<f32>( 2.0 ) ) - vec2<f32>( 1.0 ) ), ( nodeConst11 - vec2<f32>( 1.0, 1.0 ) ) ), nodeConst12.x ), nodeConst12.y ) * 1.6 ) ) );
			nodeVar6 = ( nodeVar6 * 0.5 );
			nodeVar4 = ( nodeVar4 * vec2<f32>( 2.0 ) );
			nodeVar4 = ( nodeVar4 + vec2<f32>( ( ( render.nodeUniform7 * object.nodeUniform8 ) * 300.0 ) ) );

		}

		nodeVar11 = clamp( ( ( nodeVar5 * 0.7 ) + 0.5 ), 0.0, 1.0 );
		let nodeConst13 = ( nodeVar3 * vec2<f32>( 300.0 ) );
		let nodeConst14 = floor( nodeConst13 );
		nodeVar12 = fract( ( vec3<f32>( nodeConst14, 0.0 ).xyx * vec3<f32>( 0.1031, 0.103, 0.0973 ) ) );
		nodeVar12 = ( nodeVar12 + vec3<f32>( dot( nodeVar12, ( nodeVar12.yzx + vec3<f32>( 33.33 ) ) ) ) );
		let nodeConst15 = fract( nodeConst13 );
		nodeVar13 = fract( ( vec3<f32>( ( nodeConst14 + vec2<f32>( 1.0, 0.0 ) ), 0.0 ).xyx * vec3<f32>( 0.1031, 0.103, 0.0973 ) ) );
		nodeVar13 = ( nodeVar13 + vec3<f32>( dot( nodeVar13, ( nodeVar13.yzx + vec3<f32>( 33.33 ) ) ) ) );
		let nodeConst16 = ( ( ( nodeConst15 * nodeConst15 ) * nodeConst15 ) * ( ( nodeConst15 * ( ( nodeConst15 * vec2<f32>( 6.0 ) ) - vec2<f32>( 15.0 ) ) ) + vec2<f32>( 10.0 ) ) );
		nodeVar14 = fract( ( vec3<f32>( ( nodeConst14 + vec2<f32>( 0.0, 1.0 ) ), 0.0 ).xyx * vec3<f32>( 0.1031, 0.103, 0.0973 ) ) );
		nodeVar14 = ( nodeVar14 + vec3<f32>( dot( nodeVar14, ( nodeVar14.yzx + vec3<f32>( 33.33 ) ) ) ) );
		nodeVar15 = fract( ( vec3<f32>( ( nodeConst14 + vec2<f32>( 1.0, 1.0 ) ), 0.0 ).xyx * vec3<f32>( 0.1031, 0.103, 0.0973 ) ) );
		nodeVar15 = ( nodeVar15 + vec3<f32>( dot( nodeVar15, ( nodeVar15.yzx + vec3<f32>( 33.33 ) ) ) ) );
		nodeVar16 = ( 1.0 - clamp( ( object.nodeUniform4 + ( ( ( ( ( mix( mix( dot( ( ( fract( ( ( nodeVar12.xx + nodeVar12.yz ) * nodeVar12.zy ) ) * vec2<f32>( 2.0 ) ) - vec2<f32>( 1.0 ) ), nodeConst15 ), dot( ( ( fract( ( ( nodeVar13.xx + nodeVar13.yz ) * nodeVar13.zy ) ) * vec2<f32>( 2.0 ) ) - vec2<f32>( 1.0 ) ), ( nodeConst15 - vec2<f32>( 1.0, 0.0 ) ) ), nodeConst16.x ), mix( dot( ( ( fract( ( ( nodeVar14.xx + nodeVar14.yz ) * nodeVar14.zy ) ) * vec2<f32>( 2.0 ) ) - vec2<f32>( 1.0 ) ), ( nodeConst15 - vec2<f32>( 0.0, 1.0 ) ) ), dot( ( ( fract( ( ( nodeVar15.xx + nodeVar15.yz ) * nodeVar15.zy ) ) * vec2<f32>( 2.0 ) ) - vec2<f32>( 1.0 ) ), ( nodeConst15 - vec2<f32>( 1.0, 1.0 ) ) ), nodeConst16.x ), nodeConst16.y ) * 1.6 ) * 0.37 ) + 0.5 ) - 0.5 ) * 0.6 ) ), 0.0, 1.0 ) );
		nodeVar17 = smoothstep( nodeVar16, ( nodeVar16 + 0.3 ), nodeVar11 );
		let nodeConst17 = smoothstep( 0.0, ( 0.03 + ( 0.06 * object.nodeUniform5 ) ), nodeConst0.y );
		nodeVar17 = ( nodeVar17 * nodeConst17 );
		nodeVar18 = ( ( ( vec3<f32>( nodeVarying5 ) * nodeConst7 ) * vec3<f32>( 0.22 ) ) * vec3<f32>( 0.04 ) );
		nodeVar19 = max( 0.0, ( nodeVar11 - nodeVar16 ) );
		nodeVar20 = exp( ( nodeVar19 * -4.0 ) );
		nodeVar21 = ( ( ( nodeVar0 * vec3<f32>( 0.04 ) ) + vec3<f32>( 0.0, 0.0003, 0.00075 ) ) + ( nodeVar18 * vec3<f32>( mix( 0.45, 1.0, clamp( ( ( nodeVar20 * ( 1.0 - ( nodeVar20 * nodeVar20 ) ) ) * 2.6 ), 0.0, 1.0 ) ) ) ) );
		nodeVar21 = ( nodeVar21 + ( ( ( nodeVar18 * vec3<f32>( clamp( ( 0.51 / pow( ( 1.49 - ( nodeConst1 * 1.4 ) ), 1.5 ) ), 0.0, 3.0 ) ) ) * vec3<f32>( ( ( nodeVar17 * ( 1.0 - nodeVar17 ) ) * 4.0 ) ) ) * vec3<f32>( 0.6 ) ) );
		nodeVar21 = ( nodeVar21 * vec3<f32>( max( smoothstep( -0.08, 0.3, nodeVarying7.y ), 0.03 ) ) );
		nodeVar22 = ( ( 1.0 - exp( ( ( nodeVar19 * object.nodeUniform9 ) * -12.0 ) ) ) * nodeConst17 );
		nodeVar2 = ( nodeVar2 - ( ( ( nodeConst8 * vec3<f32>( 0.04 ) ) + nodeConst9 ) * vec3<f32>( nodeVar22 ) ) );
		nodeVar2 = mix( nodeVar2, mix( nodeVar2, nodeVar21, nodeConst7 ), nodeVar22 );
		

	}

	DiffuseColor = vec4<f32>( nodeVar2, 1.0 );
	DiffuseColor.w = ( DiffuseColor.w * object.nodeUniform10 );
	DiffuseColor.w = 1.0;
	let nodeConst18 = max( vec4<f32>( DiffuseColor.xyz, DiffuseColor.w ), vec4<f32>( 0.0 ) );
	Output = nodeConst18;

	// result

	output.color = nodeConst18;

	return output;

}
