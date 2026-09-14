// Runs three.js' own comparator, unchanged, over our rendered frame.
//
// Mirrors `test/e2e/puppeteer.js` checkFile():
//
//   const screenshot = ( await Image.read( await page.screenshot() ) ).scale( 1 / viewScale );
//   expected = await Image.read( `examples/screenshots/${ file }.jpg` );
//   const diff = screenshot.clone();
//   numDifferentPixels = expected.compare( screenshot, diff, pixelThreshold );
//   differentPixels = numDifferentPixels / ( actual.width * actual.height ) * 100;
//   pass = differentPixels < maxDifferentPixels
//
// The actual image goes through no JPEG round-trip before compare(), exactly as
// in the grader: `page.screenshot()` is a PNG and is compared as decoded pixels.

import * as path from 'path';

const [ threeDir, actualPath, expectedPath, outDir ] = process.argv.slice( 2 );

const { Image } = await import( path.join( threeDir, 'test/e2e/image.js' ) );

const viewScale = 2;
const pixelThreshold = 0.1;
const maxDifferentPixels = 0.1;
const jpgQuality = 95;

const screenshot = ( await Image.read( actualPath ) ).scale( 1 / viewScale );
const expected = await Image.read( expectedPath );

const actual = screenshot.bitmap;
const diff = screenshot.clone();

const numDifferentPixels = expected.compare( screenshot, diff, pixelThreshold );
const differentPixels = numDifferentPixels / ( actual.width * actual.height ) * 100;

await screenshot.write( path.join( outDir, 'actual.jpg' ), jpgQuality );
await expected.write( path.join( outDir, 'expected.jpg' ), jpgQuality );
await diff.write( path.join( outDir, 'diff.jpg' ), jpgQuality );

console.log( JSON.stringify( {
	width: actual.width,
	height: actual.height,
	numDifferentPixels,
	differentPixels,
	maxDifferentPixels,
	pass: differentPixels < maxDifferentPixels
} ) );
