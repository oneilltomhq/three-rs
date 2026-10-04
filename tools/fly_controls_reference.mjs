// Runs three.js' own `FlyControls` under node and writes what it does to a
// camera into `tests/fixtures/fly_controls.json`, which
// `tests/addons_fly_controls.rs` replays the Rust port against.
//
//   node tools/fly_controls_reference.mjs [three.js checkout] [output]
//
// The checkout defaults to $THREE_JS_DIR, then ~/src/vendor/three.js, as
// `three_rs::testing::three_js_dir()` does. The output defaults to the
// committed fixture; the Rust test passes a scratch path instead, to check
// the fixture is still what three.js does.
//
// The fixture is one JSON object, `{ "<scenario>": [ step, step, … ] }`,
// where each step is the camera's position and quaternion after one scripted
// action, and whether the controls dispatched `change` during it. The Rust
// test hard-codes the same scripts, so a change to one side's script is a
// change to both.
//
// # No DOM
//
// As in `tools/orbit_controls_reference.mjs`: the class is constructed with a
// null element, so `connect()` registers no listener, and `domElement` is then
// set to the stub below, whose `offsetWidth` / `offsetHeight` /
// `offsetLeft` / `offsetTop` are what `_getContainerDimensions()` reads. The
// handlers are invoked through the bound `_onKeyDown`, `_onKeyUp`,
// `_onPointerDown`, `_onPointerMove`, `_onPointerUp` and `_onPointerCancel`
// with plain objects carrying the fields they read. `document` is a global
// stub only so that `this.domElement != document` can be evaluated.
//
// The element sits at a non-zero offset and the pointer events carry page
// coordinates; the Rust port takes element-relative ones, so the test passes
// each `pageX - OFFSET_LEFT`, `pageY - OFFSET_TOP`.
//
// The class imports from the bare specifier `'three'`, which node cannot
// resolve out of a checkout, so the file is copied to a temporary one with
// that specifier rewritten to the checkout's own ES build. Nothing in the
// checkout is modified.

import * as fs from 'fs';
import * as os from 'os';
import * as path from 'path';
import { fileURLToPath, pathToFileURL } from 'url';

const here = path.dirname( fileURLToPath( import.meta.url ) );
const threeDir = process.argv[ 2 ] || process.env.THREE_JS_DIR || path.join( os.homedir(), 'src/vendor/three.js' );
const fixture = process.argv[ 3 ] || path.join( here, '../tests/fixtures/fly_controls.json' );

const buildUrl = pathToFileURL( path.join( threeDir, 'build/three.module.js' ) ).href;
const THREE = await import( buildUrl );

// The fixture is three.js at the pinned commit (5f610f5); another revision's
// output is not a fixture.
if ( THREE.REVISION !== '187dev' ) throw new Error( `expected three.js r187dev, ${ threeDir } is r${ THREE.REVISION }` );

const source = fs
	.readFileSync( path.join( threeDir, 'examples/jsm/controls/FlyControls.js' ), 'utf8' )
	.replace( /from 'three'/g, `from '${ buildUrl }'` );

const temporary = fs.mkdtempSync( path.join( os.tmpdir(), 'three-rs-fly-' ) );
fs.writeFileSync( path.join( temporary, 'FlyControls.js' ), source );

const { FlyControls } = await import( pathToFileURL( path.join( temporary, 'FlyControls.js' ) ).href );
fs.rmSync( temporary, { recursive: true, force: true } );

globalThis.document = { isStubDocument: true };

const ELEMENT_WIDTH = 640;
const ELEMENT_HEIGHT = 360;
const OFFSET_LEFT = 30;
const OFFSET_TOP = 20;

function element() {

	return {
		offsetWidth: ELEMENT_WIDTH,
		offsetHeight: ELEMENT_HEIGHT,
		offsetLeft: OFFSET_LEFT,
		offsetTop: OFFSET_TOP,
		style: {},
		addEventListener: () => {},
		removeEventListener: () => {}
	};

}

// A camera off every axis and turned off every axis.
function camera() {

	const camera = new THREE.PerspectiveCamera( 45, ELEMENT_WIDTH / ELEMENT_HEIGHT, 0.25, 200 );
	camera.position.set( 3, 4, 5 );
	camera.lookAt( - 1, 1.5, 0.5 );
	camera.updateMatrixWorld();
	return camera;

}

// The controls, and a counter of the `change` events they dispatch.
function controlsOn( camera ) {

	const controls = new FlyControls( camera, null );
	controls.domElement = element();
	const changes = { count: 0 };
	controls.addEventListener( 'change', () => changes.count ++ );
	return { controls, changes };

}

function record( camera, changes ) {

	const changed = changes === undefined ? null : changes.count > 0;
	if ( changes !== undefined ) changes.count = 0;
	return {
		position: camera.position.toArray(),
		quaternion: camera.quaternion.toArray(),
		changed
	};

}

const DT = 1 / 60;

// x and y are element-relative; the event carries page coordinates.
function pointer( button, x, y ) {

	return { pointerId: 1, button, pageX: x + OFFSET_LEFT, pageY: y + OFFSET_TOP };

}

const key = ( code, altKey = false ) => ( { code, altKey } );

// Runs `update( delta )` `n` times, recording after each.
function frames( controls, changes, c, steps, n, delta = DT ) {

	for ( let i = 0; i < n; i ++ ) {

		controls.update( delta );
		steps.push( record( c, changes ) );

	}

}

// One step per line, so a diff of a regenerated fixture reads step by step.
function format( scenarios ) {

	const blocks = Object.entries( scenarios ).map( ( [ name, steps ] ) =>
		`\t${ JSON.stringify( name ) }: [\n${ steps.map( ( s ) => `\t\t${ JSON.stringify( s ) }` ).join( ',\n' ) }\n\t]`
	);
	return `{\n${ blocks.join( ',\n' ) }\n}\n`;

}

const scenarios = {};

// 1. Keys: translate along each axis, pitch / yaw / roll, Shift (which sets a
// multiplier `update()` never reads), an Alt-modified key (ignored) and an
// unbound one.
{

	const c = camera();
	const { controls, changes } = controlsOn( c );
	controls.movementSpeed = 4;
	controls.rollSpeed = 0.6;
	const steps = [ record( c ) ];

	controls._onKeyDown( key( 'KeyW' ) );
	controls._onKeyDown( key( 'KeyA' ) );
	frames( controls, changes, c, steps, 5 );

	controls._onKeyDown( key( 'ShiftLeft' ) );
	controls._onKeyDown( key( 'KeyR' ) );
	controls._onKeyDown( key( 'ArrowUp' ) );
	controls._onKeyDown( key( 'ArrowLeft' ) );
	frames( controls, changes, c, steps, 5 );

	controls._onKeyUp( key( 'ShiftLeft' ) );
	controls._onKeyUp( key( 'KeyW' ) );
	controls._onKeyUp( key( 'KeyA' ) );
	controls._onKeyUp( key( 'KeyR' ) );
	controls._onKeyUp( key( 'ArrowUp' ) );
	controls._onKeyUp( key( 'ArrowLeft' ) );
	controls._onKeyDown( key( 'KeyS' ) );
	controls._onKeyDown( key( 'KeyD' ) );
	controls._onKeyDown( key( 'KeyF' ) );
	controls._onKeyDown( key( 'ArrowDown' ) );
	controls._onKeyDown( key( 'ArrowRight' ) );
	controls._onKeyDown( key( 'KeyQ' ) );
	frames( controls, changes, c, steps, 5 );

	controls._onKeyUp( key( 'KeyQ' ) );
	controls._onKeyDown( key( 'KeyE' ) );
	controls._onKeyDown( key( 'KeyZ' ) );
	frames( controls, changes, c, steps, 3 );

	// Everything up: nothing moves and `change` stops.
	for ( const code of [ 'KeyS', 'KeyD', 'KeyF', 'ArrowDown', 'ArrowRight', 'KeyE' ] ) controls._onKeyUp( key( code ) );
	controls._onKeyDown( key( 'KeyW', true ) ); // Alt held: ignored
	frames( controls, changes, c, steps, 3 );

	scenarios.keys = steps;

}

// 2. Pointer steering (no dragToLook): the pointer's offset from the centre
// yaws and pitches, the left button flies forward, the right back, and a
// cancel releases both.
{

	const c = camera();
	const { controls, changes } = controlsOn( c );
	controls.rollSpeed = 0.4;
	controls.movementSpeed = 2;
	const steps = [ record( c ) ];

	controls._onPointerMove( pointer( 0, 620, 130 ) );
	frames( controls, changes, c, steps, 4 );

	controls._onPointerDown( pointer( 0, 620, 130 ) );
	frames( controls, changes, c, steps, 4 );

	controls._onPointerMove( pointer( 0, 150, 420 ) );
	controls._onPointerDown( pointer( 2, 150, 420 ) );
	frames( controls, changes, c, steps, 4 );

	controls._onPointerUp( pointer( 0, 150, 420 ) );
	frames( controls, changes, c, steps, 4 );

	controls._onPointerCancel( pointer( 2, 150, 420 ) );
	controls._onPointerMove( pointer( 0, 320, 180 ) ); // dead centre: no turn
	frames( controls, changes, c, steps, 3 );

	scenarios.pointer_steer = steps;

}

// 3. dragToLook: the pointer only steers while a button is down, and up or
// cancel stop the turn; buttons never translate.
{

	const c = camera();
	const { controls, changes } = controlsOn( c );
	controls.dragToLook = true;
	controls.rollSpeed = 0.5;
	const steps = [ record( c ) ];

	controls._onPointerMove( pointer( 0, 700, 50 ) ); // not dragging: ignored
	frames( controls, changes, c, steps, 2 );

	controls._onPointerDown( pointer( 0, 700, 50 ) );
	controls._onPointerMove( pointer( 0, 90, 380 ) );
	frames( controls, changes, c, steps, 5 );

	controls._onPointerUp( pointer( 0, 90, 380 ) );
	frames( controls, changes, c, steps, 2 );

	controls._onPointerDown( pointer( 2, 500, 250 ) );
	controls._onPointerMove( pointer( 2, 500, 100 ) );
	frames( controls, changes, c, steps, 3 );

	controls._onPointerCancel( pointer( 2, 500, 100 ) );
	controls._onPointerMove( pointer( 2, 10, 10 ) ); // cancelled: ignored
	frames( controls, changes, c, steps, 2 );

	scenarios.drag_to_look = steps;

}

// 4. autoForward, which S overrides, with a large delta.
{

	const c = camera();
	const { controls, changes } = controlsOn( c );
	controls.autoForward = true;
	controls.movementSpeed = 3;
	const steps = [ record( c ) ];

	// autoForward only reaches the move vector through _updateMovementVector().
	controls._onKeyDown( key( 'KeyR' ) );
	controls._onKeyUp( key( 'KeyR' ) );
	frames( controls, changes, c, steps, 3, 0.25 );

	controls._onKeyDown( key( 'KeyS' ) );
	frames( controls, changes, c, steps, 3, 0.25 );

	controls._onKeyUp( key( 'KeyS' ) );
	frames( controls, changes, c, steps, 3, 0.25 );

	scenarios.auto_forward = steps;

}

// 5. Disabled: input is dropped and update() does nothing; re-enabled, the
// state from before still drives it. A tiny delta stays under _EPS.
{

	const c = camera();
	const { controls, changes } = controlsOn( c );
	const steps = [ record( c ) ];

	controls._onKeyDown( key( 'KeyD' ) );
	controls.enabled = false;
	controls._onKeyDown( key( 'KeyW' ) );
	controls._onKeyUp( key( 'KeyD' ) );
	controls._onPointerMove( pointer( 0, 0, 0 ) );
	frames( controls, changes, c, steps, 2 );

	controls.enabled = true;
	frames( controls, changes, c, steps, 2 );
	frames( controls, changes, c, steps, 3, 0.0002 );
	frames( controls, changes, c, steps, 1, 0.5 );

	scenarios.disabled = steps;

}

fs.writeFileSync( fixture, format( scenarios ) );
console.log( `wrote ${ path.relative( process.cwd(), fixture ) }` );
