#!/usr/bin/env node
// Headless QA of the built documentation site, plus a github.com preview of Markdown files.
//
//   node scripts/qa_site.mjs --site http://127.0.0.1:8793/highuvlith --modes light,dark,print,phone --sample --out <dir>
//   node scripts/qa_site.mjs --site <url> --all --github-preview --out <dir>        # nightly / INT
//   node scripts/qa_site.mjs --github-preview README.md CONTRIBUTING.md docs/future/index.md   # no server
//   node scripts/qa_site.mjs --self-test                                            # fixtures prove the asserts fire
//
// Two classes of assert (DESIGN §10.4, m2):
//   STRUCTURAL — errors in every mode: html.scrollWidth <= clientWidth; mjx-merror = 0; no broken image;
//     exactly one visible <img> per #gh-*/#only-* pair (normal, reduced-motion, motion-toggle-on and print
//     emulation) and the hidden variant in another lightbox gallery; no visible a.glightbox without a data-gallery
//     or sharing its gallery with a hidden anchor (the gallery a reader opens holds only what they can see); no
//     rendered focusable element in <article> with a 0x0 box; no clipped div.arithmatex / .huv-diagram /
//     .md-typeset__scrollwrap / pre > code (cut-off content is always an error; a scrollable block without the
//     data-huv-overflow cue is an error once layout.js is loaded); every python fence has its marker comment;
//     [data-huv-mounted] = [data-huv-calc]; no details element whose text (outside pre/code) holds |---, ]( or ![.
//   PAGE-FORM — warnings unless HUV_QA_STRICT=1 or --strict (--tolerant forces warnings): a chip in every
//     p.huv-status; the hero figure within 900 px at 1280 px / two screens at 390 px; loading="lazy" after the
//     first figure; data-huv-section per path; image bytes and animated weight within §10.2; <= 1 autoplaying
//     raster; every -anim- image has a resolvable data-huv-still and shows its still under reduced motion; every
//     figure image is in the lightbox (a.glightbox, .huv-no-zoom opts out); no reveal effect leaves a figure at
//     opacity 0 on the first screen, under reduced motion or in print. Pairs inside figure[data-huv-fallback]
//     (calculator fallbacks hidden on mount) and closed <details> are skipped.
// The github.com preview (markdown-it 14.1.0 GFM, vendored under scripts/cdp/vendor/) checks the §10.1-8 leak
// heuristic, the collapsed-block form, <picture> structure and the verify-example output rule; see qa_lib.mjs.
// Output: a Markdown report (<out>/qa-report.md, plus qa-report.json) listing errors and warnings separately,
// full-page screenshots per page x mode (--all/--shots always, --sample on failure). Exit 1 on any error,
// 2 on a usage or driver problem.

import { existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, statSync, writeFileSync } from "node:fs";
import { createServer } from "node:http";
import { tmpdir } from "node:os";
import { basename, extname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { launch } from "./cdp/driver.mjs";
import {
  BUDGETS, GITHUB_PREVIEW_SAMPLE, MODES, SAMPLE_PAGES, findingsFromProbe, githubPreview, lightboxFindings, pageSlug,
  pairFindings, pairsExpression, probeExpression, sitemapPaths, wrapPreviewHtml,
} from "./cdp/qa_lib.mjs";

const HERE = fileURLToPath(new URL(".", import.meta.url));
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

function usage(code = 0) {
  const text = readFileSync(fileURLToPath(import.meta.url), "utf8").split("\n").filter((l) => l.startsWith("//")).slice(0, 28).map((l) => l.slice(3)).join("\n");
  (code ? console.error : console.log)(text);
  process.exit(code);
}

function parseArgs(args) {
  const o = { modes: ["light", "dark", "print", "phone"], pages: [], files: [], sample: false, all: false, preview: false, shots: false, strict: null, settle: 300, out: null, site: null, root: process.cwd(), chrome: null, selfTest: false, quiet: false };
  for (let i = 0; i < args.length; i++) {
    const a = args[i], next = () => { if (i + 1 >= args.length) usage(2); return args[++i]; };
    if (a === "--site") o.site = next().replace(/\/+$/, "");
    else if (a === "--modes") o.modes = next().split(",").map((s) => s.trim()).filter(Boolean);
    else if (a === "--sample") o.sample = true;
    else if (a === "--all") o.all = true;
    else if (a === "--pages") o.pages.push(...next().split(",").map((s) => s.trim()).filter(Boolean));
    else if (a === "--github-preview") o.preview = true;
    else if (a === "--out") o.out = next();
    else if (a === "--root") o.root = next();
    else if (a === "--shots") o.shots = true;
    else if (a === "--strict") o.strict = true;
    else if (a === "--tolerant") o.strict = false;
    else if (a === "--settle") o.settle = Number(next()) || 0;
    else if (a === "--chrome") o.chrome = next();
    else if (a === "--self-test") o.selfTest = true;
    else if (a === "--quiet" || a === "-q") o.quiet = true;
    else if (a === "-h" || a === "--help") usage(0);
    else if (a.startsWith("--")) { console.error(`unknown option ${a}`); usage(2); }
    else o.files.push(a);
  }
  for (const m of o.modes) if (!MODES[m]) { console.error(`unknown mode "${m}" (light, dark, print, phone)`); process.exit(2); }
  if (o.strict === null) o.strict = process.env.HUV_QA_STRICT === "1";
  if (!o.site && !o.preview && !o.selfTest) usage(2);
  if (!o.sample && !o.all && !o.pages.length) o.sample = true;
  return o;
}

// ------------------------------------------------------------------------------------------ site QA

async function fetchSitemap(site) {
  const url = `${site}/sitemap.xml`;
  const r = await fetch(url);
  if (!r.ok) throw new Error(`${url}: HTTP ${r.status}`);
  return sitemapPaths(await r.text());
}

async function runSite(o, report) {
  let paths;
  try { paths = await fetchSitemap(o.site); } catch (e) { console.error(`cannot read the sitemap: ${e.message}`); process.exit(2); }
  let selected;
  if (o.pages.length) selected = o.pages.map((p) => (p.startsWith("/") ? p : "/" + p)).map((p) => (p.endsWith("/") || /\.[a-z0-9]+$/i.test(p) ? p : p + "/"));
  else if (o.all) selected = paths;
  else {
    selected = SAMPLE_PAGES.filter((p) => paths.includes(p));
    for (const p of SAMPLE_PAGES) if (!paths.includes(p)) report.notes.push(`sample page ${p} is not in the sitemap (skipped)`);
  }
  report.pages = selected.length;
  const page = await launch({ ...MODES[o.modes[0]], chrome: o.chrome });
  try {
    for (const mode of o.modes) {
      const m = MODES[mode];
      await page.setViewport({ width: m.width, height: m.height, mobile: m.mobile });
      await page.setMedia({ dark: m.dark, media: m.media, reducedMotion: false });
      for (const path of selected) {
        const url = `${o.site}${path}`;
        const row = { page: path, mode, errors: [], warnings: [], info: [], screenshot: null };
        page.resetLog();
        const t0 = Date.now();
        try {
          await page.navigate(url);
          await page.settle({ extra: o.settle });
          // The site is dark by default and its palette ignores the OS preference, so the
          // emulated prefers-color-scheme no longer picks the scheme: set Material's scheme
          // attribute for the mode explicitly (light/print/phone = default, dark = slate).
          await page.evaluate(`document.body.setAttribute("data-md-color-scheme", ${m.dark ? '"slate"' : '"default"'}); true`);
          await page.settle({ extra: 150 });
          const probe = await page.evaluate(probeExpression({ path, mode }));
          const f = findingsFromProbe(probe, { path, mode, strict: o.strict });
          row.errors.push(...f.errors); row.warnings.push(...f.warnings); row.info.push(...f.info);
          if (mode === "light") await extraPasses(page, row, o);
        } catch (e) {
          row.errors.push({ rule: "driver", msg: `probe failed: ${e.message.slice(0, 200)}` });
        }
        for (const line of page.log) {
          if (/api\.github\.com/.test(line)) continue;
          if (/^EXCEPTION/.test(line)) row.warnings.push({ rule: "console", msg: line.slice(0, 200) });
          else if (/^log\.error/.test(line)) row.warnings.push({ rule: "console", msg: line.slice(0, 200) });
        }
        if (o.strict && row.warnings.length) { row.errors.push(...row.warnings.map((w) => ({ ...w, promoted: true }))); row.warnings = []; }
        if (o.all || o.shots || row.errors.length) {
          const file = join(o.out, `${pageSlug(path)}-${mode}.png`);
          try { await page.screenshot(file, { full: true }); row.screenshot = file; } catch (e) { row.warnings.push({ rule: "screenshot", msg: e.message.slice(0, 120) }); }
        }
        row.ms = Date.now() - t0;
        report.rows.push(row);
        if (!o.quiet) console.log(`${(row.errors.length ? "FAIL" : row.warnings.length ? "warn" : "ok  ")} ${mode.padEnd(5)} ${path.padEnd(34)} e=${row.errors.length} w=${row.warnings.length} ${row.info[0] || ""} (${row.ms} ms)`);
      }
    }
  } finally {
    await page.close();
  }
}

/** Light mode only: the pair rule under reduced-motion emulation, with the motion toggle pressed, and in print emulation. */
async function extraPasses(page, row, o) {
  const label = (s) => s;
  await page.setMedia({ reducedMotion: true });
  await sleep(400);
  let p = await page.evaluate(pairsExpression());
  row.errors.push(...pairFindings(p.pairs, label("reduced-motion")), ...lightboxFindings(p.anchors, "reduced-motion"));
  for (const a of p.anim) if (a.visible && !a.currentIsStill) row.warnings.push({ rule: "anim-still-swap", msg: `reduced-motion: ${a.src} still shows its animated source (expected the -still- swap)` });
  for (const f of p.faded || []) row.warnings.push({ rule: "effect-hidden", msg: `reduced-motion: figure "${f.text}" at opacity ${f.opacity} (the reveal effect must be disabled under prefers-reduced-motion)` });
  if (p.motionToggle) {
    await page.evaluate(`document.querySelector("button.huv-motion").click()`);
    await sleep(300);
    const t = await page.evaluate(pairsExpression());
    row.errors.push(...pairFindings(t.pairs, "toggle-on"), ...lightboxFindings(t.anchors, "toggle-on"));
    if (!t.motionPaused) row.warnings.push({ rule: "motion-toggle", msg: "pressing button.huv-motion did not set html[data-huv-motion=paused]" });
    for (const a of t.anim) if (a.visible && !a.currentIsStill) row.warnings.push({ rule: "anim-still-swap", msg: `toggle-on: ${a.src} still shows its animated source` });
    await page.evaluate(`document.querySelector("button.huv-motion").click()`);
    await sleep(100);
  } else {
    row.info.push("no motion toggle on this page (button.huv-motion absent)");
  }
  await page.setMedia({ reducedMotion: false, media: "print" });
  await sleep(400);
  p = await page.evaluate(pairsExpression());
  row.errors.push(...pairFindings(p.pairs, "print"), ...lightboxFindings(p.anchors, "print"));
  for (const f of p.faded || []) row.warnings.push({ rule: "effect-hidden", msg: `print: figure "${f.text}" at opacity ${f.opacity} (every figure prints)` });
  await page.setMedia({ media: "" });
  await sleep(100);
}

// ------------------------------------------------------------------------------------------ github preview

function listDocs(root) {
  const skip = new Set(["figures", "includes", "hooks", "overrides", "javascripts", "stylesheets", "assets", "__pycache__"]);
  const out = [];
  const walk = (dir) => {
    for (const name of readdirSync(dir)) {
      const p = join(dir, name);
      const st = statSync(p);
      if (st.isDirectory()) { if (!skip.has(name)) walk(p); }
      else if (name.endsWith(".md")) out.push(p);
    }
  };
  walk(join(root, "docs"));
  return out.sort();
}

function runPreview(o, report) {
  const files = o.files.length ? o.files.map((f) => resolve(o.root, f)) : (o.all ? listDocs(o.root) : GITHUB_PREVIEW_SAMPLE.map((f) => resolve(o.root, f)));
  const dir = join(o.out, "github-preview");
  mkdirSync(dir, { recursive: true });
  for (const file of files) {
    const rel = relative(o.root, file) || basename(file);
    const row = { page: rel, mode: "github", errors: [], warnings: [], info: [], screenshot: null };
    if (!existsSync(file)) { row.errors.push({ rule: "missing", msg: `${rel}: no such file` }); report.rows.push(row); continue; }
    const kind = /^(README|CONTRIBUTING)\.md$/i.test(basename(file)) ? "readme" : "docs";
    const text = readFileSync(file, "utf8");
    const r = githubPreview(text, { path: rel, kind });
    row.errors.push(...r.errors); row.warnings.push(...r.warnings);
    const html = join(dir, rel.replace(/^docs\//, "").replace(/[\/\\]/g, "-").replace(/\.md$/, "") + ".html");
    writeFileSync(html, wrapPreviewHtml(rel, r.html));
    row.screenshot = html;
    row.info.push(`kind=${kind} rendered=${html}`);
    if (o.strict && row.warnings.length) { row.errors.push(...row.warnings.map((w) => ({ ...w, promoted: true }))); row.warnings = []; }
    report.rows.push(row);
    if (!o.quiet) console.log(`${(row.errors.length ? "FAIL" : row.warnings.length ? "warn" : "ok  ")} gh    ${rel.padEnd(34)} e=${row.errors.length} w=${row.warnings.length}`);
  }
}

// ------------------------------------------------------------------------------------------ report

function writeReport(o, report) {
  const errors = report.rows.reduce((n, r) => n + r.errors.length, 0);
  const warnings = report.rows.reduce((n, r) => n + r.warnings.length, 0);
  const lines = [];
  lines.push("# qa_site report", "");
  lines.push(`- date: ${new Date().toISOString()}`);
  if (o.site) lines.push(`- site: ${o.site} · modes: ${o.modes.join(", ")} · pages: ${report.pages}`);
  if (o.preview) lines.push(`- github preview: ${report.rows.filter((r) => r.mode === "github").length} file(s)`);
  lines.push(`- mode: ${o.strict ? "STRICT (page-form findings are errors)" : "tolerant (page-form findings are warnings)"}`);
  lines.push(`- result: **${errors} error(s), ${warnings} warning(s)**`, "");
  for (const n of report.notes) lines.push(`- note: ${n}`);
  if (report.notes.length) lines.push("");
  lines.push("| page | mode | errors | warnings | info | artefact |", "|---|---|---|---|---|---|");
  for (const r of report.rows) lines.push(`| \`${r.page}\` | ${r.mode} | ${r.errors.length} | ${r.warnings.length} | ${(r.info[0] || "").replace(/\|/g, "/")} | ${r.screenshot ? "`" + basename(r.screenshot) + "`" : ""} |`);
  lines.push("", `## Errors (${errors})`, "");
  for (const r of report.rows) for (const e of r.errors) lines.push(`- **${r.page}** [${r.mode}] \`${e.rule}\`${e.promoted ? " (page-form, strict)" : ""} — ${e.msg}`);
  lines.push("", `## Warnings (${warnings})`, "");
  for (const r of report.rows) for (const w of r.warnings) lines.push(`- **${r.page}** [${r.mode}] \`${w.rule}\` — ${w.msg}`);
  const byRule = {};
  for (const r of report.rows) for (const f of [...r.errors, ...r.warnings]) byRule[f.rule] = (byRule[f.rule] || 0) + 1;
  lines.push("", "## Findings by rule", "", "| rule | count |", "|---|---|");
  for (const [k, v] of Object.entries(byRule).sort((a, b) => b[1] - a[1])) lines.push(`| \`${k}\` | ${v} |`);
  lines.push("");
  mkdirSync(o.out, { recursive: true });
  writeFileSync(join(o.out, "qa-report.md"), lines.join("\n"));
  writeFileSync(join(o.out, "qa-report.json"), JSON.stringify({ ...report, strict: o.strict, errors, warnings }, null, 1));
  return { errors, warnings, byRule };
}

// ------------------------------------------------------------------------------------------ self-test

const PNG_1x1 = Buffer.from("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==", "base64");

function serveDir(dir) {
  const types = { ".html": "text/html; charset=utf-8", ".png": "image/png", ".svg": "image/svg+xml", ".xml": "application/xml", ".js": "text/javascript", ".css": "text/css" };
  const server = createServer((req, res) => {
    let p = decodeURIComponent(new URL(req.url, "http://x").pathname);
    if (p.endsWith("/")) p += "index.html";
    const file = join(dir, p);
    if (!file.startsWith(dir) || !existsSync(file) || statSync(file).isDirectory()) { res.writeHead(404); res.end("not found"); return; }
    res.writeHead(200, { "content-type": types[extname(file)] || "application/octet-stream" });
    res.end(req.method === "HEAD" ? "" : readFileSync(file));
  });
  return new Promise((r) => server.listen(0, "127.0.0.1", () => r({ server, port: server.address().port })));
}

async function selfTest(o) {
  const dir = mkdtempSync(join(tmpdir(), "huv-qa-selftest-"));
  const img = (name) => writeFileSync(join(dir, name), PNG_1x1);
  img("x-light.png"); img("x-dark.png"); img("y-light.png"); img("y-dark.png"); img("n.png");
  const head = (extra = "") => `<!doctype html><html><head><meta charset="utf-8"><title>fixture</title><style>body{margin:0;width:100%}img[src$="#gh-dark-mode-only"]{display:none}a.glightbox:has(> img[src$="#gh-dark-mode-only"]){display:none}${extra}</style></head><body data-md-color-scheme="default"><div class="md-content"><div class="md-content__inner md-typeset">`;
  const tail = `</div></div></body></html>`;
  mkdirSync(join(dir, "good"), { recursive: true });
  mkdirSync(join(dir, "bad"), { recursive: true });
  writeFileSync(join(dir, "good", "index.html"), head() + `
<p class="huv-status huv-status--implemented"><strong>Status:</strong> <span class="huv-chip huv-chip--implemented">✅ Implemented</span></p>
<span class="huv-section" data-huv-section="simulator" hidden></span>
<figure><a class="glightbox" data-gallery="light" href="../x-light.png"><img src="../x-light.png#gh-light-mode-only" alt="x"></a><a class="glightbox" data-gallery="dark" href="../x-dark.png"><img src="../x-dark.png#gh-dark-mode-only" alt="x" loading="lazy"></a></figure>
<figure><a class="glightbox" data-gallery="any" href="../n.png"><img src="../n.png" alt="neutral" loading="lazy"></a></figure>
<figure data-huv-fallback="hidden" style="display:none"><img src="../y-light.png#gh-light-mode-only" alt="y" loading="lazy"><img src="../y-dark.png#gh-dark-mode-only" alt="y" loading="lazy"></figure>
<div class="arithmatex" style="overflow:auto"><mjx-container>E = mc²</mjx-container></div>
<!-- verify-example -->
<div class="language-python highlight"><pre><code>print(1)</code></pre></div>
<details class="success"><summary>Output</summary><pre><code>| raw | inside code is fine |</code></pre></details>
<div class="admonition example"><p class="admonition-title">Try it</p><p>prose</p><p><!-- example-skip: fixture --></p><div class="language-python highlight"><pre><code>print(3)</code></pre></div></div>
<div class="admonition example"><p class="admonition-title">Try it</p><p><!-- verify-example -->
<div class="language-python highlight"><pre><code>print(4)</code></pre></div></div>
<details><summary>Validation</summary><table><tr><th>Claim</th></tr><tr><td><a href="x">link</a></td></tr></table></details>
<div class="huv-calc" data-huv-calc="rayleigh" data-huv-mounted="true"></div>
` + tail);
  writeFileSync(join(dir, "bad", "index.html"), head(`img[src$="#gh-dark-mode-only"]{display:inline}`) + `
<script src="../javascripts/layout.js"></script>
<p class="huv-status huv-status--implemented"><strong>Status:</strong> ✅ Implemented</p>
<div style="width:2000px;height:4px;background:#ccc"></div>
<figure><img src="../x-light.png#gh-light-mode-only" alt="x"><img src="../x-dark.png#gh-dark-mode-only" alt="x"></figure>
<figure><a class="glightbox" href="../n.png"><img src="../n.png" alt="neutral"></a><a class="glightbox" data-gallery="dark" href="../y-dark.png" style="display:block;width:fit-content"><img src="../y-dark.png#gh-dark-mode-only" alt="hidden" style="display:none"></a></figure>
<p><a href="#zero" style="display:block;width:0;height:0"></a></p>
<img src="../missing.png" alt="missing">
<figure style="opacity:0"><img src="../y-light.png#gh-light-mode-only" alt="y"><img src="../y-dark.png#gh-dark-mode-only" alt="y"></figure>
<div class="arithmatex" style="overflow:hidden;width:200px"><mjx-container><span style="display:inline-block;width:900px">wide math</span></mjx-container></div>
<div class="arithmatex" style="overflow:auto;width:200px"><mjx-container><span style="display:inline-block;width:900px">wide math</span></mjx-container></div>
<mjx-merror>bad</mjx-merror>
<div class="language-python highlight"><pre><code>print(2)</code></pre></div>
<details><summary>Validation — 3 tests</summary>
| Claim | Test | Tolerance |
|---|---|---|
| a | [t](x) | 1e-9 |
</details>
<div class="huv-calc" data-huv-calc="rayleigh"></div>
` + tail);
  writeFileSync(join(dir, "sitemap.xml"), `<urlset><url><loc>http://x/</loc></url><url><loc>http://x/good/</loc></url><url><loc>http://x/bad/</loc></url></urlset>`);
  const { server, port } = await serveDir(dir);
  const page = await launch({ ...MODES.light, chrome: o.chrome });
  let failed = 0;
  const expect = (cond, what) => { console.log(`${cond ? "PASS" : "FAIL"}  ${what}`); if (!cond) failed++; };
  try {
    for (const which of ["good", "bad"]) {
      await page.navigate(`http://127.0.0.1:${port}/${which}/`);
      await page.settle({ extra: 100 });
      const probe = await page.evaluate(probeExpression({ path: `/${which}/`, mode: "light" }));
      const f = findingsFromProbe(probe, { path: `/${which}/`, mode: "light", strict: false });
      const rules = new Set(f.errors.map((e) => e.rule));
      if (which === "good") {
        expect(f.errors.length === 0, `good fixture: 0 structural errors (${f.errors.map((e) => e.rule).join(",") || "none"}) — the hidden figure[data-huv-fallback] pair is skipped, the <p>-wrapped markers inside admonitions count, the hidden dark anchor is no tab stop and the "any" gallery holds no hidden image`);
        expect(probe.fences.length === 3 && probe.fences.every((x) => x.ok), `good fixture: 3 python fences all carry a marker (${probe.fences.map((x) => x.marker).join(" | ")})`);
        expect(f.warnings.length === 0, `good fixture: 0 page-form warnings (${f.warnings.map((w) => w.rule).join(",") || "none"})`);
      } else {
        for (const rule of ["overflow-x", "pair-visible", "lightbox-gallery", "focus-zero", "broken-image", "overflow-clipped", "overflow-cue", "mathjax-error", "fence-marker", "details-leak", "calc-mounted"]) {
          expect(rules.has(rule), `bad fixture: ${rule} fires`);
        }
        expect(f.warnings.some((w) => w.rule === "status-chip"), "bad fixture: status-chip is a page-form warning");
        expect(f.warnings.some((w) => w.rule === "lightbox"), "bad fixture: a figure image outside a.glightbox is a page-form warning (lightbox)");
        expect(f.warnings.some((w) => w.rule === "effect-hidden"), "bad fixture: a first-screen figure at opacity 0 is a page-form warning (effect-hidden)");
        expect(!f.errors.some((e) => /pair y-\*/.test(e.msg)), "bad fixture: the faded pair is still exactly one visible variant (opacity is not visibility)");
        expect(f.errors.some((e) => e.rule === "focus-zero" && /a\.glightbox/.test(e.msg)), "bad fixture: a rendered 0x0 a.glightbox around a hidden image is an invisible tab stop (focus-zero)");
        expect(f.errors.some((e) => e.rule === "lightbox-gallery" && /no data-gallery/.test(e.msg)), "bad fixture: a visible anchor without data-gallery pages through the hidden image (lightbox-gallery)");
        const strict = findingsFromProbe(probe, { path: "/bad/", mode: "light", strict: true });
        expect(strict.warnings.length === 0 && strict.errors.some((e) => e.rule === "status-chip"), "strict mode promotes page-form findings to errors");
      }
    }
  } finally {
    await page.close();
    server.close();
  }
  // github preview fixtures
  const bareDetails = `# T\n\n<details>\n<summary>The ladder as a table</summary>\n| a | b |\n|---|---|\n| 1 | [x](y) |\n</details>\n`;
  let r = githubPreview(bareDetails, { path: "f.md", kind: "docs" });
  expect(r.errors.some((e) => e.rule === "details-form"), "preview: bare <details> on a docs page is an error");
  expect(r.errors.some((e) => e.rule === "details-leak"), "preview: a table inside a details block without a blank line leaks on github.com");
  const goodDetails = `# T\n\n<details markdown="1">\n<summary>The ladder as a table</summary>\n\n| a | b |\n|---|---|\n| 1 | [x](y) |\n\n</details>\n`;
  r = githubPreview(goodDetails, { path: "g.md", kind: "docs" });
  expect(r.errors.length === 0, `preview: <details markdown="1"> + blank line renders its table (${r.errors.map((e) => e.rule).join(",") || "no errors"})`);
  r = githubPreview(`# T\n\n??? info "Data"\n\n    body\n`, { path: "h.md", kind: "docs" });
  expect(r.errors.some((e) => e.rule === "admonition-leak"), "preview: a ??? line other than success \"Output\" is a leak");
  r = githubPreview(`# T\n\n<!-- verify-example -->\n\`\`\`python\nprint(1)\n\`\`\`\n\nA sentence.\n\n???+ success "Output"\n\n    \`\`\`text\n    1\n    \`\`\`\n`, { path: "i.md", kind: "docs" });
  expect(r.errors.some((e) => e.rule === "output-unchecked"), "preview: an output block separated from its fence is silently unchecked (error)");
  r = githubPreview(`# T\n\n<!-- verify-example -->\n\`\`\`python\nprint(1)\n\`\`\`\n\n\`\`\`text\n1\n\`\`\`\n`, { path: "r.md", kind: "readme" });
  expect(r.errors.length === 0 && r.warnings.length === 0, "preview: the README bare text-fence idiom passes clean");
  r = githubPreview(`# T\n\nYou must never use this.\n\n!!! note "x"\n\n    body\n`, { path: "k.md", kind: "readme" });
  expect(r.errors.some((e) => e.rule === "admonition-leak"), "preview: !!! in a GitHub-only file is a leak");
  console.log(failed ? `self-test: ${failed} FAILED` : "self-test: all expectations hold");
  process.exit(failed ? 1 : 0);
}

// ------------------------------------------------------------------------------------------ main

const o = parseArgs(process.argv.slice(2));
if (o.selfTest) await selfTest(o);
if (!o.out) o.out = mkdtempSync(join(tmpdir(), "huv-qa-"));
mkdirSync(o.out, { recursive: true });
const report = { rows: [], notes: [], pages: 0 };
if (o.site) await runSite(o, report);
if (o.preview) runPreview(o, report);
const { errors, warnings, byRule } = writeReport(o, report);
const top = Object.entries(byRule).sort((a, b) => b[1] - a[1]).slice(0, 8).map(([k, v]) => `${k}=${v}`).join(" ");
console.log(`report: ${join(o.out, "qa-report.md")}`);
console.log(`qa_site: ${errors} error(s), ${warnings} warning(s) [${o.strict ? "strict" : "tolerant"}]${top ? " — " + top : ""}`);
process.exit(errors ? 1 : 0);
