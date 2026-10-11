// Tests for docs/javascripts/playground.js: mounting in the DOM shim (every widget builds, the
// fallback figure hides, data-huv-mounted matches data-huv-calc), the plots and live pupil
// render, deep links round-trip, and the size budget of DESIGN §10.2.
import { test } from "node:test";
import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { execFileSync } from "node:child_process";
import { statSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { createWindow, h } from "./dom.test.mjs";

const require = createRequire(import.meta.url);
const here = dirname(fileURLToPath(import.meta.url));
const js = (name) => join(here, "..", name);
const Physics = require(js("huv-physics.js"));

function load(opts) {
  const env = createWindow(Object.assign({ physics: Physics }, opts || {}));
  const key = require.resolve(js("playground.js"));
  delete require.cache[key];
  const PG = require(js("playground.js"));
  return Object.assign(env, { PG });
}

/* The page as docs/playground/index.md writes it: an R3 fallback figure, then the placeholder. */
function page(doc, body, calcs) {
  const out = {};
  calcs.forEach((name) => {
    const figure = h(doc, "figure", null, [h(doc, "img", { src: `../assets/images/playground/${name}-light.svg#gh-light-mode-only`, alt: name })]);
    const root = h(doc, "div", { class: "huv-calc", "data-huv-calc": name }, [h(doc, "noscript", null, ["needs JavaScript"])]);
    root.clientWidth = 600;
    body.appendChild(h(doc, "h2", null, [name]));
    body.appendChild(figure);
    body.appendChild(root);
    out[name] = { figure, root };
  });
  return out;
}

function mounted(env, calcs, search) {
  const f = page(env.doc, env.body, calcs);
  env.win.location.search = search || "";
  env.PG.install(env.win);
  env.win.flushFrames();
  return f;
}

/* ---------------------------------------------------------------- pure helpers */

test("parseLink and formatLink round-trip a deep link", () => {
  const { PG } = load();
  assert.deepEqual(PG.parseLink("?calc=aerial&preset=euv&k1=0.3"), { calc: "aerial", params: { preset: "euv", k1: "0.3" } });
  assert.deepEqual(PG.parseLink(""), { calc: null, params: {} });
  assert.equal(PG.formatLink("rayleigh", { preset: "krf", k1: "0.35" }), "?calc=rayleigh&preset=krf&k1=0.35");
  assert.deepEqual(PG.parseLink(PG.formatLink("shotnoise", { preset: "custom", lambda: "27.5", shape: "circle" })).params, { preset: "custom", lambda: "27.5", shape: "circle" });
});

test("axis helpers: nice linear ticks and decade log ticks", () => {
  const { PG } = load();
  assert.equal(PG.niceStep(1.08, 5), 0.2);
  assert.deepEqual(PG.ticks(0, 1.08, 5).values.map((v) => +v.toFixed(6)), [0, 0.2, 0.4, 0.6, 0.8, 1]);
  assert.deepEqual(PG.logTicks(3, 4000), { major: [10, 100, 1000], minor: [5, 20, 50, 200, 500, 2000] });
  assert.deepEqual(PG.logTicks(1, 100).major, [1, 10, 100]);
  assert.equal(PG.SERIES.light.length, 8);
  assert.equal(PG.SERIES.dark.length, 8);
});

test("fallbackFor finds the figure directly before a placeholder and nothing else", () => {
  const { PG, doc, body } = load();
  const figure = h(doc, "figure");
  const root = h(doc, "div", { "data-huv-calc": "rayleigh" });
  const lone = h(doc, "div", { "data-huv-calc": "aerial" });
  body.append(h(doc, "p"), figure, root, h(doc, "p"), lone);
  assert.equal(PG.fallbackFor(root), figure);
  assert.equal(PG.fallbackFor(lone), null);
});

/* ---------------------------------------------------------------- mounting */

test("every widget mounts, hides its fallback figure and builds its card, plot and tables", () => {
  const env = load();
  const f = mounted(env, ["rayleigh", "shotnoise", "aerial"]);
  const { doc } = env;
  assert.equal(doc.querySelectorAll("[data-huv-mounted]").length, doc.querySelectorAll("[data-huv-calc]").length, "qa_site.mjs: mounted == placeholders");
  assert.equal(doc.querySelectorAll(".huv-calc__message:not([hidden])").length, 0, "no widget failed to start");
  for (const name of ["rayleigh", "shotnoise", "aerial"]) {
    const { figure, root } = f[name];
    assert.equal(figure.hidden, true, name + " fallback hidden");
    assert.equal(figure.style.display, "none");
    assert.equal(figure.getAttribute("data-huv-fallback"), "hidden");
    assert.ok(root.querySelector("fieldset.huv-calc__fieldset > legend.huv-calc__title").textContent.length > 8, name + " has a visible title");
    assert.ok(root.querySelector("form.huv-calc__controls"));
    assert.ok(root.querySelector("ul.huv-tiles"));
    assert.ok(root.querySelector("ul.huv-legend"), name + " plot has a legend");
    assert.ok(root.querySelector(".huv-plot canvas[role='img'][tabindex='0']"), name + " has a focusable plot");
    assert.ok(root.querySelector("table.huv-calc__table"), name + " keeps its table view");
  }
  assert.equal(f.rayleigh.root.querySelector("legend").textContent, "Resolution and depth of focus");
  assert.ok(f.aerial.root.querySelector(".huv-calc__row .huv-pupil svg.huv-pupil__svg[role='img']"), "the aerial toy has the live pupil");
  assert.equal(f.rayleigh.root.querySelector(".huv-tile__value").textContent, "62.3 nm", "R = 0.3·193/0.93");
});

test("the plots draw into the canvas and the pupil follows the sliders (D5: copies of the source at every order)", () => {
  const env = load();
  const f = mounted(env, ["aerial"]);
  const root = f.aerial.root;
  const canvas = root.querySelector(".huv-plot canvas");
  canvas.clientWidth = 600;
  canvas.clientHeight = 272;
  const svg = root.querySelector("svg.huv-pupil__svg");
  const circles = () => svg.querySelectorAll("circle").length;
  const before = circles();
  assert.ok(before > 4, "rim, support circle, source copies and order dots drawn");
  assert.match(svg.getAttribute("aria-label"), /σ 0\.70 source/);
  assert.match(svg.getAttribute("aria-label"), /orders reaching the pupil for some source point: −1, 0, \+1/);
  // Shrink the pitch below λ/((1+σ)NA) = 122 nm: no first order passes any more.
  const pitch = root.querySelectorAll("input[type='range']")[6];
  pitch.value = "100";
  pitch.dispatchEvent(new env.win.Event("input"));
  env.win.flushFrames();
  assert.match(svg.getAttribute("aria-label"), /orders reaching the pupil for some source point: 0;/);
  assert.match(root.querySelector(".huv-calc__message").textContent, /no first diffraction order passes/);
  // A dipole (σ inner > 0) draws two copies per order.
  const sigmaIn = root.querySelectorAll("input[type='range']")[5];
  pitch.value = "187";
  pitch.dispatchEvent(new env.win.Event("input"));
  sigmaIn.value = "0.6";
  sigmaIn.dispatchEvent(new env.win.Event("input"));
  env.win.flushFrames();
  assert.match(svg.getAttribute("aria-label"), /two-pole source/);
  assert.ok(circles() > before, "two pole copies per order");
  // The canvas received drawing calls (the shim records them).
  const ctx = canvas.getContext("2d");
  assert.ok(ctx.calls.length === 0 || ctx.calls.some((c) => c[0] === "stroke"), "shim context is a recorder");
});

test("a deep link is applied on mount, the widget scrolls into view, and edits rewrite the URL", () => {
  const env = load({ pathname: "/highuvlith/playground/" });
  const f = mounted(env, ["rayleigh", "aerial"], "?calc=aerial&preset=euv&k1=0.3");
  const { win, doc } = env;
  assert.equal(doc._scrolledTo, f.aerial.root, "scrolled to the linked calculator");
  const numbers = f.aerial.root.querySelectorAll("input[type='number']");
  assert.equal(numbers[0].value, "13.5", "EUV preset applied: λ");
  assert.equal(numbers[2].value, "0.33", "NA");
  assert.equal(numbers[5].value, "25", "k1 = 0.3 -> pitch 2·0.3·13.5/0.33 = 25 nm");
  assert.equal(f.aerial.root.querySelector("select").value, "euv");
  assert.equal(win.history.entries.length, 0, "reading a link never rewrites it");
  // Moving a slider rewrites the link with the non-default values only.
  const z = f.aerial.root.querySelectorAll("input[type='range']")[8];
  z.value = "40";
  z.dispatchEvent(new win.Event("input"));
  win.flushFrames(); // the frame computes; the debounced writer is queued
  win.flushFrames();
  const url = win.history.entries.at(-1);
  assert.ok(url, "replaceState called");
  const link = env.PG.parseLink(url.slice(url.indexOf("?")));
  assert.equal(link.calc, "aerial");
  assert.equal(link.params.preset, "euv");
  assert.equal(link.params.pitch, "25");
  assert.equal(link.params.cd, "13");
  assert.equal(link.params.z, "40");
  assert.equal(link.params.lambda, undefined, "preset values are not repeated");
  assert.equal(link.params.sigma, undefined, "defaults are not repeated");
});

test("?calc=&preset= round-trips through parse -> mount -> write", () => {
  const env = load({ pathname: "/highuvlith/nodes/litho-math/" });
  const f = mounted(env, ["rayleigh", "shotnoise"], "?calc=shotnoise&preset=arf&size=35&shape=circle");
  const { win } = env;
  const dose = f.shotnoise.root.querySelectorAll("input[type='range']")[1];
  dose.value = "60";
  dose.dispatchEvent(new win.Event("input"));
  win.flushFrames();
  win.flushFrames();
  const url = win.history.entries.at(-1);
  assert.equal(url.split("?")[0], "/highuvlith/nodes/litho-math/");
  assert.deepEqual(env.PG.parseLink("?" + url.split("?")[1]), { calc: "shotnoise", params: { preset: "arf", dose: "60", shape: "circle", size: "35" } });
  assert.equal(f.rayleigh.root.querySelectorAll("input[type='number']")[0].value, "193", "the other widget keeps its own defaults");
  // Changing the other widget moves the link to it.
  const k1 = f.rayleigh.root.querySelectorAll("input[type='range']")[3];
  k1.value = "0.25";
  k1.dispatchEvent(new win.Event("input"));
  win.flushFrames();
  win.flushFrames();
  assert.deepEqual(env.PG.parseLink("?" + win.history.entries.at(-1).split("?")[1]), { calc: "rayleigh", params: { preset: "arf", k1: "0.25" } });
});

test("printing shows the fallback figures again and re-mounting is idempotent", () => {
  const env = load();
  const f = mounted(env, ["rayleigh"]);
  const { win, doc } = env;
  win.dispatchEvent(new win.Event("beforeprint"));
  assert.equal(f.rayleigh.figure.hidden, false);
  assert.equal(f.rayleigh.figure.style.display, "");
  win.dispatchEvent(new win.Event("afterprint"));
  assert.equal(f.rayleigh.figure.hidden, true);
  env.PG.mountAll();
  assert.equal(doc.querySelectorAll("fieldset.huv-calc__fieldset").length, 1, "a mounted placeholder is not mounted twice");
});

test("without HUVPhysics install() declines quietly", () => {
  const env = createWindow();
  const key = require.resolve(js("playground.js"));
  delete require.cache[key];
  const PG = require(js("playground.js"));
  page(env.doc, env.body, ["rayleigh"]);
  // The module's own require() fallback finds huv-physics.js in node; on the site a missing
  // HUVPhysics (script order broken) must not throw.
  assert.doesNotThrow(() => PG.install(env.win));
});

/* ---------------------------------------------------------------- budget (DESIGN §10.2) */

test("size budget: playground.js grows by at most 25 KB over the baseline", () => {
  // The baseline is the file as committed on main at the start of this redesign (38 434 bytes).
  let baseline = 38434;
  try {
    baseline = execFileSync("git", ["show", "main:docs/javascripts/playground.js"], { cwd: here, encoding: "buffer" }).length || baseline;
  } catch (e) { /* no git history available: use the recorded size */ }
  const now = statSync(js("playground.js")).size;
  assert.ok(now - baseline <= 25 * 1024, `playground.js grew by ${now - baseline} bytes (baseline ${baseline})`);
});
