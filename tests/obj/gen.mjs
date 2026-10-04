// Regenerates `oracle.json` from three.js' own `OBJLoader`, so
// `tests/loaders_obj.rs` grades the Rust loader against the JavaScript one
// rather than against itself.
//
//     node tests/obj/gen.mjs            # THREE_JS_DIR, or ~/src/vendor/three.js
//
// `parse()` is pure string work, so it runs under node. `OBJLoader.js`
// imports from the bare specifier 'three', which node cannot resolve inside
// the vendor checkout, so it is copied to a temp file with that one import
// rewritten to the built module (as `tests/lut/gen.mjs` does). Nothing in the
// vendor tree is touched.
//
// Besides `models/obj/tree.obj` (the outline page's model), a handful of
// synthetic inputs pin down the quirks the port reproduces: n-gon fans,
// missing normals and uvs, negative and out-of-range indices, vertex colours,
// `o` / `g` / `usemtl` / `s` bookkeeping, CRLF and `\` continuations, and
// malformed numbers. Each attribute is recorded as its count plus an FNV-1a-64
// over its `Float32Array` bytes and its first values as bit patterns, so NaN
// survives JSON.

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

const THREE = process.env.THREE_JS_DIR || path.join( os.homedir(), 'src/vendor/three.js' );
const build = pathToFileURL( path.join( THREE, 'build/three.module.js' ) ).href;

const tmp = fs.mkdtempSync( path.join( os.tmpdir(), 'obj-oracle-' ) );
const patched = path.join( tmp, 'OBJLoader.mjs' );
fs.writeFileSync( patched, fs.readFileSync( path.join( THREE, 'examples/jsm/loaders/OBJLoader.js' ), 'utf8' )
	.replace( "from 'three'", `from '${ build }'` ) );
const { OBJLoader } = await import( pathToFileURL( patched ).href );

// `console.warn` for skipped lines is noise here.
console.warn = () => {};

// FNV-1a 64 over bytes.
function fnv1a64( bytes ) {

	let h = 0xcbf29ce484222325n;
	const p = 0x100000001b3n;
	const m = ( 1n << 64n ) - 1n;
	for ( let i = 0; i < bytes.length; i ++ ) h = ( ( h ^ BigInt( bytes[ i ] ) ) * p ) & m;
	return h.toString( 16 ).padStart( 16, '0' );

}

const bits = ( f32 ) => Array.from( new Uint32Array( f32.buffer, f32.byteOffset, f32.length ) );

function describe( text ) {

	const group = new OBJLoader().parse( text );
	return {
		materialLibraries: group.materialLibraries,
		meshes: group.children.map( ( mesh ) => {

			const geometry = mesh.geometry;
			const attributes = {};
			for ( const name of Object.keys( geometry.attributes ) ) {

				const array = geometry.attributes[ name ].array;
				attributes[ name ] = {
					count: geometry.attributes[ name ].count,
					hash: fnv1a64( new Uint8Array( array.buffer, array.byteOffset, array.byteLength ) ),
					head: bits( array.subarray( 0, 18 ) ),
				};

			}

			const materials = Array.isArray( mesh.material ) ? mesh.material : [ mesh.material ];
			return {
				type: mesh.type,
				name: mesh.name,
				attributes,
				groups: geometry.groups.map( ( g ) => [ g.start, g.count, g.materialIndex ] ),
				materials: materials.map( ( m ) => ( { name: m.name, flatShading: m.flatShading, vertexColors: m.vertexColors } ) ),
			};

		} ),
	};

}

const synthetic = {
	quad_fan: 'v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nv -1 1 0\nf 1 2 3 4 5\n',
	face_normals_and_uvs: 'v 0 0 0\nv 1 0 0\nv 0 1 0\nvt 0.25 0.75\nvt 1 0\nvt 0 1\nvn 0 0 1\nf 1/1 2/2 3/3\nf 1//1 2//1 3//1\nf 3 2 1\n',
	negative_and_missing: 'v 0 0 0\nv 1 0 0\nv 0 1 0\nf -3 -2 -1\nf 1 2 9\nf 1/4/2 2/1/1 3\n',
	vertex_colors: 'v 0 0 0 1 0.5 0\nv 1 0 0 0.2 0.2 0.2\nv 0 1 0\nf 1 2 3\n',
	objects_and_materials: '# comment\nmtllib a.mtl\nmtllib b c.mtl\no first\nv 0 0 0\nv 1 0 0\nv 0 1 0\nusemtl red\nf 1 2 3\nusemtl blue\ns off\nf 1 2 3\nf 3 2 1\ng second\nf 1 2 3\nusemtl green\ns 1\nf 2 3 1\ng empty\no\nusemtl tail\nf 1 2 3\n',
	crlf_and_continuation: 'v 0 0 0\r\nv 1 \\\n0 0\r\nv 0 1 0\r\n  \tf 1 2 3\r\n',
	malformed_numbers: 'v 1e1 .5 -x\nv 1.5.5 2abc +3\nv Infinity -Infinity 0\nf 1 2 3\nusemap foo\n\0\ns\n',
	usemtl_before_faces: 'v 0 0 0\nv 1 0 0\nv 0 1 0\nusemtl a\nusemtl b\nf 1 2 3\n',
};

const oracle = {
	tree: describe( fs.readFileSync( path.join( THREE, 'examples/models/obj/tree.obj' ), 'utf8' ) ),
	synthetic: Object.fromEntries( Object.entries( synthetic ).map( ( [ name, text ] ) => [ name, { text, ...describe( text ) } ] ) ),
};

fs.writeFileSync( new URL( './oracle.json', import.meta.url ), JSON.stringify( oracle, null, '\t' ) + '\n' );
fs.rmSync( tmp, { recursive: true } );
