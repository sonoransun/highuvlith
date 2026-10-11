/* layout.js (owner G0b; DESIGN §4.9, §4.12, §4.5; contract + markup: docs-refactor/impl/G0b.md).
 * window.HUVLayout + window.huvMarkOverflow (diagrams.js, mathjax.js); node: api.install(win).
 *  huvMarkOverflow(root?) -> n  OVERFLOW_SELECTOR blocks get data-huv-overflow="true", tabindex="0",
 *    aria-label="<Equation|Diagram|Table|Code> (scrolls sideways)" while scrollWidth > clientWidth + 1
 *    (one ResizeObserver); first sideways scroll: data-huv-scrolled="true".
 *  huvMotion()  button.huv-motion (delegated click) toggles html[data-huv-motion="paused"] (localStorage
 *    "huv-motion"); img.huv-anim[data-huv-still] shows its still while paused, under reduced motion and
 *    while printing; the animated src waits in data-huv-anim.
 *  huvStillUrl(src, still)  pure: still + the "#gh-*-mode-only" fragment of src.
 *  huvRosetta(root?) -> n  div.huv-rosetta (<p><strong>Python|CLI|TOML|GUI</strong></p> + blocks) gets
 *    div.huv-rosetta__tabs[role=tablist] > button.huv-rosetta__tab[data-huv-tab] and one
 *    div.huv-rosetta__panel[role=tabpanel][data-huv-tab][hidden] per label (<p> = .huv-rosetta__label, hidden);
 *    data-huv-rosetta="tabs"; localStorage "huv-rosetta" syncs the page; arrows/Home/End; print shows all.
 *  huvRosettaGroups(div)  pure -> [{label, slug, nodes}].
 */
(function (root, factory) {
  "use strict";
  var api = factory();
  if (typeof module === "object" && module.exports) module.exports = api;
  else { root.HUVLayout = api; api.install(root); }
})(typeof window !== "undefined" ? window : this, function () {
  "use strict";

  var OVERFLOW_SELECTOR = "div.arithmatex, .huv-diagram, .md-typeset__scrollwrap, .huv-calc__tablewrap, .md-typeset pre > code";
  var LABELS = ["Python", "CLI", "TOML", "GUI"];
  var REDUCE = "(prefers-reduced-motion: reduce)";
  var TAB = ".huv-rosetta__tab", PANEL = ".huv-rosetta__panel", KEY = "data-huv-tab", BUILT = "div.huv-rosetta[data-huv-rosetta]";
  var W, D, observer, printing = false, uid = 0;

  function all(sel, root) { return Array.prototype.slice.call((root || D).querySelectorAll(sel)); }
  function attr(node, map) { Object.keys(map).forEach(function (k) { node.setAttribute(k, map[k]); }); return node; }
  function el(tag, map) { return attr(D.createElement(tag), map); }
  function store(key, value) {
    try { return value === undefined ? W.localStorage.getItem(key) : W.localStorage.setItem(key, value); } catch (e) { return null; }
  }

  function onScroll(e) { if (e.currentTarget.scrollLeft > 0) e.currentTarget.setAttribute("data-huv-scrolled", "true"); }

  function mark(node) {
    var over = node.scrollWidth > node.clientWidth + 1, c = node.classList;
    if (over && node.getAttribute("data-huv-overflow") !== "true") {
      var kind = c.contains("arithmatex") ? "Equation" : c.contains("huv-diagram") ? "Diagram" : node.tagName === "CODE" ? "Code" : "Table";
      attr(node, { "data-huv-overflow": "true", tabindex: "0", "aria-label": kind + " (scrolls sideways)" });
      node.addEventListener("scroll", onScroll, { passive: true });
    } else if (!over && node.hasAttribute("data-huv-overflow")) {
      ["data-huv-overflow", "data-huv-scrolled", "tabindex", "aria-label"].forEach(function (a) { node.removeAttribute(a); });
      node.removeEventListener("scroll", onScroll);
    }
    return over;
  }

  function huvMarkOverflow(root) {
    if (!D) return 0;
    var scope = root && root.querySelectorAll ? root : D;
    var nodes = (scope.matches && scope.matches(OVERFLOW_SELECTOR) ? [scope] : []).concat(all(OVERFLOW_SELECTOR, scope));
    if (!observer && typeof W.ResizeObserver === "function") {
      observer = new W.ResizeObserver(function (entries) { entries.forEach(function (e) { mark(e.target); }); });
    }
    return nodes.filter(function (node) {
      if (observer) observer.observe(node);
      return mark(node);
    }).length;
  }

  function huvStillUrl(src, still) {
    var i = String(src).indexOf("#");
    return i < 0 ? still : String(still).split("#")[0] + src.slice(i);
  }

  function paused() { return D.documentElement.getAttribute("data-huv-motion") === "paused"; }

  function applyMotion() {
    var toStill = printing || paused() || !!(W.matchMedia && W.matchMedia(REDUCE).matches);
    all("img.huv-anim[data-huv-still]").forEach(function (img) {
      var src = img.getAttribute("src") || "", anim = img.getAttribute("data-huv-anim");
      if (toStill) {
        if (anim === null) img.setAttribute("data-huv-anim", anim = src);
        var still = huvStillUrl(anim, img.getAttribute("data-huv-still"));
        if (src !== still) img.setAttribute("src", still);
      } else if (anim !== null && src !== anim) img.setAttribute("src", anim);
    });
    all("button.huv-motion").forEach(function (b) { b.setAttribute("aria-pressed", paused() ? "true" : "false"); });
  }

  function setPaused(flag) {
    if (flag) D.documentElement.setAttribute("data-huv-motion", "paused");
    else D.documentElement.removeAttribute("data-huv-motion");
    store("huv-motion", flag ? "paused" : "playing");
    applyMotion();
  }

  function huvMotion() {
    if (store("huv-motion") === "paused") D.documentElement.setAttribute("data-huv-motion", "paused");
    applyMotion();
  }

  function huvRosettaGroups(div) {
    var groups = [];
    Array.prototype.forEach.call(div.childNodes, function (n) {
      var s = n.nodeType === 1 && n.tagName === "P" && n.children.length === 1 ? n.children[0] : null;
      var text = s && s.tagName === "STRONG" ? n.textContent.trim() : "";
      if (text && LABELS.indexOf(text) >= 0 && s.textContent.trim() === text) groups.push({ label: text, slug: text.toLowerCase(), nodes: [n] });
      else if (groups.length) groups[groups.length - 1].nodes.push(n);
    });
    return groups;
  }

  function selectTab(div, slug) {
    var tabs = all(TAB, div);
    if (!tabs.some(function (t) { return t.getAttribute(KEY) === slug; })) slug = tabs[0].getAttribute(KEY);
    tabs.forEach(function (t) {
      var on = t.getAttribute(KEY) === slug;
      attr(t, { "aria-selected": on ? "true" : "false", tabindex: on ? "0" : "-1" });
    });
    all(PANEL, div).forEach(function (p) { p.hidden = !printing && p.getAttribute(KEY) !== slug; });
  }

  function choose(slug, focusIn) {
    if (focusIn !== undefined) store("huv-rosetta", slug);
    all(BUILT).forEach(function (d) {
      selectTab(d, slug);
      if (focusIn === d) all(TAB + '[' + KEY + '="' + slug + '"]', d)[0].focus();
    });
  }

  function build(div) {
    var groups = huvRosettaGroups(div);
    if (groups.length < 2) return false;
    var id = "huv-rosetta-" + (++uid);
    var tabs = el("div", { "class": "huv-rosetta__tabs", role: "tablist", "aria-label": "Front ends" });
    var panels = groups.map(function (g) {
      var tab = el("button", { type: "button", "class": "huv-rosetta__tab", role: "tab", id: id + "-tab-" + g.slug, "aria-controls": id + "-panel-" + g.slug });
      tab.setAttribute(KEY, g.slug);
      tab.textContent = g.label;
      tabs.appendChild(tab);
      var panel = el("div", { "class": "huv-rosetta__panel", role: "tabpanel", id: id + "-panel-" + g.slug, "aria-labelledby": tab.id });
      panel.setAttribute(KEY, g.slug);
      g.nodes[0].classList.add("huv-rosetta__label");
      g.nodes[0].hidden = true;
      g.nodes.forEach(function (n) { panel.appendChild(n); });
      return panel;
    });
    div.appendChild(tabs);
    panels.forEach(function (p) { div.appendChild(p); });
    div.setAttribute("data-huv-rosetta", "tabs");
    div.addEventListener("click", function (e) {
      var tab = e.target.closest(TAB);
      if (tab) choose(tab.getAttribute(KEY), null);
    });
    div.addEventListener("keydown", function (e) {
      var tab = e.target.closest(TAB), items = all(TAB, div), i = items.indexOf(tab), n = items.length;
      var next = !tab ? -1 : e.key === "ArrowRight" ? (i + 1) % n : e.key === "ArrowLeft" ? (i + n - 1) % n : e.key === "Home" ? 0 : e.key === "End" ? n - 1 : -1;
      if (next < 0) return;
      e.preventDefault();
      choose(items[next].getAttribute(KEY), div);
    });
    return true;
  }

  function huvRosetta(root) {
    var chosen = store("huv-rosetta") || "python";
    return all("div.huv-rosetta:not([data-huv-rosetta])", root).filter(function (div) {
      if (!build(div)) return false;
      selectTab(div, chosen);
      return true;
    }).length;
  }

  function init() {
    if (observer) observer.disconnect();
    huvMarkOverflow();
    huvMotion();
    huvRosetta();
  }

  function onPrint(flag) {
    printing = flag;
    applyMotion();
    choose(store("huv-rosetta") || "python");
  }

  function install(win) {
    W = win;
    D = win.document;
    win.huvMarkOverflow = huvMarkOverflow;
    D.addEventListener("click", function (e) {
      var t = e.target;
      if (t && t.closest && t.closest("button.huv-motion")) setPaused(!paused());
    });
    win.addEventListener("beforeprint", function () { onPrint(true); });
    win.addEventListener("afterprint", function () { onPrint(false); });
    if (win.matchMedia) win.matchMedia(REDUCE).addEventListener("change", applyMotion);
    if (win.document$) win.document$.subscribe(init); else init();
  }

  return { OVERFLOW_SELECTOR: OVERFLOW_SELECTOR, install: install, init: init, huvMarkOverflow: huvMarkOverflow,
    huvMotion: huvMotion, huvStillUrl: huvStillUrl, huvRosetta: huvRosetta, huvRosettaGroups: huvRosettaGroups };
});
