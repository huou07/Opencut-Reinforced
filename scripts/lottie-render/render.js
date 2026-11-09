// Renders one Lottie animation to a video file or an image sequence.
//
// Opencut Reinforced owns no Lottie runtime: Chromium draws the animation with
// lottie-web, puppeteer-lottie captures the frames, and FFmpeg writes them. OR
// only ever sees the file this produces plus the provenance record beside it,
// so the source animation stays the thing the user edits.
//
// Usage: node render.js <animation.json> <output> [width] [height]
const fs = require('fs');
const path = require('path');
const renderLottie = require('puppeteer-lottie');

const [, , input, output, width = 640, height = 360] = process.argv;
if (!input || !output) {
  console.error('usage: node render.js <animation.json> <output> [width] [height]');
  process.exit(2);
}

// puppeteer-lottie-cli 1.0.9 exposes no launch options, and its pinned Chromium
// is x64-only on some releases, so an installed browser is preferred and the
// bundled download is left as the fallback.
function chromePath() {
  if (process.env.OR_CHROME_PATH) return process.env.OR_CHROME_PATH;
  return [
    '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',
    '/usr/bin/google-chrome',
    '/usr/bin/chromium',
    '/usr/bin/chromium-browser',
    'C:/Program Files/Google/Chrome/Application/chrome.exe',
    'C:/Program Files (x86)/Google/Chrome/Application/chrome.exe',
  ].find((candidate) => fs.existsSync(candidate));
}

const executable = chromePath();
renderLottie({
  animationData: JSON.parse(fs.readFileSync(path.resolve(input), 'utf8')),
  output: path.resolve(output),
  width: Number(width),
  height: Number(height),
  quiet: false,
  puppeteerOptions: executable ? { executablePath: executable } : {},
})
  .then(() => {
    console.log(`rendered ${output}`);
  })
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });
