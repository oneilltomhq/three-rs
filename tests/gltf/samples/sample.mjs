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

// t = 0 bone world matrices, for the AnimationMixer test.
import { AnimationMixer } from '/home/tom/src/vendor/three.js/build/three.module.js';
{
  const buf = fs.readFileSync('/home/tom/src/vendor/three.js/examples/models/gltf/Michelle.glb');
  await new Promise((res, rej) => loader.parse(buf.buffer.slice(buf.byteOffset, buf.byteOffset + buf.byteLength), '', (gltf) => {
    const mixer = new AnimationMixer(gltf.scene);
    mixer.clipAction(gltf.animations[0]).play();
    mixer.update(0);
    gltf.scene.updateMatrixWorld(true);
    console.log('### mixer t=0');
    for (const name of ['mixamorigHips', 'mixamorigSpine', 'mixamorigLeftHand', 'mixamorigRightToeBase']) {
      const bone = gltf.scene.getObjectByName(name);
      console.log(name, JSON.stringify(Array.from(bone.matrixWorld.elements)));
      console.log(name + '.pos', JSON.stringify(bone.position.toArray()), JSON.stringify(bone.quaternion.toArray()));
    }
    res();
  }, rej));
}

// Skeleton.update() output at t = 0 — what the skinning shader consumes.
{
  const buf = fs.readFileSync('/home/tom/src/vendor/three.js/examples/models/gltf/Michelle.glb');
  await new Promise((res, rej) => loader.parse(buf.buffer.slice(buf.byteOffset, buf.byteOffset + buf.byteLength), '', (gltf) => {
    const mixer = new AnimationMixer(gltf.scene);
    mixer.clipAction(gltf.animations[0]).play();
    mixer.update(0);
    gltf.scene.updateMatrixWorld(true);
    let skinned; gltf.scene.traverse(o => { if (o.isSkinnedMesh) skinned = o; });
    skinned.skeleton.update();
    console.log('### skeleton.update');
    console.log('boneMatrices[0..16]', JSON.stringify(Array.from(skinned.skeleton.boneMatrices.slice(0, 16))));
    console.log('boneMatrices[16..32]', JSON.stringify(Array.from(skinned.skeleton.boneMatrices.slice(16, 32))));
    skinned.computeBoundingBox();
    console.log('bbox', JSON.stringify(skinned.boundingBox.min.toArray()), JSON.stringify(skinned.boundingBox.max.toArray()));
    const v = new (Object.getPrototypeOf(skinned.position).constructor)();
    v.fromBufferAttribute(skinned.geometry.attributes.position, 0);
    skinned.applyBoneTransform(0, v);
    console.log('applyBoneTransform(0)', JSON.stringify(v.toArray()));
    res();
  }, rej));
}
