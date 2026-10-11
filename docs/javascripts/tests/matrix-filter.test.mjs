// Tests for docs/javascripts/matrix-filter.js (grade filter chips, counts, ?grade= deep link).
import { test } from "node:test";
import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { createWindow, h } from "./dom.test.mjs";

const require = createRequire(import.meta.url);
const M = require(join(dirname(fileURLToPath(import.meta.url)), "..", "matrix-filter.js"));

/* The matrix page as docs/hooks/page_kit.py (G0a) renders it: an h2 per domain, Material's
   scroll wrapper, table.huv-grid, tr[data-huv-grade] with id="cm-<slug>". */
function matrixFixture(doc, body) {
  const row = (grade, slug) => h(doc, "tr", { "data-huv-grade": grade, id: "cm-" + slug, class: "huv-row--" + grade }, [h(doc, "td", null, [slug])]);
  const table = (rows) => h(doc, "div", { class: "md-typeset__scrollwrap" }, [h(doc, "div", { class: "md-typeset__table" }, [
    h(doc, "table", { class: "huv-grid" }, [h(doc, "thead", null, [h(doc, "tr", null, [h(doc, "th", null, ["Capability"])])]), h(doc, "tbody", null, rows)])
  ])]);
  const h2a = h(doc, "h2", { id: "core-imaging" }, ["Core imaging"]);
  const h2b = h(doc, "h2", { id: "research-modules" }, ["Research modules"]);
  const wrapA = table([row("implemented", "hopkins"), row("implemented", "socs"), row("simplified", "flare")]);
  const wrapB = table([row("theoretical", "ptycho"), row("planned", "mnsl")]);
  const page = h(doc, "div", { class: "md-typeset" }, [h2a, h(doc, "p", null, ["lead"]), wrapA, h2b, h(doc, "p"), wrapB]);
  body.appendChild(page);
  return { page, h2a, h2b, wrapA, wrapB, rows: page.querySelectorAll("tr[data-huv-grade]") };
}

test("parseGrades and formatQuery round-trip the ?grade= deep link", () => {
  assert.deepEqual(M.parseGrades("?grade=planned,implemented"), ["planned", "implemented"]);
  assert.deepEqual(M.parseGrades("?x=1&grade=theoretical#stochastics"), ["theoretical"]);
  assert.deepEqual(M.parseGrades("?grade=bogus,simplified"), ["simplified"], "unknown grades are dropped");
  assert.deepEqual(M.parseGrades(""), []);
  assert.equal(M.formatQuery("", ["planned"]), "?grade=planned");
  assert.equal(M.formatQuery("?grade=planned&x=1", []), "?x=1", "nothing pressed: no grade parameter");
  assert.equal(M.formatQuery("?grade=planned", ["implemented", "planned"]), "?grade=implemented,planned");
  assert.deepEqual(M.parseGrades(M.formatQuery("", ["simplified", "theoretical"])), ["simplified", "theoretical"]);
});

test("the filter activates only where table.huv-grid exists and builds four chips with counts", () => {
  const { win, doc, body } = createWindow();
  assert.equal(M.activate(doc, win), false, "a page without the matrix is left alone");
  const f = matrixFixture(doc, body);
  assert.equal(M.activate(doc, win), true);
  assert.equal(M.activate(doc, win), false, "idempotent");
  const bar = doc.querySelector(".huv-matrix-filter");
  assert.equal(bar.getAttribute("role"), "group");
  assert.equal(bar.nextElementSibling, f.wrapA, "inserted directly before the first table's wrapper");
  const chips = bar.querySelectorAll("button.huv-matrix-filter__chip");
  assert.equal(chips.length, 4);
  assert.deepEqual(chips.map((c) => c.getAttribute("data-huv-grade")), ["implemented", "simplified", "theoretical", "planned"]);
  assert.deepEqual(chips.map((c) => c.querySelector(".huv-matrix-filter__count").textContent), ["2", "1", "1", "1"]);
  assert.ok(chips[0].classList.contains("huv-chip--implemented"));
  assert.equal(chips[0].querySelector(".huv-chip__glyph").getAttribute("aria-hidden"), "true");
  assert.ok(chips.every((c) => c.getAttribute("aria-pressed") === "false" && c.getAttribute("type") === "button"));
  assert.equal(bar.querySelector(".huv-matrix-filter__all").textContent, "Show all");
  assert.ok(f.rows.every((r) => !r.hidden), "nothing pressed: every row visible");
});

test("pressing chips hides and shows tr[data-huv-grade], dims empty domains and writes ?grade=", () => {
  const { win, doc, body } = createWindow({ pathname: "/highuvlith/capability-matrix/", hash: "#stochastics" });
  const f = matrixFixture(doc, body);
  M.activate(doc, win);
  const chip = (g) => doc.querySelector(`.huv-matrix-filter__chip[data-huv-grade="${g}"]`);
  chip("planned").click();
  assert.equal(chip("planned").getAttribute("aria-pressed"), "true");
  assert.deepEqual(f.rows.map((r) => r.hidden), [true, true, true, true, false]);
  assert.equal(f.h2a.getAttribute("data-huv-empty"), "true", "a domain with no visible row is dimmed");
  assert.equal(f.wrapA.getAttribute("data-huv-empty"), "true");
  assert.equal(f.wrapA.querySelector("table").getAttribute("data-huv-empty"), "true");
  assert.equal(f.h2b.hasAttribute("data-huv-empty"), false);
  assert.equal(win.history.entries.at(-1), "/highuvlith/capability-matrix/?grade=planned#stochastics", "deep link written, fragment kept");
  chip("implemented").click();
  assert.deepEqual(f.rows.map((r) => r.hidden), [false, false, true, true, false], "chips are an OR filter");
  assert.equal(f.h2a.hasAttribute("data-huv-empty"), false);
  assert.equal(win.history.entries.at(-1), "/highuvlith/capability-matrix/?grade=planned,implemented#stochastics");
  chip("planned").click();
  assert.deepEqual(f.rows.map((r) => r.hidden), [false, false, true, true, true]);
  doc.querySelector(".huv-matrix-filter__all").click();
  assert.ok(f.rows.every((r) => !r.hidden));
  assert.ok(doc.querySelectorAll('.huv-matrix-filter__chip[aria-pressed="true"]').length === 0);
  assert.equal(win.history.entries.at(-1), "/highuvlith/capability-matrix/#stochastics", "no grade parameter when nothing is pressed");
});

test("?grade= in the URL is applied on activation and round-trips", () => {
  const { win, doc, body } = createWindow({ search: "?grade=theoretical,planned", pathname: "/highuvlith/capability-matrix/" });
  const f = matrixFixture(doc, body);
  M.activate(doc, win);
  assert.deepEqual(f.rows.map((r) => r.hidden), [true, true, true, false, false]);
  assert.equal(doc.querySelector('.huv-matrix-filter__chip[data-huv-grade="theoretical"]').getAttribute("aria-pressed"), "true");
  assert.equal(win.history.entries.length, 0, "reading the link does not rewrite it");
  doc.querySelector('.huv-matrix-filter__chip[data-huv-grade="simplified"]').click();
  assert.equal(win.history.entries.at(-1), "/highuvlith/capability-matrix/?grade=theoretical,planned,simplified");
  assert.deepEqual(M.parseGrades(win.history.entries.at(-1).split("?")[1] ? "?" + win.history.entries.at(-1).split("?")[1] : ""), ["theoretical", "planned", "simplified"]);
});

test("install() activates on load and on document$", () => {
  const { win, doc, body } = createWindow();
  matrixFixture(doc, body);
  let subscriber = null;
  win.document$ = { subscribe(fn) { subscriber = fn; fn(); } };
  M.install(win);
  assert.ok(doc.querySelector(".huv-matrix-filter"));
  doc.querySelector(".huv-matrix-filter").remove();
  subscriber();
  assert.ok(doc.querySelector(".huv-matrix-filter"), "re-activated after instant navigation");
});
