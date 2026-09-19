// Dumps three.js' own GLTFLoader parse of PrimaryIonDrive.glb as a JSON oracle
// for tests/gltf_primary_ion_drive.rs. No GPU, no network, nothing written into
// the vendor tree. Run it against a three.js checkout (r186):
//
//   THREE=~/src/vendor/three.js node \
//     tests/fixtures/webgpu_postprocessing_bloom/oracle.mjs > \
//     tests/fixtures/webgpu_postprocessing_bloom/primaryiondrive.json
import fs from 'fs';

const THREE_DIR = process.env.THREE || `${ process.env.HOME }/src/vendor/three.js`;
const { GLTFLoader } = await import( `${ THREE_DIR }/examples/jsm/loaders/GLTFLoader.js` );
const { AnimationMixer } = await import( `${ THREE_DIR }/build/three.module.js` );

const FILE = `${ THREE_DIR }/examples/models/gltf/PrimaryIonDrive.glb`;

// FNV-1a over the raw bytes of a typed array: cheap, order-sensitive, stable.
function hash( ta ) {
	const b = new Uint8Array( ta.buffer, ta.byteOffset, ta.byteLength );
	let h = 0x811c9dc5 >>> 0;
	for ( let i = 0; i < b.length; i ++ ) { h ^= b[ i ]; h = Math.imul( h, 0x01000193 ) >>> 0; }
	return h >>> 0;
}
const r = ( x ) => Number( x.toFixed( 9 ) );

function attrs( g ) {
	const out = {};
	for ( const [ name, a ] of Object.entries( g.attributes ) ) {
		out[ name ] = { itemSize: a.itemSize, count: a.count, normalized: a.normalized,
			array: a.array.constructor.name, hash: hash( a.array ),
			first: Array.from( a.array.slice( 0, a.itemSize * 2 ) ).map( r ) };
	}
	if ( g.index ) out.index = { count: g.index.count, array: g.index.array.constructor.name, hash: hash( g.index.array ),
		first: Array.from( g.index.array.slice( 0, 6 ) ) };
	return out;
}

function material( m ) {
	const o = { name: m.name, type: m.type, side: m.side, transparent: m.transparent, opacity: m.opacity,
		depthWrite: m.depthWrite, alphaTest: m.alphaTest, blending: m.blending, vertexColors: m.vertexColors,
		flatShading: m.flatShading, toneMapped: m.toneMapped,
		color: m.color ? m.color.toArray().map( r ) : null,
		metalness: m.metalness, roughness: m.roughness,
		emissive: m.emissive ? m.emissive.toArray().map( r ) : null, emissiveIntensity: m.emissiveIntensity,
		ior: m.ior, specularIntensity: m.specularIntensity,
		specularColor: m.specularColor ? m.specularColor.toArray().map( r ) : null,
		normalScale: m.normalScale ? m.normalScale.toArray().map( r ) : null };
	for ( const k of [ 'map', 'normalMap', 'roughnessMap', 'metalnessMap', 'emissiveMap', 'aoMap', 'specularColorMap' ] )
		o[ k ] = m[ k ] ? { colorSpace: m[ k ].colorSpace, flipY: m[ k ].flipY, wrapS: m[ k ].wrapS, wrapT: m[ k ].wrapT,
			magFilter: m[ k ].magFilter, minFilter: m[ k ].minFilter, channel: m[ k ].channel } : null;
	return o;
}

function tree( root ) {
	const nodes = [];
	root.traverse( ( o ) => {
		o.updateMatrix();
		const e = { name: o.name, type: o.type, uuidOrder: nodes.length,
			parent: o.parent ? o.parent.name : null,
			visible: o.visible, castShadow: o.castShadow, receiveShadow: o.receiveShadow,
			renderOrder: o.renderOrder,
			position: o.position.toArray().map( r ), quaternion: o.quaternion.toArray().map( r ),
			scale: o.scale.toArray().map( r ),
			matrix: Array.from( o.matrix.elements ).map( r ),
			matrixWorld: Array.from( o.matrixWorld.elements ).map( r ) };
		if ( o.isMesh ) {
			e.geometry = { name: o.geometry.name, uuidShared: o.geometry.uuid, groups: o.geometry.groups,
				drawRange: o.geometry.drawRange, morphTargetsRelative: o.geometry.morphTargetsRelative,
				morphAttributes: Object.fromEntries( Object.entries( o.geometry.morphAttributes ).map( ( [ k, v ] ) => [ k, v.length ] ) ),
				attributes: attrs( o.geometry ) };
			e.material = material( o.material );
			e.morphTargetDictionary = o.morphTargetDictionary || null;
			e.morphTargetInfluences = o.morphTargetInfluences || null;
		}
		nodes.push( e );
	} );
	return nodes;
}

const buf = fs.readFileSync( FILE );
const ab = buf.buffer.slice( buf.byteOffset, buf.byteOffset + buf.byteLength );
const loader = new GLTFLoader();
const gltf = await new Promise( ( res, rej ) => loader.parse( ab, '', res, rej ) );

gltf.scene.updateMatrixWorld( true );

const clipInfo = ( c ) => ( { name: c.name, duration: r( c.duration ), blendMode: c.blendMode,
	tracks: c.tracks.map( ( t ) => ( { name: t.name, type: t.constructor.name, valueSize: t.getValueSize(),
		times: t.times.length, interpolation: t.getInterpolation(),
		timesHash: hash( t.times ), valuesHash: hash( t.values ),
		time0: r( t.times[ 0 ] ), timeN: r( t.times[ t.times.length - 1 ] ),
		values0: Array.from( t.values.slice( 0, t.getValueSize() ) ).map( r ) } ) ) } );

const out = {
	file: 'models/gltf/PrimaryIonDrive.glb',
	asset: gltf.asset,
	sceneName: gltf.scene.name,
	sceneChildren: gltf.scene.children.map( ( c ) => c.name ),
	nodeCount: tree( gltf.scene ).length,
	nodes: tree( gltf.scene ),
	animations: gltf.animations.map( clipInfo ),
	animationsOptimized: gltf.animations.map( ( c ) => clipInfo( c.clone().optimize() ) ),
};

// The example's frame: mixer on the optimized clip, delta 0 (the harness pins
// performance.now() to 0, so timer.getDelta() is 0 on the graded frame).
{
	const g2 = await new Promise( ( res, rej ) => loader.parse( ab, '', res, rej ) );
	const mixer = new AnimationMixer( g2.scene );
	mixer.clipAction( g2.animations[ 0 ].optimize() ).play();
	mixer.update( 0 );
	g2.scene.updateMatrixWorld( true );
	out.atT0 = tree( g2.scene ).map( ( n ) => ( { name: n.name, matrixWorld: n.matrixWorld } ) );
}

process.stdout.write( JSON.stringify( out, null, 1 ) );
