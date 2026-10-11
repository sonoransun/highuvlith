// Pure helpers for scripts/qa_site.mjs: the sample lists, the in-page probe, the finding
// rules (DESIGN §10.4: STRUCTURAL asserts are errors in every mode, PAGE-FORM asserts warn
// unless strict), and the github.com preview checks (markdown-it 14.1.0, vendored).
// Everything here is side-effect free and covered by scripts/cdp/qa_site.test.mjs.

import { createRequire } from "node:module";

const require = createRequire(import.meta.url);

/** The 13-page PR-CI sample (DESIGN §10.4), as site paths. */
export const SAMPLE_PAGES = [
  "/", "/getting-started/", "/pipeline/", "/optics/", "/sources/", "/sources/lpp/",
  "/processes/volumetric-exposure/", "/capability-matrix/", "/history/", "/history/euv/",
  "/nodes/litho-math/", "/future/", "/playground/",
];

/** The five docs pages the GitHub preview renders by default (DESIGN §10.1-8). */
export const GITHUB_PREVIEW_SAMPLE = [
  "docs/index.md", "docs/pipeline.md", "docs/sources/lpp.md", "docs/future/index.md", "docs/capability-matrix.md",
];

/** Weight budgets of DESIGN §10.2 (bytes). */
export const BUDGETS = {
  pageImageBytes: 2.5 * 1024 * 1024,
  pageImageBytesWide: { "/python-api/": 4 * 1024 * 1024, "/sources/": 4 * 1024 * 1024, "/gui/": 4 * 1024 * 1024 },
  animSvg: 120 * 1024,
  animRaster: 1024 * 1024,
  animAssets: 3,
  autoplayRasters: 1,
  heroDesktopPx: 900,
  heroPhoneScreens: 2,
};

export const MODES = {
  light: { width: 1280, height: 900, dark: false, mobile: false, media: "" },
  dark: { width: 1280, height: 900, dark: true, mobile: false, media: "" },
  print: { width: 1280, height: 900, dark: false, mobile: false, media: "print" },
  phone: { width: 390, height: 844, dark: false, mobile: true, media: "" },
};

/** Paths from a sitemap.xml, relative to the site root ("/" first). The root is the
 *  shortest <loc>; every other loc must start with it. */
export function sitemapPaths(xml) {
  const locs = [...xml.matchAll(/<loc>\s*([^<\s]+)\s*<\/loc>/g)].map((m) => m[1]);
  if (!locs.length) return [];
  const root = locs.reduce((a, b) => (b.length < a.length ? b : a));
  const base = root.endsWith("/") ? root : root + "/";
  const out = [];
  for (const loc of locs) {
    if (!loc.startsWith(base)) continue;
    let p = "/" + loc.slice(base.length);
    if (!p.endsWith("/") && !/\.[a-z0-9]+$/i.test(p)) p += "/";
    if (!out.includes(p)) out.push(p);
  }
  return out;
}

/** "/history/euv/" -> "history-euv", "/" -> "home". */
export function pageSlug(path) {
  const s = path.replace(/^\/+|\/+$/g, "").replace(/\.html$/, "").replace(/[^A-Za-z0-9]+/g, "-");
  return s || "home";
}

/** The section hue a page should declare (DESIGN §4.2); null = no expectation (_kit, 404). */
export function expectedSection(path) {
  const p = path.replace(/^\/+/, "");
  if (p === "" || p === "index.html") return "home";
  const head = p.split("/")[0];
  if (head === "_kit" || head === "404.html" || head === "search") return null;
  if (["history", "nodes", "future"].includes(head)) return head;
  if (["reference", "capability-matrix", "configuration", "cli", "gui", "python-api", "python-api-modules", "extending", "roadmap"].includes(head)) return "reference";
  return "simulator"; // the simulator section, its wings and the Playground (the §4.2 fallback by design)
}

/** Light/dark pair key of an <img src>, or null when the src is not a scheme variant. */
export function pairKey(src) {
  const m = (src || "").match(/^(.*?)-(light|dark)(\.[A-Za-z0-9]+)(\?[^#]*)?#(gh-light-mode-only|gh-dark-mode-only|only-light|only-dark)$/);
  if (!m) return null;
  return { key: m[1].replace(/-still$/, "-anim") + "-*" + m[3], variant: m[2], fragment: m[5] };
}

/**
 * The probe that runs inside a page (serialised with toString(), so it must be self-contained).
 * Returns plain JSON: every measurement the finding rules need.
 */
export function probeInPage(ctx) {
  const d = document, de = d.documentElement;
  const q = (s, r) => Array.prototype.slice.call((r || d).querySelectorAll(s));
  const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
  const inClosedDetails = (el) => {
    for (let p = el.parentElement; p; p = p.parentElement) {
      if (p.tagName === "DETAILS" && !p.open && !(el.closest("summary") && p.contains(el.closest("summary")))) return true;
    }
    return false;
  };
  const hiddenByDesign = (el) => inClosedDetails(el) || !!el.closest("figure[data-huv-fallback]");
  const visible = (el) => {
    if (!el || !el.getClientRects().length) return false;
    const cs = getComputedStyle(el);
    return cs.display !== "none" && cs.visibility !== "hidden";
  };
  const short = (s) => (s || "").split("/").slice(-2).join("/");
  const res = {};
  for (const r of performance.getEntriesByType("resource")) {
    res[r.name.split("#")[0]] = r.transferSize || r.encodedBodySize || r.decodedBodySize || 0;
  }
  const bytesOf = (img) => res[(img.currentSrc || img.src || "").split("#")[0]] || 0;

  // --- scheme pairs ---------------------------------------------------------------------
  const pairRe = /^(.*?)-(light|dark)(\.[A-Za-z0-9]+)(\?[^#]*)?#(gh-light-mode-only|gh-dark-mode-only|only-light|only-dark)$/;
  const pairs = {};
  for (const img of q("img")) {
    const src = img.getAttribute("src") || "";
    const m = src.match(pairRe);
    if (!m) continue;
    const key = m[1].replace(/-still$/, "-anim") + "-*" + m[3];
    const a = img.closest("a.glightbox");
    (pairs[key] = pairs[key] || []).push({
      src: short(src), variant: m[2], visible: visible(img), closed: hiddenByDesign(img),
      gallery: a ? a.getAttribute("data-gallery") : null,
    });
  }

  // --- lightbox anchors: GLightbox pages a gallery-less anchor through EVERY anchor -----------
  const anchors = q("a.glightbox").map((a) => {
    const im = a.querySelector("img");
    return { src: short(a.getAttribute("href") || ""), gallery: a.getAttribute("data-gallery") || "", visible: visible(im || a), closed: hiddenByDesign(a) };
  });

  // --- zero-size focusables: a rendered tab stop with no box (a hidden image's anchor) --------
  // (pymdownx anchor_linenums' empty per-line <a id="__codelineno-…"> anchors are upstream's and skipped)
  const article = d.querySelector("article") || d.querySelector(".md-content") || d.body;
  const focusZero = [];
  for (const el of q("a[href], button, input, select, textarea, [tabindex]", article)) {
    if (hiddenByDesign(el) || el.tabIndex < 0 || el.disabled || el.offsetParent === null || /^__codelineno/.test(el.id || "")) continue;
    if (el.offsetWidth === 0 && el.offsetHeight === 0) {
      focusZero.push(el.tagName.toLowerCase() + (el.className ? "." + String(el.className).trim().split(/\s+/).join(".") : "") + " " + short(el.getAttribute("href") || (el.textContent || "").trim().slice(0, 30)));
    }
  }

  // --- overflow -------------------------------------------------------------------------
  const cueMachinery = typeof window.huvMarkOverflow === "function" || !!d.querySelector('script[src*="layout.js"]');
  const overflow = [];
  for (const sel of ["div.arithmatex", ".huv-diagram", ".md-typeset__scrollwrap", ".md-typeset pre > code"]) {
    for (const el of q(sel)) {
      if (!visible(el) || el.scrollWidth <= el.clientWidth + 1) continue;
      const ox = getComputedStyle(el).overflowX;
      overflow.push({
        sel, scrollWidth: el.scrollWidth, clientWidth: el.clientWidth, overflowX: ox,
        scrollable: ox === "auto" || ox === "scroll",
        cue: el.getAttribute("data-huv-overflow") === "true",
        text: (el.textContent || "").trim().slice(0, 40),
      });
    }
  }

  // --- python fences and their markers --------------------------------------------------
  const fences = [];
  const seen = new Set();
  const codeBlocks = q("div.language-python.highlight, div.highlight.language-python").concat(
    q("code.language-python").map((c) => c.closest("div.highlight") || c.closest("pre") || c)
  );
  for (const el of codeBlocks) {
    if (seen.has(el)) continue;
    seen.add(el);
    let n = el.previousSibling, marker = null;
    while (n && n.nodeType === 3 && !n.textContent.trim()) n = n.previousSibling;
    if (n && n.nodeType === 8) marker = n.textContent.trim();
    else if (n && n.nodeType === 1 && n.tagName === "P" && !(n.textContent || "").trim()) {
      // inside an admonition Python-Markdown wraps the comment in a <p> (left open before the <div>)
      const cs = Array.prototype.slice.call(n.childNodes).filter((c) => c.nodeType === 8);
      if (cs.length) marker = cs[cs.length - 1].textContent.trim();
    }
    fences.push({ ok: !!marker && /^(verify-example|example-skip:\s*\S)/.test(marker), marker, head: (el.textContent || "").trim().split("\n")[0].slice(0, 60) });
  }

  // --- details leaks --------------------------------------------------------------------
  const detailsLeaks = [];
  for (const det of q("details")) {
    const c = det.cloneNode(true);
    q("pre, code", c).forEach((e) => e.remove());
    const t = c.textContent || "";
    const hits = ["|---", "](", "!["].filter((h) => t.indexOf(h) >= 0);
    if (hits.length) detailsLeaks.push({ summary: ((det.querySelector("summary") || {}).textContent || "").trim().slice(0, 60), hits });
  }

  // --- images: lazy, anim, bytes, broken ------------------------------------------------
  const content = d.querySelector(".md-content") || d.body;
  const imgs = q("img", content).filter((i) => !/\b(twemoji|emojione|gemoji)\b/.test(i.className));
  const firstFigureImg = imgs.find((i) => i.closest("figure"));
  const lazyMissing = [];
  let after = false;
  for (const i of imgs) {
    if (i === firstFigureImg) { after = true; continue; }
    if (after && i.getAttribute("loading") !== "lazy") lazyMissing.push(short(i.getAttribute("src") || i.src));
  }
  const anim = [];
  for (const i of q("img")) {
    const src = i.getAttribute("src") || "";
    if (src.indexOf("-anim-") < 0 && !i.classList.contains("huv-anim")) continue;
    anim.push({
      src: short(src), still: i.getAttribute("data-huv-still"), stillOk: null, hasClass: i.classList.contains("huv-anim"),
      visible: visible(i) && !hiddenByDesign(i), raster: /\.(png|gif|webp|apng)(\?[^#]*)?(#.*)?$/i.test(src),
      svg: /\.svg(\?[^#]*)?(#.*)?$/i.test(src), bytes: bytesOf(i), currentIsStill: ((i.currentSrc || i.src || "").indexOf("-still-") >= 0),
    });
  }
  let bytesVisible = 0, bytesAll = 0;
  for (const i of q("img")) {
    const b = bytesOf(i);
    bytesAll += b;
    if (visible(i) && !hiddenByDesign(i)) bytesVisible += b;
  }
  // --- effects: the lightbox on every figure image; no reveal effect hiding a first-screen figure
  const lightboxMissing = [];
  for (const i of imgs) {
    const src = i.getAttribute("src") || "";
    if (!pairRe.test(src) || !i.closest("figure") || i.classList.contains("huv-no-zoom") || i.closest("a.glightbox") || hiddenByDesign(i)) continue;
    lightboxMissing.push(short(src));
  }
  const faded = [];
  for (const fig of q(".md-content figure")) {
    if (!visible(fig) || hiddenByDesign(fig)) continue;
    const r = fig.getBoundingClientRect();
    if (r.bottom <= 0 || r.top >= window.innerHeight) continue;
    const im = fig.querySelector("img");
    const op = Math.min(parseFloat(getComputedStyle(fig).opacity), im && visible(im) ? parseFloat(getComputedStyle(im).opacity) : 1);
    if (op < 0.05) faded.push({ text: ((fig.querySelector("figcaption") || fig).textContent || "").trim().slice(0, 40), opacity: op });
  }
  const broken = [];
  const toCheck = q("img").filter((i) => (i.complete && i.naturalWidth === 0) || (!i.complete && i.getAttribute("loading") === "lazy")).slice(0, 60);
  const heads = toCheck.map((i) => {
    const url = (i.currentSrc || i.src || "").split("#")[0];
    if (!url || url.startsWith("data:")) return Promise.resolve();
    return fetch(url, { method: "HEAD" }).then((r) => { if (!r.ok) broken.push(short(url) + " (" + r.status + ")"); }, () => broken.push(short(url) + " (fetch failed)"));
  });
  const stillChecks = anim.filter((a) => a.still).map((a) => fetch(a.still, { method: "HEAD" }).then((r) => { a.stillOk = r.ok; }, () => { a.stillOk = false; }));

  // --- hero, status, section, calc, math, mermaid ---------------------------------------
  const lead = d.querySelector(".md-typeset .huv-lead");
  const heroEl = d.querySelector(".huv-lead + figure, .huv-lead + p + figure, .huv-lead + .huv-actions + figure") || d.querySelector(".huv-physics figure");
  const figures = q(".md-typeset figure").filter(visible);
  const topOf = (el) => Math.round(el.getBoundingClientRect().top + window.scrollY);
  const status = d.querySelector("p.huv-status");
  const section = d.querySelector(".huv-section[data-huv-section]");

  return Promise.all(heads.concat(stillChecks)).then(() => ({
    path: location.pathname, title: d.title,
    ox: de.scrollWidth - de.clientWidth, viewport: { w: window.innerWidth, h: window.innerHeight },
    scheme: d.body.getAttribute("data-md-color-scheme"),
    mjx: q("mjx-container").length, mjxErr: q("mjx-merror").length,
    arithUntypeset: q(".arithmatex").filter((e) => !e.querySelector("mjx-container") && /\\[(\[]/.test(e.textContent)).length,
    mermaidPending: q("pre.mermaid").length, mermaid: q("div.mermaid").length,
    pairs, anchors, focusZero, overflow, cueMachinery, fences, detailsLeaks, lazyMissing, lightboxMissing, faded, anim,
    bytesVisible, bytesAll, broken,
    calc: q("[data-huv-calc]").length, mounted: q("[data-huv-mounted]").length,
    hero: heroEl ? { top: topOf(heroEl), declaredBy: lead && heroEl.closest(".huv-physics") == null ? "lead" : "physics" } : null,
    figYs: figures.slice(0, 3).map(topOf),
    status: status ? { chip: !!status.querySelector(".huv-chip"), grade: status.getAttribute("data-huv-grade") } : null,
    section: section ? section.getAttribute("data-huv-section") : null,
    motionToggle: !!d.querySelector("button.huv-motion"),
    motionPaused: de.getAttribute("data-huv-motion") === "paused",
  }));
}

/** The expression to pass to Runtime.evaluate for probeInPage. */
export function probeExpression(ctx = {}) {
  return `(${probeInPage.toString()})(${JSON.stringify(ctx)})`;
}

/** Only the pair + animation part of the probe (cheap; re-run after re-emulation). */
export function pairsExpression() {
  return `(() => {
    const d = document, q = (s) => Array.prototype.slice.call(d.querySelectorAll(s));
    const inClosedDetails = (el) => { for (let p = el.parentElement; p; p = p.parentElement) { if (p.tagName === "DETAILS" && !p.open && !el.closest("summary")) return true; } return false; };
    const hiddenByDesign = (el) => inClosedDetails(el) || !!el.closest("figure[data-huv-fallback]");
    const visible = (el) => { if (!el || !el.getClientRects().length) return false; const cs = getComputedStyle(el); return cs.display !== "none" && cs.visibility !== "hidden"; };
    const short = (s) => (s || "").split("/").slice(-2).join("/");
    const pairRe = /^(.*?)-(light|dark)(\\.[A-Za-z0-9]+)(\\?[^#]*)?#(gh-light-mode-only|gh-dark-mode-only|only-light|only-dark)$/;
    const pairs = {};
    for (const img of q("img")) {
      const src = img.getAttribute("src") || ""; const m = src.match(pairRe); if (!m) continue;
      const key = m[1].replace(/-still$/, "-anim") + "-*" + m[3]; const a = img.closest("a.glightbox");
      (pairs[key] = pairs[key] || []).push({ src: short(src), variant: m[2], visible: visible(img), closed: hiddenByDesign(img), gallery: a ? a.getAttribute("data-gallery") : null });
    }
    const anchors = q("a.glightbox").map((a) => { const im = a.querySelector("img"); return { src: short(a.getAttribute("href") || ""), gallery: a.getAttribute("data-gallery") || "", visible: visible(im || a), closed: hiddenByDesign(a) }; });
    const anim = q("img").filter((i) => (i.getAttribute("src") || "").indexOf("-anim-") >= 0 || i.classList.contains("huv-anim") || (i.getAttribute("data-huv-anim") || "").indexOf("-anim-") >= 0)
      .map((i) => ({ src: short(i.getAttribute("src") || ""), visible: visible(i) && !hiddenByDesign(i), currentIsStill: ((i.currentSrc || i.src || "").indexOf("-still-") >= 0), raster: /\\.(png|gif|webp|apng)(\\?[^#]*)?(#.*)?$/i.test(i.getAttribute("src") || "") }));
    const faded = [];
    for (const fig of q(".md-content figure")) {
      if (!visible(fig) || hiddenByDesign(fig)) continue;
      const im = fig.querySelector("img");
      const op = Math.min(parseFloat(getComputedStyle(fig).opacity), im && visible(im) ? parseFloat(getComputedStyle(im).opacity) : 1);
      if (op < 0.05) faded.push({ text: ((fig.querySelector("figcaption") || fig).textContent || "").trim().slice(0, 40), opacity: op });
    }
    return { pairs, anchors, anim, faded, motionToggle: !!d.querySelector("button.huv-motion"), motionPaused: d.documentElement.getAttribute("data-huv-motion") === "paused", scheme: d.body.getAttribute("data-md-color-scheme") };
  })()`;
}

/** Findings for the pair rule alone (used by the normal, reduced-motion, toggle and print passes). */
export function pairFindings(pairs, label) {
  const errors = [];
  for (const [key, imgs] of Object.entries(pairs || {})) {
    const open = imgs.filter((i) => !i.closed);
    if (!open.length) continue;
    const vis = open.filter((i) => i.visible);
    if (vis.length !== 1) {
      errors.push({ rule: "pair-visible", msg: `${label}: pair ${key} has ${vis.length} visible variant(s) (${open.map((i) => i.variant + (i.visible ? "=shown" : "=hidden")).join(", ")})` });
      continue;
    }
    const hidden = open.filter((i) => !i.visible);
    for (const h of hidden) {
      if (h.gallery && vis[0].gallery && h.gallery === vis[0].gallery) {
        errors.push({ rule: "pair-gallery", msg: `${label}: hidden variant of ${key} shares lightbox gallery "${h.gallery}" with the visible one` });
      }
    }
  }
  return errors;
}

/**
 * Findings for the lightbox-gallery rule (DESIGN §4.6: the gallery a reader opens holds only what
 * they can see). GLightbox opens a gallery-less anchor on the page-wide list, so a visible anchor
 * with no data-gallery, or one whose gallery also holds a hidden anchor, pages through hidden images.
 * anchors: [{ src, gallery, visible, closed }] for every a.glightbox; closed ones are hidden by design.
 */
export function lightboxFindings(anchors, label) {
  const errors = [];
  const open = (anchors || []).filter((a) => !a.closed);
  const hidden = open.filter((a) => !a.visible);
  if (!hidden.length) return errors;
  const seen = new Set();
  for (const a of open) {
    if (!a.visible || seen.has(a.gallery)) continue;
    const shared = a.gallery ? hidden.filter((h) => h.gallery === a.gallery) : hidden;
    if (!shared.length) continue;
    seen.add(a.gallery);
    const what = a.gallery ? `shares lightbox gallery "${a.gallery}" with` : "has no data-gallery, so its lightbox pages through every anchor, including";
    errors.push({ rule: "lightbox-gallery", msg: `${label}: visible ${a.src} ${what} ${shared.length} hidden image(s) (${shared.slice(0, 2).map((h) => h.src).join(", ")}${shared.length > 2 ? ", …" : ""})` });
  }
  return errors;
}

/**
 * Turn one probe result into findings. ctx: { path, mode, strict }.
 * Returns { errors: [{rule,msg}], warnings: [{rule,msg}], info: [...] }; in strict mode the
 * page-form findings are moved into errors.
 */
export function findingsFromProbe(p, ctx) {
  const errors = [], warnings = [], info = [];
  const err = (rule, msg) => errors.push({ rule, msg });
  const form = (rule, msg) => warnings.push({ rule, msg });
  const mode = ctx.mode || "light";
  const phone = mode === "phone";

  // STRUCTURAL (errors in every mode) ------------------------------------------------------
  if (p.ox > 0) err("overflow-x", `html.scrollWidth exceeds clientWidth by ${p.ox}px (horizontal page scroll)`);
  if (p.mjxErr > 0) err("mathjax-error", `${p.mjxErr} mjx-merror element(s)`);
  for (const b of p.broken || []) err("broken-image", `broken image ${b}`);
  errors.push(...pairFindings(p.pairs, "normal"));
  errors.push(...lightboxFindings(p.anchors, "normal"));
  for (const z of p.focusZero || []) err("focus-zero", `focusable ${z} is rendered with a 0x0 box (an invisible keyboard tab stop; hide the element, not just its image)`);
  for (const o of p.overflow || []) {
    if (!o.scrollable) err("overflow-clipped", `${o.sel} is ${o.scrollWidth}px in a ${o.clientWidth}px box with overflow-x:${o.overflowX} (content cut off): "${o.text}"`);
    else if (!o.cue) {
      const msg = `${o.sel} scrolls (${o.scrollWidth}>${o.clientWidth}px) without the data-huv-overflow cue: "${o.text}"`;
      if (p.cueMachinery) err("overflow-cue", msg);
      else form("overflow-cue", msg + " (cue machinery absent: layout.js not loaded)");
    }
  }
  for (const f of p.fences || []) if (!f.ok) err("fence-marker", `python fence without <!-- verify-example --> / <!-- example-skip: … --> directly above: "${f.head}"`);
  if ((p.calc || 0) !== (p.mounted || 0)) err("calc-mounted", `${p.mounted} mounted widget(s) for ${p.calc} [data-huv-calc] placeholder(s)`);
  for (const l of p.detailsLeaks || []) err("details-leak", `details "${l.summary}" shows raw Markdown (${l.hits.join(" ")}) — use <details markdown="1"> with a blank line after </summary>`);

  // PAGE-FORM (warnings unless strict) -----------------------------------------------------
  if (p.status && !p.status.chip) form("status-chip", "the Status paragraph carries no .huv-chip (marker not followed by its own word, or the hook is the old one)");
  const expected = expectedSection(ctx.path || p.path || "/");
  if (expected) {
    if (!p.section) form("section-marker", `no .huv-section[data-huv-section] marker (expected "${expected}")`);
    else if (p.section !== expected) form("section-marker", `data-huv-section="${p.section}" but the path expects "${expected}"`);
  }
  if (p.hero) {
    const limit = phone ? BUDGETS.heroPhoneScreens * (p.viewport ? p.viewport.h : 844) : BUDGETS.heroDesktopPx;
    if (p.hero.top > limit) form("hero-fold", `the hero figure starts at ${p.hero.top}px (limit ${limit}px in ${mode})`);
  }
  if (p.lightboxMissing && p.lightboxMissing.length) form("lightbox", `${p.lightboxMissing.length} figure image(s) not wrapped by the lightbox (a.glightbox; add .huv-no-zoom to opt out): ${p.lightboxMissing.slice(0, 3).join(", ")}${p.lightboxMissing.length > 3 ? ", …" : ""}`);
  for (const f of p.faded || []) form("effect-hidden", `figure "${f.text}" sits on the first screen at opacity ${f.opacity} (a reveal effect may not hide content)`);
  if (p.lazyMissing && p.lazyMissing.length) form("lazy-loading", `${p.lazyMissing.length} image(s) after the first figure without loading="lazy": ${p.lazyMissing.slice(0, 3).join(", ")}${p.lazyMissing.length > 3 ? ", …" : ""}`);
  const budget = (BUDGETS.pageImageBytesWide[ctx.path] || BUDGETS.pageImageBytes);
  if (p.bytesVisible > budget) form("image-bytes", `visible image bytes ${(p.bytesVisible / 1048576).toFixed(2)} MB exceed ${(budget / 1048576).toFixed(1)} MB`);
  const anim = p.anim || [];
  const autoplaying = anim.filter((a) => a.visible && a.raster && !a.currentIsStill);
  if (autoplaying.length > BUDGETS.autoplayRasters) form("anim-autoplay", `${autoplaying.length} autoplaying raster animation(s) on one page (≤ ${BUDGETS.autoplayRasters})`);
  const distinct = new Set(anim.map((a) => a.src.replace(/-(light|dark)\.[a-z0-9]+.*$/i, "")));
  if (distinct.size > BUDGETS.animAssets) form("anim-count", `${distinct.size} animated assets on one page (≤ ${BUDGETS.animAssets})`);
  for (const a of anim) {
    if (a.raster && a.bytes > BUDGETS.animRaster) form("anim-weight", `${a.src} is ${(a.bytes / 1024).toFixed(0)} KB (raster animation cap ${BUDGETS.animRaster / 1024} KB)`);
    if (a.svg && a.bytes > BUDGETS.animSvg) form("anim-weight", `${a.src} is ${(a.bytes / 1024).toFixed(0)} KB (animated SVG cap ${BUDGETS.animSvg / 1024} KB)`);
    if (!a.still) form("anim-still", `${a.src} has no data-huv-still`);
    else if (a.stillOk === false) form("anim-still", `${a.src}: data-huv-still ${a.still} does not resolve`);
    if (!a.hasClass) form("anim-class", `${a.src} lacks the .huv-anim class`);
  }
  if (p.mermaidPending > 0) form("mermaid-pending", `${p.mermaidPending} mermaid fence(s) not rendered within the settle window (offline?)`);
  if (p.arithUntypeset > 0) form("mathjax-pending", `${p.arithUntypeset} math block(s) not typeset within the settle window`);

  info.push(`ox=${p.ox} mjx=${p.mjx} mjxErr=${p.mjxErr} pairs=${Object.keys(p.pairs || {}).length} lb=${(p.anchors || []).length} figs=${(p.figYs || []).join("/") || "-"} calc=${p.calc} imgMB=${(p.bytesVisible / 1048576).toFixed(2)}`);
  if (ctx.strict) { errors.push(...warnings.map((w) => ({ ...w, promoted: true }))); warnings.length = 0; }
  return { errors, warnings, info };
}

// ------------------------------------------------------------------------------------------
// GitHub preview (markdown-it GFM) — DESIGN §10.1-8 and the P9 structural checks.
// ------------------------------------------------------------------------------------------

let _md = null;
export function markdownIt() {
  if (!_md) {
    const MarkdownIt = require("./vendor/markdown-it.min.js");
    _md = new MarkdownIt({ html: true, linkify: false, typographer: false, breaks: false });
  }
  return _md;
}

/** Render Markdown the way github.com would (GFM tables/strikethrough, raw HTML allowed). */
export function renderGfm(markdown) {
  return markdownIt().render(stripFrontMatter(markdown));
}

export function stripFrontMatter(text) {
  const m = text.match(/^---\r?\n[\s\S]*?\r?\n---\r?\n/);
  return m ? text.slice(m[0].length) : text;
}

const RULE_WORDS = /\b(must|never|not modell?ed|not the engine)\b/i;
const LEAK_MARKS = ["!!!", "???", "===", "{ .", ":material-"];
const OUTPUT_LIKE = /^[ \t]*(\?\?\?\+?\s+success\s+"Output"|Output\b[^`]*:\s*)$/;
const VERIFY = /^[ \t]*<!--\s*verify-example\s*-->\s*$/;
const SKIP = /^[ \t]*<!--\s*example-skip:\s*\S.*-->\s*$/;

function stripTags(html) {
  return html.replace(/<pre\b[\s\S]*?<\/pre>/gi, " ").replace(/<code\b[\s\S]*?<\/code>/gi, " ").replace(/<[^>]+>/g, " ");
}

/**
 * Check one Markdown file as github.com would show it.
 * kind: "readme" (README.md / CONTRIBUTING.md: GitHub-only, no Material syntax at all)
 *       or "docs" (a dual-surface page: Material syntax must not carry a rule, no ??? but
 *       success "Output", <details markdown="1"> form, verify-example output rule).
 * Returns { errors, warnings, html }.
 */
export function githubPreview(markdown, { path = "?", kind = "docs" } = {}) {
  const errors = [], warnings = [];
  const err = (rule, line, msg) => errors.push({ rule, msg: `${path}:${line}: ${msg}` });
  const warn = (rule, line, msg) => warnings.push({ rule, msg: `${path}:${line}: ${msg}` });
  const text = stripFrontMatter(markdown);
  const lines = text.split(/\r?\n/);
  const readme = kind === "readme";

  // --- source-level rules -----------------------------------------------------------------
  let inFence = null;
  for (let i = 0; i < lines.length; i++) {
    const ln = lines[i], n = i + 1;
    const fence = ln.match(/^[ \t]*(```+|~~~+)\s*([A-Za-z0-9_+-]*)/);
    if (fence) {
      if (!inFence) inFence = fence[1];
      else if (ln.trim().startsWith(inFence)) inFence = null;
      if (fence[2] === "python" && inFence === fence[1] && lines[i - 1] !== undefined) {
        const prev = lines[i - 1] || "";
        if (!VERIFY.test(prev) && !SKIP.test(prev)) err("fence-marker", n, "python fence without <!-- verify-example --> or <!-- example-skip: reason --> on the line above");
        const close = closingFence(lines, i, fence[1]);
        if (close >= 0) checkOutput(lines, i, close, VERIFY.test(prev), n, readme, err, warn);
      }
      continue;
    }
    if (inFence) continue;
    const t = ln.trim();
    if (/^\?\?\?\+?(\s|$)/.test(t)) {
      if (readme) err("admonition-leak", n, `"${t.slice(0, 40)}" prints literally on github.com (no admonitions in a GitHub-only file)`);
      else if (!/^\?\?\?\+?\s+success\s+"Output"\s*$/.test(t)) err("admonition-leak", n, `"${t.slice(0, 50)}": only ???/???+ success "Output" is allowed on a docs page (use <details markdown="1">)`);
    }
    if (/^!!!(\s|$)/.test(t) && readme) err("admonition-leak", n, `"${t.slice(0, 40)}" prints literally on github.com`);
    if (/^===\s+"/.test(t)) err("tab-leak", n, "Material tab syntax prints literally on github.com");
    if (/\{\s*(\.|#|width=|data-)/.test(ln) && !/^\s*```/.test(ln)) {
      (readme ? err : warn)("attr-list", n, `attr_list "{ … }" prints literally on github.com: ${ln.trim().slice(0, 60)}`);
    }
    if (/:(material|octicons|fontawesome|simple)-[a-z0-9-]+:/.test(ln)) (readme ? err : warn)("icon-shortcode", n, "icon shortcode prints literally on github.com");
    if (/^<details\b/.test(t)) {
      if (!readme && !/markdown="1"/.test(t)) err("details-form", n, '<details> without markdown="1" on a docs page (tables, links and bold inside print literally on the site)');
      const summaryLine = /<summary>/.test(t) ? i : (lines[i + 1] && /^\s*<summary>/.test(lines[i + 1]) ? i + 1 : -1);
      if (summaryLine < 0) err("details-form", n, "<details> must be followed by its <summary> line");
      else if (!/<\/summary>\s*$/.test(lines[summaryLine]) || (lines[summaryLine + 1] || "").trim() !== "") err("details-form", summaryLine + 1, "</summary> must end its line and be followed by a blank line (otherwise github.com keeps the Markdown inside the HTML block)");
    }
    if (/^<picture\b/.test(t) && readme) {
      let j = i, block = "";
      while (j < lines.length && !/<\/picture>/.test(lines[j]) && j < i + 12) block += lines[j++] + "\n";
      block += lines[j] || "";
      if (!/<source\b/.test(block)) err("picture-form", n, "<picture> without a <source> (no dark variant)");
      const img = block.match(/<img\b[^>]*>/);
      if (!img) err("picture-form", n, "<picture> without an <img>");
      else {
        if (!/\balt="[^"]+"/.test(img[0])) err("picture-form", n, "<img> inside <picture> has no alt");
        if (!/\bwidth="?\d/.test(img[0])) warn("picture-form", n, "<img> inside <picture> has no width (the README's only sizing)");
      }
    }
    if (/<figcaption>/.test(ln) && /\*\*|(^|[^*])\*[^*\s][^*]*\*([^*]|$)/.test(ln.replace(/<[^>]+>/g, ""))) warn("figcaption-markdown", n, "Markdown emphasis inside <figcaption> prints as asterisks on github.com (use <strong>/<em>)");
  }

  // --- rendered-HTML rules ----------------------------------------------------------------
  const html = renderGfm(markdown);
  for (const m of html.matchAll(/<p>([\s\S]*?)<\/p>/g)) {
    const t = stripTags(m[1]).replace(/\s+/g, " ").trim();
    if (!RULE_WORDS.test(t)) continue;
    const hits = LEAK_MARKS.filter((k) => t.includes(k));
    if (hits.length) errors.push({ rule: "rule-leak", msg: `${path}: a rule paragraph leaks Material syntax (${hits.join(" ")}): "${t.slice(0, 80)}"` });
  }
  for (const m of html.matchAll(/<details\b[\s\S]*?<\/details>/g)) {
    const t = stripTags(m[0]);
    const hits = ["|---", "](", "!["].filter((h) => t.includes(h));
    if (hits.length) {
      const sum = (m[0].match(/<summary>([\s\S]*?)<\/summary>/) || [, "?"])[1].replace(/<[^>]+>/g, "").trim().slice(0, 60);
      errors.push({ rule: "details-leak", msg: `${path}: details "${sum}" shows raw Markdown on github.com (${hits.join(" ")})` });
    }
  }
  return { errors, warnings, html };
}

function closingFence(lines, open, fence) {
  for (let j = open + 1; j < lines.length; j++) if (lines[j].trim() === fence) return j;
  return -1;
}

/** The runner's own find_output acceptance (tests/docs/test_site_examples.py) as a lint. */
function checkOutput(lines, open, close, verified, n, readme, err, warn) {
  let k = close + 1;
  while (k < lines.length && !lines[k].trim()) k++;
  const next = k < lines.length ? lines[k] : "";
  const nextIsAdmonition = /^[ \t]*\?\?\?\+?\s+success\s+"Output"\s*$/.test(next);
  const nextIsLeadIn = /^[ \t]*Output\b[^`]*:\s*$/.test(next);
  const nextIsText = /^[ \t]*```text\s*$/.test(next);
  const accepted = nextIsAdmonition || nextIsLeadIn || nextIsText;
  if (!accepted) {
    for (let j = close + 1; j <= Math.min(close + 10, lines.length - 1); j++) {
      if (OUTPUT_LIKE.test(lines[j])) { err("output-unchecked", j + 1, "an output block that is not the next non-blank block after its python fence is never checked by the runner"); break; }
    }
    if (verified) warn("output-missing", n, "verify-example with no recorded output block");
  } else if (readme) {
    if (nextIsLeadIn) warn("output-leadin", k + 1, 'README outputs are a bare ```text fence with no "Output:" line');
    if (nextIsAdmonition) err("admonition-leak", k + 1, "??? success \"Output\" prints literally on github.com; the README uses a bare ```text fence");
  } else if (nextIsLeadIn) {
    warn("output-leadin", k + 1, 'docs pages record outputs in a ???+ / ??? success "Output" admonition, not an "Output:" lead-in');
  }
}

/** Minimal document around a preview fragment, for archiving. */
export function wrapPreviewHtml(title, body) {
  return `<!doctype html><html lang="en"><head><meta charset="utf-8"><title>${title}</title>
<meta name="viewport" content="width=device-width,initial-scale=1">
<style>body{max-width:980px;margin:2rem auto;padding:0 1rem;font:16px/1.5 -apple-system,BlinkMacSystemFont,"Segoe UI",Helvetica,Arial,sans-serif;color:#1f2328}
table{border-collapse:collapse}th,td{border:1px solid #d0d7de;padding:.3em .8em}pre{background:#f6f8fa;padding:1em;overflow:auto}code{background:#f6f8fa;padding:.1em .3em;border-radius:4px}
img{max-width:100%}details{border:1px solid #d0d7de;border-radius:6px;padding:.5em 1em;margin:1em 0}summary{cursor:pointer}
@media (prefers-color-scheme: dark){body{background:#0d1117;color:#e6edf3}pre,code{background:#161b22}th,td,details{border-color:#30363d}}</style>
</head><body>
<p><em>github.com preview rendered by markdown-it 14.1.0 (GFM options); raw HTML passes through as on github.com.</em></p>
${body}
</body></html>`;
}
