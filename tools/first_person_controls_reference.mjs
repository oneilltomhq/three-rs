// Runs three.js' own `FirstPersonControls` under node and writes what it does
// to a camera into `tests/fixtures/first_person_controls.json`, which
// `tests/addons_first_person_controls.rs` replays the Rust port against.
//
//   node tools/first_person_controls_reference.mjs [three.js checkout] [output]
//
// The checkout defaults to $THREE_JS_DIR, then ~/src/vendor/three.js, as
// `three_rs::testing::three_js_dir()` does. The output defaults to the
// committed fixture; the Rust test passes a scratch path instead, to check
// the fixture is still what three.js does.
//
// The fixture is one JSON object, `{ "<scenario>": [ step, step, … ] }`,
// where each step is the camera's position and quaternion after one scripted
// action. The Rust test hard-codes the same scripts, so a change to one
// side's script is a change to both.
//
// # No DOM
//
// As in `tools/orbit_controls_reference.mjs`: the class is constructed with a
// null element, so `connect()` registers no listener, and `domElement` is then
// set to the stub below, because `onPointerDown` / `onPointerUp` call
// `focus()`, `setPointerCapture()` and `releasePointerCapture()` on it. The
// handlers are invoked through the bound `_onPointerDown`, `_onPointerMove`,
// `_onPointerUp`, `_onKeyDown` and `_onKeyUp` the constructor puts on the
// instance, with plain objects carrying the fields they read. `document` is
// a global stub only so that `this.domElement !== document` can be evaluated.
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
const fixture = process.argv[ 3 ] || path.join( here, '../tests/fixtures/first_person_controls.json' );

const buildUrl = pathToFileURL( path.join( threeDir, 'build/three.module.js' ) ).href;
const THREE = await import( buildUrl );

const source = fs
	.readFileSync( path.join( threeDir, 'examples/jsm/controls/FirstPersonControls.js' ), 'utf8' )
	.replace( /from 'three'/g, `from '${ buildUrl }'` );

const temporary = path.join(
	fs.mkdtempSync( path.join( os.tmpdir(), 'three-rs-first-person-' ) ),
	'FirstPersonControls.js'
);
fs.writeFileSync( temporary, source );

const { FirstPersonControls } = await import( pathToFileURL( temporary ).href );

globalThis.document = { isStubDocument: true };

function element() {

	return {
		style: {},
		focus: () => {},
		setPointerCapture: () => {},
		releasePointerCapture: () => {},
		ownerDocument: { addEventListener: () => {}, removeEventListener: () => {} },
		addEventListener: () => {},
		removeEventListener: () => {}
	};

}

// A camera off every axis, looking at a point off every axis, so a swapped
// sine or a wrong sign shows up immediately.
function camera() {

	const camera = new THREE.PerspectiveCamera( 45, 800 / 500, 0.25, 200 );
	camera.position.set( 3, 4, 5 );
	camera.lookAt( - 1, 1.5, 0.5 );
	camera.updateMatrixWorld();
	return camera;

}

function controlsOn( camera ) {

	const controls = new FirstPersonControls( camera, null );
	controls.domElement = element();
	return controls;

}

function record( camera ) {

	return {
		position: camera.position.toArray(),
		quaternion: camera.quaternion.toArray()
	};

}

const DT = 1 / 60;

function pointer( button, x, y, pointerType = 'mouse' ) {

	return { pointerId: 1, button, pageX: x, pageY: y, pointerType };

}

const key = ( code ) => ( { code } );

// Runs `update( delta )` `n` times, recording after each.
function frames( controls, c, steps, n, delta = DT ) {

	for ( let i = 0; i < n; i ++ ) {

		controls.update( delta );
		steps.push( record( c ) );

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

// 1. Keys: forward, a diagonal (normalised), up and back, then release and
// let the velocity ease out.
{

	const c = camera();
	const controls = controlsOn( c );
	controls.movementSpeed = 3;
	const steps = [ record( c ) ];

	controls._onKeyDown( key( 'KeyW' ) );
	frames( controls, c, steps, 6 );

	controls._onKeyDown( key( 'KeyD' ) );
	frames( controls, c, steps, 6 );

	controls._onKeyUp( key( 'KeyW' ) );
	controls._onKeyUp( key( 'KeyD' ) );
	controls._onKeyDown( key( 'KeyR' ) );
	controls._onKeyDown( key( 'ArrowDown' ) );
	controls._onKeyDown( key( 'ArrowLeft' ) );
	frames( controls, c, steps, 6 );

	controls._onKeyUp( key( 'KeyR' ) );
	controls._onKeyUp( key( 'ArrowDown' ) );
	controls._onKeyUp( key( 'ArrowLeft' ) );
	controls._onKeyDown( key( 'KeyF' ) );
	controls._onKeyDown( key( 'KeyQ' ) ); // not bound: ignored
	frames( controls, c, steps, 3 );

	controls._onKeyUp( key( 'KeyF' ) );
	frames( controls, c, steps, 8 );

	scenarios.keys = steps;

}

// 2. A left drag: moves along the look direction and turns, then the button
// is released and both velocities ease to a stop.
{

	const c = camera();
	const controls = controlsOn( c );
	controls.lookSpeed = 0.1;
	const steps = [ record( c ) ];

	controls._onPointerDown( pointer( 0, 400, 250 ) );
	frames( controls, c, steps, 2 );

	controls._onPointerMove( pointer( 0, 460, 220 ) );
	frames( controls, c, steps, 8 );

	controls._onPointerMove( pointer( 0, 300, 330 ) );
	frames( controls, c, steps, 8 );

	controls._onPointerUp( pointer( 0, 300, 330 ) );
	controls._onPointerMove( pointer( 0, 900, 900 ) ); // no drag: ignored
	frames( controls, c, steps, 8 );

	scenarios.mouse_look = steps;

}

// 3. A right drag (backwards) with the vertical look constrained, dragged far
// enough that the latitude hits its ±85° clamp; a middle drag only looks.
{

	const c = camera();
	const controls = controlsOn( c );
	controls.lookSpeed = 0.5;
	controls.constrainVertical = true;
	controls.verticalMin = 1.0;
	controls.verticalMax = 2.0;
	const steps = [ record( c ) ];

	controls._onPointerDown( pointer( 2, 400, 250 ) );
	controls._onPointerMove( pointer( 2, 350, - 400 ) );
	frames( controls, c, steps, 12, 0.05 );

	controls._onPointerMove( pointer( 2, 450, 900 ) );
	frames( controls, c, steps, 12, 0.05 );

	controls._onPointerUp( pointer( 2, 450, 900 ) );
	controls._onPointerDown( pointer( 1, 100, 100 ) );
	controls._onPointerMove( pointer( 1, 140, 60 ) );
	frames( controls, c, steps, 6, 0.05 );
	controls._onPointerUp( pointer( 1, 140, 60 ) );
	frames( controls, c, steps, 3, 0.05 );

	scenarios.constrain_vertical = steps;

}

// 4. autoForward with heightSpeed and no vertical look; a forward key held
// while clicking means the click only looks; a heavier damping factor.
{

	const c = camera();
	const controls = controlsOn( c );
	controls.autoForward = true;
	controls.heightSpeed = true;
	controls.heightCoef = 2;
	controls.heightMin = 1;
	controls.heightMax = 10;
	controls.movementSpeed = 5;
	controls.dampingFactor = 0.3;
	controls.lookVertical = false;
	controls.lookSpeed = 0.2;
	const steps = [ record( c ) ];

	frames( controls, c, steps, 6 );

	controls._onKeyDown( key( 'KeyS' ) );
	frames( controls, c, steps, 4 );

	controls._onPointerDown( pointer( 0, 400, 250 ) ); // key held: only looks
	controls._onPointerMove( pointer( 0, 330, 300 ) );
	frames( controls, c, steps, 6 );

	controls._onKeyUp( key( 'KeyS' ) );
	frames( controls, c, steps, 4 );

	controls._onPointerUp( pointer( 0, 330, 300 ) );
	frames( controls, c, steps, 4 );

	scenarios.auto_forward_height = steps;

}

// 5. Touch: one finger forward, a second backward, lifting one back to
// forward.
{

	const c = camera();
	const controls = controlsOn( c );
	controls.lookSpeed = 0.05;
	const steps = [ record( c ) ];

	controls._onPointerDown( pointer( 0, 200, 200, 'touch' ) );
	controls._onPointerMove( pointer( 0, 260, 180, 'touch' ) );
	frames( controls, c, steps, 5 );

	controls._onPointerDown( pointer( 0, 500, 300, 'touch' ) );
	controls._onPointerMove( pointer( 0, 470, 330, 'touch' ) );
	frames( controls, c, steps, 5 );

	controls._onPointerUp( pointer( 0, 470, 330, 'touch' ) );
	frames( controls, c, steps, 5 );

	controls._onPointerUp( pointer( 0, 470, 330, 'touch' ) );
	frames( controls, c, steps, 5 );

	scenarios.touch = steps;

}

// 6. lookAt( Vector3 ) and lookAt( x, y, z ) re-seed the orientation, and a
// disabled update does nothing.
{

	const c = camera();
	const controls = controlsOn( c );
	const steps = [ record( c ) ];

	controls.lookAt( new THREE.Vector3( 7, - 2, 1 ) );
	steps.push( record( c ) );
	frames( controls, c, steps, 2 );

	controls._onKeyDown( key( 'KeyW' ) );
	frames( controls, c, steps, 3 );

	controls.lookAt( - 4, 6, - 3 );
	steps.push( record( c ) );
	frames( controls, c, steps, 3 );

	controls.enabled = false;
	frames( controls, c, steps, 2 );

	controls.enabled = true;
	frames( controls, c, steps, 2 );

	scenarios.look_at = steps;

}

// 7. A camera inside a moved and turned group: lookAt() works in world space
// (it undoes the parent's rotation), while update() adds the spherical
// direction to the *local* position. That mix is three's, and is kept.
{

	const group = new THREE.Object3D();
	group.position.set( 1, 2, - 1 );
	group.quaternion.set( 0.1, - 0.25, 0.08, 0.96 ).normalize();
	const c = camera();
	group.add( c );
	group.updateMatrixWorld();
	const controls = controlsOn( c );
	controls.lookSpeed = 0.1;
	const steps = [ record( c ) ];

	controls.lookAt( 0, 0, 0 );
	steps.push( record( c ) );

	controls._onKeyDown( key( 'KeyA' ) );
	controls._onPointerDown( pointer( 0, 400, 250 ) );
	controls._onPointerMove( pointer( 0, 380, 300 ) );
	frames( controls, c, steps, 8 );

	scenarios.parented = steps;

}

fs.writeFileSync( fixture, format( scenarios ) );
console.log( `wrote ${ path.relative( process.cwd(), fixture ) }` );
