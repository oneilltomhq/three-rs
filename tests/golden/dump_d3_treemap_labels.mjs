// Golden dump for step 6: the d33 treemap page's label arithmetic, taken from
// the JS itself. Read-only in lib3 and d33.
import * as THREE from '/home/tom/src/projects/lib3/node_modules/three/build/three.module.js';
import * as d3 from '/home/tom/src/projects/d33/rung0/examples/jsm/vendor/d3.js';
import { VectorFont } from '/home/tom/src/projects/lib3/src/sdf-text/VectorFont.js';
import { readFileSync, writeFileSync } from 'node:fs';

const DEG = Math.PI / 180;

// --- frame.js frameCamera, verbatim -----------------------------------------
function frameCamera( camera, box, { elevation = 30, azimuth = 35, fill = 0.85, points } = {} ) {
	const sphere = new THREE.Sphere();
	box.getBoundingSphere( sphere );
	const target = sphere.center.clone();
	const radius = sphere.radius || 1;
	const halfV = ( camera.fov * DEG ) / 2;
	const halfH = Math.atan( Math.tan( halfV ) * camera.aspect );
	const fitted = points && points.length ? points : boxCorners( box );
	const el = elevation * DEG;
	const az = azimuth * DEG;
	const direction = new THREE.Vector3(
		Math.cos( el ) * Math.sin( az ), Math.sin( el ), Math.cos( el ) * Math.cos( az ) );
	let distance = radius / ( fill * Math.tan( Math.min( halfV, halfH ) ) );
	const place = () => {
		camera.position.copy( target ).addScaledVector( direction, distance );
		camera.lookAt( target );
		camera.near = Math.max( distance - radius * 2, distance / 100 );
		camera.far = distance + radius * 2;
		camera.updateProjectionMatrix();
		camera.updateMatrixWorld( true );
	};
	const right = new THREE.Vector3(), up = new THREE.Vector3(), v = new THREE.Vector3();
	for ( let i = 0; i < 24; i ++ ) {
		place();
		let minX = Infinity, maxX = - Infinity, minY = Infinity, maxY = - Infinity;
		for ( const p of fitted ) {
			v.copy( p ).project( camera );
			if ( v.x < minX ) minX = v.x;
			if ( v.x > maxX ) maxX = v.x;
			if ( v.y < minY ) minY = v.y;
			if ( v.y > maxY ) maxY = v.y;
		}
		const cx = ( minX + maxX ) / 2;
		const cy = ( minY + maxY ) / 2;
		camera.matrixWorld.extractBasis( right, up, v );
		target.addScaledVector( right, cx * Math.tan( halfH ) * distance );
		target.addScaledVector( up, cy * Math.tan( halfV ) * distance );
		const extent = Math.max( maxX - minX, maxY - minY ) / 2;
		if ( extent === 0 ) break;
		const next = distance * extent / fill;
		const settled = Math.abs( next - distance ) < distance * 1e-4
			&& Math.abs( cx ) < 1e-4 && Math.abs( cy ) < 1e-4;
		distance = next;
		if ( settled ) break;
	}
	place();
	return target;
}
function boxCorners( box ) {
	const corners = [];
	for ( let i = 0; i < 8; i ++ ) corners.push( new THREE.Vector3(
		i & 1 ? box.max.x : box.min.x, i & 2 ? box.max.y : box.min.y, i & 4 ? box.max.z : box.min.z ) );
	return corners;
}

// --- labels.js basisQuaternion, verbatim ------------------------------------
function basisQuaternion( along, up, target = new THREE.Quaternion() ) {
	const _x = along.clone().normalize();
	const _z = new THREE.Vector3().crossVectors( _x, up ).normalize();
	const _y = new THREE.Vector3().crossVectors( _z, _x ).normalize();
	return target.setFromRotationMatrix( new THREE.Matrix4().makeBasis( _x, _y, _z ) );
}

// --- the page ---------------------------------------------------------------
const WIDTH = 1154, HEIGHT = 1154;
const SIDE = 20;
const S = SIDE / WIDTH;
const THICK = 0.015 * SIDE;
const LIFT = 0.012;
const LABEL_PX = 5;
const LINE = 0.9;
const LABEL_PAD_PX = 3;
const INNER_WIDTH = 800, INNER_HEIGHT = 500; // the harness viewport, dpr 1

const data = JSON.parse( readFileSync(
	'/home/tom/src/vendor/d3-gallery/notebooks/hierarchies/treemap.v2/files/flare.json', 'utf8' ) );

const format = d3.format( ',d' );

const root = d3.treemap()
	.tile( d3.treemapBinary )
	.size( [ WIDTH, HEIGHT ] )
	.padding( 1 )
	.round( true )
( d3.hierarchy( data ).sum( d => d.value ).sort( ( a, b ) => b.value - a.value ) );

const leaves = root.leaves();
const toWorld = ( x, y ) => new THREE.Vector3( x * S, 0, y * S - SIDE );

const camera = new THREE.PerspectiveCamera( 35, INNER_WIDTH / INNER_HEIGHT, 0.1, 4000 );
const box = new THREE.Box3( new THREE.Vector3( 0, 0, - SIDE ),
	new THREE.Vector3( SIDE, THICK + LIFT, 0 ) );
const target = frameCamera( camera, box, { elevation: 55, azimuth: 20, fill: 0.85 } );

const pxPerUnit = 250 / ( 2 * camera.position.distanceTo( target ) * Math.tan( camera.fov * Math.PI / 360 ) );
const fontSize = LABEL_PX / pxPerUnit;
const pad = LABEL_PAD_PX * S;

// node's fetch has no file:// scheme, so the bytes are handed over directly —
// VectorFont.load's ArrayBufferView branch, same parse.
const font = await VectorFont.load( new Uint8Array( readFileSync(
	'/home/tom/src/projects/d33/rung0/examples/fonts/Roboto-Regular.ttf' ) ) );
const measure = text => {
	let units = 0;
	for ( let i = 0; i < text.length; i ++ ) {
		units += font.advanceWidth( text[ i ] );
		if ( i ) units += font.kerning( text[ i - 1 ], text[ i ] );
	}
	return units / font.unitsPerEm * fontSize;
};

const flat = basisQuaternion( new THREE.Vector3( 1, 0, 0 ), new THREE.Vector3( 0, 0, - 1 ) );

const labelled = [];
for ( const leaf of leaves ) {
	const lines = leaf.data.name.split( /(?=[A-Z][a-z])|\s+/g ).concat( format( leaf.value ) );
	const w = ( leaf.x1 - leaf.x0 ) * S, h = ( leaf.y1 - leaf.y0 ) * S;
	const widest = Math.max( ...lines.map( measure ) );
	const fits = ! ( w < widest + 2 * pad || h < pad + lines.length * LINE * fontSize );
	if ( ! fits ) continue;
	const anchor = toWorld( leaf.x0, leaf.y0 );
	anchor.x += pad;
	anchor.z += pad;
	anchor.y = THICK + LIFT;
	labelled.push( { name: leaf.data.name, lines, widest,
		anchor: [ anchor.x, anchor.y, anchor.z ] } );
}

const out = {
	note: 'dumped from the d33 treemap page\'s own JS; see tests/golden/README.md',
	WIDTH, HEIGHT, SIDE, S, THICK, LIFT, LABEL_PX, LINE, LABEL_PAD_PX,
	INNER_WIDTH, INNER_HEIGHT,
	unitsPerEm: font.unitsPerEm,
	pxPerUnit, fontSize, pad,
	camera: {
		fov: camera.fov, aspect: camera.aspect, near: camera.near, far: camera.far,
		position: camera.position.toArray(),
		quaternion: camera.quaternion.toArray(),
		target: target.toArray()
	},
	flatQuaternion: flat.toArray(),
	leafCount: leaves.length,
	rootValue: root.value,
	leaves: leaves.map( leaf => ( {
		name: leaf.data.name, depth: leaf.depth, value: leaf.value,
		pkg: ( d => { while ( d.depth > 1 ) d = d.parent; return d.data.name; } )( leaf ),
		x0: leaf.x0, y0: leaf.y0, x1: leaf.x1, y1: leaf.y1
	} ) ),
	labelCount: labelled.length,
	labelled
};

const dest = process.argv[ 2 ];
writeFileSync( dest, JSON.stringify( out ) );
console.log( 'wrote', dest, 'leaves', leaves.length, 'labels', labelled.length,
	'fontSize', fontSize, 'pxPerUnit', pxPerUnit );
