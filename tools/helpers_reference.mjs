// Runs three.js' own core helpers (`src/helpers/`) under node and prints, as
// JSON, what each one builds: for every scenario and stage, the helper's
// scene-graph subtree with each node's type, transform, matrices, geometry
// (every attribute array and the index) and material settings.
// `tests/helpers_core.rs` builds the same helpers in the port with the same
// inputs and compares the two node for node.
//
//   node tools/helpers_reference.mjs [three.js checkout]
//
// The checkout defaults to $THREE_JS_DIR, then ~/src/vendor/three.js, as
// `three_rs::testing::three_js_dir()` does. Nothing is written: the test reads
// stdout, so the comparison is always against the pinned three.js itself.
//
// The helpers need no DOM and no renderer, so the classes are taken straight
// from the checkout's ES build. "Rendering" is `root.updateMatrixWorld()`,
// which is where `Box3Helper`, `PlaneHelper` and `SkeletonHelper` run their
// `updateMatrixWorld` overrides.
//
// Each scenario's inputs are spelled out here and again in the Rust test; a
// change to one is a change to both.

import * as os from 'os';
import * as path from 'path';
import { pathToFileURL } from 'url';

const threeDir = process.argv[ 2 ] || process.env.THREE_JS_DIR || path.join( os.homedir(), 'src/vendor/three.js' );
const THREE = await import( pathToFileURL( path.join( threeDir, 'build/three.module.js' ) ).href );

// The port tracks three.js at the pinned commit (5f610f5); another revision's
// helpers are not the reference.
if ( THREE.REVISION !== '187dev' ) throw new Error( `expected three.js r187dev, ${ threeDir } is r${ THREE.REVISION }` );

const {
	Object3D, Group, Bone, Mesh, Vector3, Box3, Plane, Color,
	BoxGeometry, SphereGeometry, MeshBasicMaterial,
	DirectionalLight, HemisphereLight, PointLight, SpotLight,
	AxesHelper, ArrowHelper, BoxHelper, Box3Helper, PlaneHelper, PolarGridHelper,
	DirectionalLightHelper, HemisphereLightHelper, PointLightHelper, SpotLightHelper, SkeletonHelper,
} = THREE;

function material( m ) {

	if ( ! m ) return null;

	return {
		color: [ m.color.r, m.color.g, m.color.b ],
		opacity: m.opacity,
		transparent: m.transparent,
		depthTest: m.depthTest,
		depthWrite: m.depthWrite,
		vertexColors: m.vertexColors,
		wireframe: m.wireframe === true,
		fog: m.fog,
		toneMapped: m.toneMapped,
	};

}

function geometry( g ) {

	if ( ! g ) return null;

	const attributes = {};
	for ( const name of Object.keys( g.attributes ).sort() ) {

		const attribute = g.attributes[ name ];
		attributes[ name ] = { itemSize: attribute.itemSize, array: Array.from( attribute.array ) };

	}

	return { index: g.index ? Array.from( g.index.array ) : null, attributes };

}

function dump( object ) {

	return {
		type: object.type,
		visible: object.visible,
		matrixAutoUpdate: object.matrixAutoUpdate,
		position: object.position.toArray(),
		quaternion: object.quaternion.toArray(),
		scale: object.scale.toArray(),
		matrix: object.matrix.toArray(),
		matrixWorld: object.matrixWorld.toArray(),
		geometry: geometry( object.geometry ),
		// Only what draws: `HemisphereLightHelper` also keeps its octahedron's
		// material as `this.material` on the plain `Object3D`, which the port
		// leaves on the mesh (the scenario checks the two are one object).
		material: object.isMesh || object.isLine ? material( object.material ) : null,
		children: object.children.map( dump ),
	};

}

// A parent with a transform of its own, so every helper's world matrix is a
// product and not just its local matrix.
function transformedRoot() {

	const root = new Object3D();
	root.position.set( 0.25, - 1, 2 );
	root.rotation.set( 0.1, 0.2, 0.3 );
	root.scale.set( 1.5, 1.5, 1.5 );
	return root;

}

const scenarios = {};

// AxesHelper -----------------------------------------------------------------

scenarios.axes_default = ( () => {

	const helper = new AxesHelper();
	return { built: dump( helper ) };

} )();

scenarios.axes_colors = ( () => {

	const helper = new AxesHelper( 2.5 );
	const built = dump( helper );
	helper.setColors( 0x123456, 0xabcdef, 0xff8800 );
	return { built, colored: dump( helper ) };

} )();

// ArrowHelper ----------------------------------------------------------------

scenarios.arrow_default = ( () => {

	const root = transformedRoot();
	const helper = new ArrowHelper();
	root.add( helper );
	root.updateMatrixWorld();
	return { built: dump( helper ) };

} )();

scenarios.arrow_full = ( () => {

	const root = transformedRoot();
	const dir = new Vector3( 1, 2, 3 ).normalize();
	const helper = new ArrowHelper( dir, new Vector3( 1, - 2, 0.5 ), 3, 0x00ff88, 0.7, 0.4 );
	root.add( helper );
	root.updateMatrixWorld();
	const built = dump( helper );

	helper.setLength( 5 );
	helper.setColor( 0x112233 );
	helper.setDirection( new Vector3( - 1, 0.5, 0.25 ).normalize() );
	root.updateMatrixWorld();

	return { built, changed: dump( helper ) };

} )();

scenarios.arrow_up = ( () => {

	const helper = new ArrowHelper( new Vector3( 0, 1, 0 ) );
	helper.updateMatrixWorld();
	return { built: dump( helper ) };

} )();

scenarios.arrow_down = ( () => {

	const helper = new ArrowHelper( new Vector3( 0, - 1, 0 ), new Vector3( 0, 0, 0 ), 2 );
	helper.updateMatrixWorld();
	return { built: dump( helper ) };

} )();

// BoxHelper ------------------------------------------------------------------

scenarios.box_helper = ( () => {

	const root = transformedRoot();
	const group = new Group();
	group.position.set( - 1, 0, 0 );
	const mesh = new Mesh( new BoxGeometry( 1, 2, 3 ), new MeshBasicMaterial() );
	mesh.position.set( 1, 2, 3 );
	mesh.rotation.set( 0.3, 0.4, 0.5 );
	mesh.scale.set( 1, 2, 0.5 );
	group.add( mesh );
	root.add( group );

	// Built before the root's first update: `expandByObject()` updates only
	// the object's own world matrix, so the group's is still the identity.
	const helper = new BoxHelper( mesh, 0xff0000 );
	root.add( helper );
	root.updateMatrixWorld();
	const built = dump( helper );

	mesh.position.set( - 2, 0, 1 );
	helper.update();
	root.updateMatrixWorld();
	const moved = dump( helper );

	const sphere = new Mesh( new SphereGeometry(), new MeshBasicMaterial() );
	sphere.position.set( 0, 3, 0 );
	root.add( sphere );
	root.updateMatrixWorld();
	helper.setFromObject( sphere );

	return { built, moved, sphere: dump( helper ) };

} )();

// Box3Helper -----------------------------------------------------------------

scenarios.box3_helper = ( () => {

	const root = transformedRoot();
	const box = new Box3( new Vector3( - 1, 0, 2 ), new Vector3( 3, 4, 5 ) );
	const helper = new Box3Helper( box, 0x00ffff );
	root.add( helper );
	const built = dump( helper );

	root.updateMatrixWorld();
	const updated = dump( helper );

	box.min.set( 0, - 2, - 1 );
	box.max.set( 0.5, 2, 1 );
	root.updateMatrixWorld();

	return { built, updated, resized: dump( helper ) };

} )();

// PlaneHelper ----------------------------------------------------------------

scenarios.plane_helper = ( () => {

	const root = transformedRoot();
	const plane = new Plane( new Vector3( 1, 1, 0 ).normalize(), - 2 );
	const helper = new PlaneHelper( plane, 3, 0xff00ff );
	root.add( helper );
	root.updateMatrixWorld();
	const updated = dump( helper );

	plane.normal.set( 0, 0, 1 );
	plane.constant = 1.5;
	helper.size = 2;
	root.updateMatrixWorld();

	return { updated, changed: dump( helper ) };

} )();

// PolarGridHelper ------------------------------------------------------------

scenarios.polar_default = { built: dump( new PolarGridHelper() ) };
scenarios.polar_custom = { built: dump( new PolarGridHelper( 5, 6, 3, 12, 0xff0000, 0x00ff00 ) ) };
scenarios.polar_one_sector = { built: dump( new PolarGridHelper( 2, 1, 2, 8 ) ) };

// DirectionalLightHelper -----------------------------------------------------

scenarios.directional = ( () => {

	const root = transformedRoot();
	const light = new DirectionalLight( 0xffaa33, 2 );
	light.position.set( 5, 10, 7.5 );
	light.target.position.set( 1, 0, - 2 );
	root.add( light );

	const helper = new DirectionalLightHelper( light, 2 );
	root.add( helper );
	root.updateMatrixWorld();
	const built = dump( helper );

	light.color.set( 0x3366ff );
	light.position.set( - 3, 4, 1 );
	helper.update();
	root.updateMatrixWorld();

	return { built, updated: dump( helper ) };

} )();

scenarios.directional_colored = ( () => {

	const root = new Object3D();
	const light = new DirectionalLight();
	root.add( light );

	const helper = new DirectionalLightHelper( light, undefined, 0xaabbcc );
	root.add( helper );
	root.updateMatrixWorld();

	return { built: dump( helper ) };

} )();

// HemisphereLightHelper ------------------------------------------------------

scenarios.hemisphere = ( () => {

	const root = transformedRoot();
	const light = new HemisphereLight( 0x123456, 0xabc012, 0.6 );
	light.position.set( 2, 3, - 1 );
	root.add( light );

	const helper = new HemisphereLightHelper( light, 1.5 );
	if ( helper.material !== helper.children[ 0 ].material ) throw new Error( 'HemisphereLightHelper.material is not its mesh\'s' );
	root.add( helper );
	root.updateMatrixWorld();
	const built = dump( helper );

	light.color.set( 0xff0000 );
	light.groundColor.set( 0x0000ff );
	light.position.set( 0, - 1, 1 );
	helper.update();
	root.updateMatrixWorld();

	return { built, updated: dump( helper ) };

} )();

scenarios.hemisphere_colored = ( () => {

	const root = new Object3D();
	const light = new HemisphereLight( 0x123456, 0xabc012, 0.6 );
	root.add( light );

	const helper = new HemisphereLightHelper( light, 1, 0xabc012 );
	root.add( helper );
	root.updateMatrixWorld();

	return { built: dump( helper ) };

} )();

// PointLightHelper -----------------------------------------------------------

scenarios.point = ( () => {

	const root = transformedRoot();
	const light = new PointLight( 0x00ff00, 1, 100 );
	light.position.set( 1, 2, 3 );
	root.add( light );

	const helper = new PointLightHelper( light, 0.5 );
	root.add( helper );
	root.updateMatrixWorld();
	const built = dump( helper );

	light.color.set( 0xff00ff );
	helper.update();
	root.updateMatrixWorld();

	return { built, updated: dump( helper ) };

} )();

scenarios.point_colored = ( () => {

	const root = new Object3D();
	const light = new PointLight( 0x00ff00 );
	root.add( light );

	const helper = new PointLightHelper( light, 1, 0xaaaaaa );
	root.add( helper );
	root.updateMatrixWorld();

	return { built: dump( helper ) };

} )();

// SpotLightHelper ------------------------------------------------------------

scenarios.spot = ( () => {

	const root = transformedRoot();
	const light = new SpotLight( 0xff8844, 1 );
	light.position.set( 3, 5, 2 );
	light.target.position.set( 0, 0, - 1 );
	light.angle = 0.4;
	light.distance = 20;
	root.add( light );

	// Built with no parent, so `update()` copies the light's world matrix;
	// after `root.add()` the next `update()` takes the parent's out of it.
	const helper = new SpotLightHelper( light );
	root.add( helper );
	root.updateMatrixWorld();
	const built = dump( helper );

	helper.update();
	root.updateMatrixWorld();

	return { built, updated: dump( helper ) };

} )();

scenarios.spot_colored = ( () => {

	const light = new SpotLight( 0xffffff );
	const helper = new SpotLightHelper( light, 0x00ffff );
	helper.updateMatrixWorld();

	return { built: dump( helper ) };

} )();

// SkeletonHelper -------------------------------------------------------------

function skeleton() {

	const root = transformedRoot();
	const character = new Group();
	character.position.set( 0, 1, 0 );
	root.add( character );

	const b0 = new Bone();
	b0.position.set( 0, 1, 0 );
	character.add( b0 );

	const b1 = new Bone();
	b1.position.set( 0, 1, 0.5 );
	b1.rotation.set( 0, 0, 0.3 );
	b0.add( b1 );

	const b2 = new Bone();
	b2.position.set( 0.5, 1, 0 );
	b1.add( b2 );

	const b3 = new Bone();
	b3.position.set( - 1, 0.5, 0 );
	b0.add( b3 );

	// A bone under a non-bone: in the list, but no segment to its parent.
	const holder = new Object3D();
	holder.position.set( 0, 0, 1 );
	b1.add( holder );
	const b4 = new Bone();
	b4.position.set( 0, 0.5, 0 );
	holder.add( b4 );

	return { root, character, b0, b1 };

}

scenarios.skeleton = ( () => {

	const { root, character, b1 } = skeleton();
	const helper = new SkeletonHelper( character );
	root.add( helper );
	root.updateMatrixWorld();
	const built = dump( helper );

	b1.rotation.set( 0.5, 0, - 0.2 );
	root.updateMatrixWorld();

	return { built, posed: dump( helper ) };

} )();

scenarios.skeleton_bone_root = ( () => {

	const { root, b0 } = skeleton();
	const helper = new SkeletonHelper( b0 );
	root.add( helper );
	root.updateMatrixWorld();

	return { built: dump( helper ) };

} )();

process.stdout.write( JSON.stringify( scenarios ) + '\n' );
