// Renders one glTF file to a PNG frame sequence with Chromium and three.js.
const fs = require('fs');
const os = require('os');
const path = require('path');
const puppeteer = require('puppeteer');

const [, , input, outDir, frames = 90, width = 640, height = 360] = process.argv;
if (!input || !outDir) {
  console.error('usage: node render.js <model.gltf|glb> <frame-dir> [frames] [width] [height]');
  process.exit(2);
}
// `three` publishes no `exports` map for these builds, so they are read from
// the folder this script is installed in rather than through require.resolve.
const vendor = path.join(__dirname, 'node_modules', 'three');
const three = fs.readFileSync(path.join(vendor, 'build', 'three.min.js'), 'utf8');
const loader = fs.readFileSync(path.join(vendor, 'examples', 'js', 'loaders', 'GLTFLoader.js'), 'utf8');
const model = fs.readFileSync(path.resolve(input), 'utf8');

function chromePath() {
  if (process.env.OR_CHROME_PATH) return process.env.OR_CHROME_PATH;
  return [
    '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',
    '/usr/bin/google-chrome', '/usr/bin/chromium', '/usr/bin/chromium-browser',
    'C:/Program Files/Google/Chrome/Application/chrome.exe',
  ].find((candidate) => fs.existsSync(candidate));
}

// The scripts are injected by token rather than by template interpolation:
// three.min.js contains backticks and `${`, which would end the template.
const html = [
  '<!doctype html><html><head><meta charset="utf-8">',
  '<style>html,body{margin:0;background:#0b0b0d}canvas{display:block}</style></head>',
  '<body><script>__THREE__</script><script>__LOADER__</script><script>',
  'const scene = new THREE.Scene();',
  'const camera = new THREE.PerspectiveCamera(35, 640 / 360, 0.1, 100);',
  'camera.position.set(3.4, 2.6, 4.2);',
  'camera.lookAt(0, 0, 0);',
  'const renderer = new THREE.WebGLRenderer({ antialias: true });',
  'renderer.setSize(640, 360);',
  'renderer.setPixelRatio(1);',
  'document.body.appendChild(renderer.domElement);',
  'scene.add(new THREE.HemisphereLight(0xffffff, 0x202028, 1.1));',
  'const key = new THREE.DirectionalLight(0xffffff, 1.4);',
  'key.position.set(2.5, 4, 3);',
  'scene.add(key);',
  'const root = new THREE.Group();',
  'scene.add(root);',
  'window.frame = (index) => {',
  '  root.rotation.y = (index / __FRAMES__) * Math.PI * 2;',
  '  root.rotation.x = 0.22;',
  '  renderer.render(scene, camera);',
  '  return true;',
  '};',
  'window.ready = false;',
  'const parsed = __MODEL__;',
  'new THREE.GLTFLoader().parse(parsed, "", (gltf) => {',
  '  root.add(gltf.scene);',
  '  window.ready = true;',
  '}, (error) => { console.error("gltf", error && error.message); });',
  '</script></body></html>',
].join('\n')
  .split('__THREE__').join(three)
  .split('__LOADER__').join(loader)
  .split('__MODEL__').join(JSON.stringify(model))
  .split('__FRAMES__').join(String(frames));

(async () => {
  const executable = chromePath();
  const browser = await puppeteer.launch(executable ? { executablePath: executable, args: ['--no-sandbox'] } : { args: ['--no-sandbox'] });
  const page = await browser.newPage();
  page.on('console', (message) => console.log('[page]', message.text()));
  page.on('pageerror', (error) => console.log('[pageerror]', error.message));
  await page.setViewport({ width: Number(width), height: Number(height), deviceScaleFactor: 1 });
  await page.setContent(html, { waitUntil: 'load' });
  await page.waitForFunction('window.ready === true', { timeout: 20000 });
  fs.mkdirSync(outDir, { recursive: true });
  for (let index = 0; index < Number(frames); index += 1) {
    await page.evaluate((i) => window.frame(i), index);
    const file = path.join(outDir, `frame-${String(index).padStart(4, '0')}.png`);
    await page.screenshot({ path: file });
  }
  await browser.close();
  console.log(`rendered ${frames} frames into ${outDir}`);
})().catch((error) => {
  console.error(error);
  process.exit(1);
});
