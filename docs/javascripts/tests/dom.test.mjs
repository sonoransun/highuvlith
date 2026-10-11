// A tiny DOM for `node --test docs/javascripts/tests/*.test.mjs` (no jsdom, no npm install).
//
// It implements the part of the DOM that docs/javascripts/*.js touch: elements with
// attributes, class lists, text, child lists and events; a selector engine covering
// tags, `#id`, `.class`, `[attr]`, `[attr=v]`, `[attr^=v]`, `[attr$=v]`, `[attr*=v]`,
// `:not(...)`, `:scope`, `:first-child`, the descendant (` `) and child (`>`) combinators
// and comma lists; a `window` with getComputedStyle, matchMedia, localStorage, history,
// location, requestAnimationFrame (flushed by hand with `win.flushFrames()`),
// MutationObserver / ResizeObserver stubs (ResizeObserver callbacks are triggered by hand
// with `win.resize(el)`), beforeprint/afterprint events and a canvas 2-D context stub.
// Geometry is whatever a test assigns to `clientWidth`, `scrollWidth`, `clientHeight`.
//
// Exports: `createWindow(opts)` -> { win, doc, body }, `h(doc, tag, attrs, children)`.
// Layout and content scripts take their `window` from `install(win)`, so every test
// runs against this shim and never against globals.
import { test } from "node:test";
import assert from "node:assert/strict";

const VOID = new Set(["img", "br", "hr", "input", "source", "meta", "link"]);

class EventShim {
  constructor(type, init) {
    this.type = type;
    Object.assign(this, init || {});
    this.defaultPrevented = false;
    this.target = null;
    this.currentTarget = null;
    this._stop = false;
  }
  preventDefault() { this.defaultPrevented = true; }
  stopPropagation() { this._stop = true; }
}

class Node {
  constructor(doc, nodeType) {
    this.ownerDocument = doc;
    this.nodeType = nodeType;
    this.parentNode = null;
    this.childNodes = [];
    this._listeners = new Map();
  }
  get parentElement() { return this.parentNode && this.parentNode.nodeType === 1 ? this.parentNode : null; }
  addEventListener(type, fn) {
    if (!this._listeners.has(type)) this._listeners.set(type, []);
    this._listeners.get(type).push(fn);
  }
  removeEventListener(type, fn) {
    const list = this._listeners.get(type) || [];
    const i = list.indexOf(fn);
    if (i >= 0) list.splice(i, 1);
  }
  dispatchEvent(event) {
    event.target = event.target || this;
    let node = this;
    while (node && !event._stop) {
      event.currentTarget = node;
      (node._listeners.get(event.type) || []).slice().forEach((fn) => fn.call(node, event));
      const onProp = node["on" + event.type];
      if (typeof onProp === "function") onProp.call(node, event);
      node = node.parentNode || (node === this.ownerDocument ? null : node.nodeType === 9 ? null : null);
    }
    return !event.defaultPrevented;
  }
}

class TextNode extends Node {
  constructor(doc, text) {
    super(doc, 3);
    this.data = String(text);
  }
  get textContent() { return this.data; }
  set textContent(v) { this.data = String(v); }
}

const ATTR_PROPS = ["id", "value", "type", "role", "title", "name", "src", "href", "scope", "placeholder"];

class Element extends Node {
  constructor(doc, tag) {
    super(doc, 1);
    this.tagName = tag.toUpperCase();
    this.localName = tag.toLowerCase();
    this.attributes = new Map();
    this.style = makeStyle();
    this.clientWidth = 0;
    this.clientHeight = 0;
    this.scrollWidth = 0;
    this.scrollHeight = 0;
    this.scrollLeft = 0;
    this.scrollTop = 0;
    this.checked = false;
    this.disabled = false;
    this._value = "";
  }
  get children() { return this.childNodes.filter((n) => n.nodeType === 1); }
  get firstChild() { return this.childNodes[0] || null; }
  get firstElementChild() { return this.children[0] || null; }
  get lastElementChild() { const c = this.children; return c[c.length - 1] || null; }
  get nextSibling() {
    if (!this.parentNode) return null;
    const list = this.parentNode.childNodes;
    return list[list.indexOf(this) + 1] || null;
  }
  get nextElementSibling() {
    let n = this.nextSibling;
    while (n && n.nodeType !== 1) n = n.nextSibling;
    return n;
  }
  get previousElementSibling() {
    if (!this.parentNode) return null;
    const list = this.parentNode.childNodes;
    for (let i = list.indexOf(this) - 1; i >= 0; i--) if (list[i].nodeType === 1) return list[i];
    return null;
  }
  get offsetWidth() { return this.clientWidth; }
  get offsetHeight() { return this.clientHeight; }
  get id() { return this.getAttribute("id") || ""; }
  set id(v) { this.setAttribute("id", v); }
  get className() { return this.getAttribute("class") || ""; }
  set className(v) { this.setAttribute("class", v); }
  get classList() {
    const self = this;
    const list = () => self.className.split(/\s+/).filter(Boolean);
    return {
      add: (...names) => { const l = list(); names.forEach((n) => { if (!l.includes(n)) l.push(n); }); self.className = l.join(" "); },
      remove: (...names) => { self.className = list().filter((n) => !names.includes(n)).join(" "); },
      contains: (n) => list().includes(n),
      toggle: (n, force) => {
        const has = list().includes(n);
        const want = force === undefined ? !has : !!force;
        if (want && !has) self.classList.add(n);
        if (!want && has) self.classList.remove(n);
        return want;
      }
    };
  }
  get hidden() { return this.hasAttribute("hidden"); }
  set hidden(v) { if (v) this.setAttribute("hidden", ""); else this.removeAttribute("hidden"); }
  get value() { return this._value; }
  set value(v) { this._value = String(v); }
  get textContent() {
    return this.childNodes.map((n) => n.textContent).join("");
  }
  set textContent(v) {
    this.childNodes.forEach((n) => { n.parentNode = null; });
    this.childNodes = [];
    if (v !== "" && v !== null && v !== undefined) this.appendChild(new TextNode(this.ownerDocument, v));
  }
  get innerHTML() { return this.childNodes.map(serialize).join(""); }
  get outerHTML() { return serialize(this); }
  get dataset() {
    const self = this;
    return new Proxy({}, {
      get: (_, key) => self.getAttribute("data-" + camelToKebab(key)),
      set: (_, key, v) => { self.setAttribute("data-" + camelToKebab(key), v); return true; }
    });
  }
  setAttribute(name, value) {
    this.attributes.set(name, String(value));
    if (name === "value") this._value = String(value);
    if (name === "checked") this.checked = true;
  }
  getAttribute(name) { return this.attributes.has(name) ? this.attributes.get(name) : null; }
  hasAttribute(name) { return this.attributes.has(name); }
  removeAttribute(name) { this.attributes.delete(name); if (name === "checked") this.checked = false; }
  appendChild(node) { return this.insertBefore(node, null); }
  insertBefore(node, ref) {
    if (node.parentNode) node.parentNode.removeChild(node);
    node.parentNode = this;
    const i = ref ? this.childNodes.indexOf(ref) : -1;
    if (i < 0) this.childNodes.push(node); else this.childNodes.splice(i, 0, node);
    return node;
  }
  removeChild(node) {
    const i = this.childNodes.indexOf(node);
    if (i < 0) throw new Error("not a child");
    this.childNodes.splice(i, 1);
    node.parentNode = null;
    return node;
  }
  replaceChild(node, old) { this.insertBefore(node, old); return this.removeChild(old); }
  remove() { if (this.parentNode) this.parentNode.removeChild(this); }
  append(...nodes) { nodes.forEach((n) => this.appendChild(typeof n === "string" ? new TextNode(this.ownerDocument, n) : n)); }
  contains(node) { for (let n = node; n; n = n.parentNode) if (n === this) return true; return false; }
  getBoundingClientRect() { return { left: 0, top: 0, width: this.clientWidth, height: this.clientHeight, right: this.clientWidth, bottom: this.clientHeight }; }
  focus() { this.ownerDocument.activeElement = this; this.dispatchEvent(new EventShim("focus")); }
  blur() { if (this.ownerDocument.activeElement === this) this.ownerDocument.activeElement = null; this.dispatchEvent(new EventShim("blur")); }
  click() { this.dispatchEvent(new EventShim("click")); }
  scrollIntoView() { this.ownerDocument._scrolledTo = this; }
  scrollTo() {}
  getContext() { return canvasContext(); }
  matches(selector, scope) { return parseSelectorList(selector).some((sel) => matchSelector(this, sel, scope)); }
  closest(selector) { for (let n = this; n && n.nodeType === 1; n = n.parentNode) if (n.matches(selector)) return n; return null; }
  querySelectorAll(selector) {
    const out = [];
    walk(this, (el) => { if (el !== this && el.matches(selector, this)) out.push(el); });
    return out;
  }
  querySelector(selector) { return this.querySelectorAll(selector)[0] || null; }
  cloneNode(deep) {
    const copy = new Element(this.ownerDocument, this.localName);
    this.attributes.forEach((v, k) => copy.setAttribute(k, v));
    if (deep) this.childNodes.forEach((n) => copy.appendChild(n.nodeType === 3 ? new TextNode(this.ownerDocument, n.data) : n.cloneNode(true)));
    return copy;
  }
}

ATTR_PROPS.forEach((name) => {
  if (name === "id" || name === "value") return;
  Object.defineProperty(Element.prototype, name, {
    get() { return this.getAttribute(name) || ""; },
    set(v) { this.setAttribute(name, v); }
  });
});

function camelToKebab(s) { return s.replace(/[A-Z]/g, (c) => "-" + c.toLowerCase()); }

function makeStyle() {
  const store = {};
  return new Proxy(store, {
    get(target, key) {
      if (key === "setProperty") return (k, v) => { target[k] = String(v); };
      if (key === "getPropertyValue") return (k) => target[k] || "";
      if (key === "removeProperty") return (k) => { delete target[k]; };
      return target[key] === undefined ? "" : target[key];
    },
    set(target, key, v) { target[key] = String(v); return true; }
  });
}

function canvasContext() {
  const calls = [];
  const ctx = { calls, fillStyle: "", strokeStyle: "", lineWidth: 1, font: "", textAlign: "", textBaseline: "", globalAlpha: 1 };
  ["setTransform", "clearRect", "beginPath", "moveTo", "lineTo", "stroke", "fill", "fillRect", "fillText", "save", "restore",
    "translate", "rotate", "setLineDash", "arc", "closePath", "strokeRect", "rect", "clip", "measureText", "scale"].forEach((m) => {
    ctx[m] = (...a) => { calls.push([m, ...a]); return m === "measureText" ? { width: 7 * String(a[0]).length } : undefined; };
  });
  return ctx;
}

function walk(el, fn) {
  el.childNodes.forEach((n) => { if (n.nodeType === 1) { fn(n); walk(n, fn); } });
}

function serialize(node) {
  if (node.nodeType === 3) return node.data.replace(/&/g, "&amp;").replace(/</g, "&lt;");
  let s = "<" + node.localName;
  node.attributes.forEach((v, k) => { s += ` ${k}="${String(v).replace(/"/g, "&quot;")}"`; });
  s += ">";
  if (VOID.has(node.localName)) return s;
  return s + node.childNodes.map(serialize).join("") + `</${node.localName}>`;
}

/* ---------------------------------------------------------------- selectors */

const cache = new Map();

function parseSelectorList(text) {
  if (cache.has(text)) return cache.get(text);
  const parts = splitTop(text, ",").map((s) => parseSelector(s.trim()));
  cache.set(text, parts);
  return parts;
}

function splitTop(text, sep) {
  const out = [];
  let depth = 0;
  let cur = "";
  for (const ch of text) {
    if (ch === "(" ) depth++;
    if (ch === ")") depth--;
    if (ch === sep && depth === 0) { out.push(cur); cur = ""; } else cur += ch;
  }
  out.push(cur);
  return out;
}

// -> [{ combinator: null|" "|">", compound: [...] }, ...] left to right
function parseSelector(text) {
  const steps = [];
  let i = 0;
  let combinator = null;
  while (i < text.length) {
    while (text[i] === " ") i++;
    if (text[i] === ">") { combinator = ">"; i++; while (text[i] === " ") i++; }
    let start = i;
    let depth = 0;
    while (i < text.length && (depth > 0 || (text[i] !== " " && text[i] !== ">"))) {
      if (text[i] === "(") depth++;
      if (text[i] === ")") depth--;
      i++;
    }
    const compound = text.slice(start, i);
    steps.push({ combinator: steps.length ? combinator || " " : null, compound: parseCompound(compound) });
    combinator = null;
  }
  return steps;
}

function parseCompound(text) {
  const parts = [];
  // Attribute values may be double-quoted, single-quoted or bare.
  const re = /([a-zA-Z][\w-]*|\*)|#([\w-]+)|\.([\w-]+)|\[([\w-]+)(?:([~|^$*]?=)(?:"([^"]*)"|'([^']*)'|([^\]]*)))?\]|:not\(([^)]*)\)|:([\w-]+)/g;
  let m;
  while ((m = re.exec(text))) {
    if (m[1]) parts.push({ tag: m[1].toLowerCase() });
    else if (m[2]) parts.push({ id: m[2] });
    else if (m[3]) parts.push({ cls: m[3] });
    else if (m[4]) parts.push({ attr: m[4], op: m[5] || null, val: m[6] !== undefined ? m[6] : m[7] !== undefined ? m[7] : m[8] });
    else if (m[9] !== undefined) parts.push({ not: parseSelectorList(m[9]) });
    else if (m[10]) parts.push({ pseudo: m[10] });
  }
  return parts;
}

function matchCompound(el, parts, scope) {
  return parts.every((p) => {
    if (p.tag) return p.tag === "*" || el.localName === p.tag;
    if (p.id) return el.id === p.id;
    if (p.cls) return el.classList.contains(p.cls);
    if (p.attr) {
      if (!el.hasAttribute(p.attr)) return false;
      const v = el.getAttribute(p.attr);
      if (!p.op) return true;
      if (p.op === "=") return v === p.val;
      if (p.op === "^=") return v.startsWith(p.val);
      if (p.op === "$=") return v.endsWith(p.val);
      if (p.op === "*=") return v.includes(p.val);
      if (p.op === "~=") return v.split(/\s+/).includes(p.val);
      return false;
    }
    if (p.not) return !p.not.some((sel) => matchSelector(el, sel, scope));
    if (p.pseudo === "scope") return el === scope;
    if (p.pseudo === "first-child") return el.parentElement && el.parentElement.children[0] === el;
    if (p.pseudo === "last-child") return el.parentElement && el.parentElement.lastElementChild === el;
    if (p.pseudo === "first-of-type") return el.parentElement && el.parentElement.children.filter((c) => c.localName === el.localName)[0] === el;
    if (p.pseudo === "checked") return !!el.checked;
    if (p.pseudo === "focus-visible" || p.pseudo === "hover") return false;
    throw new Error("unsupported pseudo-class :" + p.pseudo);
  });
}

function matchSelector(el, steps, scope) {
  // Match right to left.
  function at(index, node) {
    if (index < 0) return true;
    const step = steps[index];
    if (!node || node.nodeType !== 1 || !matchCompound(node, step.compound, scope)) return false;
    if (index === 0) return true;
    const comb = step.combinator;
    if (comb === ">") return at(index - 1, node.parentElement);
    for (let a = node.parentElement; a; a = a.parentElement) if (at(index - 1, a)) return true;
    return false;
  }
  return at(steps.length - 1, el);
}

/* ---------------------------------------------------------------- document & window */

class Document extends Node {
  constructor() {
    super(null, 9);
    this.ownerDocument = this;
    this.readyState = "complete";
    this.activeElement = null;
    this.title = "";
    this.documentElement = this.createElement("html");
    this.head = this.createElement("head");
    this.body = this.createElement("body");
    this.documentElement.appendChild(this.head);
    this.documentElement.appendChild(this.body);
    this.childNodes = [this.documentElement];
    this.documentElement.parentNode = this;
  }
  createElement(tag) { return new Element(this, tag); }
  createElementNS(ns, tag) { const el = new Element(this, tag); el.namespaceURI = ns; return el; }
  createTextNode(text) { return new TextNode(this, text); }
  createDocumentFragment() { return new Element(this, "#fragment"); }
  querySelectorAll(selector) { return this.documentElement.querySelectorAll(selector); }
  querySelector(selector) { return this.querySelectorAll(selector)[0] || null; }
  getElementById(id) { return this.querySelector("#" + id); }
  getElementsByTagName(tag) { return this.querySelectorAll(tag); }
  contains(node) { return this.documentElement.contains(node); }
}

function storage(throwing) {
  const map = new Map();
  return {
    getItem: (k) => { if (throwing) throw new Error("storage blocked"); return map.has(k) ? map.get(k) : null; },
    setItem: (k, v) => { if (throwing) throw new Error("storage blocked"); map.set(k, String(v)); },
    removeItem: (k) => { if (throwing) throw new Error("storage blocked"); map.delete(k); },
    _map: map
  };
}

export function createWindow(opts) {
  opts = opts || {};
  const doc = new Document();
  const frames = [];
  const resizeObservers = [];
  const media = Object.assign({ "(prefers-reduced-motion: reduce)": false }, opts.media || {});
  const mediaLists = new Map();
  const win = new Node(doc, 0);
  win.document = doc;
  win.window = win;
  win.devicePixelRatio = 1;
  win.innerWidth = opts.width || 1280;
  win.console = { error() {}, warn() {}, log() {} };
  win.getComputedStyle = (el) => ({ getPropertyValue: (k) => (opts.css && opts.css[k]) || el.style.getPropertyValue(k) || "", paddingLeft: "0px", paddingRight: "0px" });
  win.requestAnimationFrame = (fn) => { frames.push(fn); return frames.length; };
  win.cancelAnimationFrame = () => {};
  win.flushFrames = () => { while (frames.length) frames.shift()(); };
  win.setTimeout = (fn) => { frames.push(fn); return frames.length; };
  win.clearTimeout = () => {};
  win.matchMedia = (q) => {
    if (!mediaLists.has(q)) {
      const list = new Node(doc, 0);
      list.media = q;
      Object.defineProperty(list, "matches", { get: () => !!media[q] });
      list.addListener = list.addEventListener;
      mediaLists.set(q, list);
    }
    return mediaLists.get(q);
  };
  win.setMedia = (q, value) => {
    media[q] = value;
    const list = mediaLists.get(q);
    if (list) list.dispatchEvent(new EventShim("change", { matches: value }));
  };
  win.localStorage = storage(!!opts.storageThrows);
  win.history = { entries: [], replaceState(state, title, url) { this.entries.push(url); win.location.search = (String(url).split("?")[1] || "").replace(/#.*$/, "") ? "?" + String(url).split("?")[1].replace(/#.*$/, "") : ""; } };
  win.location = { search: opts.search || "", hash: opts.hash || "", pathname: opts.pathname || "/highuvlith/playground/", href: "" };
  win.Event = EventShim;
  win.CustomEvent = EventShim;
  win.MutationObserver = class { constructor(fn) { this.fn = fn; } observe() {} disconnect() {} };
  if (opts.resizeObserver !== false) {
    win.ResizeObserver = class {
      constructor(fn) { this.fn = fn; this.targets = new Set(); resizeObservers.push(this); }
      observe(el) { this.targets.add(el); }
      unobserve(el) { this.targets.delete(el); }
      disconnect() { this.targets.clear(); }
    };
  }
  win.resize = (el) => {
    resizeObservers.forEach((ro) => { if (!el || ro.targets.has(el)) ro.fn([...ro.targets].filter((t) => !el || t === el).map((t) => ({ target: t })), ro); });
  };
  win.resizeObserverCount = () => resizeObservers.filter((ro) => ro.targets.size > 0).length;
  win.print = () => { win.dispatchEvent(new EventShim("beforeprint")); win.dispatchEvent(new EventShim("afterprint")); };
  if (opts.physics) win.HUVPhysics = opts.physics;
  return { win, doc, body: doc.body };
}

/* Build an element: h(doc, "div", { class: "x", "data-k": "v" }, ["text", child, ...]). */
export function h(doc, tag, attrs, children) {
  const el = doc.createElement(tag);
  Object.entries(attrs || {}).forEach(([k, v]) => { if (v !== null && v !== undefined && v !== false) el.setAttribute(k, v === true ? "" : v); });
  (children || []).forEach((c) => el.appendChild(typeof c === "string" ? doc.createTextNode(c) : c));
  return el;
}

/* ---------------------------------------------------------------- self-tests */

test("dom shim: tree, attributes, classes and text", () => {
  const { doc, body } = createWindow();
  const div = h(doc, "div", { class: "a b", id: "x" }, ["hi ", h(doc, "strong", null, ["there"])]);
  body.appendChild(div);
  assert.equal(div.textContent, "hi there");
  assert.equal(div.children.length, 1);
  assert.ok(div.classList.contains("b"));
  div.classList.toggle("c");
  assert.equal(div.className, "a b c");
  div.hidden = true;
  assert.ok(div.hasAttribute("hidden"));
  assert.equal(doc.getElementById("x"), div);
  assert.equal(div.outerHTML, '<div class="a b c" id="x" hidden="">hi <strong>there</strong></div>');
});

test("dom shim: selectors with combinators, attributes, :not and :scope", () => {
  const { doc, body } = createWindow();
  const wrap = h(doc, "div", { class: "md-typeset" }, [
    h(doc, "div", { class: "arithmatex" }),
    h(doc, "pre", null, [h(doc, "code", { "data-x": "ab#cd" })]),
    h(doc, "table", { class: "huv-grid" }, [h(doc, "tbody", null, [h(doc, "tr", { "data-huv-grade": "planned" }), h(doc, "tr")])]),
    h(doc, "img", { src: "a.png#gh-light-mode-only", class: "huv-anim" })
  ]);
  body.appendChild(wrap);
  assert.equal(doc.querySelectorAll("div.arithmatex, .md-typeset pre > code").length, 2);
  assert.equal(doc.querySelectorAll("tr[data-huv-grade]").length, 1);
  assert.equal(doc.querySelectorAll("tr:not([data-huv-grade])").length, 1);
  assert.equal(doc.querySelectorAll('img[src$="#gh-light-mode-only"]').length, 1);
  assert.equal(doc.querySelectorAll('code[data-x*="#c"]').length, 1);
  assert.equal(doc.querySelectorAll("img[class='huv-anim'][src^=a]").length, 1, "single-quoted and bare attribute values");
  assert.equal(wrap.querySelectorAll(":scope > pre").length, 1);
  assert.equal(wrap.querySelectorAll("code").length, 1);
  assert.equal(doc.querySelector("code").closest(".md-typeset"), wrap);
  assert.equal(doc.querySelectorAll("tr:first-child").length, 1);
});

test("dom shim: events bubble and can be prevented", () => {
  const { doc, body, win } = createWindow();
  const button = h(doc, "button");
  body.appendChild(button);
  const seen = [];
  body.addEventListener("click", (e) => seen.push("body:" + e.target.localName));
  button.addEventListener("click", (e) => { seen.push("button"); e.preventDefault(); });
  const ok = button.dispatchEvent(new win.Event("click"));
  assert.deepEqual(seen, ["button", "body:button"]);
  assert.equal(ok, false);
});

test("dom shim: window services (media, storage, frames, resize observers)", () => {
  const { win, doc } = createWindow({ storageThrows: true });
  assert.throws(() => win.localStorage.setItem("a", "b"));
  const list = win.matchMedia("(prefers-reduced-motion: reduce)");
  assert.equal(list.matches, false);
  let changed = 0;
  list.addEventListener("change", () => changed++);
  win.setMedia("(prefers-reduced-motion: reduce)", true);
  assert.equal(list.matches, true);
  assert.equal(changed, 1);
  let ran = 0;
  win.requestAnimationFrame(() => ran++);
  assert.equal(ran, 0);
  win.flushFrames();
  assert.equal(ran, 1);
  const el = doc.createElement("div");
  let observed = null;
  new win.ResizeObserver((entries) => { observed = entries[0].target; }).observe(el);
  win.resize(el);
  assert.equal(observed, el);
});
