// Runs three.js' own `LightProbeGenerator.fromCubeTexture()` under node on six
// RGBA8 cube faces and prints the probe's spherical-harmonic coefficients, so
// that `tests/addons_light_probe_generator.rs` can assert the Rust port
// projects the same texels to the same nine colours.
//
//   node tools/light_probe_generator_reference.mjs <three.js checkout> <faces> <size> <colorSpace>
//
// `<faces>` is a file of six square `<size>`-texel RGBA8 faces back to back,
// in the order px, nx, py, ny, pz, nz, rows top-down: the bytes the test also
// hands the port. `<colorSpace>` is the cube's `colorSpace` (`''`, `'srgb'` or
// `'srgb-linear'`). It prints one JSON array, `sh.coefficients` flattened
// (`SphericalHarmonics3.toArray()`), 27 numbers.
//
// # No DOM
//
// `fromCubeTexture()` reads each face through a 2D canvas: `drawImage()` then
// `getImageData()`. The stub `document` below gives it a canvas whose context
// hands back the face's own bytes, which is what a browser returns for an
// opaque RGBA8 image drawn at its own size.
//
// `LightProbeGenerator.js` imports from the bare specifier `'three'`, which node
// cannot resolve out of a checkout, so the file is copied to a temporary one
// with that specifier rewritten to the checkout's own ES build. Nothing in the
// checkout is modified.

import * as fs from 'fs';
import * as os from 'os';
import * as path from 'path';
import { pathToFileURL } from 'url';

const [ threeDir, facesPath, sizeArg, colorSpace ] = process.argv.slice( 2 );
if ( ! threeDir || ! facesPath || ! sizeArg || colorSpace === undefined ) {

	console.error( 'usage: light_probe_generator_reference.mjs <three.js checkout> <faces> <size> <colorSpace>' );
	process.exit( 2 );

}

const buildUrl = pathToFileURL( path.join( threeDir, 'build/three.module.js' ) ).href;
const THREE = await import( buildUrl );

const source = fs
	.readFileSync( path.join( threeDir, 'examples/jsm/lights/LightProbeGenerator.js' ), 'utf8' )
	.replace( /from 'three'/g, `from '${ buildUrl }'` );

const temporary = path.join(
	fs.mkdtempSync( path.join( os.tmpdir(), 'three-rs-light-probe-' ) ),
	'LightProbeGenerator.js'
);
fs.writeFileSync( temporary, source );

const { LightProbeGenerator } = await import( pathToFileURL( temporary ).href );

globalThis.document = {
	createElement: () => ( {
		getContext: () => ( {
			drawImage( image ) {

				this.image = image;

			},
			getImageData() {

				return { data: this.image.data, width: this.image.width, height: this.image.height };

			}
		} )
	} )
};

const size = Number( sizeArg );
const bytes = fs.readFileSync( facesPath );
const faceBytes = size * size * 4;
if ( bytes.length !== faceBytes * 6 ) {

	console.error( `expected ${ faceBytes * 6 } bytes of faces, got ${ bytes.length }` );
	process.exit( 2 );

}

const images = [];
for ( let face = 0; face < 6; face ++ ) {

	const data = new Uint8ClampedArray( bytes.buffer, bytes.byteOffset + face * faceBytes, faceBytes );
	images.push( { width: size, height: size, data } );

}

const cubeTexture = new THREE.CubeTexture( images );
cubeTexture.colorSpace = colorSpace;

const probe = LightProbeGenerator.fromCubeTexture( cubeTexture );

console.log( JSON.stringify( probe.sh.toArray() ) );
