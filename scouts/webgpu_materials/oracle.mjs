import * as THREE from '/home/tom/src/vendor/three.js/src/Three.js';
import { GridHelper } from '/home/tom/src/vendor/three.js/src/helpers/GridHelper.js';

// deterministic-injection.js
let seed = Math.PI / 4;
const rnd = () => { const x = Math.sin( seed ++ ) * 10000; return x - Math.floor( x ); };

const N = 17;
const draws = [];
const objects = [];
for ( let i = 0; i < N; i ++ ) {
  const o = new THREE.Object3D();
  o.position.x = ( i % 4 ) * 200 - 400;
  o.position.z = Math.floor( i / 4 ) * 200 - 200;
  const a = rnd(), b = rnd(), c = rnd();
  draws.push( a, b, c );
  o.rotation.x = a * 200 - 100;
  o.rotation.y = b * 200 - 100;
  o.rotation.z = c * 200 - 100;
  objects.push( o );
}
// animate(): one frame
for ( const o of objects ) { o.rotation.x += 0.01; o.rotation.y += 0.005; }

const camera = new THREE.PerspectiveCamera( 45, 800 / 500, 1, 2000 );
camera.position.set( 0, 200, 800 );
const timer = 0; // 0.0001 * Date.now() with Date.now() === 0
camera.position.x = Math.cos( timer ) * 1000;
camera.position.z = Math.sin( timer ) * 1000;
camera.lookAt( new THREE.Vector3( 0, 0, 0 ) );
camera.updateMatrixWorld( true );
camera.updateProjectionMatrix();

const meshes = objects.map( ( o, i ) => {
  o.updateMatrixWorld( true );
  return {
    index: i,
    position: o.position.toArray(),
    rotation: [ o.rotation.x, o.rotation.y, o.rotation.z ],
    quaternion: o.quaternion.toArray(),
    matrixWorld: o.matrixWorld.elements.slice(),
  };
} );

const grid = new GridHelper( 1000, 40, 0x303030, 0x303030 );
grid.position.y = - 75;
grid.updateMatrixWorld( true );
const gpos = Array.from( grid.geometry.getAttribute( 'position' ).array );
const gcol = Array.from( grid.geometry.getAttribute( 'color' ).array );

console.log( JSON.stringify( {
  note: 'webgpu_materials oracle: e2e deterministic Math.random draws, mesh transforms after one animate() step, camera matrices, GridHelper buffers. Generated from three.js r186 src in node, no GPU.',
  randomSeedStart: Math.PI / 4,
  randomDrawCount: draws.length,
  randomDraws: draws,
  camera: {
    fov: 45, aspect: 800 / 500, near: 1, far: 2000,
    position: camera.position.toArray(),
    matrixWorld: camera.matrixWorld.elements.slice(),
    matrixWorldInverse: camera.matrixWorldInverse.elements.slice(),
    projectionMatrix: camera.projectionMatrix.elements.slice(),
  },
  meshes,
  gridHelper: {
    position: { y: -75 },
    vertexCount: gpos.length / 3,
    positionFirst12: gpos.slice( 0, 12 ),
    positionLast12: gpos.slice( -12 ),
    colorFirst12: gcol.slice( 0, 12 ),
    colorDistinct: Array.from( new Set( gcol ) ),
    positionSum: gpos.reduce( ( a, b ) => a + b, 0 ),
  },
}, null, 1 ) );
