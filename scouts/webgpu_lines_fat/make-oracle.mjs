// Regenerates `spline_oracle.json` — the 768 CatmullRomCurve3 points and their
// HSL colours that `webgpu_lines_fat.html` feeds to `LineGeometry`.
//
//   node make-oracle.mjs [path/to/three.js] > spline_oracle.json
//
// Imports three's own `src/` modules directly (no bundle, no DOM), and inlines
// `hilbert3D` from `examples/jsm/utils/GeometryUtils.js` verbatim because that
// file imports the bare specifier 'three'.
import { pathToFileURL } from 'node:url';
import path from 'node:path';

const root = process.argv[ 2 ] || path.join( process.env.HOME, 'src/vendor/three.js' );
const u = p => pathToFileURL( path.join( root, p ) ).href;

const { Vector3 } = await import( u( 'src/math/Vector3.js' ) );
const { Color } = await import( u( 'src/math/Color.js' ) );
const { CatmullRomCurve3 } = await import( u( 'src/extras/curves/CatmullRomCurve3.js' ) );
const { SRGBColorSpace } = await import( u( 'src/constants.js' ) );

function hilbert3D( center = new Vector3( 0, 0, 0 ), size = 10, iterations = 1,
	v0 = 0, v1 = 1, v2 = 2, v3 = 3, v4 = 4, v5 = 5, v6 = 6, v7 = 7 ) {

	const half = size / 2;

	const vec_s = [
		new Vector3( center.x - half, center.y + half, center.z - half ),
		new Vector3( center.x - half, center.y + half, center.z + half ),
		new Vector3( center.x - half, center.y - half, center.z + half ),
		new Vector3( center.x - half, center.y - half, center.z - half ),
		new Vector3( center.x + half, center.y - half, center.z - half ),
		new Vector3( center.x + half, center.y - half, center.z + half ),
		new Vector3( center.x + half, center.y + half, center.z + half ),
		new Vector3( center.x + half, center.y + half, center.z - half )
	];

	const vec = [ vec_s[ v0 ], vec_s[ v1 ], vec_s[ v2 ], vec_s[ v3 ], vec_s[ v4 ], vec_s[ v5 ], vec_s[ v6 ], vec_s[ v7 ] ];

	if ( -- iterations >= 0 ) {

		let tmp = [];

		tmp.push( ...hilbert3D( vec[ 0 ], half, iterations, v0, v3, v4, v7, v6, v5, v2, v1 ) );
		tmp.push( ...hilbert3D( vec[ 1 ], half, iterations, v0, v7, v6, v1, v2, v5, v4, v3 ) );
		tmp.push( ...hilbert3D( vec[ 2 ], half, iterations, v0, v7, v6, v1, v2, v5, v4, v3 ) );
		tmp.push( ...hilbert3D( vec[ 3 ], half, iterations, v2, v3, v0, v1, v6, v7, v4, v5 ) );
		tmp.push( ...hilbert3D( vec[ 4 ], half, iterations, v2, v3, v0, v1, v6, v7, v4, v5 ) );
		tmp.push( ...hilbert3D( vec[ 5 ], half, iterations, v4, v3, v2, v5, v6, v1, v0, v7 ) );
		tmp.push( ...hilbert3D( vec[ 6 ], half, iterations, v4, v3, v2, v5, v6, v1, v0, v7 ) );
		tmp.push( ...hilbert3D( vec[ 7 ], half, iterations, v6, v5, v2, v1, v0, v3, v4, v7 ) );

		return tmp;

	}

	return vec;

}

const points = hilbert3D( new Vector3( 0, 0, 0 ), 20.0, 1, 0, 1, 2, 3, 4, 5, 6, 7 );
const spline = new CatmullRomCurve3( points );
const divisions = Math.round( 12 * points.length );

const positions = [];
const colors = [];
const point = new Vector3();
const lineColor = new Color();

for ( let i = 0, l = divisions; i < l; i ++ ) {

	const t = i / l;

	spline.getPoint( t, point );
	positions.push( point.x, point.y, point.z );

	lineColor.setHSL( t, 1.0, 0.5, SRGBColorSpace );
	colors.push( lineColor.r, lineColor.g, lineColor.b );

}

const f32 = a => Array.from( new Float32Array( a ) );

process.stdout.write( JSON.stringify( {
	note: 'webgpu_lines_fat.html: hilbert3D(0,0,0 / 20 / 1) -> CatmullRomCurve3 -> 768 samples',
	hilbertPointCount: points.length,
	divisions,
	instanceCount: divisions - 1,
	hilbert: f32( points.flatMap( p => [ p.x, p.y, p.z ] ) ),
	positions: f32( positions ),
	colors: f32( colors )
}, null, 0 ) );
