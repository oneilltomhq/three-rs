// Emit a Rust `GeometrySample` literal for a three.js geometry.
// usage: node sample.mjs '<name>' '<js expression returning a geometry>'
const THREE = await import( '/home/tom/src/vendor/three.js/src/Three.js' );
const { TeapotGeometry } = await import( '/home/tom/src/vendor/three.js/examples/jsm/geometries/TeapotGeometry.js' );
Object.assign( globalThis, THREE, { TeapotGeometry } );

const name = process.argv[ 2 ];
const expr = process.argv[ 3 ];
const geom = eval( expr );

const N = 4; // items sampled from each end

function num( v ) {
	if ( Object.is( v, -0 ) ) return '-0.0';
	if ( Number.isInteger( v ) ) return v.toFixed( 1 );
	return v.toPrecision( 17 );
}

function attr( a, itemSize ) {
	if ( a === undefined ) return 'None';
	const arr = a.array;
	const n = Math.min( N * itemSize, arr.length );
	const head = Array.from( arr.slice( 0, n ) ).map( num );
	const tail = Array.from( arr.slice( Math.max( 0, arr.length - n ) ) ).map( num );
	return `Some(AttrSample { count: ${a.count}, head: &[${head.join( ', ' )}], tail: &[${tail.join( ', ' )}] })`;
}

function idx( i ) {
	if ( i === null ) return 'None';
	const arr = i.array;
	const n = Math.min( 12, arr.length );
	const head = Array.from( arr.slice( 0, n ) );
	const tail = Array.from( arr.slice( Math.max( 0, arr.length - n ) ) );
	const kind = arr.constructor.name === 'Uint16Array' ? 'U16' : 'U32';
	return `Some(IndexSample { kind: IndexKind::${kind}, count: ${arr.length}, head: &[${head.join( ', ' )}], tail: &[${tail.join( ', ' )}] })`;
}

const groups = geom.groups.map( g => `Group { start: ${g.start}, count: ${g.count}, material_index: ${g.materialIndex} }` );

geom.computeBoundingBox();
geom.computeBoundingSphere();
const bb = geom.boundingBox, bs = geom.boundingSphere;

console.log( `const ${name}: GeometrySample = GeometrySample {` );
console.log( `    expr: ${JSON.stringify( expr )},` );
console.log( `    position: ${attr( geom.attributes.position, 3 )},` );
console.log( `    normal: ${attr( geom.attributes.normal, 3 )},` );
console.log( `    uv: ${attr( geom.attributes.uv, 2 )},` );
console.log( `    index: ${idx( geom.index )},` );
console.log( `    groups: &[${groups.join( ', ' )}],` );
console.log( `    bounding_box: (&[${num( bb.min.x )}, ${num( bb.min.y )}, ${num( bb.min.z )}], &[${num( bb.max.x )}, ${num( bb.max.y )}, ${num( bb.max.z )}]),` );
console.log( `    bounding_sphere: (&[${num( bs.center.x )}, ${num( bs.center.y )}, ${num( bs.center.z )}], ${num( bs.radius )}),` );
console.log( `};` );
