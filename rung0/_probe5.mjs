import puppeteer from 'puppeteer';
import * as fs from 'fs/promises';
import { createServer } from '../../utils/server.js';
import { Image } from './image.js';
const server = createServer(); await new Promise(r => server.listen(1237, r));
const injection = await fs.readFile('test/e2e/deterministic-injection.js','utf8');
const base = ['--hide-scrollbars','--ignore-gpu-blocklist','--disable-gpu-driver-bug-workarounds','--disable-gpu-watchdog','--no-sandbox'];
const cfgs = {
  lvp_upstream: { lvp: true, args: ['--enable-unsafe-webgpu','--enable-features=Vulkan','--disable-vulkan-surface'] },
  intel_angle_vk: { lvp: false, args: ['--enable-unsafe-webgpu','--enable-features=Vulkan','--use-angle=vulkan','--disable-vulkan-surface'] },
  intel_angle_vk_surface: { lvp: false, args: ['--enable-unsafe-webgpu','--enable-features=Vulkan','--use-angle=vulkan'] },
  intel_plain: { lvp: false, args: ['--enable-unsafe-webgpu'] },
};
function stats(img){ let nz=0; const d=img.data; for(let i=0;i<d.length;i+=4){ if(d[i]>8||d[i+1]>8||d[i+2]>8) nz++; } return (100*nz/(d.length/4)).toFixed(1)+'% non-black'; }
for (const inject of [false, true]) for (const [name, c] of Object.entries(cfgs)) {
  const env = { ...process.env }; if (c.lvp) env.VK_DRIVER_FILES = '/usr/share/vulkan/icd.d/lvp_icd.x86_64.json';
  let browser;
  try {
    browser = await puppeteer.launch({ headless: true, env, args: [...base, ...c.args], defaultViewport: { width: 800, height: 500 }, protocolTimeout: 0 });
    const page = await browser.newPage(); const logs = [];
    page.on('console', m => logs.push(m.type() + ': ' + m.text().slice(0, 120))); page.on('pageerror', e => logs.push('pageerror: ' + e.message.slice(0, 120)));
    if (inject) await page.evaluateOnNewDocument(injection);
    await page.goto('http://localhost:1237/examples/_probe/raw.html', { waitUntil: 'networkidle0' });
    if (inject) await page.evaluate(() => { window._renderStarted = true; });
    await new Promise(r => setTimeout(r, 1500));
    const done = await page.evaluate(() => window._rawDone);
    const img = await Image.read(await page.screenshot());
    console.log(`inject=${inject} ${name.padEnd(24)} frames=${done} ${stats(img)}  ${logs.join(' ;; ')}`);
  } catch (e) { console.log(name, 'ERR', e.message.slice(0, 200)); }
  browser?.process()?.kill('SIGKILL');
}
server.close();
