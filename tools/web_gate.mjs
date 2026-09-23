#!/usr/bin/env node
// The web gate (issue #128): every graded example, rendered by the wasm build
// in headless Chrome on software WebGPU, graded by three.js' own comparator.
//
// Usage:
//   node tools/web_gate.mjs [<example>...] [--dist DIR] [--out DIR]
//                           [--hardware] [--timeout SECONDS] [--chrome PATH]
//
// What it does, per example, in one headless Chrome:
//
//   1. opens web/dist's page as `?example=<name>&hold`: the shell renders the
//      graded frame (800x500, both clocks pinned to 0), reads the renderer's
//      canvas texture back and publishes it as `window.__three_rs_graded`
//      instead of going live (see "Held on the graded frame" in
//      web/src/shell.rs);
//   2. waits for `<body data-graded>` or `<body data-error>`;
//   3. writes the pixels as `actual.png` and runs `src/testing/compare.mjs` on
//      it — the very script `tests/e2e/main.rs` runs natively, which imports
//      three.js' `test/e2e/image.js` unmodified, downscales by the grader's
//      viewScale and compares against `examples/screenshots/<name>.jpg` with
//      Three's own threshold. There is no second comparator here.
//
// The example list is the committed manifests (web/manifests/*.json), so it
// cannot drift from what the shell was built with. Examples named in
// tools/web_gate.skip still run and are reported, but do not fail the gate.
//
// The assets the page would fetch from raw.githubusercontent.com at the
// pinned r186 tag are answered from the local three.js checkout instead (the
// same tag, so the same bytes the native ladder reads off disk): no network,
// no rate limit.
//
// Needs: web/build.sh run first; a three.js r186 checkout at $THREE_JS_DIR
// (default ~/src/vendor/three.js) with `npm ci` done, for puppeteer-core,
// pngjs, image.js, the screenshots and (unless --chrome or $CHROME says
// otherwise) the Chrome puppeteer downloaded. Writes under --out (default
// target/web-gate/): <example>/{actual.png,actual.jpg,expected.jpg,diff.jpg}
// and summary.json. Exits non-zero if any example not in the skip list fails.
//
// --hardware drops the SwiftShader flags and lets Chrome pick the machine's
// GPU, for comparing a desk's hardware WebGPU against the software adapter CI
// has. The gate as enforced is the software one.

import { spawnSync } from 'node:child_process';
import fs from 'node:fs/promises';
import { existsSync } from 'node:fs';
import http from 'node:http';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const __dirname = path.dirname( fileURLToPath( import.meta.url ) );
const repoRoot = path.resolve( __dirname, '..' );

// The shell's own constants (web/src/shell.rs `ASSET_BASE`/`THREE_JS_TAG`).
const ASSET_PREFIX = 'https://raw.githubusercontent.com/mrdoob/three.js/r186/';

// Software WebGPU: Dawn on SwiftShader's Vulkan, which Chrome ships. Probed on
// a Fedora desk (Chrome 151/152, Intel Iris Xe) and on GitHub's ubuntu-latest:
// with these, `navigator.gpu.requestAdapter()` in headless returns the
// `google`/`swiftshader` fallback adapter on both, and never the desk's GPU,
// so the two grade the same renderer. See web/README.md, "Grading in the
// browser".
const SOFTWARE_FLAGS = [
	'--enable-unsafe-webgpu',
	'--enable-features=Vulkan',
	'--use-angle=swiftshader',
	'--use-vulkan=swiftshader',
	'--ignore-gpu-blocklist',
];

// test/e2e/puppeteer.js' own flags, less its headful-only ones: the desk's GPU.
const HARDWARE_FLAGS = [
	'--enable-unsafe-webgpu',
	'--enable-features=Vulkan',
	'--use-angle=vulkan',
	'--ignore-gpu-blocklist',
	'--disable-gpu-driver-bug-workarounds',
	'--disable-gpu-watchdog',
];

function usage( message ) {

	if ( message ) console.error( message );
	console.error( 'usage: node tools/web_gate.mjs [<example>...] [--dist DIR] [--out DIR] [--hardware] [--timeout SECONDS] [--chrome PATH]' );
	process.exit( 2 );

}

function parseArgs( argv ) {

	const options = {
		names: [],
		dist: path.join( repoRoot, 'web/dist' ),
		out: path.join( repoRoot, 'target/web-gate' ),
		hardware: false,
		timeoutMs: 300_000,
		chrome: process.env.CHROME || null,
	};
	for ( let i = 0; i < argv.length; i ++ ) {

		const arg = argv[ i ];
		const value = () => argv[ ++ i ] ?? usage( `${arg} needs a value` );
		if ( arg === '--dist' ) options.dist = path.resolve( value() );
		else if ( arg === '--out' ) options.out = path.resolve( value() );
		else if ( arg === '--hardware' ) options.hardware = true;
		else if ( arg === '--timeout' ) options.timeoutMs = Number( value() ) * 1000;
		else if ( arg === '--chrome' ) options.chrome = value();
		else if ( arg === '-h' || arg === '--help' ) usage();
		else if ( arg.startsWith( '-' ) ) usage( `unrecognised argument: ${arg}` );
		else options.names.push( arg );

	}

	return options;

}

const options = parseArgs( process.argv.slice( 2 ) );

const threeDir = process.env.THREE_JS_DIR || path.join( process.env.HOME, 'src/vendor/three.js' );

async function importFromVendor( pkg, rel ) {

	const file = path.join( threeDir, 'node_modules', pkg, rel );
	if ( ! existsSync( file ) ) {

		throw new Error( `${pkg} not found under ${threeDir}/node_modules — run "npm ci" in the three.js checkout first` );

	}

	return import( pathToFileURL( file ).href );

}

/** The graded examples: one per committed manifest. */
async function gradedExamples() {

	const dir = path.join( repoRoot, 'web/manifests' );
	return ( await fs.readdir( dir ) )
		.filter( ( file ) => file.endsWith( '.json' ) )
		.map( ( file ) => file.slice( 0, - '.json'.length ) )
		.sort();

}

/** tools/web_gate.skip: `<example>  <reason>` per line, `#` comments. */
async function skipList() {

	const file = path.join( __dirname, 'web_gate.skip' );
	const skip = new Map();
	if ( ! existsSync( file ) ) return skip;
	for ( const raw of ( await fs.readFile( file, 'utf8' ) ).split( '\n' ) ) {

		const line = raw.replace( /#.*/, '' ).trim();
		if ( ! line ) continue;
		const [ name, ...reason ] = line.split( /\s+/ );
		skip.set( name, reason.join( ' ' ) );

	}

	return skip;

}

const MIME = {
	'.html': 'text/html; charset=utf-8',
	'.js': 'text/javascript; charset=utf-8',
	'.wasm': 'application/wasm',
	'.json': 'application/json',
};

/** A static server over web/dist, on a free localhost port. */
async function serve( root ) {

	const server = http.createServer( async ( request, response ) => {

		const url = new URL( request.url, 'http://localhost' );
		const relative = decodeURIComponent( url.pathname ).replace( /^\/+/, '' ) || 'index.html';
		if ( relative === 'favicon.ico' ) {

			response.writeHead( 204 ).end();
			return;

		}

		const file = path.join( root, relative );
		if ( ! file.startsWith( root ) ) {

			response.writeHead( 403 ).end();
			return;

		}

		try {

			const body = await fs.readFile( file );
			response.writeHead( 200, { 'content-type': MIME[ path.extname( file ) ] || 'application/octet-stream' } );
			response.end( body );

		} catch {

			response.writeHead( 404 ).end();

		}

	} );
	await new Promise( ( resolve ) => server.listen( 0, '127.0.0.1', resolve ) );
	return server;

}

async function chromePath() {

	if ( options.chrome ) return options.chrome;
	// The Chrome three.js' own `npm ci` downloaded for its e2e suite.
	const { default: puppeteer } = await importFromVendor( 'puppeteer', 'lib/puppeteer/puppeteer.js' );
	return await puppeteer.executablePath();

}

/** One example: open it held, wait, take the pixels, grade them. */
async function gradeOne( browser, origin, name, PNG ) {

	const out = path.join( options.out, name );
	await fs.mkdir( out, { recursive: true } );

	const page = await browser.newPage();
	const errors = [];
	page.on( 'pageerror', ( error ) => errors.push( `pageerror: ${error.message || error}` ) );
	page.on( 'console', ( message ) => {

		if ( message.type() === 'error' ) errors.push( message.text() );

	} );

	await page.setRequestInterception( true );
	page.on( 'request', async ( request ) => {

		const url = request.url();
		if ( url.startsWith( ASSET_PREFIX ) ) {

			const relative = decodeURIComponent( url.slice( ASSET_PREFIX.length ) );
			try {

				const body = await fs.readFile( path.join( threeDir, relative ) );
				await request.respond( {
					status: 200,
					contentType: 'application/octet-stream',
					headers: { 'access-control-allow-origin': '*' },
					body,
				} );

			} catch {

				await request.respond( { status: 404, body: `not in ${threeDir}: ${relative}` } );

			}

		} else {

			await request.continue();

		}

	} );

	const started = performance.now();
	try {

		await page.goto( `${origin}/?example=${name}&hold`, { waitUntil: 'load' } );
		await page.waitForFunction(
			() => document.body.dataset.graded || document.body.dataset.error,
			{ timeout: options.timeoutMs, polling: 250 },
		);
		const state = await page.evaluate( () => ( {
			error: document.body.dataset.error || null,
		} ) );
		const ms = Math.round( performance.now() - started );
		if ( state.error ) return { name, ms, error: state.error, errors };

		// The pixels, out of the page as base64: a 1.6 MB Uint8Array does not
		// survive CDP's JSON as an array of numbers in any reasonable time.
		const graded = await page.evaluate( () => {

			const { width, height, pixels } = window.__three_rs_graded;
			let binary = '';
			for ( let i = 0; i < pixels.length; i += 0x8000 ) {

				binary += String.fromCharCode.apply( null, pixels.subarray( i, i + 0x8000 ) );

			}

			return { width, height, base64: btoa( binary ) };

		} );

		const png = new PNG( { width: graded.width, height: graded.height } );
		Buffer.from( graded.base64, 'base64' ).copy( png.data );
		const actual = path.join( out, 'actual.png' );
		await fs.writeFile( actual, PNG.sync.write( png ) );

		if ( graded.width !== 800 || graded.height !== 500 ) {

			return { name, ms, error: `graded frame is ${graded.width}x${graded.height}, not 800x500`, errors };

		}

		const expected = path.join( threeDir, 'examples/screenshots', `${name}.jpg` );
		const compare = spawnSync( process.execPath, [
			path.join( repoRoot, 'src/testing/compare.mjs' ), threeDir, actual, expected, out,
		], { encoding: 'utf8' } );
		if ( compare.status !== 0 ) {

			return { name, ms, error: `comparator failed: ${compare.stderr.trim()}`, errors };

		}

		const result = JSON.parse( compare.stdout.trim() );
		return { name, ms, result, errors };

	} catch ( error ) {

		const ms = Math.round( performance.now() - started );
		const status = await page.evaluate( () => document.getElementById( 'status' )?.textContent ).catch( () => null );
		return { name, ms, error: `${error.message}${status ? ` (status line: ${status})` : ''}`, errors };

	} finally {

		await page.close().catch( () => {} );

	}

}

async function main() {

	if ( ! existsSync( path.join( options.dist, 'three_rs_web_bg.wasm' ) ) ) {

		usage( `${options.dist} has no build; run web/build.sh first` );

	}

	const all = await gradedExamples();
	for ( const name of options.names ) {

		if ( ! all.includes( name ) ) usage( `no committed manifest for ${name}` );

	}

	const names = options.names.length ? options.names : all;
	const skip = await skipList();
	for ( const name of skip.keys() ) {

		if ( ! all.includes( name ) ) usage( `tools/web_gate.skip names ${name}, which has no committed manifest` );

	}

	const { default: puppeteer } = await importFromVendor( 'puppeteer-core', 'lib/puppeteer/puppeteer-core.js' );
	const pngjs = await importFromVendor( 'pngjs', 'lib/png.js' );
	const PNG = pngjs.PNG ?? pngjs.default.PNG;

	await fs.mkdir( options.out, { recursive: true } );
	const server = await serve( options.dist );
	const origin = `http://localhost:${server.address().port}`;

	const flags = options.hardware ? HARDWARE_FLAGS : SOFTWARE_FLAGS;
	const executablePath = await chromePath();
	console.log( `chrome: ${executablePath}` );
	console.log( `flags:  ${flags.join( ' ' )}` );

	const browser = await puppeteer.launch( {
		executablePath,
		headless: true,
		args: [ ...flags, '--no-sandbox', '--hide-scrollbars' ],
		defaultViewport: { width: 1000, height: 800 },
		protocolTimeout: 0,
		userDataDir: path.join( options.out, '.profile' ),
	} );
	const browserVersion = browser.version();
	console.log( `browser: ${await browserVersion}` );

	// Which WebGPU adapter this Chrome hands a page, asked the way the shell
	// asks. No adapter at all is the finding that fails everything below, so
	// say it once, first, rather than once per example.
	const probe = await browser.newPage();
	await probe.goto( `${origin}/`, { waitUntil: 'load' } );
	const adapter = await probe.evaluate( async () => {

		if ( ! navigator.gpu ) return { error: 'navigator.gpu is undefined' };
		const found = await navigator.gpu.requestAdapter();
		if ( ! found ) return { error: 'requestAdapter() returned null' };
		const { vendor, architecture, device, description } = found.info;
		return { vendor, architecture, device, description, fallback: found.info.isFallbackAdapter ?? null };

	} ).catch( ( error ) => ( { error: String( error ) } ) );
	await probe.close();
	const adapterLine = adapter.error ? `none (${adapter.error})` : `${adapter.vendor}/${adapter.architecture}${adapter.fallback ? ', fallback adapter' : ''}`;
	console.log( `adapter: ${adapterLine}` );

	const rows = [];
	try {

		for ( const name of names ) {

			const row = await gradeOne( browser, origin, name, PNG );
			row.skipped = skip.has( name ) ? skip.get( name ) : null;
			row.pass = Boolean( row.result?.pass ) && ! row.error;
			rows.push( row );
			const score = row.result ? `${row.result.differentPixels.toFixed( 2 )}%` : '-';
			const verdict = row.pass ? 'pass' : ( row.skipped !== null ? 'fail (skipped)' : 'FAIL' );
			console.log( `${name.padEnd( 40 )} ${score.padStart( 8 )}  ${verdict.padEnd( 14 )} ${String( row.ms ).padStart( 7 )} ms${row.error ? `  ${row.error}` : ''}` );
			for ( const error of row.errors.slice( 0, 5 ) ) console.log( `    console: ${error.split( '\n' )[ 0 ].slice( 0, 300 )}` );

		}

	} finally {

		await browser.close();
		server.close();

	}

	const failed = rows.filter( ( row ) => ! row.pass && row.skipped === null );
	const skippedButPassing = rows.filter( ( row ) => row.pass && row.skipped !== null );
	const passed = rows.filter( ( row ) => row.pass );

	await fs.writeFile( path.join( options.out, 'summary.json' ), JSON.stringify( { browser: await browserVersion, adapter, flags, rows }, null, '\t' ) );

	// The table again, in one piece, as markdown — for the log's tail and for
	// GitHub's job summary when there is one.
	const lines = [
		`${await browserVersion}, WebGPU adapter: ${adapterLine}`,
		'',
		'| example | different pixels | limit | verdict | ms |',
		'|---|---:|---:|---|---:|',
		...rows.map( ( row ) => {

			const score = row.result ? `${row.result.differentPixels.toFixed( 2 )}%` : '-';
			const limit = row.result ? `${row.result.maxDifferentPixels}%` : '-';
			let verdict = row.pass ? 'pass' : ( row.error ? `error: ${row.error.replace( /\s+/g, ' ' ).replace( /\|/g, '\\|' ).slice( 0, 200 )}` : 'fail' );
			if ( row.skipped !== null ) verdict += ` — skipped: ${row.skipped}`;
			return `| ${row.name} | ${score} | ${limit} | ${verdict} | ${row.ms} |`;

		} ),
		'',
		`${passed.length} of ${rows.length} pass; ${failed.length} fail outside tools/web_gate.skip.`,
	];
	if ( skippedButPassing.length ) {

		lines.push( '', `Passing but still in tools/web_gate.skip (remove them): ${skippedButPassing.map( ( row ) => row.name ).join( ', ' )}` );

	}

	console.log( '\n' + lines.join( '\n' ) );
	if ( process.env.GITHUB_STEP_SUMMARY ) {

		await fs.appendFile( process.env.GITHUB_STEP_SUMMARY, `## Web gate\n\n${lines.join( '\n' )}\n` );

	}

	process.exit( failed.length ? 1 : 0 );

}

main().catch( ( error ) => {

	console.error( error );
	process.exit( 2 );

} );
