// Graba brag.html cuadro a cuadro con el Chrome instalado (Playwright, sin descargar navegadores).
//   node render.js stills 1.2 3.9 …   capturas sueltas en stills/ para revisar
//   node render.js                    todos los cuadros en frames/ (30 fps, 20,727 s)
const { chromium } = require("playwright");
const fs = require("fs");
const path = require("path");

const FPS = 30;
const SECONDS = 20.727;

(async () => {
  const stills = process.argv[2] === "stills";
  const times = stills ? process.argv.slice(3).map(Number) : null;
  const out = path.join(__dirname, stills ? "stills" : "frames");
  fs.rmSync(out, { recursive: true, force: true });
  fs.mkdirSync(out, { recursive: true });
  const browser = await chromium.launch({ channel: "chrome" });
  const page = await browser.newPage({ viewport: { width: 1920, height: 1080 } });
  await page.goto("file://" + path.join(__dirname, "brag.html") + "?render");
  await page.evaluate(() => document.fonts.ready);
  await page.waitForFunction(() => [...document.images].every(i => i.complete && i.naturalWidth > 0));
  await page.evaluate(() => document.getAnimations().forEach(a => a.pause()));
  const shot = async (ms, file) => {
    await page.evaluate(t => document.getAnimations().forEach(a => (a.currentTime = t)), ms);
    await page.screenshot({ path: path.join(out, file) });
  };
  if (stills) {
    for (const s of times) await shot(s * 1000, `t${s.toFixed(2)}.png`);
  } else {
    const total = Math.round(FPS * SECONDS);
    for (let f = 0; f < total; f++) {
      await shot((f * 1000) / FPS, `f${String(f).padStart(4, "0")}.png`);
      if (f % FPS === 0) process.stdout.write(`\r${f / FPS}/${Math.round(SECONDS)} s`);
    }
    process.stdout.write("\n");
  }
  await browser.close();
})();
