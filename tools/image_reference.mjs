// Decodes images the way three.js' `GLTFLoader` does in the browser and writes
// the texels a WebGPU texture ends up holding, so that
// `tests/loaders_webp_avif.rs` can compare the port's decoders against them.
//
//   node tools/image_reference.mjs <three.js checkout> <list.json> <out dir>
//
// `list.json` is `[ { "file": "<path>", "mime": "image/webp" }, … ]`. For
// entry `i` the script writes `<out>/<i>.rgba` (tightly packed RGBA8, top row
// first) and `<out>/<i>.json` (`{ "width", "height" }`), or `<i>.error` with
// the browser's message when the image does not decode.
//
// # What the browser does, and what this repeats
//
// Three has no WebP or AVIF decoder of its own: `GLTFParser.loadImageSource`
// wraps the `bufferView` in a `Blob` of the image's `mimeType`, and
// `ImageBitmapLoader` hands that to
//
//   createImageBitmap( blob, { premultiplyAlpha: 'none', colorSpaceConversion: 'none' } )
//
// (its default options, plus the `colorSpaceConversion` it always adds).
// `WebGPUTextureUtils._copyImageToTexture` then uploads the bitmap with
// `copyExternalImageToTexture`, `premultipliedAlpha: false` (a glTF texture's
// `premultiplyAlpha` is false) and `flipY: false` (`GLTFLoader` sets
// `texture.flipY = false`). This script does exactly those two calls, into an
// `rgba8unorm` texture, and reads the texture back. The oracle is therefore
// Chromium's own decoders — libwebp, and libavif over dav1d — plus Chromium's
// YUV→RGB conversion, which is part of what a page gets and is therefore part
// of what the port has to match.
//
// Chrome is the one three.js' own `npm ci` downloads for its e2e suite, on
// software WebGPU (the same flags as `tools/web_gate.mjs`); the decode itself
// happens on the CPU either way.

import { existsSync } from 'node:fs';
import fs from 'node:fs/promises';
import http from 'node:http';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

const [ threeDir, listFile, outDir ] = process.argv.slice( 2 );
if ( ! threeDir || ! listFile || ! outDir ) {

	console.error( 'usage: node tools/image_reference.mjs <three.js checkout> <list.json> <out dir>' );
	process.exit( 2 );

}

async function importFromVendor( pkg, rel ) {

	const file = path.join( threeDir, 'node_modules', pkg, rel );
	if ( ! existsSync( file ) ) {

		throw new Error( `${pkg} not found under ${threeDir}/node_modules — run "npm ci" in the three.js checkout first` );

	}

	return import( pathToFileURL( file ).href );

}

// `tools/web_gate.mjs`' SOFTWARE_FLAGS: Dawn on SwiftShader, so no desk GPU
// is involved and CI gets the same adapter.
const FLAGS = [
	'--enable-unsafe-webgpu',
	'--enable-features=Vulkan',
	'--use-angle=swiftshader',
	'--use-vulkan=swiftshader',
	'--ignore-gpu-blocklist',
];

const list = JSON.parse( await fs.readFile( listFile, 'utf8' ) );
await fs.mkdir( outDir, { recursive: true } );

// `GET /in/<i>` serves entry i with its mime type (what the glTF `Blob` gets);
// `POST /out/<i>` takes the result back, so megabytes of texels never cross
// the DevTools protocol as base64.
const server = http.createServer( async ( request, response ) => {

	try {

		const [ , kind, index ] = request.url.split( '/' );
		const entry = list[ Number( index ) ];

		if ( request.method === 'GET' && kind === 'in' && entry ) {

			const body = await fs.readFile( entry.file );
			response.writeHead( 200, { 'content-type': entry.mime } );
			response.end( body );
			return;

		}

		if ( request.method === 'GET' && kind === '' ) {

			response.writeHead( 200, { 'content-type': 'text/html' } );
			response.end( '<!doctype html><title>image reference</title>' );
			return;

		}

		if ( request.method === 'POST' && kind === 'out' && entry ) {

			const chunks = [];
			for await ( const chunk of request ) chunks.push( chunk );
			const name = request.headers[ 'x-name' ];
			await fs.writeFile( path.join( outDir, `${index}.${name}` ), Buffer.concat( chunks ) );
			response.writeHead( 204 ).end();
			return;

		}

		response.writeHead( 404 ).end();

	} catch ( error ) {

		response.writeHead( 500 ).end( String( error ) );

	}

} );
await new Promise( ( resolve ) => server.listen( 0, '127.0.0.1', resolve ) );
const origin = `http://localhost:${server.address().port}`;

const { default: puppeteer } = await importFromVendor( 'puppeteer', 'lib/puppeteer/puppeteer.js' );
const browser = await puppeteer.launch( {
	executablePath: process.env.CHROME || await puppeteer.executablePath(),
	headless: true,
	args: [ ...FLAGS, '--no-sandbox' ],
} );

try {

	const page = await browser.newPage();
	await page.goto( `${origin}/`, { waitUntil: 'load' } );

	const failure = await page.evaluate( async ( count ) => {

		const adapter = navigator.gpu && await navigator.gpu.requestAdapter();
		if ( ! adapter ) return 'no WebGPU adapter';
		const device = await adapter.requestDevice();

		const post = ( i, name, body ) => fetch( `/out/${i}`, { method: 'POST', headers: { 'x-name': name }, body } );

		for ( let i = 0; i < count; i ++ ) {

			let bitmap;
			try {

				// ImageBitmapLoader.load: fetch, blob, createImageBitmap.
				const response = await fetch( `/in/${i}` );
				if ( ! response.ok ) throw new Error( `the input did not load: ${await response.text()}` );
				const blob = await response.blob();
				bitmap = await createImageBitmap( blob, { premultiplyAlpha: 'none', colorSpaceConversion: 'none' } );

			} catch ( error ) {

				await post( i, 'error', String( error ) );
				continue;

			}

			const { width, height } = bitmap;
			const texture = device.createTexture( {
				size: [ width, height ],
				format: 'rgba8unorm',
				usage: GPUTextureUsage.COPY_SRC | GPUTextureUsage.COPY_DST | GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.TEXTURE_BINDING,
			} );

			// WebGPUTextureUtils._copyImageToTexture, for a glTF texture.
			device.queue.copyExternalImageToTexture(
				{ source: bitmap, flipY: false },
				{ texture, mipLevel: 0, origin: { x: 0, y: 0, z: 0 }, premultipliedAlpha: false },
				{ width, height, depthOrArrayLayers: 1 }
			);

			const bytesPerRow = Math.ceil( width * 4 / 256 ) * 256;
			const buffer = device.createBuffer( { size: bytesPerRow * height, usage: GPUBufferUsage.COPY_DST | GPUBufferUsage.MAP_READ } );
			const encoder = device.createCommandEncoder();
			encoder.copyTextureToBuffer( { texture }, { buffer, bytesPerRow }, [ width, height ] );
			device.queue.submit( [ encoder.finish() ] );
			await buffer.mapAsync( GPUMapMode.READ );

			const mapped = new Uint8Array( buffer.getMappedRange() );
			const packed = new Uint8Array( width * height * 4 );
			for ( let y = 0; y < height; y ++ ) {

				packed.set( mapped.subarray( y * bytesPerRow, y * bytesPerRow + width * 4 ), y * width * 4 );

			}

			buffer.unmap();
			buffer.destroy();
			texture.destroy();
			bitmap.close();

			// As a `Blob`: a typed-array body takes Chromium ~2 s per 16 MB
			// upload, a `Blob` ~0.1 s.
			await post( i, 'rgba', new Blob( [ packed ] ) );
			await post( i, 'json', JSON.stringify( { width, height } ) );

		}

		return null;

	}, list.length );

	if ( failure ) throw new Error( failure );

} finally {

	await browser.close();
	server.close();

}
