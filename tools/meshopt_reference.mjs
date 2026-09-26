// Runs three.js' own `GLTFLoader` + `MeshoptDecoder` under node over one glTF
// asset and prints what its `EXT_meshopt_compression` data decodes to, so
// that `tests/gltf_meshopt.rs` can assert the Rust loader produces the same
// bytes, accessors and geometry.
//
//   node tools/meshopt_reference.mjs <three.js checkout> <asset.glb>
//   node tools/meshopt_reference.mjs <three.js checkout> --decode \
//        <mode> <filter> <count> <byteStride> <encoded.bin>
//
// The second form runs `MeshoptDecoder.decodeGltfBuffer` alone over one
// encoded stream and prints the decoded bytes as base64; the test uses it to
// check the filters on inputs the two example assets do not reach.
//
// The first form prints one JSON object:
//
//   { "bufferViews": [ { "index": 5, "data": "<base64>" }, … ],
//     "accessors": [ { "type": "Int16Array", "itemSize": 3,
//                      "normalized": true, "data": "<base64>" }, … ],
//     "primitives": [ { "mesh": 0, "primitive": 0,
//                       "index": { "type": "Uint16Array", "data": "…" } | null,
//                       "attributes": { "position": { type, itemSize,
//                                       normalized, data }, … },
//                       "morphAttributes": { "position": [ { … }, … ], … } },
//                     … ] }
//
// * `bufferViews` is every bufferView carrying the extension, as
//   `GLTFMeshoptCompression.loadBufferView` resolves it: the decoder's
//   output, filter applied, byte for byte.
// * `accessors` is every accessor, as `GLTFParser.loadAccessor` builds it.
// * `primitives` is every mesh primitive, as `GLTFParser.loadGeometries`
//   builds it (before `loadMesh`, which may rewrite `skinWeight`).
//
// Typed arrays are printed as their raw little-endian bytes so the
// comparison can be exact; an interleaved attribute is compacted to
// `itemSize` components per vertex first.
//
// # The decoder
//
// `examples/jsm/libs/meshopt_decoder.module.js` is meshoptimizer's own
// decoder compiled to WebAssembly, and has no dependency on `three` or the
// DOM: node runs it as is. It picks its SIMD build when
// `WebAssembly.validate` accepts SIMD, which node does, as every current
// browser does; so the bytes here are what the SIMD decoder and filters
// produce, and that is the build three.js' examples run.
//
// Textures are stubbed to `null` (they need an image decoder, and both
// meshopt assets in the examples use KTX2 textures, which have no bearing on
// geometry).
//
// `GLTFLoader` imports from the bare specifier `'three'`, which node cannot
// resolve out of a checkout, so it (and the two utils it imports) are copied
// to a temporary directory with that specifier rewritten to the checkout's
// own ES build. Nothing in the checkout is modified.

import * as fs from 'fs';
import * as os from 'os';
import * as path from 'path';
import { pathToFileURL } from 'url';

const [ threeDir, assetPath ] = process.argv.slice( 2 );
if ( ! threeDir || ! assetPath ) {

	console.error( 'usage: meshopt_reference.mjs <three.js checkout> <asset.glb>' );
	process.exit( 2 );

}

const { MeshoptDecoder } = await import( pathToFileURL( path.join( threeDir, 'examples/jsm/libs/meshopt_decoder.module.js' ) ).href );
await MeshoptDecoder.ready;

if ( assetPath === '--decode' ) {

	const [ mode, filter, count, stride, file ] = process.argv.slice( 4 );
	const source = new Uint8Array( fs.readFileSync( file ) );
	const target = new Uint8Array( Number( count ) * Number( stride ) );
	MeshoptDecoder.decodeGltfBuffer( target, Number( count ), Number( stride ), source, mode, filter );
	// Exit only once the pipe has taken everything; `process.exit()` straight
	// after a large `write` truncates it.
	process.stdout.write( Buffer.from( target ).toString( 'base64' ), () => process.exit( 0 ) );
	await new Promise( () => {} );

}

const buildUrl = pathToFileURL( path.join( threeDir, 'build/three.module.js' ) ).href;

const temporary = fs.mkdtempSync( path.join( os.tmpdir(), 'three-rs-meshopt-' ) );
const copies = {
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

const { GLTFLoader, GLTFParser } = await import( pathToFileURL( path.join( temporary, 'loaders/GLTFLoader.js' ) ).href );
fs.rmSync( temporary, { recursive: true, force: true } );

GLTFParser.prototype.loadTexture = () => Promise.resolve( null );
GLTFParser.prototype.loadTextureImage = () => Promise.resolve( null );
GLTFParser.prototype.assignTexture = () => Promise.resolve( null );

const loader = new GLTFLoader();
loader.setMeshoptDecoder( MeshoptDecoder );

const data = fs.readFileSync( assetPath );
const arrayBuffer = data.buffer.slice( data.byteOffset, data.byteOffset + data.byteLength );
const gltf = await loader.parseAsync( arrayBuffer, '' );
const parser = gltf.parser;
const json = parser.json;

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

function describe( attribute ) {

	const array = compact( attribute );
	return {
		type: array.constructor.name,
		itemSize: attribute.itemSize,
		normalized: attribute.normalized,
		data: base64( array ),
	};

}

// `loadMesh` normalizes skin weights in the cached geometry; drop every
// cache so what follows is what the decoder and `loadAccessor` produce.
parser.cache.removeAll();
parser.primitiveCache = {};

const bufferViews = [];
for ( let i = 0; i < ( json.bufferViews || [] ).length; i ++ ) {

	const extensions = json.bufferViews[ i ].extensions;
	if ( ! extensions || ! extensions.EXT_meshopt_compression ) continue;

	const buffer = await parser.getDependency( 'bufferView', i );
	bufferViews.push( { index: i, data: base64( new Uint8Array( buffer ) ) } );

}

const accessors = [];
for ( let i = 0; i < ( json.accessors || [] ).length; i ++ ) {

	accessors.push( describe( await parser.getDependency( 'accessor', i ) ) );

}

const primitives = [];
const meshes = json.meshes || [];
for ( let m = 0; m < meshes.length; m ++ ) {

	const defs = meshes[ m ].primitives;
	const geometries = await parser.loadGeometries( defs );
	for ( let p = 0; p < defs.length; p ++ ) {

		const geometry = geometries[ p ];
		const attributes = {};
		for ( const [ name, attribute ] of Object.entries( geometry.attributes ) ) {

			attributes[ name ] = describe( attribute );

		}

		const morphAttributes = {};
		for ( const [ name, list ] of Object.entries( geometry.morphAttributes ) ) {

			morphAttributes[ name ] = list.map( describe );

		}

		primitives.push( {
			mesh: m,
			primitive: p,
			index: geometry.index ? describe( geometry.index ) : null,
			attributes,
			morphAttributes,
		} );

	}

}

process.stdout.write( JSON.stringify( { bufferViews, accessors, primitives } ) );
