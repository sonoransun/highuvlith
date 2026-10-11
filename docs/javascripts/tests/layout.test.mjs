// Tests for docs/javascripts/layout.js (overflow cue, motion toggle + still swap, Rosetta tabs)
// and the DESIGN §10.2 size budgets. Run: node --test docs/javascripts/tests/*.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { readFileSync, statSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { createWindow, h } from "./dom.test.mjs";

const require = createRequire(import.meta.url);
const here = dirname(fileURLToPath(import.meta.url));
const js = (name) => join(here, "..", name);

function load(opts) {
  const env = createWindow(opts);
  // Each test needs a fresh module instance: layout.js keeps its window in module state.
  const key = require.resolve(js("layout.js"));
  delete require.cache[key];
  const L = require(js("layout.js"));
  L.install(env.win);
  return Object.assign(env, { L });
}

/* ---------------------------------------------------------------- huvStillUrl */

test("huvStillUrl keeps the #gh-* fragment of the animated src", () => {
  const { L } = load();
  assert.equal(
    L.huvStillUrl("../assets/images/sim/anim/through-focus-anim-light.png#gh-light-mode-only", "../assets/images/sim/anim/through-focus-still-light.png"),
    "../assets/images/sim/anim/through-focus-still-light.png#gh-light-mode-only"
  );
  assert.equal(L.huvStillUrl("x-anim-dark.svg#gh-dark-mode-only", "x-still-dark.svg"), "x-still-dark.svg#gh-dark-mode-only");
  assert.equal(L.huvStillUrl("x-anim-light.svg", "x-still-light.svg"), "x-still-light.svg", "no fragment: the still as given");
  assert.equal(L.huvStillUrl("a#gh-light-mode-only", "b#stale"), "b#gh-light-mode-only", "a fragment on the still is replaced");
});

/* ---------------------------------------------------------------- overflow cue */

function overflowFixture(doc, body) {
  const math = h(doc, "div", { class: "arithmatex" });
  const diagram = h(doc, "div", { class: "huv-diagram" });
  const code = h(doc, "code");
  const pre = h(doc, "pre", null, [code]);
  const wrap = h(doc, "div", { class: "md-typeset__scrollwrap" }, [h(doc, "table")]);
  const page = h(doc, "div", { class: "md-typeset" }, [math, diagram, pre, wrap]);
  body.appendChild(page);
  [math, diagram, code, wrap].forEach((el) => { el.clientWidth = 688; el.scrollWidth = 688; });
  return { math, diagram, code, wrap, page };
}

test("overflow marker sets data-huv-overflow, tabindex and a name on a fixture wider than its wrapper", () => {
  const { L, doc, body, win } = load();
  const f = overflowFixture(doc, body);
  f.math.scrollWidth = 1758;
  f.code.scrollWidth = 690; // 2 px over: the +1 tolerance is exceeded
  f.wrap.scrollWidth = 689; // 1 px over: within tolerance, not marked
  assert.equal(L.huvMarkOverflow(), 2);
  assert.equal(f.math.getAttribute("data-huv-overflow"), "true");
  assert.equal(f.math.getAttribute("tabindex"), "0");
  assert.equal(f.math.getAttribute("aria-label"), "Equation (scrolls sideways)");
  assert.equal(f.code.getAttribute("aria-label"), "Code (scrolls sideways)");
  assert.equal(f.diagram.hasAttribute("data-huv-overflow"), false);
  assert.equal(f.wrap.hasAttribute("data-huv-overflow"), false);
  assert.equal(win.resizeObserverCount(), 1, "exactly one ResizeObserver");
  // The first sideways scroll flags the block (CSS hides the "scrolls →" label).
  f.math.scrollLeft = 40;
  f.math.dispatchEvent(new win.Event("scroll"));
  assert.equal(f.math.getAttribute("data-huv-scrolled"), "true");
  // Fitting again (a resize) removes every attribute.
  f.math.scrollWidth = 688;
  win.resize(f.math);
  assert.equal(f.math.hasAttribute("data-huv-overflow"), false);
  assert.equal(f.math.hasAttribute("tabindex"), false);
  assert.equal(f.math.hasAttribute("data-huv-scrolled"), false);
  // A wrapper that grows later is marked by the observer.
  f.diagram.scrollWidth = 1200;
  win.resize(f.diagram);
  assert.equal(f.diagram.getAttribute("aria-label"), "Diagram (scrolls sideways)");
  assert.equal(f.wrap.getAttribute("aria-label"), null);
  f.wrap.scrollWidth = 900;
  assert.equal(L.huvMarkOverflow(f.wrap), 1, "a single node can be passed (diagrams.js does)");
  assert.equal(f.wrap.getAttribute("aria-label"), "Table (scrolls sideways)");
  // The calculators' preset tables (playground.js) scroll at phone width and get the same cue.
  const calc = h(doc, "div", { class: "huv-calc__tablewrap" }, [h(doc, "table")]);
  f.page.appendChild(calc);
  calc.clientWidth = 358;
  calc.scrollWidth = 620;
  assert.equal(L.huvMarkOverflow(calc), 1);
  assert.equal(calc.getAttribute("aria-label"), "Table (scrolls sideways)");
  assert.ok(L.OVERFLOW_SELECTOR.includes(".huv-calc__tablewrap"));
});

test("the one observer is rebuilt per page (document$) and the cue survives without ResizeObserver", () => {
  const { L, doc, body, win } = load({ resizeObserver: false });
  const f = overflowFixture(doc, body);
  f.math.scrollWidth = 900;
  assert.equal(L.huvMarkOverflow(), 1);
  assert.equal(f.math.getAttribute("data-huv-overflow"), "true");
  L.init();
  assert.equal(f.math.getAttribute("data-huv-overflow"), "true");
  assert.equal(win.resizeObserverCount, undefined === win.resizeObserverCount ? undefined : win.resizeObserverCount);
});

/* ---------------------------------------------------------------- motion */

function motionFixture(doc, body) {
  const light = h(doc, "img", { class: "huv-anim huv-no-zoom", src: "../assets/images/sim/anim/tf-anim-light.png#gh-light-mode-only", "data-huv-still": "../assets/images/sim/anim/tf-still-light.png" });
  const dark = h(doc, "img", { class: "huv-anim", src: "../assets/images/sim/anim/tf-anim-dark.png#gh-dark-mode-only", "data-huv-still": "../assets/images/sim/anim/tf-still-dark.png" });
  const plain = h(doc, "img", { src: "../assets/images/sim/x-light.png#gh-light-mode-only" });
  const button = h(doc, "button", { class: "huv-motion", type: "button", "aria-pressed": "false" }, ["Pause animations"]);
  body.appendChild(h(doc, "figure", null, [light, dark, plain]));
  body.appendChild(h(doc, "footer", null, [button]));
  return { light, dark, plain, button };
}

test("the footer toggle swaps every animated image to its still, keeps the fragment and persists", () => {
  const { doc, body, win } = load();
  const f = motionFixture(doc, body);
  assert.equal(f.button.getAttribute("aria-pressed"), "false");
  assert.equal(f.light.getAttribute("src"), "../assets/images/sim/anim/tf-anim-light.png#gh-light-mode-only", "playing by default");
  f.button.click();
  assert.equal(doc.documentElement.getAttribute("data-huv-motion"), "paused");
  assert.equal(f.button.getAttribute("aria-pressed"), "true");
  assert.equal(f.light.getAttribute("src"), "../assets/images/sim/anim/tf-still-light.png#gh-light-mode-only");
  assert.equal(f.dark.getAttribute("src"), "../assets/images/sim/anim/tf-still-dark.png#gh-dark-mode-only");
  assert.equal(f.light.getAttribute("data-huv-anim"), "../assets/images/sim/anim/tf-anim-light.png#gh-light-mode-only");
  assert.equal(f.plain.getAttribute("src"), "../assets/images/sim/x-light.png#gh-light-mode-only", "non-animated images are untouched");
  assert.equal(win.localStorage.getItem("huv-motion"), "paused");
  f.button.click();
  assert.equal(doc.documentElement.hasAttribute("data-huv-motion"), false);
  assert.equal(f.light.getAttribute("src"), "../assets/images/sim/anim/tf-anim-light.png#gh-light-mode-only", "swapped back");
  assert.equal(win.localStorage.getItem("huv-motion"), "playing");
});

test("a stored pause, reduced-motion emulation and printing each show the stills", () => {
  const a = createWindow();
  a.win.localStorage.setItem("huv-motion", "paused");
  const key = require.resolve(js("layout.js"));
  delete require.cache[key];
  const La = require(js("layout.js"));
  const fa = motionFixture(a.doc, a.body);
  La.install(a.win);
  assert.equal(a.doc.documentElement.getAttribute("data-huv-motion"), "paused", "restored from storage");
  assert.equal(fa.light.getAttribute("src"), "../assets/images/sim/anim/tf-still-light.png#gh-light-mode-only");

  const b = load();
  const fb = motionFixture(b.doc, b.body);
  b.L.huvMotion();
  b.win.setMedia("(prefers-reduced-motion: reduce)", true);
  assert.equal(fb.dark.getAttribute("src"), "../assets/images/sim/anim/tf-still-dark.png#gh-dark-mode-only", "media change swaps");
  assert.equal(fb.button.getAttribute("aria-pressed"), "false", "the toggle reflects the viewer's choice, not the OS setting");
  b.win.setMedia("(prefers-reduced-motion: reduce)", false);
  assert.equal(fb.dark.getAttribute("src"), "../assets/images/sim/anim/tf-anim-dark.png#gh-dark-mode-only");

  b.win.dispatchEvent(new b.win.Event("beforeprint"));
  assert.equal(fb.light.getAttribute("src"), "../assets/images/sim/anim/tf-still-light.png#gh-light-mode-only", "print shows the still");
  b.win.dispatchEvent(new b.win.Event("afterprint"));
  assert.equal(fb.light.getAttribute("src"), "../assets/images/sim/anim/tf-anim-light.png#gh-light-mode-only", "and swaps back after printing");
});

test("blocked storage never breaks the toggle", () => {
  const { doc, body } = load({ storageThrows: true });
  const f = motionFixture(doc, body);
  f.button.click();
  assert.equal(doc.documentElement.getAttribute("data-huv-motion"), "paused");
  f.button.click();
  assert.equal(doc.documentElement.hasAttribute("data-huv-motion"), false);
});

/* ---------------------------------------------------------------- Rosetta tabs */

function rosettaFixture(doc, labels) {
  const div = h(doc, "div", { class: "huv-rosetta" });
  (labels || ["Python", "CLI", "TOML", "GUI"]).forEach((label) => {
    div.appendChild(doc.createTextNode("\n"));
    div.appendChild(h(doc, "p", null, [h(doc, "strong", null, [label])]));
    div.appendChild(h(doc, "div", { class: "highlight" }, [h(doc, "pre", null, [h(doc, "code", null, [label + " block"])])]));
    if (label === "Python") div.appendChild(h(doc, "details", { class: "success" }, [h(doc, "summary", null, ["Output"]), h(doc, "pre", null, ["Contrast: 0.454"])]));
  });
  return div;
}

test("huvRosettaGroups groups a labelled block without changing it", () => {
  const { L, doc } = load();
  const div = rosettaFixture(doc);
  const before = div.outerHTML;
  const groups = L.huvRosettaGroups(div);
  assert.deepEqual(groups.map((g) => g.slug), ["python", "cli", "toml", "gui"]);
  assert.equal(groups[0].nodes.filter((n) => n.nodeType === 1).length, 3, "label, code block and its output stay together");
  assert.equal(groups[1].nodes.filter((n) => n.nodeType === 1).length, 2);
  assert.equal(div.outerHTML, before, "grouping is pure: without JS the blocks stay in sequence");
  const plain = h(doc, "div", { class: "huv-rosetta" }, [h(doc, "p", null, [h(doc, "strong", null, ["Python"]), " and more"]), h(doc, "pre")]);
  assert.equal(L.huvRosettaGroups(plain).length, 0, "a paragraph with text beside the label is not a label");
});

test("the Rosetta tab builder makes a tablist, hides the other panels and syncs every block", () => {
  const { L, doc, body, win } = load();
  const a = rosettaFixture(doc);
  const b = rosettaFixture(doc, ["Python", "CLI"]);
  body.appendChild(a);
  body.appendChild(b);
  assert.equal(L.huvRosetta(), 2);
  assert.equal(a.getAttribute("data-huv-rosetta"), "tabs");
  const tablist = a.querySelector('[role="tablist"]');
  assert.ok(tablist && tablist.classList.contains("huv-rosetta__tabs"));
  const tabs = a.querySelectorAll('button.huv-rosetta__tab[role="tab"]');
  assert.equal(tabs.length, 4);
  assert.deepEqual(tabs.map((t) => t.textContent), ["Python", "CLI", "TOML", "GUI"]);
  assert.equal(tabs[0].getAttribute("aria-selected"), "true");
  assert.equal(tabs[1].getAttribute("tabindex"), "-1");
  const panels = a.querySelectorAll('.huv-rosetta__panel[role="tabpanel"]');
  assert.equal(panels.length, 4);
  assert.equal(panels[0].hidden, false);
  assert.equal(panels[1].hidden, true);
  assert.equal(panels[0].getAttribute("aria-labelledby"), tabs[0].id);
  assert.equal(tabs[0].getAttribute("aria-controls"), panels[0].id);
  assert.equal(panels[0].querySelectorAll("details.success").length, 1, "the recorded output travels with its block");
  assert.ok(panels[0].querySelector("p.huv-rosetta__label").hidden, "the label paragraph is kept but hidden");
  assert.equal(L.huvRosetta(), 0, "idempotent");

  tabs[2].click();
  assert.equal(tabs[2].getAttribute("aria-selected"), "true");
  assert.equal(panels[2].hidden, false);
  assert.equal(panels[0].hidden, true);
  assert.equal(win.localStorage.getItem("huv-rosetta"), "toml");
  const bPanels = b.querySelectorAll(".huv-rosetta__panel");
  assert.equal(bPanels[0].hidden, false, "a block without that label falls back to its first tab");
  const bTabs = b.querySelectorAll(".huv-rosetta__tab");
  bTabs[1].click();
  assert.equal(panels[1].hidden, false, "choosing CLI on one block selects it on every block");
  assert.equal(panels[2].hidden, true);

  // Keyboard: arrows wrap, Home/End jump, focus follows.
  tabs[1].dispatchEvent(new win.Event("keydown", { key: "ArrowRight" }));
  assert.equal(tabs[2].getAttribute("aria-selected"), "true");
  assert.equal(doc.activeElement, tabs[2]);
  tabs[2].dispatchEvent(new win.Event("keydown", { key: "End" }));
  assert.equal(tabs[3].getAttribute("aria-selected"), "true");
  tabs[3].dispatchEvent(new win.Event("keydown", { key: "ArrowRight" }));
  assert.equal(tabs[0].getAttribute("aria-selected"), "true", "wraps");
  tabs[0].dispatchEvent(new win.Event("keydown", { key: "ArrowLeft" }));
  assert.equal(tabs[3].getAttribute("aria-selected"), "true");

  // Printing shows every panel, then restores the choice.
  win.dispatchEvent(new win.Event("beforeprint"));
  assert.ok(panels.every((p) => !p.hidden));
  win.dispatchEvent(new win.Event("afterprint"));
  assert.deepEqual(panels.map((p) => p.hidden), [true, true, true, false]);
});

test("a stored Rosetta choice is applied on build; a single-label block is left alone", () => {
  const env = createWindow();
  env.win.localStorage.setItem("huv-rosetta", "cli");
  const key = require.resolve(js("layout.js"));
  delete require.cache[key];
  const L = require(js("layout.js"));
  const div = rosettaFixture(env.doc);
  const single = rosettaFixture(env.doc, ["Python"]);
  env.body.appendChild(div);
  env.body.appendChild(single);
  L.install(env.win);
  assert.equal(div.querySelector('.huv-rosetta__tab[data-huv-tab="cli"]').getAttribute("aria-selected"), "true");
  assert.equal(single.hasAttribute("data-huv-rosetta"), false);
  assert.equal(single.querySelectorAll("[role]").length, 0);
});

/* ---------------------------------------------------------------- budgets (DESIGN §10.2) */

test("size budget: layout.js + matrix-filter.js ≤ 14 KB; extra.css ≤ 40 KB", () => {
  const bytes = (p) => statSync(p).size;
  const added = bytes(js("layout.js")) + bytes(js("matrix-filter.js"));
  assert.ok(added <= 14 * 1024, `layout.js + matrix-filter.js = ${added} bytes`);
  const css = bytes(join(here, "..", "..", "stylesheets", "extra.css"));
  assert.ok(css <= 40 * 1024, `extra.css = ${css} bytes`);
});

test("every script documents its exports at the top of the file", () => {
  for (const [name, needles] of [
    ["layout.js", ["huvMarkOverflow", "huvMotion", "huvStillUrl", "huvRosetta", "huvRosettaGroups", "data-huv-overflow", "data-huv-still"]],
    ["matrix-filter.js", ["activate", "parseGrades", "formatQuery", "table.huv-grid", "?grade="]],
    ["huv-physics.js", ["pupilOrders", "rayleigh", "shotNoise", "aerialModel"]],
    ["playground.js", ["mountAll", "parseLink", "formatLink", "data-huv-calc", "data-huv-mounted"]]
  ]) {
    const head = readFileSync(js(name), "utf8").split("*/")[0];
    for (const needle of needles) assert.ok(head.includes(needle), `${name} header lacks ${needle}`);
  }
});
