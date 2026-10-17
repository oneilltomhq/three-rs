// Writes teapot-tables.rs to the current directory. three.js comes from
// THREE_JS_DIR, or ~/src/vendor/three.js.
import fs from 'fs';
import os from 'node:os';
import path from 'node:path';

const THREE_DIR = process.env.THREE_JS_DIR || path.join( os.homedir(), 'src/vendor/three.js' );
const src = fs.readFileSync( path.join( THREE_DIR, 'examples/jsm/geometries/TeapotGeometry.js' ), 'utf8' );

function table( name ) {
	const i = src.indexOf( 'const ' + name + ' = [' );
	const j = src.indexOf( '];', i );
	return src.slice( src.indexOf( '[', i ) + 1, j );
}

const patches = table( 'teapotPatches' );
const verts = table( 'teapotVertices' );

function nums( s ) {
	return s.replace( /\/\*[^*]*\*\//g, '' ).split( ',' ).map( x => x.trim() ).filter( x => x.length ).map( x => x.replace( /\s+/g, '' ) );
}

const p = nums( patches );
const v = nums( verts );
if ( p.length !== 512 ) throw new Error( 'patches ' + p.length );
if ( v.length % 3 !== 0 ) throw new Error( 'verts ' + v.length );

let out = '/// `teapotPatches` from `TeapotGeometry.js`, 32 * 4 * 4 Bezier spline patches.\n';
out += `const TEAPOT_PATCHES: [usize; ${p.length}] = [\n`;
for ( let i = 0; i < p.length; i += 16 ) out += '    ' + p.slice( i, i + 16 ).join( ', ' ) + ',\n';
out += '];\n\n';
out += '/// `teapotVertices` from `TeapotGeometry.js`, ' + ( v.length / 3 ) + ' control points.\n';
out += `const TEAPOT_VERTICES: [f64; ${v.length}] = [\n`;
for ( let i = 0; i < v.length; i += 3 ) out += '    ' + v.slice( i, i + 3 ).map( x => x.includes( '.' ) ? x : x + '.0' ).join( ', ' ) + ',\n';
out += '];\n';
fs.writeFileSync( 'teapot-tables.rs', out );
console.log( 'patches', p.length, 'vertices', v.length );
