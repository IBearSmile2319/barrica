// Graba tools/promo/promo.html cuadro a cuadro y genera el video de presentación:
//   site/video/barrica.mp4         1280×720, 30 fps, H.264 (la web)
//   site/video/barrica-poster.jpg  el cuadro de portada
//   site/video/barrica.gif         versión corta y ligera para el README
//
// Requisitos: Node con Playwright (npm i -D playwright) y ffmpeg con libx264.
//   node tools/promo/render.js
const { chromium } = require(process.env.PLAYWRIGHT || "playwright");
const { execFileSync } = require("child_process");
const fs = require("fs");
const os = require("os");
const path = require("path");

const ROOT = path.resolve(__dirname, "../..");
const OUT = path.join(ROOT, "site/video");
const FPS = 30;
const SECONDS = 28;

(async () => {
  const frames = fs.mkdtempSync(path.join(os.tmpdir(), "barrica-promo-"));
  const browser = await chromium.launch();
  const page = await browser.newPage({ viewport: { width: 1280, height: 720 } });
  await page.goto("file://" + path.join(__dirname, "promo.html") + "?render");
  await page.evaluate(() => document.fonts.ready);
  await page.waitForFunction(() => [...document.images].every(i => i.complete));
  await page.evaluate(() => document.getAnimations().forEach(a => a.pause()));
  const total = FPS * SECONDS;
  for (let f = 0; f < total; f++) {
    const ms = (f * 1000) / FPS;
    await page.evaluate(t => document.getAnimations().forEach(a => (a.currentTime = t)), ms);
    await page.screenshot({ path: path.join(frames, `f${String(f).padStart(4, "0")}.png`) });
    if (f % FPS === 0) process.stdout.write(`\r${f / FPS}/${SECONDS} s`);
  }
  await browser.close();
  process.stdout.write("\n");

  fs.mkdirSync(OUT, { recursive: true });
  const ff = (...args) => execFileSync("ffmpeg", ["-hide_banner", "-loglevel", "error", "-y", ...args], { stdio: "inherit" });
  const input = ["-framerate", String(FPS), "-i", path.join(frames, "f%04d.png")];
  ff(...input, "-c:v", "libx264", "-preset", "slow", "-crf", "20", "-pix_fmt", "yuv420p", "-movflags", "+faststart", path.join(OUT, "barrica.mp4"));
  ff("-i", path.join(frames, "f0045.png"), "-q:v", "3", path.join(OUT, "barrica-poster.jpg"));
  // GIF: 600 px, 10 fps y paleta propia para que pese poco y se vea bien en GitHub.
  const palette = path.join(frames, "palette.png");
  const scale = "fps=10,scale=600:-1:flags=lanczos";
  ff(...input, "-vf", `${scale},palettegen=stats_mode=diff`, palette);
  ff(...input, "-i", palette, "-lavfi", `${scale}[x];[x][1:v]paletteuse=dither=bayer:bayer_scale=4`, path.join(OUT, "barrica.gif"));
  fs.rmSync(frames, { recursive: true, force: true });
  console.log("Listo en", OUT);
})();
