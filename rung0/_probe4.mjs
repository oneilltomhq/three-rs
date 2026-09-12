import puppeteer from 'puppeteer';
import * as fs from 'fs/promises';
import { createServer } from '../../utils/server.js';
import { Image } from './image.js';
const server = createServer(); await new Promise(r => server.listen(1236, r));
const injection = await fs.readFile('test/e2e/deterministic-injection.js','utf8');
const cleanPage = await fs.readFile('test/e2e/clean-page.js','utf8');
const base = ['--hide-scrollbars','--ignore-gpu-blocklist','--disable-gpu-driver-bug-workarounds','--disable-gpu-watchdog','--no-sandbox'];
const cfgs = {
  lvp_upstream: { lvp: true, args: ['--enable-unsafe-webgpu','--enable-features=Vulkan','--disable-vulkan-surface'] },
  intel_upstream: { lvp: false, args: ['--enable-unsafe-webgpu','--enable-features=Vulkan','--disable-vulkan-surface'] },
  intel_angle_vk: { lvp: false, args: ['--enable-unsafe-webgpu','--enable-features=Vulkan','--use-angle=vulkan','--disable-vulkan-surface'] },
  lvp_angle_vk: { lvp: true, args: ['--enable-unsafe-webgpu','--enable-features=Vulkan','--use-angle=vulkan','--disable-vulkan-surface'] },
};
const file = process.argv[2] || 'webgpu_rtt';
const out = '/tmp/claude-1000/-home-tom-src-projects-three-rs/77c2d8d8-7d7b-4e8b-82c7-0fcdad29b224/scratchpad/probe4';
await fs.mkdir(out, { recursive: true });
function stats(img){ let nz=0; const d=img.data; for(let i=0;i<d.length;i+=4){ if(d[i]>8||d[i+1]>8||d[i+2]>8) nz++; } return (100*nz/(d.length/4)).toFixed(1)+'% non-black'; }
for (const [name, c] of Object.entries(cfgs)) {
  const env = { ...process.env }; if (c.lvp) env.VK_DRIVER_FILES = '/usr/share/vulkan/icd.d/lvp_icd.x86_64.json';
  let browser;
  try {
    browser = await puppeteer.launch({ headless: true, env, args: [...base, ...c.args], defaultViewport: { width: 800, height: 500 }, protocolTimeout: 0 });
    const page = await browser.newPage();
    const logs = [];
    page.on('console', m => logs.push(m.type() + ': ' + m.text().slice(0, 160)));
    page.on('pageerror', e => logs.push('pageerror: ' + e.message.slice(0, 160)));
    await page.evaluateOnNewDocument(injection);
    await page.goto(`http://localhost:1236/examples/${file}.html`, { waitUntil: 'networkidle0', timeout: 120000 });
    const adapter = await page.evaluate(async () => { const a = await navigator.gpu?.requestAdapter(); return a ? (a.info.vendor + '/' + a.info.architecture) : 'none'; });
    await page.evaluate(cleanPage);
    await page.waitForNetworkIdle({ idleTime: 2000, timeout: 120000 });
    await page.evaluate(async () => { window._renderStarted = true; await new Promise((res, rej) => { const t0 = performance._now(); const id = setInterval(() => { if (window._renderFinished) { clearInterval(id); res(); } else if (performance._now() - t0 > 5000) { clearInterval(id); rej('timeout'); } }, 100); }); }).catch(e => logs.push('render: ' + e));
    const results = [];
    for (const delay of [0, 500, 2000]) {
      await new Promise(r => setTimeout(r, delay));
      const buf = await page.screenshot();
      const img = await Image.read(buf);
      await fs.writeFile(`${out}/${file}-${name}-${delay}.png`, buf);
      results.push(`+${delay}ms ${stats(img)}`);
    }
    console.log(`${name.padEnd(16)} adapter=${adapter.padEnd(22)} ${results.join(' | ')}`);
    for (const l of logs.slice(0, 6)) console.log('    ' + l);
  } catch (e) { console.log(name, 'ERR', e.message.slice(0, 200)); }
  if (browser) { const p = browser.process(); p?.kill('SIGKILL'); }
}
server.close();
