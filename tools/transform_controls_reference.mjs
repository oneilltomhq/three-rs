// Runs three.js' own `TransformControls` under node and writes what it does
// into `tests/fixtures/transform_controls.json`, which
// `tests/addons_transform_controls.rs` replays the Rust port against.
//
//   node tools/transform_controls_reference.mjs [three.js checkout] [output]
//
// The checkout defaults to $THREE_JS_DIR, then ~/src/vendor/three.js, as
// `three_rs::testing::three_js_dir()` does. The output defaults to the
// committed fixture; the Rust test passes a scratch path instead, to check
// the fixture is still what three.js does.
//
// # What the fixture holds
//
// - `graph`: every handle of the gizmo, picker and helper groups and the
//   plane, in scene-graph order: its group, type, name, tag, renderOrder,
//   material settings and a fingerprint of its baked geometry.
// - `scenarios`: each one's setup (camera, element size, viewport, the
//   object's transform and parents) and its steps. A step is an action and
//   what followed it: the events dispatched, the object's transform, `axis`,
//   `mode`, `dragging`, `rotationAngle`, which handles of the current mode
//   are visible and highlighted, and the plane's orientation. Steps marked
//   `full` also carry every handle's transform, colour and opacity and the
//   controls' working vectors.
//
// The actions are in the fixture, so the Rust test is an interpreter of them
// and the scripts live here only. Pointer positions are computed here, by
// projecting a point inside a picker handle to the element, so a drag starts
// on the handle it names; the test only replays the pixels.
//
// # No DOM
//
// The class is constructed with a null element, so `connect()` registers no
// listener, and `domElement` is then set to a stub with
// `getBoundingClientRect()`, pointer capture and an `ownerDocument` whose
// `pointerLockElement` a scenario can set. The handlers are invoked through
// the bound `_onPointerDown`, `_onPointerHover`, `_onPointerMove` and
// `_onPointerUp`, with `_onPointerMove` only between a down and an up, as the
// listener `onPointerDown` adds and `onPointerUp` removes would be.
//
// Rendering is `scene.updateMatrixWorld()`, which is where three's
// `updateMatrixWorld` overrides on the root, gizmo and plane run.
//
// Each scenario imports its own copy of the module, so the module-level
// `_raycaster` and `_dirVector` start fresh, as the port's per-instance ones
// do.
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
const fixture = process.argv[ 3 ] || path.join( here, '../tests/fixtures/transform_controls.json' );

const buildUrl = pathToFileURL( path.join( threeDir, 'build/three.module.js' ) ).href;
const THREE = await import( buildUrl );

// The fixture is three.js at the pinned commit (5f610f5); another revision's
// output is not a fixture.
if ( THREE.REVISION !== '187dev' ) throw new Error( `expected three.js r187dev, ${ threeDir } is r${ THREE.REVISION }` );

const source = fs
	.readFileSync( path.join( threeDir, 'examples/jsm/controls/TransformControls.js' ), 'utf8' )
	.replace( /from 'three'/g, `from '${ buildUrl }'` );

const temporary = fs.mkdtempSync( path.join( os.tmpdir(), 'three-rs-transform-' ) );
const moduleUrl = pathToFileURL( path.join( temporary, 'TransformControls.js' ) ).href;
fs.writeFileSync( path.join( temporary, 'TransformControls.js' ), source );
process.on( 'exit', () => fs.rmSync( temporary, { recursive: true, force: true } ) );

const MODES = [ 'translate', 'rotate', 'scale' ];
const GROUPS = [ 'gizmo', 'picker', 'helper' ];
const DEG = Math.PI / 180;

const v3 = ( v ) => [ v.x, v.y, v.z ];
const q4 = ( q ) => [ q.x, q.y, q.z, q.w ];

// The gizmo, picker and helper groups and the plane: what the port builds.
function graph( controls ) {

	const rows = [];
	const material = ( m ) => ( {
		color: [ m.color.r, m.color.g, m.color.b ],
		opacity: m.opacity,
		transparent: m.transparent,
		depthTest: m.depthTest,
		depthWrite: m.depthWrite,
		fog: m.fog,
		side: m.side,
		visible: m.visible,
		wireframe: m.wireframe === true
	} );
	const geometry = ( g ) => {

		const p = g.attributes.position.array;
		let x = 0, y = 0, z = 0, w = 0;
		for ( let i = 0; i < p.length / 3; i ++ ) {

			x += p[ 3 * i ];
			y += p[ 3 * i + 1 ];
			z += p[ 3 * i + 2 ];
			w += ( i + 1 ) * ( p[ 3 * i ] + 2 * p[ 3 * i + 1 ] + 3 * p[ 3 * i + 2 ] );

		}

		return { vertices: p.length / 3, indices: g.index ? g.index.count : 0, sums: [ x, y, z, w ] };

	};

	const gizmo = controls._gizmo;
	const root = controls.getHelper();
	rows.push( { group: 'root', type: root.type, children: root.children.map( ( c ) => c.type ), visible: root.visible } );
	rows.push( { group: 'gizmo', type: gizmo.type, children: gizmo.children.length } );
	for ( const kind of GROUPS ) {

		for ( const mode of MODES ) {

			const group = gizmo[ kind ][ mode ];
			if ( gizmo.children.indexOf( group ) !== 3 * GROUPS.indexOf( kind ) + MODES.indexOf( mode ) ) throw new Error( 'group order' );
			rows.push( { group: `${ kind }.${ mode }`, type: group.type, visible: group.visible, children: group.children.length } );
			for ( const handle of group.children ) {

				rows.push( {
					group: `${ kind }.${ mode }`,
					type: handle.type,
					name: handle.name,
					tag: handle.tag === undefined ? null : handle.tag,
					renderOrder: String( handle.renderOrder ),
					transform: [ ...v3( handle.position ), ...q4( handle.quaternion ), ...v3( handle.scale ) ],
					material: material( handle.material ),
					geometry: geometry( handle.geometry )
				} );

			}

		}

	}

	const plane = controls._plane;
	rows.push( { group: 'plane', type: plane.type, name: plane.name, material: material( plane.material ), geometry: geometry( plane.geometry ) } );
	return rows;

}

let count = 0;

async function scenario( name, setup, body ) {

	const { TransformControls } = await import( `${ moduleUrl }?s=${ count ++ }` );

	const [ width, height ] = setup.element || [ 800, 600 ];
	const doc = { pointerLockElement: null };
	globalThis.document = doc;
	const element = {
		style: {},
		ownerDocument: doc,
		getBoundingClientRect: () => ( { left: 0, top: 0, width, height } ),
		setPointerCapture: () => {},
		releasePointerCapture: () => {},
		addEventListener: () => {},
		removeEventListener: () => {}
	};

	const cam = setup.camera || {};
	let camera;
	if ( cam.type === 'orthographic' ) {

		const a = width / height;
		camera = new THREE.OrthographicCamera( - 4 * a, 4 * a, 4, - 4, 0.1, 100 );
		camera.zoom = cam.zoom || 1;

	} else {

		camera = new THREE.PerspectiveCamera( cam.fov || 50, width / height, 0.1, 100 );
		camera.zoom = cam.zoom || 1;

	}

	camera.coordinateSystem = THREE.WebGPUCoordinateSystem;
	camera.updateProjectionMatrix();
	camera.position.fromArray( cam.position || [ 3, 4, 5 ] );
	camera.lookAt( new THREE.Vector3().fromArray( cam.target || [ 0.4, 0.2, - 0.3 ] ) );
	camera.updateMatrixWorld();

	const scene = new THREE.Scene();
	const transform = ( o, t ) => {

		if ( ! t ) return;
		o.position.fromArray( t.p );
		o.quaternion.fromArray( t.q ).normalize();
		o.scale.fromArray( t.s );

	};

	const object = new THREE.Object3D();
	transform( object, setup.object || { p: [ 0.4, 0.2, - 0.3 ], q: [ 0, 0, 0, 1 ], s: [ 1, 1, 1 ] } );
	let objectParent = null;
	if ( setup.objectParent ) {

		objectParent = new THREE.Object3D();
		transform( objectParent, setup.objectParent );
		objectParent.add( object );
		scene.add( objectParent );

	} else {

		scene.add( object );

	}

	const controls = new TransformControls( camera );
	controls.domElement = element;
	const helper = controls.getHelper();
	if ( setup.helperParent === 'objectParent' ) {

		objectParent.add( helper );

	} else if ( setup.helperParent ) {

		const group = new THREE.Object3D();
		transform( group, setup.helperParent );
		group.add( helper );
		scene.add( group );

	} else {

		scene.add( helper );

	}

	let events = [];
	const dispatch = controls.dispatchEvent.bind( controls );
	controls.dispatchEvent = ( event ) => {

		events.push( event.mode !== undefined ? `${ event.type }:${ event.mode }` : event.type );
		dispatch( event );

	};

	controls.attach( object );
	events = [];
	scene.updateMatrixWorld();

	let listening = false;
	const steps = [];

	const region = () => {

		const vp = controls.viewport;
		return vp === null ? [ 0, 0, width, height ] : [ vp.x, height - vp.y - vp.w, vp.z, vp.w ];

	};

	const record = ( action, options = {} ) => {

		if ( options.render !== false ) scene.updateMatrixWorld();
		const mode = controls.mode;
		const handles = [ 'picker', 'gizmo', 'helper' ].flatMap( ( kind ) => controls._gizmo[ kind ][ mode ].children );
		const active = controls._gizmo.materialLib.active.color;
		const step = {
			do: action,
			events: options.fieldSet ? null : events,
			p: v3( object.position ),
			q: q4( object.quaternion ),
			s: v3( object.scale ),
			axis: controls.axis,
			mode,
			dragging: controls.dragging,
			angle: controls.rotationAngle,
			visible: handles.map( ( h ) => ( h.visible ? '1' : '0' ) ).join( '' ),
			highlight: controls._gizmo.gizmo[ mode ].children.map( ( h ) => ( h.material.opacity === 1 && h.material.color.equals( active ) ? '1' : '0' ) ).join( '' ),
			rootVisible: helper.visible,
			// The plane's position is always `worldPosition`.
			plane: q4( controls._plane.quaternion )
		};
		if ( options.render === false ) step.render = false;
		if ( options.full ) {

			step.full = {
				handles: handles.map( ( h ) => [
					h.name, h.visible, ...v3( h.position ), ...q4( h.quaternion ), ...v3( h.scale ),
					// Three's pickers share one material, so their colour is
					// whichever picker wrote it last; it is not compared.
					...( controls._gizmo.picker[ mode ].children.includes( h ) ? [] : [ h.material.color.r, h.material.color.g, h.material.color.b, h.material.opacity ] )
				] ),
				root: helper.matrixWorld.elements.slice(),
				plane: controls._plane.matrixWorld.elements.slice(),
				worldPosition: v3( controls.worldPosition ),
				worldQuaternion: q4( controls.worldQuaternion ),
				eye: v3( controls.eye ),
				pointStart: v3( controls.pointStart ),
				pointEnd: v3( controls.pointEnd ),
				rotationAxis: v3( controls.rotationAxis )
			};

		}

		steps.push( step );
		events = [];

	};

	const event = ( x, y, o ) => ( { clientX: x, clientY: y, button: o.button, pointerType: o.type || 'mouse', pointerId: 1 } );

	const t = {
		controls,
		steps,
		// Element pixels of the first of `candidates`, points in the space
		// of the picker handle called `name`, at which a hover would pick
		// that handle; so a scripted drag starts on the handle it names.
		at( name, ...candidates ) {

			const pickers = controls._gizmo.picker[ controls.mode ].children;
			const [ ox, oy, w, h ] = region();
			const round = ( v ) => Math.round( v * 100 ) / 100;
			for ( const handle of pickers.filter( ( p ) => p.name === name ) ) {

				for ( const local of candidates ) {

					const p = handle.localToWorld( new THREE.Vector3().fromArray( local ) ).project( camera );
					const pixel = [ round( ox + ( p.x + 1 ) / 2 * w ), round( oy + ( 1 - p.y ) / 2 * h ) ];
					const ndc = doc.pointerLockElement ? new THREE.Vector2() : new THREE.Vector2( ( pixel[ 0 ] - ox ) / w * 2 - 1, - ( pixel[ 1 ] - oy ) / h * 2 + 1 );
					const raycaster = new THREE.Raycaster();
					raycaster.setFromCamera( ndc, camera );
					const hit = raycaster.intersectObject( controls._gizmo.picker[ controls.mode ], true ).find( ( i ) => i.object.visible );
					if ( hit && hit.object.name === name ) return pixel;

				}

			}

			throw new Error( `${ name }: no candidate point picks it` );

		},
		down( [ x, y ], o = {} ) {

			o = { button: 0, ...o };
			if ( controls.enabled ) listening = true;
			controls._onPointerDown( event( x, y, o ) );
			record( { op: 'down', x, y, button: o.button, pointerType: o.type || 'mouse' }, o );

		},
		move( [ x, y ], o = {} ) {

			o = { button: - 1, ...o };
			const e = event( x, y, o );
			controls._onPointerHover( e );
			if ( listening ) controls._onPointerMove( e );
			record( { op: 'move', x, y, button: o.button, pointerType: o.type || 'mouse' }, o );

		},
		up( [ x, y ], o = {} ) {

			o = { button: 0, ...o };
			if ( controls.enabled ) listening = false;
			controls._onPointerUp( event( x, y, o ) );
			record( { op: 'up', x, y, button: o.button, pointerType: o.type || 'mouse' }, o );

		},
		// A drag from `start` through each pixel offset, then the up.
		drag( start, offsets, o = {} ) {

			t.move( start, o );
			t.down( start, o );
			let last = start;
			for ( const [ dx, dy, more ] of offsets ) {

				last = [ start[ 0 ] + dx, start[ 1 ] + dy ];
				t.move( last, { ...o, ...more } );

			}

			t.up( last, o );

		},
		api( name, pointer, o = {} ) {

			const method = { hover: 'pointerHover', down: 'pointerDown', move: 'pointerMove', up: 'pointerUp' }[ name ];
			controls[ method ]( pointer );
			record( { op: 'api', name, pointer }, o );

		},
		set( key, value, o = {} ) {

			const setters = { mode: 'setMode', space: 'setSpace', size: 'setSize', translationSnap: 'setTranslationSnap', rotationSnap: 'setRotationSnap', scaleSnap: 'setScaleSnap' };
			let fieldSet = false;
			if ( setters[ key ] ) {

				controls[ setters[ key ] ]( value );

			} else if ( key === 'viewport' ) {

				controls.viewport = value === null ? null : new THREE.Vector4().fromArray( value );
				fieldSet = true;

			} else if ( key === 'pointerLocked' ) {

				doc.pointerLockElement = value ? element : null;
				fieldSet = true;

			} else {

				// `enabled`, `showX` … `showE`, `minX` … `maxZ`: plain
				// fields in the port, whose assignment sends nothing.
				controls[ key ] = value;
				fieldSet = true;

			}

			record( { op: 'set', key, value }, { ...o, fieldSet } );

		},
		call( name, o = {} ) {

			if ( name === 'attach' ) controls.attach( object );
			else controls[ name ]();
			record( { op: 'call', name }, o );

		},
		colors( x, y, z, active, o = {} ) {

			controls.setColors( x, y, z, active );
			record( { op: 'colors', colors: [ x, y, z, active ] }, o );

		},
		render( o = {} ) {

			record( { op: 'render' }, o );

		}
	};

	await body( t );

	const out = {
		name,
		setup: {
			element: [ width, height ],
			camera: {
				type: camera.isOrthographicCamera ? 'orthographic' : 'perspective',
				fov: camera.fov,
				left: camera.left,
				right: camera.right,
				top: camera.top,
				bottom: camera.bottom,
				near: camera.near,
				far: camera.far,
				zoom: camera.zoom,
				position: v3( camera.position ),
				quaternion: q4( camera.quaternion )
			},
			object: setup.object || { p: [ 0.4, 0.2, - 0.3 ], q: [ 0, 0, 0, 1 ], s: [ 1, 1, 1 ] },
			objectParent: setup.objectParent || null,
			helperParent: setup.helperParent || null
		},
		steps
	};
	return out;

}

// Points around a ring picker, in its own space, off the other rings.
const around = ( r, plane ) => [ 20, 50, 70, 110, 130, 160, 200, 230, 250, 290, 310, 340 ].map( ( a ) => {

	const c = r * Math.cos( a * DEG ), s = r * Math.sin( a * DEG );
	return plane === 'yz' ? [ 0, c, s ] : plane === 'xz' ? [ c, 0, s ] : [ c, s, 0 ];

} );
const ringX = around( 0.5, 'yz' );
const ringY = around( 0.5, 'xz' );
const ringZ = around( 0.5, 'xy' );
const ringE = around( 0.75, 'xy' );
const plane = ( a, b ) => [ [ 0.15, 0.15 ], [ 0.2, 0.2 ], [ 0.22, 0.12 ], [ 0.12, 0.22 ], [ 0.23, 0.23 ] ].map( ( [ u, v ] ) => {

	const p = [ 0, 0, 0 ];
	p[ a ] = u;
	p[ b ] = v;
	return p;

} );
const XY = plane( 0, 1 ), YZ = plane( 1, 2 ), XZ = plane( 0, 2 );

const turned = { p: [ 0.4, 0.2, - 0.3 ], q: [ 0.2, - 0.3, 0.1, 0.9 ], s: [ 1, 1, 1 ] };
const parent = { p: [ - 0.5, 0.3, 0.2 ], q: [ - 0.1, 0.25, 0.15, 0.95 ], s: [ 1.5, 0.8, 1.2 ] };

const scenarios = [];

// Translate along each axis and in each plane, and from the centre.
scenarios.push( await scenario( 'translate_world', {}, ( t ) => {

	t.render();
	t.drag( t.at( 'X', [ 0.45, 0, 0 ] ), [ [ 20, 5 ], [ 60, - 10, { full: true } ] ] );
	t.drag( t.at( 'X', [ 0.45, 0, 0 ], [ 0.35, 0, 0 ] ), [ [ - 30, 8 ] ] );
	t.drag( t.at( 'Y', [ 0, 0.45, 0 ] ), [ [ 5, - 40 ] ] );
	t.drag( t.at( 'Z', [ 0, 0, 0.45 ] ), [ [ - 25, 30 ] ] );
	t.drag( t.at( 'XY', ...XY ), [ [ 15, - 12 ], [ 40, 25 ] ] );
	t.drag( t.at( 'YZ', ...YZ ), [ [ - 20, - 15 ] ] );
	// Mid-drag, a ray above the horizon: the plane is horizontal and finite,
	// so pointerMove finds no hit and returns before any event or movement.
	const xz = t.at( 'XZ', ...XZ );
	t.move( xz );
	t.down( xz );
	t.move( [ xz[ 0 ] + 35, xz[ 1 ] + 20 ] );
	const above = new THREE.Raycaster();
	above.setFromCamera( new THREE.Vector2( 0, 5 ), t.controls.camera );
	if ( above.intersectObject( t.controls._plane, true ).length !== 0 ) throw new Error( 'the ray above the horizon hits the plane' );
	const before = v3( t.controls.object.position );
	t.api( 'move', { x: 0, y: 5, button: - 1 } );
	if ( t.steps.at( - 1 ).events.length !== 0 || v3( t.controls.object.position ).some( ( v, i ) => v !== before[ i ] ) ) throw new Error( 'a missed plane moved the object or sent events' );
	t.move( [ xz[ 0 ] + 45, xz[ 1 ] + 28 ] );
	t.up( [ xz[ 0 ] + 45, xz[ 1 ] + 28 ] );
	t.drag( t.at( 'XYZ', [ 0, 0, 0 ] ), [ [ - 25, - 30 ] ] );

} ) );

// Local space with a turned object; the centre handle still moves in world
// space.
scenarios.push( await scenario( 'translate_local', { object: turned }, ( t ) => {

	t.set( 'space', 'local' );
	t.render();
	t.drag( t.at( 'X', [ 0.45, 0, 0 ] ), [ [ 30, 10 ], [ 55, - 20 ] ] );
	t.drag( t.at( 'Z', [ 0, 0, 0.45 ] ), [ [ - 20, 25 ] ] );
	t.drag( t.at( 'YZ', ...YZ ), [ [ 20, - 18, { full: true } ] ] );
	t.drag( t.at( 'XZ', ...XZ ), [ [ - 15, 22 ] ] );
	t.drag( t.at( 'XYZ', [ 0, 0, 0 ] ), [ [ 18, 14 ] ] );

} ) );

// World snapping under a moved, turned and scaled parent, which goes
// through getWorldPosition() and the parent's worldToLocal(). The helper is
// not under the object's parent here.
scenarios.push( await scenario( 'translate_snap_world', { objectParent: parent }, ( t ) => {

	t.set( 'translationSnap', 0.5 );
	t.drag( t.at( 'X', [ 0.45, 0, 0 ] ), [ [ 25, 3 ], [ 70, - 5 ] ] );
	t.drag( t.at( 'XY', ...XY ), [ [ - 40, - 35 ] ] );
	t.drag( t.at( 'XYZ', [ 0, 0, 0 ] ), [ [ 30, 30, { full: true } ] ] );
	// Zero is no snap.
	t.set( 'translationSnap', 0 );
	t.drag( t.at( 'Y', [ 0, 0.45, 0 ] ), [ [ 2, - 33 ] ] );
	t.set( 'translationSnap', null );

} ) );

// Local snapping, and the helper inside the object's parent, so that
// pointerDown's `object.parent.updateMatrixWorld()` runs the helper's
// overrides.
scenarios.push( await scenario( 'translate_snap_local', { object: turned, objectParent: parent, helperParent: 'objectParent' }, ( t ) => {

	t.set( 'space', 'local' );
	t.set( 'translationSnap', 0.25 );
	t.render();
	t.drag( t.at( 'Y', [ 0, 0.45, 0 ] ), [ [ 6, - 45 ], [ 10, - 80 ] ] );
	t.drag( t.at( 'YZ', ...YZ ), [ [ 25, 30, { full: true } ] ] );

} ) );

// minX … maxZ clamp the position.
scenarios.push( await scenario( 'translate_clamp', {}, ( t ) => {

	t.set( 'minX', 0.2 );
	t.set( 'maxX', 0.6 );
	t.set( 'maxY', 0.3 );
	t.set( 'minZ', - 0.4 );
	t.drag( t.at( 'XYZ', [ 0, 0, 0 ] ), [ [ 80, - 60 ], [ - 90, 70 ] ] );
	t.drag( t.at( 'X', [ 0.45, 0, 0 ] ), [ [ 200, 0 ] ] );

} ) );

// Rotate in world space about each ring, the eye ring and the trackball,
// then with a snap.
scenarios.push( await scenario( 'rotate_world', { object: turned }, ( t ) => {

	t.set( 'mode', 'rotate' );
	t.render();
	t.drag( t.at( 'X', ...ringX ), [ [ 20, 10 ], [ 45, 30, { full: true } ] ] );
	t.drag( t.at( 'Y', ...ringY ), [ [ - 25, 12 ] ] );
	t.drag( t.at( 'Z', ...ringZ ), [ [ 10, - 30 ] ] );
	t.drag( t.at( 'E', ...ringE ), [ [ 15, 40 ], [ - 30, 60 ] ] );
	t.drag( t.at( 'XYZE', [ 0, 0, 0 ] ), [ [ 25, - 15, { full: true } ], [ 40, 5 ] ] );
	t.set( 'rotationSnap', 15 * DEG );
	t.drag( t.at( 'Z', ...ringZ ), [ [ 30, - 40 ] ] );
	t.drag( t.at( 'E', ...ringE ), [ [ 20, 35 ] ] );

} ) );

// Rotate in local space under a parent: the rings follow the object, and E
// and the trackball still rotate in world space through the parent's
// inverse.
scenarios.push( await scenario( 'rotate_local', { object: turned, objectParent: parent }, ( t ) => {

	t.set( 'mode', 'rotate' );
	t.set( 'space', 'local' );
	t.render();
	t.drag( t.at( 'X', ...ringX ), [ [ 25, 15 ] ] );
	t.drag( t.at( 'Y', ...ringY ), [ [ - 20, - 10, { full: true } ] ] );
	t.drag( t.at( 'E', ...ringE ), [ [ 25, - 30 ] ] );
	t.drag( t.at( 'XYZE', [ 0, 0, 0 ] ), [ [ - 20, 25 ] ] );
	t.set( 'space', 'world' );
	t.drag( t.at( 'Z', ...ringZ ), [ [ 15, 20 ] ] );

} ) );

// An axis left over from translate: `XY` hovered there, then setMode('rotate')
// and a drag. No rotate branch matches `XY`, so the drag applies whatever
// rotationAxis and rotationAngle the last rotate drag left, and in world space
// turns rotationAxis by the parent's inverse again on every move.
scenarios.push( await scenario( 'rotate_stale_axis', { object: turned, objectParent: parent }, ( t ) => {

	t.set( 'mode', 'rotate' );
	t.render();
	t.drag( t.at( 'Y', ...ringY ), [ [ - 25, 12 ] ] );
	t.set( 'mode', 'translate' );
	t.move( t.at( 'XY', ...XY ) );
	t.set( 'mode', 'rotate' );
	if ( t.controls.axis !== 'XY' ) throw new Error( `expected a stale XY axis, got ${ t.controls.axis }` );
	t.api( 'down', null );
	t.api( 'move', { x: 0.1, y: 0.05, button: - 1 }, { full: true } );
	t.api( 'move', { x: 0.15, y: 0.1, button: - 1 }, { full: true } );
	t.api( 'up', null );

} ) );

// A ring axis exactly along the eye: the cross product is zero and the drag
// falls back to the in-plane rotation.
scenarios.push( await scenario( 'rotate_in_plane', { camera: { position: [ 6, 0, 0 ], target: [ 0, 0, 0 ] }, object: { p: [ 0, 0, 0 ], q: [ 0, 0, 0, 1 ], s: [ 1, 1, 1 ] } }, ( t ) => {

	t.set( 'mode', 'rotate' );
	t.render( { full: true } );
	t.drag( t.at( 'X', ...ringX ), [ [ 20, 25 ], [ - 10, 45, { full: true } ] ] );

} ) );

// Scale: always local; per axis, a plane, and uniform from the centre.
scenarios.push( await scenario( 'scale', { object: turned }, ( t ) => {

	t.set( 'mode', 'scale' );
	t.render();
	t.drag( t.at( 'X', [ 0.45, 0, 0 ] ), [ [ 30, 5 ], [ 60, 10, { full: true } ] ] );
	t.drag( t.at( 'Y', [ 0, 0.45, 0 ] ), [ [ 0, 25 ] ] );
	t.drag( t.at( 'XZ', ...XZ ), [ [ 25, - 20 ] ] );
	t.drag( t.at( 'YZ', ...YZ ), [ [ - 10, 15 ] ] );
	t.drag( t.at( 'XYZ', [ 0.05, 0.05, 0.05 ], [ 0, 0, 0 ] ), [ [ 15, - 15 ], [ - 25, 25 ] ] );

} ) );

// The scale snap, including `|| scaleSnap` when the snapped scale is 0.
scenarios.push( await scenario( 'scale_snap', {}, ( t ) => {

	t.set( 'mode', 'scale' );
	t.set( 'scaleSnap', 0.5 );
	t.drag( t.at( 'X', [ 0.45, 0, 0 ] ), [ [ 40, - 5 ], [ - 45, 8 ] ] );
	t.drag( t.at( 'XYZ', [ 0.05, 0.05, 0.05 ], [ 0, 0, 0 ] ), [ [ 30, - 30 ], [ - 4, 3 ] ] );
	// Inward until the scale rounds to 0, which `|| scaleSnap` turns back
	// into the snap.
	t.drag( t.at( 'X', [ 0.45, 0, 0 ] ), [ [ - 10, 2 ], [ - 20, 3 ], [ - 30, 5 ], [ - 40, 6 ], [ - 50, 8 ], [ - 60, 9 ] ] );

} ) );

// An orthographic camera: the eye is the view direction and the gizmo size
// does not depend on the distance.
scenarios.push( await scenario( 'orthographic', { camera: { type: 'orthographic', zoom: 1.25, position: [ - 4, 3, 6 ] } }, ( t ) => {

	t.render( { full: true } );
	t.drag( t.at( 'X', [ 0.45, 0, 0 ] ), [ [ 30, - 10 ] ] );
	t.drag( t.at( 'XY', ...XY ), [ [ 20, 20 ] ] );
	t.set( 'mode', 'rotate' );
	t.drag( t.at( 'E', ...ringE ), [ [ - 20, 30, { full: true } ] ] );
	t.drag( t.at( 'XYZE', [ 0, 0, 0 ] ), [ [ 25, 10 ] ] );

} ) );

// Hover only, the show flags, size and setColors().
scenarios.push( await scenario( 'hover_and_flags', { object: turned }, ( t ) => {

	const x = t.at( 'X', [ 0.45, 0, 0 ] );
	const xy = t.at( 'XY', ...XY );
	t.move( x, { full: true } );
	t.move( xy );
	t.move( [ 5, 5 ] );
	t.set( 'showX', false );
	t.move( x );
	t.set( 'showX', true );
	t.set( 'showXY', false );
	t.move( xy );
	t.set( 'showXY', true );
	t.set( 'size', 2 );
	t.move( t.at( 'Z', [ 0, 0, 0.45 ] ) );
	t.colors( new THREE.Color( 0x3366ff ), new THREE.Color( 0xff8800 ), new THREE.Color( 0x22cc44 ), new THREE.Color( 0xff00ff ), { full: true } );
	t.set( 'space', 'local' );
	t.set( 'mode', 'rotate' );
	t.move( t.at( 'Y', ...ringY ), { full: true } );
	t.set( 'showE', false );
	t.set( 'showXYZE', false );
	t.move( t.at( 'Y', ...ringY ) );
	t.set( 'showZ', false );
	t.set( 'mode', 'scale' );
	t.set( 'showYZ', false );
	t.set( 'showXZ', false, { full: true } );
	// Setting a property to its value sends nothing.
	t.set( 'mode', 'scale' );
	t.set( 'size', 2 );

} ) );

// reset() mid-drag, moves without a render between them, enabled, a
// non-main button, detach() and attach().
scenarios.push( await scenario( 'reset_enabled_detach', {}, ( t ) => {

	const x = t.at( 'X', [ 0.45, 0, 0 ] );
	t.move( x );
	t.down( x );
	t.move( [ x[ 0 ] + 30, x[ 1 ] - 5 ] );
	t.call( 'reset' );
	t.move( [ x[ 0 ] + 45, x[ 1 ] - 8 ], { render: false } );
	t.move( [ x[ 0 ] + 50, x[ 1 ] - 12 ], { render: false } );
	t.render();
	t.up( [ x[ 0 ] + 50, x[ 1 ] - 12 ] );
	// Not dragging: reset() does nothing.
	t.call( 'reset' );
	// A right-button down neither starts a drag nor, as a pointer that has
	// not gone up, stops the hover.
	t.move( x );
	t.down( x, { button: 2 } );
	t.move( [ x[ 0 ] + 20, x[ 1 ] ] );
	t.up( [ x[ 0 ] + 20, x[ 1 ] ], { button: 2 } );
	t.set( 'enabled', false );
	t.move( x );
	t.down( x );
	t.move( [ x[ 0 ] + 25, x[ 1 ] ] );
	t.call( 'reset' );
	t.up( [ x[ 0 ] + 25, x[ 1 ] ] );
	t.set( 'enabled', true );
	t.move( x );
	t.call( 'detach', { full: true } );
	t.move( x );
	t.down( x );
	t.up( x );
	t.call( 'attach' );
	t.drag( x, [ [ - 20, 4 ] ] );

} ) );

// Touch: no hover on a move, so the axis comes from the hover inside
// onPointerDown and is null between drags; the plane keeps the direction it
// last had.
scenarios.push( await scenario( 'touch', { object: turned }, ( t ) => {

	const touch = { type: 'touch' };
	t.move( [ 3, 3 ], touch );
	t.drag( t.at( 'Y', [ 0, 0.45, 0 ] ), [ [ 4, - 30 ] ], touch );
	t.render();
	t.drag( t.at( 'XZ', ...XZ ), [ [ 25, 10 ] ], touch );
	t.render();
	t.set( 'space', 'local' );
	t.render( { full: true } );
	t.down( [ 5, 5 ], touch );
	t.move( [ 30, 30 ], touch );
	t.up( [ 30, 30 ], touch );
	t.move( t.at( 'X', [ 0.45, 0, 0 ] ), { type: 'pen' } );

} ) );

// A viewport inside the element, then a locked pointer (always the centre).
scenarios.push( await scenario( 'viewport_and_lock', { element: [ 900, 700 ] }, ( t ) => {

	t.set( 'viewport', [ 100, 50, 600, 450 ] );
	t.render();
	t.drag( t.at( 'X', [ 0.45, 0, 0 ] ), [ [ 30, - 6 ] ] );
	t.drag( t.at( 'XY', ...XY ), [ [ - 25, 18 ] ] );
	t.set( 'viewport', null );
	t.set( 'pointerLocked', true );
	t.move( [ 10, 20 ] );
	t.down( [ 600, 10 ] );
	t.move( [ 50, 300 ] );
	t.up( [ 50, 300 ] );

} ) );

// The NDC methods directly, null reusing the last ray, and the button
// filters on each.
scenarios.push( await scenario( 'pointer_api', {}, ( t ) => {

	const [ x, y ] = t.at( 'Z', [ 0, 0, 0.45 ] );
	const ndc = { x: x / 800 * 2 - 1, y: - y / 600 * 2 + 1 };
	t.api( 'hover', { ...ndc, button: - 1 } );
	t.api( 'down', { ...ndc, button: 1 } );
	t.api( 'down', null );
	t.api( 'move', { x: ndc.x + 0.05, y: ndc.y - 0.04, button: 0 } );
	t.api( 'move', { x: ndc.x + 0.05, y: ndc.y - 0.04, button: - 1 } );
	t.api( 'move', null );
	t.api( 'hover', { x: 0.9, y: 0.9, button: - 1 } );
	t.api( 'up', { x: 0, y: 0, button: 2 } );
	t.api( 'up', null, { full: true } );
	t.api( 'hover', null );

} ) );

// The helper under a moved, turned and scaled group the object is not in:
// the root cancels the group's transform.
scenarios.push( await scenario( 'helper_parented', { helperParent: parent, object: turned }, ( t ) => {

	t.set( 'space', 'local' );
	t.render();
	t.drag( t.at( 'Z', [ 0, 0, 0.45 ] ), [ [ 20, 30, { full: true } ] ] );
	// No render after setSize(): pointerDown's `object.parent.updateMatrixWorld()`
	// reaches the helper below that parent and brings the gizmo up to date.
	t.set( 'size', 1.6, { render: false } );
	const z = t.at( 'Z', [ 0, 0, 0.45 ] );
	t.move( z, { render: false } );
	t.down( z, { render: false, full: true } );
	t.up( z );
	t.set( 'mode', 'rotate' );
	t.drag( t.at( 'X', ...ringX ), [ [ 20, 10 ] ] );

} ) );

const fresh = await import( `${ moduleUrl }?s=graph` );
const output = { revision: THREE.REVISION, graph: graph( new fresh.TransformControls( new THREE.PerspectiveCamera() ) ), scenarios };

// Twelve significant digits keep the fixture small and are a thousand times
// finer than the test's tolerance.
const trim = ( key, value ) => ( typeof value === 'number' && Number.isFinite( value ) ? Number( value.toPrecision( 12 ) ) : value );
// One graph row and one step per line, so a change in three.js or in the
// scripts reads as a diff.
const line = ( value ) => JSON.stringify( value, trim );
const text = [
	'{',
	`"revision": ${ line( output.revision ) },`,
	'"graph": [',
	output.graph.map( line ).join( ',\n' ),
	'],',
	'"scenarios": [',
	output.scenarios.map( ( s ) => `{"name": ${ line( s.name ) }, "setup": ${ line( s.setup ) }, "steps": [\n${ s.steps.map( line ).join( ',\n' ) }\n]}` ).join( ',\n' ),
	']',
	'}'
].join( '\n' );
JSON.parse( text );
fs.writeFileSync( fixture, text + '\n' );
console.log( `wrote ${ path.relative( process.cwd(), fixture ) } (${ fs.statSync( fixture ).size } bytes)` );
