// node --test scripts/cdp/qa_site.test.mjs — the pure helpers of qa_lib.mjs (no Chrome needed).
import assert from "node:assert/strict";
import { test } from "node:test";
import {
  SAMPLE_PAGES, expectedSection, findingsFromProbe, githubPreview, lightboxFindings, pageSlug, pairFindings, pairKey,
  pairsExpression, probeExpression, renderGfm, sitemapPaths, stripFrontMatter,
} from "./qa_lib.mjs";

test("sitemapPaths strips the site_url prefix and keeps the root first", () => {
  const xml = `<urlset><url><loc>https://sonoransun.github.io/highuvlith/</loc></url>
    <url><loc>https://sonoransun.github.io/highuvlith/history/euv/</loc></url>
    <url><loc>https://sonoransun.github.io/highuvlith/pipeline/</loc></url></urlset>`;
  assert.deepEqual(sitemapPaths(xml), ["/", "/history/euv/", "/pipeline/"]);
  assert.deepEqual(sitemapPaths(""), []);
});

test("pageSlug and expectedSection", () => {
  assert.equal(pageSlug("/"), "home");
  assert.equal(pageSlug("/history/euv/"), "history-euv");
  assert.equal(expectedSection("/"), "home");
  assert.equal(expectedSection("/history/euv/"), "history");
  assert.equal(expectedSection("/nodes/litho-math/"), "nodes");
  assert.equal(expectedSection("/future/"), "future");
  assert.equal(expectedSection("/playground/"), "simulator");
  assert.equal(expectedSection("/sources/lpp/"), "simulator");
  assert.equal(expectedSection("/capability-matrix/"), "reference");
  assert.equal(expectedSection("/roadmap/"), "reference");
  assert.equal(expectedSection("/python-api-modules/"), "reference");
  assert.equal(expectedSection("/_kit/"), null);
  assert.equal(SAMPLE_PAGES.length, 13);
});

test("pairKey groups light/dark variants, both fragment conventions, stills and cache-busts", () => {
  assert.deepEqual(pairKey("../assets/images/sim/x-light.png#gh-light-mode-only"), { key: "../assets/images/sim/x-*.png", variant: "light", fragment: "gh-light-mode-only" });
  assert.equal(pairKey("../a/x-dark.svg#only-dark").key, "../a/x-*.svg");
  assert.equal(pairKey("a/x-anim-light.svg?v=2#gh-light-mode-only").key, pairKey("a/x-still-dark.svg#gh-dark-mode-only").key);
  assert.equal(pairKey("a/x-light.png"), null);
  assert.equal(pairKey("a/x.png#gh-light-mode-only"), null);
});

test("pairFindings: exactly one visible variant, closed details ignored, gallery must differ", () => {
  const ok = { "a-*.png": [{ variant: "light", visible: true, closed: false, gallery: "light" }, { variant: "dark", visible: false, closed: false, gallery: "dark" }] };
  assert.deepEqual(pairFindings(ok, "normal"), []);
  const both = { "a-*.png": [{ variant: "light", visible: true, closed: false }, { variant: "dark", visible: true, closed: false }] };
  assert.equal(pairFindings(both, "normal")[0].rule, "pair-visible");
  const none = { "a-*.png": [{ variant: "light", visible: false, closed: false }, { variant: "dark", visible: false, closed: false }] };
  assert.equal(pairFindings(none, "print")[0].rule, "pair-visible");
  const closed = { "a-*.png": [{ variant: "light", visible: false, closed: true }, { variant: "dark", visible: false, closed: true }] };
  assert.deepEqual(pairFindings(closed, "normal"), []);   // closed <details> or figure[data-huv-fallback]: hidden by design
  const gallery = { "a-*.png": [{ variant: "light", visible: true, closed: false, gallery: "g" }, { variant: "dark", visible: false, closed: false, gallery: "g" }] };
  assert.equal(pairFindings(gallery, "normal")[0].rule, "pair-gallery");
});

test("lightboxFindings: a visible anchor never shares its gallery (or the page-wide set) with a hidden image", () => {
  const paired = [{ src: "x-light.png", gallery: "light", visible: true, closed: false }, { src: "x-dark.png", gallery: "dark", visible: false, closed: false }];
  assert.deepEqual(lightboxFindings(paired, "normal"), []);
  // the GUI captures before the fix: no data-gallery → GLightbox opens the page-wide list, hidden dark variants included
  const bare = paired.concat([{ src: "gui-aerial-euv.png", gallery: "", visible: true, closed: false }]);
  const f = lightboxFindings(bare, "normal");
  assert.equal(f.length, 1);
  assert.equal(f[0].rule, "lightbox-gallery");
  assert.match(f[0].msg, /^normal: visible gui-aerial-euv\.png has no data-gallery.*1 hidden image\(s\) \(x-dark\.png\)/);
  // the hook's data-gallery="any": only scheme-neutral (always visible) images share it
  const any = paired.concat([{ src: "a.png", gallery: "any", visible: true, closed: false }, { src: "b.png", gallery: "any", visible: true, closed: false }]);
  assert.deepEqual(lightboxFindings(any, "normal"), []);
  // a hidden anchor in the visible one's gallery; one finding per gallery, not per anchor
  const shared = [{ src: "a.png", gallery: "g", visible: true, closed: false }, { src: "b.png", gallery: "g", visible: true, closed: false }, { src: "h.png", gallery: "g", visible: false, closed: false }];
  const g = lightboxFindings(shared, "print");
  assert.equal(g.length, 1);
  assert.match(g[0].msg, /^print: visible a\.png shares lightbox gallery "g" with 1 hidden image\(s\) \(h\.png\)/);
  // closed <details> / fallback figures are hidden by design; a page with no hidden anchor at all is clean
  assert.deepEqual(lightboxFindings([{ src: "a.png", gallery: "", visible: true, closed: false }, { src: "c.png", gallery: "", visible: false, closed: true }], "normal"), []);
  assert.deepEqual(lightboxFindings([{ src: "a.png", gallery: "", visible: true, closed: false }], "normal"), []);
  assert.deepEqual(lightboxFindings(undefined, "normal"), []);
});

const cleanProbe = () => ({
  ox: 0, mjx: 3, mjxErr: 0, broken: [], pairs: {}, overflow: [], cueMachinery: true, fences: [{ ok: true }], calc: 1, mounted: 1,
  detailsLeaks: [], status: { chip: true }, section: "simulator", hero: { top: 400 }, viewport: { w: 1280, h: 900 }, lazyMissing: [],
  bytesVisible: 100, anim: [], mermaidPending: 0, arithUntypeset: 0, figYs: [400],
});

test("findingsFromProbe: a clean probe yields no findings", () => {
  const f = findingsFromProbe(cleanProbe(), { path: "/pipeline/", mode: "light", strict: false });
  assert.deepEqual(f.errors, []);
  assert.deepEqual(f.warnings, []);
});

test("findingsFromProbe: structural rules are errors in tolerant mode", () => {
  const p = { ...cleanProbe(), ox: 12, mjxErr: 1, broken: ["x.png (404)"], calc: 2, mounted: 1, fences: [{ ok: false, head: "import x" }],
    detailsLeaks: [{ summary: "Validation", hits: ["|---"] }],
    anchors: [{ src: "n.png", gallery: "", visible: true, closed: false }, { src: "x-dark.png", gallery: "dark", visible: false, closed: false }],
    focusZero: ["a.glightbox sim/x-dark.png"],
    overflow: [{ sel: "div.arithmatex", scrollWidth: 900, clientWidth: 600, overflowX: "hidden", scrollable: false, cue: false, text: "" },
               { sel: ".huv-diagram", scrollWidth: 900, clientWidth: 600, overflowX: "auto", scrollable: true, cue: false, text: "" }] };
  const f = findingsFromProbe(p, { path: "/x/", mode: "light", strict: false });
  const rules = f.errors.map((e) => e.rule).sort();
  assert.deepEqual(rules, ["broken-image", "calc-mounted", "details-leak", "fence-marker", "focus-zero", "lightbox-gallery", "mathjax-error", "overflow-clipped", "overflow-cue", "overflow-x"]);
});

test("findingsFromProbe: a scrollable block without a cue is only a warning while layout.js is absent", () => {
  const p = { ...cleanProbe(), cueMachinery: false, overflow: [{ sel: "div.arithmatex", scrollWidth: 760, clientWidth: 688, overflowX: "auto", scrollable: true, cue: false, text: "TCC" }] };
  const f = findingsFromProbe(p, { path: "/pipeline/", mode: "light", strict: false });
  assert.deepEqual(f.errors, []);
  assert.equal(f.warnings[0].rule, "overflow-cue");
  const g = findingsFromProbe({ ...p, cueMachinery: true }, { path: "/pipeline/", mode: "light", strict: false });
  assert.equal(g.errors[0].rule, "overflow-cue");
  const h = findingsFromProbe({ ...p, overflow: [{ ...p.overflow[0], cue: true }] }, { path: "/pipeline/", mode: "light", strict: false });
  assert.deepEqual(h.warnings, []);
});

test("findingsFromProbe: page-form rules warn, then become errors under strict", () => {
  const p = { ...cleanProbe(), status: { chip: false }, section: null, hero: { top: 1400 }, lazyMissing: ["a.png"], bytesVisible: 3 * 1024 * 1024,
    anim: [{ src: "a-anim-light.png", still: null, stillOk: null, hasClass: false, visible: true, raster: true, svg: false, bytes: 1.2e6, currentIsStill: false },
           { src: "b-anim-light.png", still: "b-still-light.png", stillOk: false, hasClass: true, visible: true, raster: true, svg: false, bytes: 10, currentIsStill: false }] };
  const f = findingsFromProbe(p, { path: "/x/", mode: "light", strict: false });
  assert.deepEqual(f.errors, []);
  const rules = new Set(f.warnings.map((w) => w.rule));
  for (const r of ["status-chip", "section-marker", "hero-fold", "lazy-loading", "image-bytes", "anim-autoplay", "anim-weight", "anim-still", "anim-class"]) assert.ok(rules.has(r), r);
  const s = findingsFromProbe(p, { path: "/x/", mode: "light", strict: true });
  assert.equal(s.warnings.length, 0);
  assert.ok(s.errors.every((e) => e.promoted));
  // the wide-page budget and the phone hero limit
  const wide = findingsFromProbe({ ...cleanProbe(), section: "reference", bytesVisible: 3 * 1024 * 1024 }, { path: "/python-api/", mode: "light", strict: false });
  assert.deepEqual(wide.warnings, []);
  const phone = findingsFromProbe({ ...cleanProbe(), hero: { top: 1500 }, viewport: { w: 390, h: 844 } }, { path: "/x/", mode: "phone", strict: false });
  assert.deepEqual(phone.warnings, []);
});

test("findingsFromProbe: the effect rules (lightbox, effect-hidden) are page-form", () => {
  const p = { ...cleanProbe(), lightboxMissing: ["sim/a-light.png#gh-light-mode-only"], faded: [{ text: "Figure 1.", opacity: 0 }] };
  const f = findingsFromProbe(p, { path: "/x/", mode: "light", strict: false });
  assert.deepEqual(f.errors, []);
  assert.deepEqual(f.warnings.map((w) => w.rule).sort(), ["effect-hidden", "lightbox"]);
  const s = findingsFromProbe(p, { path: "/x/", mode: "light", strict: true });
  assert.equal(s.warnings.length, 0);
  assert.equal(s.errors.length, 2);
});

test("probeExpression is a self-contained expression", () => {
  const src = probeExpression({ path: "/x/", mode: "light" });
  assert.ok(src.startsWith("(function probeInPage"));
  assert.ok(src.endsWith(`)({"path":"/x/","mode":"light"})`));
  assert.doesNotThrow(() => new Function(`return ${src.replace(/\)\(\{[^)]*\}\)$/, ")")}`));
  for (const expr of [src, pairsExpression()]) {
    assert.ok(expr.includes('q("a.glightbox")') && expr.includes("anchors"), "both probes collect the lightbox anchors");
  }
  assert.doesNotThrow(() => new Function(`return ${pairsExpression()}`));
});

test("renderGfm renders tables and passes raw HTML through; front matter is stripped", () => {
  const html = renderGfm("---\ndescription: x\n---\n# T\n\n| a |\n|---|\n| 1 |\n\n<details markdown=\"1\">\n<summary>S</summary>\n\n**bold**\n\n</details>\n");
  assert.ok(html.includes("<table>"));
  assert.ok(html.includes("<strong>bold</strong>"));
  assert.ok(!html.includes("description"));
  assert.equal(stripFrontMatter("---\na: 1\n---\nbody"), "body");
});

test("githubPreview: the docs-page rules", () => {
  const leak = githubPreview("# T\n\nThis value must never be assumed. !!! warning\n\n??? info \"Data\"\n\n    x\n\n=== \"Tab\"\n", { path: "d.md", kind: "docs" });
  const rules = new Set(leak.errors.map((e) => e.rule));
  assert.ok(rules.has("rule-leak") && rules.has("admonition-leak") && rules.has("tab-leak"));
  const ok = githubPreview("# T\n\n!!! note \"What highuvlith models here\"\n\n    body\n\n<!-- verify-example -->\n```python\nprint(1)\n```\n\n???+ success \"Output\"\n\n    ```text\n    1\n    ```\n", { path: "e.md", kind: "docs" });
  assert.deepEqual(ok.errors, []);
  assert.deepEqual(ok.warnings, []);
  const leadIn = githubPreview("# T\n\n<!-- verify-example -->\n```python\nprint(1)\n```\n\nOutput:\n\n```text\n1\n```\n", { path: "f.md", kind: "docs" });
  assert.deepEqual(leadIn.errors, []);
  assert.equal(leadIn.warnings[0].rule, "output-leadin");
  const noOut = githubPreview("# T\n\n<!-- verify-example -->\n```python\nprint(1)\n```\n\nProse.\n", { path: "g.md", kind: "docs" });
  assert.deepEqual(noOut.errors, []);
  assert.equal(noOut.warnings[0].rule, "output-missing");
  const noMarker = githubPreview("# T\n\n```python\nprint(1)\n```\n", { path: "h.md", kind: "docs" });
  assert.equal(noMarker.errors[0].rule, "fence-marker");
  const attr = githubPreview("# T\n\n![a](x.png){ width=\"700\" }\n", { path: "i.md", kind: "docs" });
  assert.equal(attr.warnings[0].rule, "attr-list");
});

test("githubPreview: the README rules", () => {
  const good = githubPreview(`# R\n\n<picture>\n  <source media="(prefers-color-scheme: dark)" srcset="d.png">\n  <img alt="a" width="800" src="l.png">\n</picture>\n\n<details>\n<summary>2. Immersion</summary>\n\n<!-- verify-example -->\n\`\`\`python\nprint(1)\n\`\`\`\n\n\`\`\`text\n1\n\`\`\`\n\n</details>\n`, { path: "README.md", kind: "readme" });
  assert.deepEqual(good.errors, []);
  assert.deepEqual(good.warnings, []);
  const bad = githubPreview(`# R\n\n<picture>\n  <img src="l.png">\n</picture>\n\n<details><summary>x</summary>\n| a |\n|---|\n| 1 |\n</details>\n\n<!-- verify-example -->\n\`\`\`python\nprint(1)\n\`\`\`\n\n???+ success "Output"\n\n    \`\`\`text\n    1\n    \`\`\`\n`, { path: "README.md", kind: "readme" });
  const rules = new Set(bad.errors.map((e) => e.rule));
  assert.ok(rules.has("picture-form"), "picture without source/alt");
  assert.ok(rules.has("details-form"), "summary not followed by a blank line");
  assert.ok(rules.has("details-leak"), "table leaked inside details");
  assert.ok(rules.has("admonition-leak"), "??? in the README");
  const leadIn = githubPreview("# R\n\n<!-- verify-example -->\n```python\nprint(1)\n```\n\nOutput:\n\n```text\n1\n```\n", { path: "README.md", kind: "readme" });
  assert.equal(leadIn.warnings[0].rule, "output-leadin");
});
