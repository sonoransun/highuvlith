// Minimal Chrome DevTools Protocol driver for headless site QA.
//
// No npm dependencies: it uses Node's global fetch and WebSocket (Node >= 22). Chrome is
// found through $CHROME, then the usual install paths and PATH names (findChrome()).
//
//   import { launch } from "./driver.mjs";
//   const page = await launch({ width: 1280, height: 900, dark: false });
//   await page.navigate("http://127.0.0.1:8793/highuvlith/");
//   await page.settle();                                   // load + MathJax + mermaid + images
//   const title = await page.evaluate("document.title");   // any expression; promises awaited
//   await page.setMedia({ reducedMotion: true });          // prefers-reduced-motion: reduce
//   await page.setMedia({ media: "print" });               // print emulation
//   await page.screenshot("/tmp/page.png", { full: true });
//   await page.close();
//
// Console errors/warnings, uncaught exceptions and failed network requests are collected
// in page.log (and printed when { verbose: true }); page.resetLog() clears it per page.

import { spawn } from "node:child_process";
import { accessSync, constants, existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { delimiter, join } from "node:path";

const CHROME_CANDIDATES = [
  "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
  "/Applications/Chromium.app/Contents/MacOS/Chromium",
  "/usr/bin/google-chrome",
  "/usr/bin/google-chrome-stable",
  "/usr/bin/chromium",
  "/usr/bin/chromium-browser",
  "/opt/google/chrome/chrome",
  "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe",
];
const CHROME_NAMES = ["google-chrome", "google-chrome-stable", "chromium", "chromium-browser", "chrome"];

function executable(p) {
  try { accessSync(p, constants.X_OK); return true; } catch { return false; }
}

/** The Chrome binary to use: $CHROME, a known install path, or a PATH name. */
export function findChrome() {
  if (process.env.CHROME && executable(process.env.CHROME)) return process.env.CHROME;
  for (const p of CHROME_CANDIDATES) if (executable(p)) return p;
  const dirs = (process.env.PATH || "").split(delimiter).filter(Boolean);
  for (const name of CHROME_NAMES) for (const dir of dirs) {
    const p = join(dir, name);
    if (executable(p)) return p;
  }
  return null;
}

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

/** Launch headless Chrome and attach to its first page target. */
export async function launch(opts = {}) {
  const width = opts.width || 1280, height = opts.height || 900;
  const chromePath = opts.chrome || findChrome();
  if (!chromePath) throw new Error("Chrome not found: set $CHROME to the browser binary");
  const profile = mkdtempSync(join(tmpdir(), "huv-cdp-"));
  const args = [
    "--headless=new", "--disable-gpu", "--no-first-run", "--no-default-browser-check",
    "--disable-extensions", "--disable-background-networking", "--disable-sync",
    "--remote-debugging-port=0", `--user-data-dir=${profile}`, "--hide-scrollbars",
    `--window-size=${width},${height}`,
  ];
  if (process.env.CI || process.env.CHROME_NO_SANDBOX || (process.getuid && process.getuid() === 0)) {
    args.push("--no-sandbox", "--disable-dev-shm-usage");
  }
  args.push("about:blank");
  const chrome = spawn(chromePath, args, { stdio: "ignore" });
  let port = 0;
  const portFile = join(profile, "DevToolsActivePort");
  for (let i = 0; i < 150 && !port; i++) {
    if (existsSync(portFile)) {
      const first = readFileSync(portFile, "utf8").split("\n")[0].trim();
      port = Number(first) || 0;
    }
    if (!port) await sleep(100);
  }
  let targets = null;
  for (let i = 0; i < 100 && port; i++) {
    try {
      targets = await (await fetch(`http://127.0.0.1:${port}/json`)).json();
      if (targets.some((t) => t.type === "page")) break;
    } catch { /* not up yet */ }
    await sleep(100);
  }
  if (!port || !targets || !targets.some((t) => t.type === "page")) {
    chrome.kill();
    rmSync(profile, { recursive: true, force: true });
    throw new Error(`Chrome did not expose a DevTools page (${chromePath})`);
  }
  const target = targets.find((t) => t.type === "page");
  const ws = new WebSocket(target.webSocketDebuggerUrl);
  await new Promise((resolve, reject) => {
    ws.addEventListener("open", resolve, { once: true });
    ws.addEventListener("error", (e) => reject(new Error(`DevTools websocket: ${e.message || e}`)), { once: true });
  });

  let seq = 0;
  const pending = new Map();
  const loadWaiters = [];
  const log = [];
  const verbose = !!opts.verbose;
  const note = (line) => { log.push(line); if (verbose) console.log(line); };
  ws.addEventListener("message", (ev) => {
    const msg = JSON.parse(ev.data);
    if (msg.id && pending.has(msg.id)) { pending.get(msg.id)(msg); pending.delete(msg.id); return; }
    switch (msg.method) {
      case "Page.loadEventFired":
        while (loadWaiters.length) loadWaiters.shift()();
        break;
      case "Runtime.consoleAPICalled":
        if (["error", "warning", "assert"].includes(msg.params.type)) {
          const text = msg.params.args.map((a) => a.value ?? a.description ?? "").join(" ");
          note(`console.${msg.params.type}: ${text.slice(0, 300)}`);
        }
        break;
      case "Runtime.exceptionThrown": {
        const d = msg.params.exceptionDetails;
        const where = d.url ? ` (${d.url}:${d.lineNumber})` : "";
        note(`EXCEPTION: ${((d.exception && d.exception.description) || d.text || "").slice(0, 300)}${where}`);
        break;
      }
      case "Log.entryAdded": {
        const e = msg.params.entry;
        if (["error", "warning"].includes(e.level)) note(`log.${e.level}: ${e.text} ${e.url || ""}`.slice(0, 300));
        break;
      }
      default:
    }
  });
  const send = (method, params = {}) => new Promise((resolve, reject) => {
    const id = ++seq;
    pending.set(id, (m) => (m.error ? reject(new Error(`${method}: ${JSON.stringify(m.error)}`)) : resolve(m.result)));
    ws.send(JSON.stringify({ id, method, params }));
  });

  await send("Page.enable");
  await send("Runtime.enable");
  await send("Log.enable");
  await send("Network.enable");

  const state = { width, height, mobile: !!opts.mobile, scheme: opts.dark ? "dark" : "light", reduced: !!opts.reducedMotion, media: opts.media || "" };
  async function applyViewport() {
    await send("Emulation.setDeviceMetricsOverride", { width: state.width, height: state.height, deviceScaleFactor: 1, mobile: state.mobile });
    await send("Emulation.setTouchEmulationEnabled", { enabled: state.mobile });
  }
  async function applyMedia() {
    await send("Emulation.setEmulatedMedia", {
      media: state.media,
      features: [
        { name: "prefers-color-scheme", value: state.scheme },
        { name: "prefers-reduced-motion", value: state.reduced ? "reduce" : "no-preference" },
      ],
    });
  }
  await applyViewport();
  await applyMedia();

  const page = {
    chromePath,
    log,
    state,
    send,
    resetLog() { log.length = 0; },
    /** Change the viewport; { width, height, mobile }. */
    async setViewport(v) { Object.assign(state, v); await applyViewport(); },
    /** Change emulated media; { dark, reducedMotion, media } ("print" | "screen" | ""). */
    async setMedia(m) {
      if (m.dark !== undefined) state.scheme = m.dark ? "dark" : "light";
      if (m.reducedMotion !== undefined) state.reduced = !!m.reducedMotion;
      if (m.media !== undefined) state.media = m.media || "";
      await applyMedia();
    },
    /** Navigate and wait for the load event (timeout in ms). */
    async navigate(url, timeout = 20000) {
      const loaded = new Promise((r) => loadWaiters.push(r));
      await send("Page.navigate", { url });
      let timer;
      const ok = await Promise.race([loaded.then(() => true), new Promise((r) => { timer = setTimeout(() => r(false), timeout); })]);
      clearTimeout(timer);
      if (!ok) note(`warn: load event timeout for ${url}`);
      return ok;
    },
    /** Evaluate an expression in the page (promises awaited, value returned by JSON). */
    async evaluate(expression) {
      const r = await send("Runtime.evaluate", { expression, returnByValue: true, awaitPromise: true });
      if (r.exceptionDetails) {
        const d = r.exceptionDetails;
        throw new Error(`evaluate: ${(d.exception && d.exception.description) || d.text}`);
      }
      return r.result.value;
    },
    /** Wait until the page is quiet: readyState complete, MathJax typeset, mermaid rendered,
     *  images decoded (each with its own timeout), then `extra` ms. */
    async settle({ extra = 300, timeout = 8000 } = {}) {
      await page.evaluate(`(async () => {
        const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
        const until = async (test, limit) => { const t0 = Date.now(); while (!test() && Date.now() - t0 < limit) await sleep(100); };
        if (document.readyState !== "complete") await new Promise((r) => window.addEventListener("load", r, { once: true }));
        if (window.MathJax && MathJax.startup && MathJax.startup.promise) { try { await MathJax.startup.promise; } catch (e) {} }
        await until(() => ![...document.querySelectorAll(".arithmatex")].some((e) => !e.querySelector("mjx-container") && /\\\\[(\\[]/.test(e.textContent)), ${timeout});
        await until(() => !document.querySelector("pre.mermaid"), ${timeout});
        const imgs = [...document.images].filter((i) => !i.complete && i.loading !== "lazy");
        await Promise.race([Promise.all(imgs.map((i) => new Promise((r) => { i.addEventListener("load", r, { once: true }); i.addEventListener("error", r, { once: true }); }))), sleep(${timeout})]);
        await sleep(${extra});
        return true;
      })()`);
    },
    async scrollTo(y) { await page.evaluate(`window.scrollTo(0, ${Number(y) || 0})`); await sleep(150); },
    async click(x, y) {
      await send("Input.dispatchMouseEvent", { type: "mouseMoved", x, y });
      await send("Input.dispatchMouseEvent", { type: "mousePressed", x, y, button: "left", clickCount: 1 });
      await send("Input.dispatchMouseEvent", { type: "mouseReleased", x, y, button: "left", clickCount: 1 });
    },
    async mouse(x, y) { await send("Input.dispatchMouseEvent", { type: "mouseMoved", x, y }); },
    async key(key) {
      await send("Input.dispatchKeyEvent", { type: "keyDown", key });
      await send("Input.dispatchKeyEvent", { type: "keyUp", key });
    },
    /** PNG screenshot; { full: true } captures the whole document (height capped at 30000). */
    async screenshot(path, { full = false } = {}) {
      let restore = false;
      if (full) {
        const m = await send("Page.getLayoutMetrics");
        const h = Math.min(Math.ceil((m.cssContentSize || m.contentSize).height), 30000);
        await send("Emulation.setDeviceMetricsOverride", { width: state.width, height: h, deviceScaleFactor: 1, mobile: state.mobile });
        await sleep(300);
        restore = true;
      }
      const shot = await send("Page.captureScreenshot", { format: "png", captureBeyondViewport: full });
      writeFileSync(path, Buffer.from(shot.data, "base64"));
      if (restore) { await applyViewport(); await sleep(100); }
      return path;
    },
    async close() {
      try { ws.close(); } catch { /* already closed */ }
      chrome.kill();
      await sleep(100);
      try { rmSync(profile, { recursive: true, force: true }); } catch { /* best effort */ }
    },
  };
  return page;
}
