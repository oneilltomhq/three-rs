// Runs three.js' own `StereoCamera`, `CameraUtils.frameCorners()` and
// `AnaglyphPassNode` under node and prints what they do to their cameras, so
// that `tests/cameras_stereo_camera.rs` can assert the Rust ports do the same
// thing to the same inputs. Its output is committed as
// `tests/fixtures/stereo_camera/three_r187dev.json`:
//
//   node tools/stereo_camera_reference.mjs <three.js checkout> \
//     > tests/fixtures/stereo_camera/three_r187dev.json
//
// It refuses to run against any checkout but r187dev, the revision the port
// follows, so the fixture cannot silently drift to another one.
//
// It prints one JSON object with four keys:
//
//   * `stereo`: scenarios of `StereoCamera.update( camera )`, each a list of
//     steps; a step is both eyes' `projectionMatrix`, `matrix`,
//     `matrixWorldNeedsUpdate`, `matrixAutoUpdate` and `layers.mask` after
//     one `update()`. Later steps change the source camera between updates,
//     so the projection cache is exercised both ways.
//   * `frame_corners`: `frameCorners()` results — the camera's
//     `projectionMatrix`, `projectionMatrixInverse`, `quaternion` and `fov`.
//   * `anaglyph`: the node's defaults, all 21 `ANAGLYPH_MATRICES` pairs read
//     back out of its two `mat3` uniforms (`Matrix3.elements`, column-major),
//     and the eyes `updateStereoCamera()` builds.
//   * `stereo_pass`: `stereoPass( scene, camera ).stereo.aspect`.
//
// # No GPU
//
// Nothing here renders. Building a pass node only builds node objects, so
// `three.webgpu.js` loads under node without `navigator.gpu`. The addon files
// import from the bare specifiers `'three'`, `'three/webgpu'` and
// `'three/tsl'`, which node cannot resolve out of a checkout, so each is
// copied to a temporary directory with those specifiers rewritten to the
// checkout's own ES builds. Nothing in the checkout is modified.

import * as fs from 'fs';
import * as os from 'os';
import * as path from 'path';
import { pathToFileURL } from 'url';

const threeDir = process.argv[ 2 ];
if ( ! threeDir ) {

	console.error( 'usage: stereo_camera_reference.mjs <three.js checkout>' );
	process.exit( 2 );

}

const webgpuUrl = pathToFileURL( path.join( threeDir, 'build/three.webgpu.js' ) ).href;
const tslUrl = pathToFileURL( path.join( threeDir, 'build/three.tsl.js' ) ).href;
const THREE = await import( webgpuUrl );

if ( THREE.REVISION !== '187dev' ) {

	console.error( `stereo_camera_reference.mjs: three.js REVISION is '${ THREE.REVISION }', the port follows '187dev'` );
	process.exit( 1 );

}

const temporary = fs.mkdtempSync( path.join( os.tmpdir(), 'three-rs-stereo-' ) );
for ( const file of [
	'tsl/display/AnaglyphPassNode.js',
	'tsl/display/StereoCompositePassNode.js',
	'tsl/display/StereoPassNode.js',
	'utils/CameraUtils.js',
] ) {

	const source = fs
		.readFileSync( path.join( threeDir, 'examples/jsm', file ), 'utf8' )
		.replace( /from 'three\/webgpu'/g, `from '${ webgpuUrl }'` )
		.replace( /from 'three\/tsl'/g, `from '${ tslUrl }'` )
		.replace( /from 'three'/g, `from '${ webgpuUrl }'` )
		.replace( /from '\.\.\/\.\.\/utils\/CameraUtils\.js'/g, `from './CameraUtils.js'` );
	fs.writeFileSync( path.join( temporary, path.basename( file ) ), source );

}

const load = ( file ) => import( pathToFileURL( path.join( temporary, file ) ).href );
const { frameCorners } = await load( 'CameraUtils.js' );
const { anaglyphPass, AnaglyphAlgorithm, AnaglyphColorMode } = await load( 'AnaglyphPassNode.js' );
const { stereoPass } = await load( 'StereoPassNode.js' );

const array = ( m ) => Array.from( m.elements );

function eye( camera ) {

	return {
		projection: array( camera.projectionMatrix ),
		matrix: array( camera.matrix ),
		matrix_world_needs_update: camera.matrixWorldNeedsUpdate,
		matrix_auto_update: camera.matrixAutoUpdate,
		layers: camera.layers.mask,
	};

}

function step( stereo ) {

	return { left: eye( stereo.cameraL ), right: eye( stereo.cameraR ) };

}

// The page's camera: `new PerspectiveCamera( 60, 800 / 500, 0.1, 100 )` at
// `z = 3`, the frame the e2e rung grades.
function pageCamera() {

	const camera = new THREE.PerspectiveCamera( 60, 800 / 500, 0.1, 100 );
	camera.position.z = 3;
	camera.updateMatrixWorld();
	return camera;

}

// A camera with every input the cache keys on away from its default.
function posedCamera() {

	const camera = new THREE.PerspectiveCamera( 45, 2, 0.5, 50 );
	camera.focus = 5;
	camera.zoom = 2;
	camera.position.set( 1, 2, 3 );
	camera.rotation.set( 0.3, - 0.4, 0.1 );
	camera.updateProjectionMatrix();
	camera.updateMatrixWorld();
	return camera;

}

const stereo = {};

{

	const camera = pageCamera();
	const s = new THREE.StereoCamera();
	s.update( camera );
	stereo.page = [ step( s ) ];

}

{

	// `StereoPassNode`'s `stereo.aspect = 0.5`.
	const camera = pageCamera();
	const s = new THREE.StereoCamera();
	s.aspect = 0.5;
	s.update( camera );
	stereo.page_half_aspect = [ step( s ) ];

}

{

	const camera = posedCamera();
	const s = new THREE.StereoCamera();
	s.eyeSep = 0.1;
	const steps = [];

	// 0: the first update always builds the projections.
	s.update( camera );
	steps.push( step( s ) );

	// 1: the camera moves; the cache hits, only the eye matrices follow.
	camera.position.set( - 2, 0.5, 4 );
	camera.updateMatrixWorld();
	s.update( camera );
	steps.push( step( s ) );

	// 2: the camera's projection changes through a field the cache does not
	// key on (`filmOffset`): the eyes keep their old projections.
	camera.filmOffset = 2;
	camera.updateProjectionMatrix();
	s.update( camera );
	steps.push( step( s ) );

	// 3: a keyed field changes: the projections are rebuilt from the
	// camera's current one, film offset and all.
	camera.zoom = 1;
	camera.updateProjectionMatrix();
	s.update( camera );
	steps.push( step( s ) );

	// 4: the stereo camera's own `eyeSep` is keyed too.
	s.eyeSep = 0.02;
	s.update( camera );
	steps.push( step( s ) );

	stereo.posed = steps;

}

function framed( camera ) {

	return {
		projection: array( camera.projectionMatrix ),
		projection_inverse: array( camera.projectionMatrixInverse ),
		quaternion: camera.quaternion.toArray(),
		fov: camera.fov,
	};

}

const frame_corners = {};

{

	const camera = new THREE.PerspectiveCamera( 50, 1.5, 0.2, 30 );
	camera.position.set( 0.3, - 0.2, 4 );
	frameCorners(
		camera,
		new THREE.Vector3( - 1, - 0.5, 0 ),
		new THREE.Vector3( 1, - 0.5, 0 ),
		new THREE.Vector3( - 1, 0.5, 0 ),
		true
	);
	frame_corners.estimated = framed( camera );

}

{

	// A tilted screen, no estimate: `fov` is left alone.
	const camera = new THREE.PerspectiveCamera( 70, 0.5, 0.05, 10 );
	camera.position.set( 1, 1, 2 );
	frameCorners(
		camera,
		new THREE.Vector3( - 1, - 1, - 0.5 ),
		new THREE.Vector3( 1.2, - 0.8, 0.3 ),
		new THREE.Vector3( - 1.1, 0.9, - 0.2 ),
		false
	);
	frame_corners.tilted = framed( camera );

}

function anaglyphEye( camera ) {

	return {
		projection: array( camera.projectionMatrix ),
		projection_inverse: array( camera.projectionMatrixInverse ),
		matrix_world: array( camera.matrixWorld ),
		matrix_world_inverse: array( camera.matrixWorldInverse ),
		position: camera.position.toArray(),
		quaternion: camera.quaternion.toArray(),
		fov: camera.fov,
		near: camera.near,
		far: camera.far,
	};

}

const anaglyph = {};

{

	const node = anaglyphPass( new THREE.Scene(), pageCamera() );
	anaglyph.defaults = {
		eye_sep: node.eyeSep,
		plane_distance: node.planeDistance,
		algorithm: node.algorithm,
		color_mode: node.colorMode,
		stereo_aspect: node.stereo.aspect,
		color_matrix_left: array( node._colorMatrixLeft.value ),
		color_matrix_right: array( node._colorMatrixRight.value ),
	};

	const matrices = {};
	for ( const algorithm of Object.values( AnaglyphAlgorithm ) ) {

		matrices[ algorithm ] = {};
		for ( const colorMode of Object.values( AnaglyphColorMode ) ) {

			node.algorithm = algorithm;
			node.colorMode = colorMode;
			matrices[ algorithm ][ colorMode ] = {
				left: array( node._colorMatrixLeft.value ),
				right: array( node._colorMatrixRight.value ),
			};

		}

	}

	anaglyph.matrices = matrices;

}

{

	// The page: `anaglyph.eyeSep = 0.064; anaglyph.planeDistance = 3`.
	const node = anaglyphPass( new THREE.Scene(), pageCamera() );
	node.eyeSep = 0.064;
	node.planeDistance = 3;
	node.updateStereoCamera( THREE.WebGPUCoordinateSystem );
	anaglyph.page = { left: anaglyphEye( node.stereo.cameraL ), right: anaglyphEye( node.stereo.cameraR ) };

}

{

	// The node's own defaults, under a posed camera.
	const node = anaglyphPass( new THREE.Scene(), posedCamera() );
	node.updateStereoCamera( THREE.WebGPUCoordinateSystem );
	anaglyph.posed = { left: anaglyphEye( node.stereo.cameraL ), right: anaglyphEye( node.stereo.cameraR ) };

}

const stereo_pass = { stereo_aspect: stereoPass( new THREE.Scene(), pageCamera() ).stereo.aspect };

console.log( JSON.stringify( { revision: THREE.REVISION, stereo, frame_corners, anaglyph, stereo_pass }, null, '\t' ) );
