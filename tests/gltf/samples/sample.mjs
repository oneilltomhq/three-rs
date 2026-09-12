import { GLTFLoader } from '/home/tom/src/vendor/three.js/examples/jsm/loaders/GLTFLoader.js';
import fs from 'fs';
const loader = new GLTFLoader();
loader.register(() => ({ name: 'stub', loadTexture: () => Promise.resolve(null) }));
for (const file of ['Soldier.glb', 'Michelle.glb']) {
  const buf = fs.readFileSync('/home/tom/src/vendor/three.js/examples/models/gltf/' + file);
  await new Promise((res, rej) => loader.parse(buf.buffer.slice(buf.byteOffset, buf.byteOffset + buf.byteLength), '', (gltf) => {
    const names = []; let skinned = null; const skels = [];
    gltf.scene.traverse(o => { names.push(o.name + ':' + o.type); if (o.isSkinnedMesh && !skinned) skinned = o; if (o.isSkinnedMesh) skels.push(o.skeleton.bones.length); });
    console.log('###', file);
    console.log('nodes', names.length);
    console.log('names', JSON.stringify(names));
    console.log('skeletons', JSON.stringify(skels));
    console.log('clips', JSON.stringify(gltf.animations.map(c => [c.name, c.duration, c.tracks.length])));
    const g = skinned.geometry;
    console.log('posCount', g.attributes.position.count, 'idx', g.index ? g.index.count : null);
    console.log('pos0', JSON.stringify(Array.from(g.attributes.position.array.slice(0, 9))));
    console.log('skinIndex0', JSON.stringify(Array.from(g.attributes.skinIndex.array.slice(0,8))));
    console.log('boneInv0', JSON.stringify(Array.from(skinned.skeleton.boneInverses[0].elements)));
    console.log('track0', gltf.animations[0].tracks[0].name, JSON.stringify(Array.from(gltf.animations[0].tracks[0].values.slice(0,4))));
    res();
  }, rej));
}
