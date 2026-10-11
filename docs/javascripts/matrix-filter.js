/* matrix-filter.js (owner G0b; DESIGN §7.9; contract: docs-refactor/impl/G0b.md). Grade filter for
 * docs/capability-matrix.md; runs only where table.huv-grid exists (page_kit.py).
 * Before the first table's wrapper: div.huv-matrix-filter[role=group] > 4 × button.huv-matrix-filter__chip
 * .huv-chip.huv-chip--<grade>[aria-pressed][data-huv-grade] (+ span.huv-matrix-filter__count) + button
 * .huv-matrix-filter__all. Pressed chips = OR filter (other rows hidden); an all-hidden table, its wrapper and h2
 * get data-huv-empty="true"; ?grade=a,b read on load, written with replaceState.
 * Exports: GRADES, install(win), activate(doc, win), parseGrades(search), formatQuery(search, grades).
 */
(function (root, factory) {
  "use strict";
  var api = factory();
  if (typeof module === "object" && module.exports) module.exports = api;
  else { root.HUVMatrixFilter = api; api.install(root); }
})(typeof window !== "undefined" ? window : this, function () {
  "use strict";

  var GRADES = [["implemented", "✅", "Implemented"], ["simplified", "🔶", "Simplified"], ["theoretical", "🧪", "Theoretical"], ["planned", "🗺️", "Planned"]];
  var G = "data-huv-grade";

  function list(nodes) { return Array.prototype.slice.call(nodes); }

  function parseGrades(search) {
    var m = /[?&]grade=([^&#]*)/.exec(search || "");
    return m ? decodeURIComponent(m[1]).split(",").filter(function (g) { return GRADES.some(function (x) { return x[0] === g; }); }) : [];
  }

  function formatQuery(search, grades) {
    var keep = (search || "").replace(/^\?/, "").split("&").filter(function (p) { return p && p.indexOf("grade=") !== 0; });
    if (grades.length) keep.push("grade=" + grades.join(","));
    return keep.length ? "?" + keep.join("&") : "";
  }

  function activate(doc, win) {
    var tables = list(doc.querySelectorAll("table.huv-grid"));
    if (!tables.length || doc.querySelector(".huv-matrix-filter")) return false;
    var rows = list(doc.querySelectorAll("table.huv-grid tr[" + G + "]"));
    function make(parent, tag, cls, text) {
      var n = doc.createElement(tag);
      n.className = cls;
      if (text) n.textContent = text;
      if (tag === "button") n.type = "button";
      return parent.appendChild(n);
    }
    var bar = doc.createElement("div");
    bar.className = "huv-matrix-filter";
    bar.setAttribute("role", "group");
    bar.setAttribute("aria-label", "Filter by grade");
    var chips = GRADES.map(function (g) {
      var b = make(bar, "button", "huv-matrix-filter__chip huv-chip huv-chip--" + g[0]);
      b.setAttribute("aria-pressed", "false");
      b.setAttribute(G, g[0]);
      make(b, "span", "huv-chip__glyph", g[1]).setAttribute("aria-hidden", "true");
      b.appendChild(doc.createTextNode(g[2] + " "));
      make(b, "span", "huv-matrix-filter__count", String(rows.filter(function (r) { return r.getAttribute(G) === g[0]; }).length));
      return b;
    });
    make(bar, "button", "huv-matrix-filter__all", "Show all");
    var first = tables[0].closest(".md-typeset__scrollwrap") || tables[0];
    first.parentNode.insertBefore(bar, first);

    function apply(active, write) {
      chips.forEach(function (c) { c.setAttribute("aria-pressed", active.indexOf(c.getAttribute(G)) >= 0 ? "true" : "false"); });
      rows.forEach(function (r) { r.hidden = active.length > 0 && active.indexOf(r.getAttribute(G)) < 0; });
      tables.forEach(function (t) {
        var empty = list(t.querySelectorAll("tr[" + G + "]")).every(function (r) { return r.hidden; });
        var wrap = t.closest(".md-typeset__scrollwrap") || t, h2 = wrap;
        while (h2 && h2.tagName !== "H2") h2 = h2.previousElementSibling;
        [t, wrap, h2].forEach(function (n) { if (n) { if (empty) n.setAttribute("data-huv-empty", "true"); else n.removeAttribute("data-huv-empty"); } });
      });
      var loc = win && win.location;
      if (write && loc) win.history.replaceState(null, "", loc.pathname + formatQuery(loc.search, active) + (loc.hash || ""));
    }

    bar.addEventListener("click", function (e) {
      var chip = e.target.closest("button");
      if (!chip || !bar.contains(chip)) return;
      var g = chip.getAttribute(G);
      var active = g ? chips.filter(function (c) { return c.getAttribute("aria-pressed") === "true"; }).map(function (c) { return c.getAttribute(G); }) : [];
      if (g) active.indexOf(g) >= 0 ? active.splice(active.indexOf(g), 1) : active.push(g);
      apply(active, true);
    });
    apply(parseGrades(win && win.location ? win.location.search : ""), false);
    return true;
  }

  function install(win) {
    var run = function () { activate(win.document, win); };
    if (win.document$) win.document$.subscribe(run); else run();
  }

  return { GRADES: GRADES, install: install, activate: activate, parseGrades: parseGrades, formatQuery: formatQuery };
});
