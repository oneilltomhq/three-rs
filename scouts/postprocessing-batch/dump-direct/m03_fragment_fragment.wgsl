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
	nodeUniform0 : vec3<f32>,
	nodeUniform1 : f32,
	nodeUniform2 : f32,
	nodeUniform3 : vec3<f32>,
	nodeUniform4 : vec3<f32>,
	nodeUniform5 : f32,
	nodeUniform8 : mat4x4<f32>,
	nodeUniform13 : f32
};
@binding( 0 ) @group( 1 )
var<uniform> object : objectStruct;

struct renderStruct {
	cameraProjectionMatrix : mat4x4<f32>,
	cameraViewMatrix : mat4x4<f32>,
	nodeUniform14 : f32,
	nodeUniform6 : vec3<f32>,
	nodeUniform12 : vec3<f32>,
	nodeUniform10 : vec3<f32>,
	nodeUniform11 : vec3<f32>
};
@binding( 0 ) @group( 0 )
var<uniform> render : renderStruct;

// vars
var<private> DiffuseColor : vec4<f32>;
var<private> Shininess : f32;
var<private> SpecularColor : vec3<f32>;
var<private> EmissiveColor : vec3<f32>;
var<private> Output : vec4<f32>;
var<private> irradiance : vec3<f32>;
var<private> nodeVar0 : vec3<f32>;
var<private> directDiffuse : vec3<f32>;
var<private> normalFlat : vec3<f32>;
var<private> normalViewGeometry : vec3<f32>;
var<private> NORMAL_normalView : vec3<f32>;
var<private> normalView : vec3<f32>;
var<private> nodeVar1 : vec3<f32>;
var<private> nodeVar2 : vec4<f32>;
var<private> nodeVar3 : vec4<f32>;
var<private> nodeVar4 : vec3<f32>;
var<private> nodeVar5 : vec3<f32>;
var<private> nodeVar6 : f32;
var<private> nodeVar7 : vec3<f32>;
var<private> nodeVar8 : vec3<f32>;
var<private> nodeVar9 : vec3<f32>;
var<private> nodeVar10 : vec3<f32>;
var<private> directSpecular : vec3<f32>;
var<private> positionViewDirection : vec3<f32>;
var<private> nodeVar11 : vec3<f32>;
var<private> nodeVar12 : f32;
var<private> nodeVar13 : f32;
var<private> nodeVar14 : vec3<f32>;
var<private> nodeVar15 : vec3<f32>;
var<private> nodeVar16 : vec3<f32>;
var<private> nodeVar17 : vec3<f32>;
var<private> indirectDiffuse : vec3<f32>;
var<private> nodeVar18 : vec4<f32>;
var<private> nodeVar19 : vec4<f32>;
var<private> nodeVar20 : vec4<f32>;
var<private> ambientOcclusion : f32;
var<private> nodeVar21 : vec3<f32>;
var<private> totalDiffuse : vec3<f32>;
var<private> nodeVar22 : vec3<f32>;
var<private> totalSpecular : vec3<f32>;
var<private> indirectSpecular : vec3<f32>;
var<private> nodeVar23 : vec3<f32>;
var<private> outgoingLight : vec3<f32>;
var<private> nodeVar24 : vec3<f32>;
var<private> nodeVar25 : vec4<f32>;
var<private> nodeVar26 : vec4<f32>;
var<private> nodeVar27 : vec4<f32>;
var<private> nodeVar28 : vec4<f32>;

// codes
fn fn1 ( color : vec4<f32> ) -> vec4<f32> {

	var nodeVar0 : vec4<f32>;


	if ( ( color.w == 0.0 ) ) {

		nodeVar0 = vec4<f32>( 0.0, 0.0, 0.0, 0.0 );

	} else {

		nodeVar0 = vec4<f32>( ( color.xyz / vec3<f32>( color.w ) ), color.w );

	}


	return nodeVar0;

}


fn neutralToneMapping ( color : vec3<f32>, exposure : f32 ) -> vec3<f32> {

	var nodeVar0 : vec3<f32>;
	var nodeVar1 : f32;
	var nodeVar2 : f32;
	var nodeVar3 : f32;
	var nodeVar4 : f32;
	var nodeVar5 : f32;

	nodeVar0 = ( color * vec3<f32>( exposure ) );
	nodeVar2 = min( nodeVar0.x, min( nodeVar0.y, nodeVar0.z ) );

	if ( ( nodeVar2 < 0.08 ) ) {

		nodeVar1 = ( nodeVar2 - ( 6.25 * ( nodeVar2 * nodeVar2 ) ) );

	} else {

		nodeVar1 = 0.04;

	}

	nodeVar0 = ( nodeVar0 - vec3<f32>( nodeVar1 ) );
	nodeVar3 = max( nodeVar0.x, max( nodeVar0.y, nodeVar0.z ) );

	if ( ( nodeVar3 < 0.76 ) ) {

		return nodeVar0;

	}

	nodeVar4 = ( 1.0 - 0.76 );
	nodeVar5 = ( 1.0 - ( ( nodeVar4 * nodeVar4 ) / ( nodeVar3 + ( nodeVar4 - 0.76 ) ) ) );
	nodeVar0 = ( nodeVar0 * vec3<f32>( ( nodeVar5 / nodeVar3 ) ) );

	return mix( nodeVar0, vec3<f32>( nodeVar5 ), ( 1.0 - ( 1.0 / ( ( 0.15 * ( nodeVar3 - nodeVar5 ) ) + 1.0 ) ) ) );

}


fn sRGBTransferOETF ( color : vec3<f32> ) -> vec3<f32> {

	


	return mix( ( ( pow( color, vec3<f32>( 0.41666 ) ) * vec3<f32>( 1.055 ) ) - vec3<f32>( 0.055 ) ), ( color * vec3<f32>( 12.92 ) ), vec3<f32>( ( color <= vec3<f32>( 0.0031308 ) ) ) );

}


fn fn0 ( color : vec4<f32> ) -> vec4<f32> {

	


	return vec4<f32>( ( color.xyz * vec3<f32>( color.w ) ), color.w );

}




@fragment
fn main( @location( 0 ) v_positionView : vec3<f32>,
	@location( 1 ) v_positionViewDirection : vec3<f32> ) -> OutputStruct {

	// flow
	// code

	DiffuseColor = vec4<f32>( object.nodeUniform0, 1.0 );
	DiffuseColor.w = ( DiffuseColor.w * object.nodeUniform1 );
	DiffuseColor.w = 1.0;
	Shininess = max( object.nodeUniform2, 0.0001 );
	SpecularColor = object.nodeUniform3;
	EmissiveColor = ( object.nodeUniform4 * vec3<f32>( object.nodeUniform5 ) );
	irradiance = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar0 = ( irradiance + render.nodeUniform6 );
	irradiance = nodeVar0;
	directDiffuse = vec3<f32>( 0.0, 0.0, 0.0 );
	normalFlat = normalize( cross( dpdx( v_positionView ), - dpdy( v_positionView ) ) );
	normalViewGeometry = normalFlat;
	NORMAL_normalView = normalViewGeometry;
	normalView = NORMAL_normalView;
	nodeVar1 = ( render.nodeUniform10 - render.nodeUniform11 );
	nodeVar2 = vec4<f32>( nodeVar1, 0.0 );
	nodeVar3 = ( render.cameraViewMatrix * nodeVar2 );
	nodeVar4 = normalize( nodeVar3.xyz );
	nodeVar5 = nodeVar4;
	nodeVar6 = dot( normalView, nodeVar5 );
	nodeVar7 = ( vec3<f32>( clamp( nodeVar6, 0.0, 1.0 ) ) * render.nodeUniform12 );
	nodeVar8 = ( DiffuseColor.xyz * vec3<f32>( 0.3183098861837907 ) );
	nodeVar9 = ( nodeVar7 * nodeVar8 );
	nodeVar10 = ( directDiffuse + nodeVar9 );
	directDiffuse = nodeVar10;
	directSpecular = vec3<f32>( 0.0, 0.0, 0.0 );
	positionViewDirection = normalize( v_positionViewDirection );
	nodeVar11 = normalize( ( nodeVar5 + positionViewDirection ) );
	nodeVar12 = clamp( dot( positionViewDirection, nodeVar11 ), 0.0, 1.0 );
	nodeVar13 = exp2( ( ( ( nodeVar12 * -5.55473 ) - 6.98316 ) * nodeVar12 ) );
	nodeVar14 = ( ( ( ( SpecularColor * vec3<f32>( ( 1.0 - nodeVar13 ) ) ) + vec3<f32>( ( 1.0 * nodeVar13 ) ) ) * vec3<f32>( 0.25 ) ) * vec3<f32>( ( ( ( ( Shininess * 0.5 ) + 1.0 ) * 0.3183098861837907 ) * pow( clamp( dot( normalView, nodeVar11 ), 0.0, 1.0 ), Shininess ) ) ) );
	nodeVar15 = ( nodeVar7 * nodeVar14 );
	nodeVar16 = ( nodeVar15 * vec3<f32>( 1.0 ) );
	nodeVar17 = ( directSpecular + nodeVar16 );
	directSpecular = nodeVar17;
	indirectDiffuse = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar18 = ( DiffuseColor * vec4<f32>( 0.3183098861837907 ) );
	nodeVar19 = ( vec4<f32>( irradiance, 1.0 ) * nodeVar18 );
	nodeVar20 = ( vec4<f32>( indirectDiffuse, 1.0 ) + nodeVar19 );
	indirectDiffuse = nodeVar20.xyz;
	ambientOcclusion = 1.0;
	nodeVar21 = ( indirectDiffuse * vec3<f32>( ambientOcclusion ) );
	indirectDiffuse = nodeVar21;
	nodeVar22 = ( directDiffuse + indirectDiffuse );
	totalDiffuse = nodeVar22;
	indirectSpecular = vec3<f32>( 0.0, 0.0, 0.0 );
	nodeVar23 = ( directSpecular + indirectSpecular );
	totalSpecular = nodeVar23;
	nodeVar24 = ( totalDiffuse + totalSpecular );
	outgoingLight = nodeVar24;
	nodeVar25 = max( vec4<f32>( ( outgoingLight + EmissiveColor ), DiffuseColor.w ), vec4<f32>( 0.0 ) );
	Output = nodeVar25;
	Output = nodeVar25;
	nodeVar26 = vec4<f32>( max( mix( vec3<f32>( dot( Output.xyz, vec3<f32>( 0.2126, 0.7152, 0.0722 ) ) ), Output.xyz, object.nodeUniform13 ), vec3<f32>( 0.0 ) ), Output.w );
	nodeVar27 = fn1( vec4<f32>( nodeVar26.xyz, clamp( nodeVar26.w, 0.0, 1.0 ) ) );
	nodeVar28 = vec4<f32>( neutralToneMapping( nodeVar27.xyz, render.nodeUniform14 ), nodeVar27.w );

	// result

	output.color = fn0( vec4<f32>( sRGBTransferOETF( nodeVar28.xyz ), nodeVar28.w ) );

	return output;

}
