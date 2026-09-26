// Runs three.js' own `GLTFLoader` + `DRACOLoader` under node over one glTF
// asset and prints what every `KHR_draco_mesh_compression` primitive decodes
// to, so that `tests/gltf_draco.rs` can assert the Rust loader produces the
// same attributes, component types, `normalized` flags and index.
//
//   node tools/draco_reference.mjs <three.js checkout> <asset.glb>
//
// It prints one JSON object:
//
//   { "primitives": [ { "mesh": 0, "primitive": 0,
//                       "index": "<base64 Uint32Array>" | null,
//                       "attributes": { "position": { "type": "Float32Array",
//                                       "itemSize": 3, "normalized": false,
//                                       "data": "<base64, little endian>" }, … } },
//                     … ] }
//
// Only Draco primitives are listed. The data is the typed array's raw bytes,
// so the comparison can be exact; an interleaved attribute (DRACOLoader pads
// a stride that is not a multiple of four bytes) is compacted to `itemSize`
// components per vertex first.
//
// # No DOM, no workers
//
// `DRACOLoader` decodes in a Web Worker whose body is the module-level
// `DRACOWorker` function, stringified. Node has no `Worker` with that
// protocol, so the worker body is run in this thread instead: `DRACOWorker()`
// installs its `onmessage`, the decoder is the plain-JS Emscripten build
// (`examples/jsm/libs/draco/gltf/draco_decoder.js`, what the deprecated
// `setDecoderConfig( { type: 'js' } )` selects; it is loaded here directly),
// and
// `DRACOLoader.decodeGeometry` is replaced by a function that posts the same
// `init` / `decode` messages to it and hands the reply to the loader's own
// `_createGeometry`. Every line that turns Draco output into a
// `BufferGeometry` is three.js' own.
//
// Textures are stubbed to `null` (they need an image decoder and have no
// bearing on geometry).
//
// The loaders import from the bare specifier `'three'`, which node cannot
// resolve out of a checkout, so they are copied to a temporary directory with
// that specifier rewritten to the checkout's own ES build. Nothing in the
// checkout is modified.

import * as fs from 'fs';
import * as os from 'os';
import * as path from 'path';
import * as vm from 'vm';
import { createRequire } from 'module';
import { pathToFileURL } from 'url';

const [ threeDir, assetPath ] = process.argv.slice( 2 );
if ( ! threeDir || ! assetPath ) {

	console.error( 'usage: draco_reference.mjs <three.js checkout> <asset.glb>' );
	process.exit( 2 );

}

const buildUrl = pathToFileURL( path.join( threeDir, 'build/three.module.js' ) ).href;

// The loaders, and the two utils GLTFLoader imports, copied with `'three'`
// rewritten; `DRACOWorker` and `GLTFParser` are exported so they can be
// driven and stubbed from here.
const temporary = fs.mkdtempSync( path.join( os.tmpdir(), 'three-rs-draco-' ) );
const copies = {
	'loaders/DRACOLoader.js': '\nexport { DRACOWorker };\n',
	'loaders/GLTFLoader.js': '\nexport { GLTFParser };\n',
	'utils/BufferGeometryUtils.js': '',
	'utils/SkeletonUtils.js': '',
};
for ( const [ file, extra ] of Object.entries( copies ) ) {

	const source = fs
		.readFileSync( path.join( threeDir, 'examples/jsm', file ), 'utf8' )
		.replace( /from 'three'/g, `from '${ buildUrl }'` );
	fs.mkdirSync( path.join( temporary, path.dirname( file ) ), { recursive: true } );
	fs.writeFileSync( path.join( temporary, file ), source + extra );

}

const { DRACOLoader, DRACOWorker } = await import( pathToFileURL( path.join( temporary, 'loaders/DRACOLoader.js' ) ).href );
const { GLTFLoader, GLTFParser } = await import( pathToFileURL( path.join( temporary, 'loaders/GLTFLoader.js' ) ).href );
fs.rmSync( temporary, { recursive: true, force: true } );

// The Emscripten JS build defines a global `DracoDecoderModule`; it reads
// `require`, `__filename` and `__dirname` when it detects node.
globalThis.require = createRequire( import.meta.url );
const decoderPath = path.join( threeDir, 'examples/jsm/libs/draco/gltf/draco_decoder.js' );
globalThis.__filename = decoderPath;
globalThis.__dirname = path.dirname( decoderPath );
vm.runInThisContext( fs.readFileSync( decoderPath, 'utf8' ), { filename: decoderPath } );

// The worker body, in this thread. `onmessage` must exist before the
// (strict-mode) worker assigns it.
let reply = null;
globalThis.onmessage = null;
globalThis.self = globalThis;
globalThis.postMessage = ( message ) => reply( message );
DRACOWorker();
const worker = globalThis.onmessage;
worker( { data: { type: 'init', decoderConfig: {} } } );

// One decode at a time: the worker's reply resolves the pending promise.
let queue = Promise.resolve();
DRACOLoader.prototype.decodeGeometry = function ( buffer, taskConfig ) {

	const run = () => new Promise( ( resolve, reject ) => {

		reply = ( message ) => message.type === 'error' ? reject( new Error( message.error ) ) : resolve( message );
		worker( { data: { type: 'decode', id: 1, taskConfig, buffer } } );

	} );
	const result = queue.then( run ).then( ( message ) => this._createGeometry( message.geometry ) );
	queue = result.catch( () => {} );
	return result;

};

// Nothing to fetch: the decoder is already loaded above.
DRACOLoader.prototype._initDecoder = () => Promise.resolve();

GLTFParser.prototype.loadTexture = () => Promise.resolve( null );
GLTFParser.prototype.loadTextureImage = () => Promise.resolve( null );
GLTFParser.prototype.assignTexture = () => Promise.resolve( null );

const dracoLoader = new DRACOLoader();
const loader = new GLTFLoader();
loader.setDRACOLoader( dracoLoader );

const data = fs.readFileSync( assetPath );
const arrayBuffer = data.buffer.slice( data.byteOffset, data.byteOffset + data.byteLength );
const gltf = await loader.parseAsync( arrayBuffer, '' );
const parser = gltf.parser;

function base64( typed ) {

	return Buffer.from( typed.buffer, typed.byteOffset, typed.byteLength ).toString( 'base64' );

}

function compact( attribute ) {

	if ( ! attribute.isInterleavedBufferAttribute ) return attribute.array;

	const out = new attribute.array.constructor( attribute.count * attribute.itemSize );
	const stride = attribute.data.stride;
	for ( let i = 0; i < attribute.count; i ++ ) {

		for ( let c = 0; c < attribute.itemSize; c ++ ) {

			out[ i * attribute.itemSize + c ] = attribute.array[ i * stride + attribute.offset + c ];

		}

	}

	return out;

}

// `parseAsync` built the scene, and `loadMesh` calls `normalizeSkinWeights()`
// on every skinned mesh, which rewrites `skinWeight` in the cached geometry.
// What is wanted is what `DRACOLoader` produced, so the caches (the parsed
// bufferViews and `primitiveCache`) are dropped and every primitive decoded
// again from a fresh bufferView copy.
parser.cache.removeAll();
parser.primitiveCache = {};

const primitives = [];
const meshes = parser.json.meshes || [];
for ( let m = 0; m < meshes.length; m ++ ) {

	const defs = meshes[ m ].primitives;
	const geometries = await parser.loadGeometries( defs );
	for ( let p = 0; p < defs.length; p ++ ) {

		if ( ! defs[ p ].extensions || ! defs[ p ].extensions.KHR_draco_mesh_compression ) continue;

		const geometry = geometries[ p ];
		const attributes = {};
		for ( const [ name, attribute ] of Object.entries( geometry.attributes ) ) {

			const array = compact( attribute );
			attributes[ name ] = {
				type: array.constructor.name,
				itemSize: attribute.itemSize,
				normalized: attribute.normalized,
				data: base64( array ),
			};

		}

		primitives.push( {
			mesh: m,
			primitive: p,
			index: geometry.index ? base64( geometry.index.array ) : null,
			indexType: geometry.index ? geometry.index.array.constructor.name : null,
			attributes,
		} );

	}

}

process.stdout.write( JSON.stringify( { primitives } ) );
