// Runs three.js' own `OrbitControls` under node and prints what it does to a
// camera, so that `tests/addons_orbit_controls.rs` can assert the Rust port
// does the same thing to the same inputs.
//
//   node tools/orbit_controls_reference.mjs <three.js checkout>
//
// It prints one JSON object: `{ "<scenario>": [ step, step, … ] }`, where each
// step is the camera's position and quaternion, the controls' target, distance,
// polar and azimuthal angles, and what `update()` returned.
//
// # No DOM
//
// `OrbitControls` mixes DOM listeners into the class, and node has no DOM. Two
// things make it run anyway:
//
//   * `new OrbitControls( camera, null )` skips `connect()`, so no listener is
//     ever registered. `domElement` is then set to the stub below, because the
//     handlers read `clientWidth` / `clientHeight` / `getBoundingClientRect()`
//     off it — nothing else.
//   * the handlers are invoked through the bound `_onMouseDown`,
//     `_onMouseMove`, `_onMouseWheel` and `_onKeyDown` the constructor puts on
//     the instance, with plain objects carrying the fields they read. That is
//     the same seam the Rust port exposes as `pointer_down`, `pointer_move`,
//     `wheel` and `key`.
//
// `OrbitControls.js` imports from the bare specifier `'three'`, which node
// cannot resolve out of a checkout, so the file is copied to a temporary one
// with that specifier rewritten to the checkout's own ES build. Nothing in the
// checkout is modified.

import * as fs from 'fs';
import * as os from 'os';
import * as path from 'path';
import { pathToFileURL } from 'url';

const threeDir = process.argv[ 2 ];
if ( ! threeDir ) {

	console.error( 'usage: orbit_controls_reference.mjs <three.js checkout>' );
	process.exit( 2 );

}

const buildUrl = pathToFileURL( path.join( threeDir, 'build/three.module.js' ) ).href;
const THREE = await import( buildUrl );

const source = fs
	.readFileSync( path.join( threeDir, 'examples/jsm/controls/OrbitControls.js' ), 'utf8' )
	.replace( /from 'three'/g, `from '${ buildUrl }'` );

const temporary = path.join(
	fs.mkdtempSync( path.join( os.tmpdir(), 'three-rs-orbit-' ) ),
	'OrbitControls.js'
);
fs.writeFileSync( temporary, source );

const { OrbitControls } = await import( pathToFileURL( temporary ).href );

// The element, reduced to the three properties the handlers read.
const ELEMENT_WIDTH = 800;
const ELEMENT_HEIGHT = 500;

function element() {

	return {
		clientWidth: ELEMENT_WIDTH,
		clientHeight: ELEMENT_HEIGHT,
		getBoundingClientRect: () => ( {
			left: 0,
			top: 0,
			width: ELEMENT_WIDTH,
			height: ELEMENT_HEIGHT
		} ),
		style: {},
		setPointerCapture: () => {},
		releasePointerCapture: () => {},
		getRootNode: () => ( { addEventListener: () => {}, removeEventListener: () => {} } ),
		ownerDocument: { addEventListener: () => {}, removeEventListener: () => {} },
		addEventListener: () => {},
		removeEventListener: () => {}
	};

}

// The camera every scenario starts from: the shape of a graded page's, with a
// position that is on none of the axes so a wrong column or a swapped sine
// shows up immediately.
function camera() {

	const camera = new THREE.PerspectiveCamera( 45, ELEMENT_WIDTH / ELEMENT_HEIGHT, 0.25, 200 );
	camera.position.set( 3, 4, 5 );
	camera.updateMatrixWorld();
	return camera;

}

function controlsOn( camera ) {

	const controls = new OrbitControls( camera, null );
	controls.domElement = element();
	return controls;

}

function record( controls, camera, changed ) {

	return {
		position: camera.position.toArray(),
		quaternion: camera.quaternion.toArray(),
		target: controls.target.toArray(),
		distance: controls.getDistance(),
		polar: controls.getPolarAngle(),
		azimuth: controls.getAzimuthalAngle(),
		changed: changed === undefined ? null : changed
	};

}

function pointer( button, x, y, modifiers = {} ) {

	return {
		pointerId: 1,
		button,
		clientX: x,
		clientY: y,
		ctrlKey: false,
		metaKey: false,
		shiftKey: false,
		pointerType: 'mouse',
		preventDefault: () => {},
		...modifiers
	};

}

function wheel( x, y, deltaY, deltaMode = 0, ctrlKey = false ) {

	return {
		clientX: x,
		clientY: y,
		deltaY,
		deltaMode,
		ctrlKey,
		preventDefault: () => {}
	};

}

const scenarios = {};

// 1. A plain left drag: rotate, no damping.
{

	const c = camera();
	const controls = controlsOn( c );
	const steps = [ record( controls, c ) ];

	controls._onMouseDown( pointer( 0, 400, 250 ) );
	controls._onMouseMove( pointer( 0, 460, 220 ) );
	c.updateMatrixWorld();
	steps.push( record( controls, c ) );

	controls._onMouseMove( pointer( 0, 500, 300 ) );
	c.updateMatrixWorld();
	steps.push( record( controls, c ) );

	steps.push( record( controls, c, controls.update() ) );

	scenarios.rotate = steps;

}

// 2. The same drag with damping on, then several updates with no input: the
// spherical delta decays by ( 1 - dampingFactor ) a frame and the camera keeps
// coasting.
{

	const c = camera();
	const controls = controlsOn( c );
	controls.enableDamping = true;
	controls.dampingFactor = 0.1;
	const steps = [ record( controls, c ) ];

	controls._onMouseDown( pointer( 0, 400, 250 ) );
	controls._onMouseMove( pointer( 0, 470, 210 ) );
	c.updateMatrixWorld();
	steps.push( record( controls, c ) );

	for ( let i = 0; i < 6; i ++ ) {

		const changed = controls.update();
		c.updateMatrixWorld();
		steps.push( record( controls, c, changed ) );

	}

	scenarios.rotate_damped = steps;

}

// 3. The wheel, against both distance clamps.
{

	const c = camera();
	const controls = controlsOn( c );
	controls.minDistance = 4;
	controls.maxDistance = 9;
	const steps = [ record( controls, c ) ];

	// In, well past the near clamp.
	for ( let i = 0; i < 8; i ++ ) {

		controls._onMouseWheel( wheel( 400, 250, - 120 ) );
		c.updateMatrixWorld();
		steps.push( record( controls, c ) );

	}

	// Out, well past the far clamp.
	for ( let i = 0; i < 12; i ++ ) {

		controls._onMouseWheel( wheel( 400, 250, 120 ) );
		c.updateMatrixWorld();
		steps.push( record( controls, c ) );

	}

	// And the two other deltaModes, plus a pinch.
	controls._onMouseWheel( wheel( 400, 250, - 3, 1 ) );
	c.updateMatrixWorld();
	steps.push( record( controls, c ) );
	controls._onMouseWheel( wheel( 400, 250, - 1, 2 ) );
	c.updateMatrixWorld();
	steps.push( record( controls, c ) );
	controls._onMouseWheel( wheel( 400, 250, - 10, 0, true ) );
	c.updateMatrixWorld();
	steps.push( record( controls, c ) );

	scenarios.wheel_dolly = steps;

}

// 4. A right drag: pan in screen space (the default).
{

	const c = camera();
	const controls = controlsOn( c );
	const steps = [ record( controls, c ) ];

	controls._onMouseDown( pointer( 2, 400, 250 ) );
	controls._onMouseMove( pointer( 2, 480, 190 ) );
	c.updateMatrixWorld();
	steps.push( record( controls, c ) );

	controls._onMouseMove( pointer( 2, 420, 300 ) );
	c.updateMatrixWorld();
	steps.push( record( controls, c ) );

	scenarios.pan_screen_space = steps;

}

// 5. The same drag with `screenSpacePanning = false`: up is the cross of the
// camera's up with its X column, not its own Y column.
{

	const c = camera();
	const controls = controlsOn( c );
	controls.screenSpacePanning = false;
	const steps = [ record( controls, c ) ];

	controls._onMouseDown( pointer( 2, 400, 250 ) );
	controls._onMouseMove( pointer( 2, 480, 190 ) );
	c.updateMatrixWorld();
	steps.push( record( controls, c ) );

	controls._onMouseMove( pointer( 2, 420, 300 ) );
	c.updateMatrixWorld();
	steps.push( record( controls, c ) );

	scenarios.pan_world_space = steps;

}

// 6. Auto-rotate, with and without a delta: `webgpu_mesh_batch` and
// `webgpu_postprocessing_ca` both turn the camera this way.
{

	const c = camera();
	const controls = controlsOn( c );
	controls.autoRotate = true;
	controls.autoRotateSpeed = 1.0;
	const steps = [ record( controls, c ) ];

	for ( let i = 0; i < 4; i ++ ) {

		const changed = controls.update();
		c.updateMatrixWorld();
		steps.push( record( controls, c, changed ) );

	}

	for ( let i = 0; i < 4; i ++ ) {

		const changed = controls.update( 1 / 60 );
		c.updateMatrixWorld();
		steps.push( record( controls, c, changed ) );

	}

	// A negative speed, as `webgpu_postprocessing_ca` sets.
	controls.autoRotateSpeed = - 0.1;
	for ( let i = 0; i < 2; i ++ ) {

		const changed = controls.update( 1 / 60 );
		c.updateMatrixWorld();
		steps.push( record( controls, c, changed ) );

	}

	scenarios.auto_rotate = steps;

}

// 7. The polar clamp `webgpu_materials_cubemap_mipmaps` and
// `webgpu_postprocessing_bloom` set, dragged hard into both ends.
{

	const c = camera();
	const controls = controlsOn( c );
	controls.minPolarAngle = Math.PI / 4;
	controls.maxPolarAngle = Math.PI / 1.5;
	const steps = [ record( controls, c ) ];

	controls._onMouseDown( pointer( 0, 400, 250 ) );
	for ( let i = 0; i < 5; i ++ ) {

		controls._onMouseMove( pointer( 0, 400, 250 - ( i + 1 ) * 120 ) );
		c.updateMatrixWorld();
		steps.push( record( controls, c ) );

	}

	for ( let i = 0; i < 10; i ++ ) {

		controls._onMouseMove( pointer( 0, 400, - 600 + ( i + 1 ) * 120 ) );
		c.updateMatrixWorld();
		steps.push( record( controls, c ) );

	}

	scenarios.polar_clamp = steps;

}

// 8. The azimuth clamp, which no graded page sets but `update()` implements in
// two branches — a normal interval and one that wraps.
{

	const c = camera();
	const controls = controlsOn( c );
	controls.minAzimuthAngle = - 0.3;
	controls.maxAzimuthAngle = 0.9;
	const steps = [ record( controls, c ) ];

	controls._onMouseDown( pointer( 0, 400, 250 ) );
	for ( let i = 0; i < 8; i ++ ) {

		controls._onMouseMove( pointer( 0, 400 + ( i + 1 ) * 140, 250 ) );
		c.updateMatrixWorld();
		steps.push( record( controls, c ) );

	}

	scenarios.azimuth_clamp = steps;

}

// 9. The arrow keys: pan, and rotate with a modifier.
{

	const c = camera();
	const controls = controlsOn( c );
	const steps = [ record( controls, c ) ];

	for ( const code of [ 'ArrowLeft', 'ArrowUp', 'ArrowRight', 'ArrowDown' ] ) {

		controls._onKeyDown( { code, ctrlKey: false, metaKey: false, shiftKey: false, preventDefault: () => {} } );
		c.updateMatrixWorld();
		steps.push( record( controls, c ) );

	}

	for ( const code of [ 'ArrowLeft', 'ArrowUp', 'ArrowRight', 'ArrowDown' ] ) {

		controls._onKeyDown( { code, ctrlKey: false, metaKey: false, shiftKey: true, preventDefault: () => {} } );
		c.updateMatrixWorld();
		steps.push( record( controls, c ) );

	}

	scenarios.keys = steps;

}

// 10. A left drag with a modifier, which the default mouse bindings turn into
// a pan, and a middle-button drag, which they turn into a dolly.
{

	const c = camera();
	const controls = controlsOn( c );
	const steps = [ record( controls, c ) ];

	controls._onMouseDown( pointer( 0, 400, 250, { shiftKey: true } ) );
	controls._onMouseMove( pointer( 0, 470, 300, { shiftKey: true } ) );
	c.updateMatrixWorld();
	steps.push( record( controls, c ) );

	controls.state = - 1;
	controls._onMouseDown( pointer( 1, 400, 250 ) );
	controls._onMouseMove( pointer( 1, 400, 340 ) );
	c.updateMatrixWorld();
	steps.push( record( controls, c ) );
	controls._onMouseMove( pointer( 1, 400, 180 ) );
	c.updateMatrixWorld();
	steps.push( record( controls, c ) );

	scenarios.modified_buttons = steps;

}

// 11. Zoom to the cursor: the camera moves down the pointer ray rather than
// towards the target, and the target is re-placed in front of it.
{

	const c = camera();
	const controls = controlsOn( c );
	controls.zoomToCursor = true;
	const steps = [ record( controls, c ) ];

	for ( const [ x, y ] of [ [ 200, 120 ], [ 650, 400 ], [ 400, 250 ] ] ) {

		controls._onMouseWheel( wheel( x, y, - 120 ) );
		c.updateMatrixWorld();
		steps.push( record( controls, c ) );

	}

	scenarios.zoom_to_cursor = steps;

}

// 12. `saveState()` / `reset()`.
{

	const c = camera();
	const controls = controlsOn( c );
	const steps = [ record( controls, c ) ];

	controls.saveState();

	controls._onMouseDown( pointer( 0, 400, 250 ) );
	controls._onMouseMove( pointer( 0, 520, 160 ) );
	c.updateMatrixWorld();
	steps.push( record( controls, c ) );

	controls.reset();
	c.updateMatrixWorld();
	steps.push( record( controls, c ) );

	scenarios.save_and_reset = steps;

}

process.stdout.write( JSON.stringify( scenarios ) );
