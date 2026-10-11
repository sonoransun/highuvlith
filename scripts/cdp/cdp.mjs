// Script-driven headless Chrome session (the baseline QA driver, vendored).
// Usage: node scripts/cdp/cdp.mjs <script.json>
// script.json: { "width": 1280, "height": 900, "dark": false, "mobile": false, "print": false,
//                "reduced": false, "steps": [
//   {"goto": "http://127.0.0.1:8793/highuvlith/"},   // navigate and wait for the load event
//   {"wait": 2000},                                   // sleep ms
//   {"settle": 300},                                  // wait for MathJax/mermaid/images, then ms
//   {"eval": "document.title", "label": "title"},     // prints: eval> <label or expr> => <JSON>
//   {"shot": "/abs/path/file.png", "full": true},     // screenshot (full page when "full")
//   {"scroll": 800},                                  // window.scrollTo(0, y)
//   {"media": {"dark": true, "reducedMotion": true, "media": "print"}},   // re-emulate
//   {"mouse": [x, y]}, {"click": [x, y]}, {"key": "ArrowRight"} ] }
// Console errors/warnings, uncaught exceptions and failed network requests are printed as
// "console.<type>: ...", "EXCEPTION: ..." and "log.<level>: ..." lines.
// "dark": true emulates prefers-color-scheme: dark (Material's auto palette follows it;
// picture elements pick their dark source). "mobile": true adds a touch/mobile viewport.
import { readFileSync } from "node:fs";
import { launch } from "./driver.mjs";

const file = process.argv[2];
if (!file) { console.error("usage: node scripts/cdp/cdp.mjs <script.json>"); process.exit(2); }
const script = JSON.parse(readFileSync(file, "utf8"));
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
let page;
try {
  page = await launch({
    width: script.width, height: script.height, dark: !!script.dark, mobile: !!script.mobile,
    reducedMotion: !!script.reduced, media: script.print ? "print" : "", verbose: true,
  });
} catch (e) {
  console.error(e.message);
  process.exit(2);
}
try {
  for (const step of script.steps || []) {
    if (step.goto) await page.navigate(step.goto);
    else if (step.wait) await sleep(step.wait);
    else if (step.settle !== undefined) await page.settle({ extra: Number(step.settle) || 0 });
    else if (step.eval !== undefined) {
      const label = step.label || step.eval.slice(0, 80).replace(/\n/g, " ");
      try { console.log(`eval> ${label} => ${JSON.stringify(await page.evaluate(step.eval))}`); }
      catch (e) { console.log(`eval> ${label} => ERROR ${e.message}`); }
    }
    else if (step.scroll !== undefined) await page.scrollTo(step.scroll);
    else if (step.media) await page.setMedia(step.media);
    else if (step.mouse) await page.mouse(step.mouse[0], step.mouse[1]);
    else if (step.click) await page.click(step.click[0], step.click[1]);
    else if (step.key) await page.key(step.key);
    else if (step.shot) { await page.screenshot(step.shot, { full: !!step.full }); console.log(`shot> ${step.shot}`); }
  }
} catch (e) {
  console.log(`DRIVER ERROR: ${e.message}`);
} finally {
  await page.close();
}
