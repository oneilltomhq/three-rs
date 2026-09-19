// Generates the 50-sphere oracle for webgpu_postprocessing_bloom_selective by
// running the page's own loop against three.js' own Color/Vector3, under the
// e2e harness's seeded Math.random (test/e2e/deterministic-injection.js).
// Run it against a three.js checkout's own build:
//
//   THREE=~/src/vendor/three.js node tests/fixtures/webgpu_postprocessing_bloom_selective/oracle.mjs
//
// The build is patched first, exactly as `test/e2e/puppeteer.js` patches it
// before injecting it into the page: `generateUUID()` draws
// `Math.random() * 0xffffffff` per object, and the harness rewrites those to
// `Math._random()` so that object construction does not eat the seeded
// sequence. Without that rewrite every `new Object3D()` shifts the draws and
// the whole scene moves.
// test/e2e/deterministic-injection.js
Math._random = Math.random;
let seed = Math.PI / 4;
Math.random = function () {
	const x = Math.sin( seed ++ ) * 10000;
	return x - Math.floor( x );
};

import { readFile, writeFile, mkdtemp } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const root = process.env.THREE ?? '../../../../three.js';
const patch = ( code ) => code.replace( /Math\.random\(\) \* 0xffffffff/g, 'Math._random() * 0xffffffff' );
const dir = await mkdtemp( join( tmpdir(), 'three-oracle-' ) );
for ( const file of [ 'three.module.js', 'three.core.js' ] ) {

	await writeFile( join( dir, file ), patch( await readFile( `${ root }/build/${ file }`, 'utf8' ) ) );

}

const THREE = await import( join( dir, 'three.module.js' ) );

const objects = [];

// --- verbatim from examples/webgpu_postprocessing_bloom_selective.html ---
const scene = new THREE.Scene();
const geometry = null;

for ( let i = 0; i < 50; i ++ ) {

	const color = new THREE.Color();
	color.setHSL( Math.random(), 0.7, Math.random() * 0.2 + 0.05 );

	const bloomIntensity = Math.random() > 0.5 ? 1 : 0;

	const sphere = new THREE.Object3D();
	sphere.position.x = Math.random() * 10 - 5;
	sphere.position.y = Math.random() * 10 - 5;
	sphere.position.z = Math.random() * 10 - 5;
	sphere.position.normalize().multiplyScalar( Math.random() * 4.0 + 2.0 );
	sphere.scale.setScalar( Math.random() * Math.random() + 0.5 );
	scene.add( sphere );

	objects.push( { position: sphere.position.toArray(), scale: sphere.scale.x, color: color.getHex( THREE.LinearSRGBColorSpace ), bloomIntensity } );

}
// --- end verbatim ---

console.log( JSON.stringify( { spheres: objects }, null, '\t' ) );
