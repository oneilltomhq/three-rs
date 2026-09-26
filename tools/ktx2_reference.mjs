// Runs three.js' own `KTX2Loader` under node over one `.ktx2` file and writes
// what it builds, so that `tests/ktx2_loader.rs` can assert the Rust loader
// produces the same texture, byte for byte.
//
//   node tools/ktx2_reference.mjs <three.js checkout> <file.ktx2> <out dir> <rgba|bc|astc|etc2>
//
// The last argument is the device `detectSupport()` is stubbed to describe —
// a `WebGPURenderer` whose `hasFeature()` answers yes to one WebGPU feature:
//
//   rgba  none at all, so Basis Universal textures transcode to the
//         uncompressed fallbacks (`RGBA32`, or `RGBA_HALF` for HDR);
//   bc    `texture-compression-bc`, the desktop case: BC7 for ETC1S and
//         UASTC, BC6H for UASTC HDR;
//   astc  `texture-compression-astc`: ASTC 4x4 for UASTC, the uncompressed
//         fallbacks for the rest;
//   etc2  `texture-compression-etc2`: ETC1 / ETC2 for ETC1S and UASTC.
//
// `texture-compression-s3tc`, `-etc1` and `-pvrtc` are not WebGPU feature
// names, so a browser never answers yes to them and no profile does either.
//
// It writes `<out dir>/texture.json`:
//
//   { "class": "CompressedTexture" | "CompressedArrayTexture" | "DataTexture" | …,
//     "format": <three.js format constant>, "type": <three.js type constant>,
//     "colorSpace": "srgb" | "srgb-linear" | "", "premultiplyAlpha": bool,
//     "minFilter": <constant>, "magFilter": <constant>, "generateMipmaps": bool,
//     "width": w, "height": h, "depth": layers or 0,
//     "normalized": bool | null,
//     "faces": [ [ { "width": w, "height": h, "file": "mip0.bin" }, … ] ] }
//
// `faces` has one entry, the texture's `mipmaps`, except for a
// `CompressedCubeTexture`, which has six (`face0_mip0.bin` …). Each mip's
// data is written as the loader hands it to the renderer (every array layer
// of a level concatenated, as `KTX2Loader` does), raw little-endian bytes of
// the typed array.
//
// # No DOM, no workers
//
// `KTX2Loader` transcodes in a Web Worker whose body is the stringified
// `KTX2Loader.BasisWorker`, with the Emscripten `basis_transcoder.js` pasted in
// front of it. Node has no such `Worker`, so the worker body runs in this
// thread instead: the transcoder script is evaluated here, the worker's
// `self.addEventListener( 'message' )` handler is captured, and the loader's
// `workerPool.postMessage` hands it the same `init` / `transcode` messages the
// pool would. `init()` (which only fetches the transcoder and builds the
// worker source) is replaced with the in-thread setup. Everything that turns a
// transcode into a texture — `_createTextureFrom`, `createRawTexture`,
// `parseColorSpace` — is three.js' own.
//
// The loader modules import the bare specifier `'three'`, which node cannot
// resolve out of a checkout, so they are copied to a temporary directory with
// that specifier rewritten to the checkout's ES build. Nothing in the checkout
// is modified.

import * as fs from 'fs';
import * as os from 'os';
import * as path from 'path';
import * as vm from 'vm';
import { createRequire } from 'module';
import { pathToFileURL } from 'url';

const [ threeDir, filePath, outDir, device ] = process.argv.slice( 2 );
const profiles = {
	rgba: [],
	bc: [ 'texture-compression-bc' ],
	astc: [ 'texture-compression-astc' ],
	etc2: [ 'texture-compression-etc2' ],
};

if ( ! threeDir || ! filePath || ! outDir || ! ( device in profiles ) ) {

	console.error( 'usage: ktx2_reference.mjs <three.js checkout> <file.ktx2> <out dir> <rgba|bc|astc|etc2>' );
	process.exit( 2 );

}

const buildUrl = pathToFileURL( path.join( threeDir, 'build/three.module.js' ) ).href;

const temporary = fs.mkdtempSync( path.join( os.tmpdir(), 'three-rs-ktx2-' ) );
const copies = [
	'loaders/KTX2Loader.js',
	'utils/WorkerPool.js',
	'libs/ktx-parse.module.js',
	'libs/zstddec.module.js',
	'math/ColorSpaces.js',
];
for ( const file of copies ) {

	const source = fs
		.readFileSync( path.join( threeDir, 'examples/jsm', file ), 'utf8' )
		.replace( /from 'three'/g, `from '${ buildUrl }'` );
	fs.mkdirSync( path.join( temporary, path.dirname( file ) ), { recursive: true } );
	fs.writeFileSync( path.join( temporary, file ), source );

}

let KTX2Loader;
try {

	( { KTX2Loader } = await import( pathToFileURL( path.join( temporary, 'loaders/KTX2Loader.js' ) ).href ) );

} finally {

	fs.rmSync( temporary, { recursive: true, force: true } );

}

// --- the worker, in this thread -------------------------------------------

// The constants `init()` prepends to the worker source.
globalThis._EngineFormat = KTX2Loader.EngineFormat;
globalThis._EngineType = KTX2Loader.EngineType;
globalThis._TranscoderFormat = KTX2Loader.TranscoderFormat;
globalThis._BasisFormat = KTX2Loader.BasisFormat;

// `basis_transcoder.js` defines the global `BASIS`; it probes `module` and
// `require` when it detects node, and the wasm binary is handed to it below.
globalThis.require = createRequire( import.meta.url );
const transcoderDir = path.join( threeDir, 'examples/jsm/libs/basis' );
const transcoderJs = path.join( transcoderDir, 'basis_transcoder.js' );
globalThis.__filename = transcoderJs;
globalThis.__dirname = transcoderDir;
vm.runInThisContext( fs.readFileSync( transcoderJs, 'utf8' ) + '\nglobalThis.BASIS = BASIS;', { filename: transcoderJs } );

let handler = null;
let reply = null;
globalThis.self = {
	addEventListener: ( type, fn ) => {

		if ( type === 'message' ) handler = fn;

	},
	postMessage: ( message ) => reply( message ),
};
KTX2Loader.BasisWorker();

// `detectSupport( renderer )` for a WebGPURenderer, against the stub device.
const features = new Set( profiles[ device ] );
const renderer = { isWebGPURenderer: true, hasFeature: ( name ) => features.has( name ) };

const loader = new KTX2Loader();
loader.detectSupport( renderer );

const transcoderBinary = fs.readFileSync( path.join( transcoderDir, 'basis_transcoder.wasm' ) );
handler( { data: {
	type: 'init',
	config: loader.workerConfig,
	transcoderBinary: transcoderBinary.buffer.slice( transcoderBinary.byteOffset, transcoderBinary.byteOffset + transcoderBinary.byteLength ),
} } );

loader.init = () => Promise.resolve();
loader.workerPool = {
	postMessage: ( message ) => new Promise( ( resolve ) => {

		reply = ( data ) => resolve( { data } );
		handler( { data: message } );

	} ),
	dispose: () => {},
};

// --- load and dump ----------------------------------------------------------

const bytes = fs.readFileSync( filePath );
const buffer = bytes.buffer.slice( bytes.byteOffset, bytes.byteOffset + bytes.byteLength );

const texture = await new Promise( ( resolve, reject ) => loader.parse( buffer, resolve, reject ) );

fs.mkdirSync( outDir, { recursive: true } );

function dumpMips( mips, prefix ) {

	return mips.map( ( mip, i ) => {

		const file = `${ prefix }mip${ i }.bin`;
		const data = mip.data;
		fs.writeFileSync( path.join( outDir, file ), Buffer.from( data.buffer, data.byteOffset, data.byteLength ) );
		return { width: mip.width, height: mip.height, file };

	} );

}

// A `CompressedCubeTexture`'s image is its six faces, each with its own mips.
const isCube = texture.isCompressedCubeTexture === true;
const image = isCube ? texture.image[ 0 ] : texture.image;

const json = {
	class: texture.constructor.name,
	format: texture.format,
	type: texture.type,
	colorSpace: texture.colorSpace,
	premultiplyAlpha: texture.premultiplyAlpha,
	minFilter: texture.minFilter,
	magFilter: texture.magFilter,
	generateMipmaps: texture.generateMipmaps,
	width: image.width,
	height: image.height,
	depth: image.depth || 0,
	normalized: texture.normalized === undefined ? null : texture.normalized,
	faces: isCube
		? texture.image.map( ( face, f ) => dumpMips( face.mipmaps, `face${ f }_` ) )
		: [ dumpMips( texture.mipmaps, '' ) ],
};

fs.writeFileSync( path.join( outDir, 'texture.json' ), JSON.stringify( json, null, '\t' ) + '\n' );
