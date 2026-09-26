// Runs three.js' own curves, `ShapePath`, `Earcut`, `ShapeGeometry`,
// `ExtrudeGeometry` and `TubeGeometry` under node and prints what they build,
// so that `tests/geometries_shape_oracle.rs` can assert the Rust port builds
// the same thing from the same inputs.
//
//   node tools/geometry_reference.mjs <three.js checkout>
//
// It prints one JSON object keyed by scenario. A geometry scenario holds every
// attribute array in full (`position`, `normal`, `uv`), the index (or null)
// and the groups; the other scenarios hold plain numbers. Every scenario is
// rebuilt from the same literal inputs on the Rust side, so the two must stay
// in step: change one, change the other.
//
// It imports the checkout's `src/Three.js` rather than `build/three.module.js`,
// because the build is only as fresh as the last `npm run build` and the port
// tracks the source. Nothing in the checkout is modified.

import * as path from 'path';
import { pathToFileURL } from 'url';

const threeDir = process.argv[ 2 ];
if ( ! threeDir ) {

	console.error( 'usage: geometry_reference.mjs <three.js checkout>' );
	process.exit( 2 );

}

const THREE = await import( pathToFileURL( path.join( threeDir, 'src/Three.js' ) ).href );
const { Earcut } = await import( pathToFileURL( path.join( threeDir, 'src/extras/Earcut.js' ) ).href );

function geometry( g ) {

	const attributes = {};
	for ( const name of [ 'position', 'normal', 'uv' ] ) {

		const a = g.getAttribute( name );
		attributes[ name ] = a === undefined ? null : Array.from( a.array );

	}

	return {
		...attributes,
		index: g.index === null ? null : Array.from( g.index.array ),
		groups: g.groups.map( ( { start, count, materialIndex } ) => [ start, count, materialIndex ] ),
	};

}

const xy = ( v ) => [ v.x, v.y ];
const xyz = ( v ) => [ v.x, v.y, v.z ];

// webgl_geometry_shapes' heart, in canvas units.
function heartShape() {

	const x = 0, y = 0;
	return new THREE.Shape()
		.moveTo( x + 25, y + 25 )
		.bezierCurveTo( x + 25, y + 25, x + 20, y, x, y )
		.bezierCurveTo( x - 30, y, x - 30, y + 35, x - 30, y + 35 )
		.bezierCurveTo( x - 30, y + 55, x - 10, y + 77, x + 25, y + 95 )
		.bezierCurveTo( x + 60, y + 77, x + 80, y + 55, x + 80, y + 35 )
		.bezierCurveTo( x + 80, y + 35, x + 80, y, x + 50, y )
		.bezierCurveTo( x + 35, y, x + 25, y + 25, x + 25, y + 25 );

}

// webgpu_instance_path's heart, a `Path`.
function instancePathHeart() {

	const x = 0, y = 0;
	return new THREE.Path()
		.moveTo( x - 2.5, y - 2.5 )
		.bezierCurveTo( x - 2.5, y - 2.5, x - 2, y, x, y )
		.bezierCurveTo( x + 3, y, x + 3, y - 3.5, x + 3, y - 3.5 )
		.bezierCurveTo( x + 3, y - 5.5, x + 1, y - 7.7, x - 2.5, y - 9.5 )
		.bezierCurveTo( x - 6, y - 7.7, x - 8, y - 5.5, x - 8, y - 3.5 )
		.bezierCurveTo( x - 8, y - 3.5, x - 8, y, x - 5, y )
		.bezierCurveTo( x - 3.5, y, x - 2.5, y - 2.5, x - 2.5, y - 2.5 );

}

// A square with a round hole and a triangular one: exercises hole bridging
// and every curve type a `Path` builds.
function holedShape() {

	const shape = new THREE.Shape()
		.moveTo( - 4, - 4 )
		.lineTo( 4, - 4 )
		.quadraticCurveTo( 5, 0, 4, 4 )
		.splineThru( [ new THREE.Vector2( 0, 5 ), new THREE.Vector2( - 4, 4 ) ] )
		.lineTo( - 4, - 4 );

	const round = new THREE.Path().absarc( - 1.5, 0, 1.2, 0, Math.PI * 2, true );
	const triangle = new THREE.Path()
		.moveTo( 1, - 2 )
		.lineTo( 3, - 2 )
		.lineTo( 2, 1 )
		.lineTo( 1, - 2 );
	const ellipse = new THREE.Path().absellipse( 1, 2.5, 1, 0.5, 0, Math.PI * 2, false, 0.3 );

	shape.holes.push( round, triangle, ellipse );
	return shape;

}

function knotCurve() {

	return new THREE.CatmullRomCurve3( [
		new THREE.Vector3( - 10, 0, 10 ), new THREE.Vector3( - 5, 5, 5 ),
		new THREE.Vector3( 0, 0, 0 ), new THREE.Vector3( 5, - 5, 5 ),
		new THREE.Vector3( 10, 0, 10 )
	] );

}

const out = {};

// Curves and paths.

const instanceHeart = instancePathHeart();
out.instance_path_points = [];
for ( let i = 0; i < 1000; i += 37 ) out.instance_path_points.push( xy( instanceHeart.getPointAt( i / 1000 ) ) );
out.instance_path_length = instanceHeart.getLength();

const holed = holedShape();
out.holed_extract = ( () => {

	const { shape, holes } = holed.extractPoints( 12 );
	return { shape: shape.map( xy ), holes: holes.map( ( h ) => h.map( xy ) ) };

} )();
out.holed_spaced = holed.getSpacedPoints( 40 ).map( xy );

const knot = knotCurve();
const frames = knot.computeFrenetFrames( 16, false );
out.knot_frames = {
	tangents: frames.tangents.map( xyz ),
	normals: frames.normals.map( xyz ),
	binormals: frames.binormals.map( xyz ),
};

// ShapePath.toShapes: two outers, one with a hole, drawn in mixed windings.
out.shape_path = ( () => {

	const sp = new THREE.ShapePath();
	sp.moveTo( 0, 0 ); sp.lineTo( 10, 0 ); sp.lineTo( 10, 10 ); sp.lineTo( 0, 10 ); sp.lineTo( 0, 0 );
	sp.moveTo( 2, 2 ); sp.lineTo( 2, 8 ); sp.lineTo( 8, 8 ); sp.lineTo( 8, 2 ); sp.lineTo( 2, 2 );
	sp.moveTo( 20, 0 ); sp.quadraticCurveTo( 25, 10, 30, 0 ); sp.lineTo( 20, 0 );
	const result = {};
	for ( const rule of [ 'nonzero', 'evenodd' ] ) {

		sp.userData = { style: { fillRule: rule } };
		result[ rule ] = sp.toShapes().map( ( s ) => {

			const { shape, holes } = s.extractPoints( 4 );
			return { shape: shape.map( xy ), holes: holes.map( ( h ) => h.map( xy ) ) };

		} );

	}

	return result;

} )();

// Earcut directly, both below and above the 80-vertex z-order hashing cut-off.
out.earcut = ( () => {

	const star = [];
	for ( let i = 0; i < 12; i ++ ) {

		const r = i % 2 ? 4 : 10, a = i / 12 * Math.PI * 2;
		star.push( r * Math.cos( a ), r * Math.sin( a ) );

	}

	const gear = [];
	for ( let i = 0; i < 120; i ++ ) {

		const r = ( i % 4 < 2 ) ? 10 : 8.5, a = - i / 120 * Math.PI * 2;
		gear.push( r * Math.cos( a ), r * Math.sin( a ) );

	}

	const gearHoleStart = gear.length / 2;
	for ( let i = 0; i < 24; i ++ ) {

		const a = i / 24 * Math.PI * 2;
		gear.push( 3 * Math.cos( a ), 3 * Math.sin( a ) );

	}

	return {
		star: { data: star, holes: [], triangles: Earcut.triangulate( star, [], 2 ) },
		gear: { data: gear, holes: [ gearHoleStart ], triangles: Earcut.triangulate( gear, [ gearHoleStart ], 2 ) },
	};

} )();

// Geometries.

out.shape_default = geometry( new THREE.ShapeGeometry() );
out.shape_heart_30 = geometry( new THREE.ShapeGeometry( heartShape(), 30 ) );
out.shape_multi = geometry( new THREE.ShapeGeometry( [ heartShape(), holedShape() ], 6 ) );

out.extrude_default = geometry( new THREE.ExtrudeGeometry() );
out.extrude_heart = geometry( new THREE.ExtrudeGeometry( heartShape(), {
	depth: 8, bevelEnabled: true, bevelSegments: 2, steps: 2, bevelSize: 1, bevelThickness: 1
} ) );
out.extrude_holed = geometry( new THREE.ExtrudeGeometry( holedShape(), {
	curveSegments: 8, depth: 2, steps: 3, bevelOffset: 0.05
} ) );
out.extrude_flat = geometry( new THREE.ExtrudeGeometry( [ heartShape(), holedShape() ], {
	curveSegments: 5, bevelEnabled: false
} ) );
out.extrude_path = geometry( new THREE.ExtrudeGeometry( holedShape(), {
	curveSegments: 4, steps: 20, extrudePath: knotCurve()
} ) );
out.extrude_path_closed = ( () => {

	const curve = knotCurve();
	curve.closed = true;
	return geometry( new THREE.ExtrudeGeometry( new THREE.Shape( [
		new THREE.Vector2( 0, 1 ), new THREE.Vector2( - 1, - 1 ), new THREE.Vector2( 1, - 1 )
	] ), { steps: 30, extrudePath: curve } ) );

} )();

out.tube_default = geometry( new THREE.TubeGeometry() );
out.tube_knot = geometry( new THREE.TubeGeometry( knotCurve(), 40, 0.5, 6, false ) );
out.tube_knot_closed = ( () => {

	const curve = knotCurve();
	curve.closed = true;
	return geometry( new THREE.TubeGeometry( curve, 40, 0.5, 6, true ) );

} )();

// `webgpu_modifier_curve`: the page's text, and the spline texture `Flow`
// fills from the page's curve. These come from `examples/jsm`, whose bare
// `three` imports resolve (by package self-reference) to the checkout's
// committed `build/`, which is built from the same commit as `src/`.

const { FontLoader } = await import( pathToFileURL( path.join( threeDir, 'examples/jsm/loaders/FontLoader.js' ) ).href );
const { TextGeometry } = await import( pathToFileURL( path.join( threeDir, 'examples/jsm/geometries/TextGeometry.js' ) ).href );
const { Flow } = await import( pathToFileURL( path.join( threeDir, 'examples/jsm/modifiers/CurveModifierGPU.js' ) ).href );
const fs = await import( 'fs' );

const font = new FontLoader().parse( JSON.parse( fs.readFileSync( path.join( threeDir, 'examples/fonts/helvetiker_regular.typeface.json' ), 'utf8' ) ) );

out.text_modifier_curve = ( () => {

	const g = new TextGeometry( 'Hello three.js!', {
		font: font,
		size: 0.2,
		depth: 0.05,
		curveSegments: 12,
		bevelEnabled: true,
		bevelThickness: 0.02,
		bevelSize: 0.01,
		bevelOffset: 0,
		bevelSegments: 5,
	} );
	g.rotateX( Math.PI );
	return geometry( g );

} )();

out.text_defaults = geometry( new TextGeometry( 'a\nb?', { font: font, curveSegments: 3 } ) );

out.flow_modifier_curve = ( () => {

	const curve = new THREE.CatmullRomCurve3( [
		new THREE.Vector3( 1, 0, - 1 ),
		new THREE.Vector3( 1, 0, 1 ),
		new THREE.Vector3( - 1, 0, 1 ),
		new THREE.Vector3( - 1, 0, - 1 ),
	] );
	curve.curveType = 'centripetal';
	curve.closed = true;

	const flow = new Flow( new THREE.Object3D() );
	flow.updateCurve( 0, curve );
	return {
		spineLength: flow.uniforms.spineLength,
		data: Array.from( flow.splineTexture.image.data ),
		points: curve.getPoints( 50 ).flatMap( ( p ) => [ p.x, p.y, p.z ] ),
	};

} )();

console.log( JSON.stringify( out ) );
