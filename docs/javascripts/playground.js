/* Playground calculators for docs/playground/index.md.
 *
 * Each <div class="huv-calc" data-huv-calc="rayleigh|shotnoise|aerial"> placeholder is
 * turned into a widget. The physics lives in huv-physics.js (window.HUVPhysics); this
 * file only builds the controls, tables and the canvas plot. Widgets re-mount on every
 * page change (Material's instant navigation) and redraw when the palette changes.
 */
(function () {
  "use strict";

  var P = window.HUVPhysics;
  if (!P) return;

  /* ---------------------------------------------------------------- formatting */

  var SUPERSCRIPT = { "-": "⁻", "0": "⁰", "1": "¹", "2": "²", "3": "³",
    "4": "⁴", "5": "⁵", "6": "⁶", "7": "⁷", "8": "⁸", "9": "⁹" };

  function fixed(v, digits) {
    if (!isFinite(v)) return "—";
    return v.toLocaleString("en-US", { minimumFractionDigits: digits, maximumFractionDigits: digits });
  }

  function sig(v, digits) {
    if (!isFinite(v)) return "—";
    if (Math.abs(v) >= Math.pow(10, digits)) return Math.round(v).toLocaleString("en-US");
    return v.toLocaleString("en-US", { maximumSignificantDigits: digits });
  }

  function sci(v, digits) {
    if (!isFinite(v) || v === 0) return sig(v, digits);
    var parts = v.toExponential(digits - 1).split("e");
    var exponent = String(parseInt(parts[1], 10)).replace(/[-0-9]/g, function (c) { return SUPERSCRIPT[c]; });
    return parts[0] + " × 10" + exponent;
  }

  function percent(v) {
    if (!isFinite(v)) return "—";
    return sig(v * 100, 2) + " %";
  }

  function nm(v, digits) {
    return (digits === undefined ? sig(v, 3) : fixed(v, digits)) + " nm";
  }

  /* ---------------------------------------------------------------- DOM helpers */

  var uid = 0;

  function el(tag, props, children) {
    var node = document.createElement(tag);
    if (props) {
      Object.keys(props).forEach(function (key) {
        var value = props[key];
        if (value === null || value === undefined || value === false) return;
        if (key === "text") node.textContent = value;
        else if (key === "className") node.className = value;
        else node.setAttribute(key, value === true ? "" : String(value));
      });
    }
    (children || []).forEach(function (child) {
      if (child === null || child === undefined) return;
      node.appendChild(typeof child === "string" ? document.createTextNode(child) : child);
    });
    return node;
  }

  function decimals(step) {
    var s = String(step);
    return s.indexOf(".") < 0 ? 0 : s.length - s.indexOf(".") - 1;
  }

  /* A labelled slider paired with a number box. Both edit the same value; typing an
     out-of-range number marks the box invalid until it is fixed or left (then clamped). */
  function rangeField(opts) {
    var id = "huv-field-" + ++uid;
    var hintId = opts.hint ? id + "-hint" : null;
    var min = opts.min;
    var max = opts.max;
    var step = opts.step;
    var label = el("label", { for: id, id: id + "-label", text: opts.label + (opts.unit ? " (" + opts.unit + ")" : "") });
    var range = el("input", {
      type: "range", id: id + "-range", min: min, max: max, step: step, value: opts.value,
      "aria-labelledby": id + "-label", "aria-describedby": hintId
    });
    var number = el("input", {
      type: "number", id: id, min: min, max: max, step: "any", value: opts.value,
      inputmode: "decimal", "aria-describedby": hintId
    });
    var hint = opts.hint ? el("span", { className: "huv-field__hint", id: hintId, text: opts.hint }) : null;
    var node = el("div", { className: "huv-field" }, [label, el("div", { className: "huv-field__row" }, [range, number]), hint]);
    var value = Number(opts.value);

    function clamp(v) {
      return Math.min(max, Math.max(min, v));
    }

    function show(v) {
      value = v;
      range.value = String(v);
      number.value = String(Number(v.toFixed(Math.max(decimals(step), 0) + 2)));
      number.removeAttribute("aria-invalid");
    }

    range.addEventListener("input", function () {
      value = Number(range.value);
      number.value = range.value;
      number.removeAttribute("aria-invalid");
      opts.onInput();
    });
    number.addEventListener("input", function () {
      var v = parseFloat(number.value);
      if (isFinite(v) && v >= min && v <= max) {
        value = v;
        range.value = String(v);
        number.removeAttribute("aria-invalid");
        opts.onInput();
      } else {
        number.setAttribute("aria-invalid", "true");
      }
    });
    number.addEventListener("change", function () {
      var v = parseFloat(number.value);
      show(clamp(isFinite(v) ? v : value));
      opts.onInput();
    });

    return {
      node: node,
      get: function () { return value; },
      set: function (v) { show(clamp(v)); },
      setLimits: function (newMin, newMax, newStep) {
        if (newStep === undefined) newStep = step;
        if (newMin === min && newMax === max && newStep === step) return;
        min = newMin;
        max = newMax;
        step = newStep;
        range.min = number.min = String(min);
        range.max = number.max = String(max);
        range.step = String(step);
        show(clamp(value));
      },
      setHint: function (text) { if (hint) hint.textContent = text; }
    };
  }

  function selectField(opts) {
    var id = "huv-field-" + ++uid;
    var select = el("select", { id: id }, opts.options.map(function (o) {
      return el("option", { value: o[0], text: o[1] });
    }));
    select.value = opts.value;
    select.addEventListener("change", function () { opts.onChange(select.value); });
    return {
      node: el("div", { className: "huv-field" }, [el("label", { for: id, text: opts.label }), select]),
      get: function () { return select.value; },
      set: function (v) { select.value = v; }
    };
  }

  function checkField(opts) {
    var id = "huv-field-" + ++uid;
    var input = el("input", { type: "checkbox", id: id, checked: opts.value ? true : null });
    input.addEventListener("change", opts.onChange);
    return {
      node: el("div", { className: "huv-check" }, [input, el("label", { for: id, text: opts.label })]),
      get: function () { return input.checked; }
    };
  }

  /* Stat tiles: label, value, optional note. Returns setters keyed like `specs`. */
  function tiles(specs) {
    var list = el("ul", { className: "huv-tiles" });
    var setters = {};
    specs.forEach(function (spec) {
      var value = el("span", { className: "huv-tile__value", text: "—" });
      var note = el("span", { className: "huv-tile__note" });
      list.appendChild(el("li", null, [el("span", { className: "huv-tile__label", text: spec[1] }), value, note]));
      setters[spec[0]] = function (text, noteText) {
        value.textContent = text;
        note.textContent = noteText || "";
      };
    });
    return { node: list, set: setters };
  }

  function message() {
    var node = el("p", { className: "huv-calc__message", role: "status", hidden: true });
    return {
      node: node,
      show: function (text) {
        if (text) {
          node.textContent = text;
          node.hidden = false;
        } else {
          node.textContent = "";
          node.hidden = true;
        }
      }
    };
  }

  function table(caption, headers) {
    var captionNode = el("caption", { className: "huv-sr-only", text: caption });
    var head = el("thead", null, [el("tr", null, headers.map(function (h) { return el("th", { scope: "col", text: h }); }))]);
    var body = el("tbody");
    var node = el("table", { className: "huv-calc__table" }, [captionNode, head, body]);
    return {
      node: el("div", { className: "huv-calc__tablewrap" }, [node]),
      caption: function (text) { captionNode.textContent = text; },
      rows: function (rows) {
        body.textContent = "";
        rows.forEach(function (row) {
          var tr = el("tr", { "aria-current": row.current ? "true" : null });
          row.cells.forEach(function (cell, i) {
            tr.appendChild(el(i === 0 ? "th" : "td", i === 0 ? { scope: "row", text: cell } : { text: cell }));
          });
          body.appendChild(tr);
        });
      }
    };
  }

  /* The inputs live in a <form> for grouping only: it must never submit. */
  function controls(label, children) {
    var form = el("form", { className: "huv-calc__controls", "aria-label": label }, children);
    form.addEventListener("submit", function (event) { event.preventDefault(); });
    return form;
  }

  /* A polite live region that speaks a one-line summary once the inputs settle, instead
     of announcing every tile on every slider step. */
  function announcer() {
    var node = el("p", { className: "huv-sr-only", "aria-live": "polite" });
    var timer = null;
    return {
      node: node,
      say: function (text) {
        window.clearTimeout(timer);
        timer = window.setTimeout(function () { node.textContent = text; }, 700);
      }
    };
  }

  function scheduler(fn) {
    var pending = false;
    return function () {
      if (pending) return;
      pending = true;
      window.requestAnimationFrame(function () {
        pending = false;
        fn();
      });
    };
  }

  function presetById(id) {
    for (var i = 0; i < P.PRESETS.length; i++) if (P.PRESETS[i].id === id) return P.PRESETS[i];
    return null;
  }

  function presetOptions(list) {
    return list.map(function (p) { return [p.id, p.label + (p.illustrative ? " *" : "")]; })
      .concat([["custom", "Custom values"]]);
  }

  /* ---------------------------------------------------------------- (a) Rayleigh */

  function initRayleigh(root) {
    var update;
    var schedule = scheduler(function () { update(); });
    var preset = selectField({
      label: "Wavelength preset (* = illustrative NA)",
      options: presetOptions(P.PRESETS),
      value: "arf",
      onChange: function (id) {
        var p = presetById(id);
        if (p) {
          lambda.set(p.wavelength);
          na.set(p.na);
          index.set(p.n);
        }
        schedule();
      }
    });
    function edited() {
      var p = presetById(preset.get());
      if (p && (p.wavelength !== lambda.get() || p.na !== na.get() || p.n !== index.get())) preset.set("custom");
      schedule();
    }
    var lambda = rangeField({ label: "Wavelength λ", unit: "nm", min: 1, max: 500, step: 0.1, value: 193, onInput: edited });
    var na = rangeField({ label: "Numerical aperture NA", min: 0.05, max: 1.7, step: 0.01, value: 0.93, onInput: edited });
    var index = rangeField({
      label: "Medium index n", min: 1, max: 1.9, step: 0.01, value: 1, onInput: edited,
      hint: "1 for air or vacuum; 1.437 for water at 193 nm"
    });
    var k1 = rangeField({
      label: "Process factor k₁", min: 0.2, max: 1, step: 0.01, value: 0.3, onInput: schedule,
      hint: "0.25 is the single-exposure floor for dense lines"
    });
    var k2 = rangeField({
      label: "DOF factor k₂", min: 0.2, max: 1.5, step: 0.05, value: 1, onInput: schedule,
      hint: "1 reproduces Rayleigh's quarter-wave criterion"
    });
    var out = tiles([
      ["r", "Half-pitch R = k₁λ/NA"],
      ["p", "Minimum pitch 2R"],
      ["dofp", "Depth of focus, paraxial"],
      ["dofe", "Depth of focus, exact"]
    ]);
    var note = message();
    var tbl = table("Rayleigh resolution and depth of focus for every preset", [
      "Preset", "λ (nm)", "NA", "R (nm)", "Pitch (nm)", "DOF paraxial (nm)", "DOF exact (nm)"
    ]);
    var tableNote = el("p", { className: "huv-field__hint" });

    var speak = announcer();
    root.appendChild(controls("Resolution inputs", [preset.node, lambda.node, na.node, index.node, k1.node, k2.node]));
    root.appendChild(out.node);
    root.appendChild(speak.node);
    root.appendChild(note.node);
    root.appendChild(tbl.node);
    root.appendChild(tableNote);

    update = function () {
      var r = P.rayleigh({ wavelength: lambda.get(), na: na.get(), n: index.get(), k1: k1.get(), k2: k2.get() });
      out.set.r(nm(r.halfPitch), "λ/NA = " + nm(lambda.get() / na.get()));
      out.set.p(nm(r.pitch));
      out.set.dofp(nm(r.dofParaxial), "k₂λ/NA²");
      out.set.dofe(isFinite(r.dofExact) ? nm(r.dofExact) : "—", "quarter-wave, non-paraxial");
      speak.say("Half-pitch " + nm(r.halfPitch) + ", pitch " + nm(r.pitch) + ", depth of focus " +
        nm(r.dofParaxial) + " paraxial" + (isFinite(r.dofExact) ? ", " + nm(r.dofExact) + " exact." : "."));

      if (na.get() >= index.get()) {
        note.show("NA cannot reach the medium index n, because NA = n·sinθ. NA above 1 needs an immersion liquid: pick the ArF immersion preset or raise n.");
      } else if (k1.get() < 0.25) {
        note.show("k₁ below 0.25 is out of reach for dense lines in one exposure: the first diffraction orders no longer fit through the pupil together with the zeroth. Multiple patterning splits such pitches over several exposures.");
      } else {
        note.show(null);
      }

      tbl.rows(P.PRESETS.map(function (p) {
        var q = P.rayleigh({ wavelength: p.wavelength, na: p.na, n: p.n, k1: k1.get(), k2: k2.get() });
        return {
          current: p.id === preset.get(),
          cells: [p.label + (p.illustrative ? " *" : ""), sig(p.wavelength, 3), fixed(p.na, 2),
            sig(q.halfPitch, 3), sig(q.pitch, 3), sig(q.dofParaxial, 3), sig(q.dofExact, 3)]
        };
      }));
      var caption = "Every preset at k₁ = " + fixed(k1.get(), 2) + " and k₂ = " + fixed(k2.get(), 2) +
        ", each with its own NA and medium index. * Illustrative NA, not a specific tool.";
      tbl.caption(caption);
      tableNote.textContent = caption;
    };
    update();
  }

  /* ---------------------------------------------------------------- (b) shot noise */

  var WAVELENGTHS = [
    { id: "g", label: "436 nm, g-line", wavelength: 436 },
    { id: "i", label: "365 nm, i-line", wavelength: 365 },
    { id: "krf", label: "248 nm, KrF", wavelength: 248 },
    { id: "arf", label: "193 nm, ArF", wavelength: 193 },
    { id: "f2", label: "157 nm, F₂", wavelength: 157 },
    { id: "euv", label: "13.5 nm, EUV", wavelength: 13.5 },
    { id: "beuv", label: "6.7 nm, beyond-EUV", wavelength: 6.7 }
  ];

  function wavelengthById(id) {
    for (var i = 0; i < WAVELENGTHS.length; i++) if (WAVELENGTHS[i].id === id) return WAVELENGTHS[i];
    return null;
  }

  function initShotNoise(root) {
    var update;
    var schedule = scheduler(function () { update(); });
    var preset = selectField({
      label: "Wavelength preset",
      options: WAVELENGTHS.map(function (p) { return [p.id, p.label]; }).concat([["custom", "Custom wavelength"]]),
      value: "euv",
      onChange: function (id) {
        var p = wavelengthById(id);
        if (p) lambda.set(p.wavelength);
        schedule();
      }
    });
    var lambda = rangeField({
      label: "Wavelength λ", unit: "nm", min: 1, max: 500, step: 0.1, value: 13.5,
      onInput: function () {
        var p = wavelengthById(preset.get());
        if (p && p.wavelength !== lambda.get()) preset.set("custom");
        schedule();
      }
    });
    var dose = rangeField({ label: "Dose", unit: "mJ/cm²", min: 1, max: 200, step: 1, value: 30, onInput: schedule });
    var shape = selectField({
      label: "Feature shape",
      options: [["square", "Square pixel (size = side)"], ["circle", "Round contact (size = diameter)"]],
      value: "square",
      onChange: schedule
    });
    var size = rangeField({ label: "Feature size", unit: "nm", min: 2, max: 200, step: 1, value: 20, onInput: schedule });
    var absorbed = rangeField({
      label: "Fraction absorbed in the resist", min: 0.01, max: 1, step: 0.01, value: 1, onInput: schedule,
      hint: "1 counts every incident photon"
    });
    var target = rangeField({ label: "Target 1σ fluctuation", unit: "%", min: 0.2, max: 10, step: 0.1, value: 2, onInput: schedule });
    var out = tiles([
      ["energy", "Photon energy"],
      ["density", "Photons per nm²"],
      ["count", "Photons in the feature"],
      ["sigma", "1σ fluctuation 1/√N"],
      ["dose", "Dose for the target"]
    ]);
    var tbl = table("Photon statistics at every preset wavelength", [
      "Wavelength", "E (eV)", "Photons/nm²", "Photons in feature", "1σ", "Dose for target (mJ/cm²)"
    ]);
    var tableNote = el("p", { className: "huv-field__hint" });

    var speak = announcer();
    root.appendChild(controls("Shot-noise inputs", [preset.node, lambda.node, dose.node, shape.node, size.node, absorbed.node, target.node]));
    root.appendChild(out.node);
    root.appendChild(speak.node);
    root.appendChild(tbl.node);
    root.appendChild(tableNote);

    update = function () {
      var area = P.featureArea(shape.get(), size.get());
      var common = { dose: dose.get(), area: area, absorbed: absorbed.get() };
      var r = P.shotNoise(Object.assign({ wavelength: lambda.get() }, common));
      var need = P.doseForNoise({ wavelength: lambda.get(), area: area, absorbed: absorbed.get(), relSigma: target.get() / 100 });
      out.set.energy(sig(r.energyEV, 4) + " eV", sci(r.energyJ, 4) + " J");
      out.set.density(sig(r.perNm2, 3), "at " + sig(dose.get(), 3) + " mJ/cm²");
      out.set.count(sig(r.count, 3), "area " + sig(area, 3) + " nm²");
      out.set.sigma(percent(r.relSigma), "3σ = " + percent(3 * r.relSigma));
      out.set.dose(sig(need, 3) + " mJ/cm²", "for 1σ = " + sig(target.get(), 2) + " %");
      speak.say(sig(r.count, 3) + " photons in the feature, a 1 sigma fluctuation of " + percent(r.relSigma) +
        "; a dose of " + sig(need, 3) + " millijoules per square centimetre gives " + sig(target.get(), 2) + " %.");
      tbl.rows(WAVELENGTHS.map(function (p) {
        var q = P.shotNoise(Object.assign({ wavelength: p.wavelength }, common));
        var d = P.doseForNoise({ wavelength: p.wavelength, area: area, absorbed: absorbed.get(), relSigma: target.get() / 100 });
        return {
          current: p.id === preset.get(),
          cells: [p.label, sig(q.energyEV, 3), sig(q.perNm2, 3), sig(q.count, 3), percent(q.relSigma), sig(d, 3)]
        };
      }));
      var caption = "Every preset wavelength at " + sig(dose.get(), 3) + " mJ/cm² on " + sig(area, 3) +
        " nm² with f = " + fixed(absorbed.get(), 2) + "; last column: dose for a " + sig(target.get(), 2) + " % 1σ fluctuation.";
      tbl.caption(caption);
      tableNote.textContent = caption;
    };
    update();
  }

  /* ---------------------------------------------------------------- (c) aerial image */

  function css(node, name, fallback) {
    var v = window.getComputedStyle(node).getPropertyValue(name).trim();
    return v || fallback;
  }

  function niceStep(span, target) {
    var raw = span / Math.max(1, target);
    var mag = Math.pow(10, Math.floor(Math.log10(raw)));
    var norm = raw / mag;
    return (norm < 1.5 ? 1 : norm < 3.5 ? 2 : norm < 7.5 ? 5 : 10) * mag;
  }

  function ticks(lo, hi, target) {
    var step = niceStep(hi - lo, target);
    var out = [];
    for (var v = Math.ceil(lo / step) * step; v <= hi + step * 1e-9; v += step) out.push(Math.abs(v) < step * 1e-9 ? 0 : v);
    return { values: out, step: step };
  }

  function presetGeometry(p) {
    // A pitch that images comfortably: half-pitch at k1 = 0.45, rounded.
    var pitch = Math.max(10, Math.round((2 * 0.45 * p.wavelength) / p.na));
    return { pitch: pitch, cd: Math.round(pitch / 2) };
  }

  function initAerial(root) {
    var update;
    var draw;
    var schedule = scheduler(function () { update(); });
    var start = presetById("arf");
    var preset = selectField({
      label: "Wavelength preset (* = illustrative NA)",
      options: presetOptions(P.PRESETS),
      value: "arf",
      onChange: function (id) {
        var p = presetById(id);
        if (p) {
          lambda.set(p.wavelength);
          index.set(p.n);
          na.setLimits(0.05, Math.round((p.n - 0.01) * 100) / 100, 0.01);
          na.set(p.na);
          var g = presetGeometry(p);
          pitch.set(g.pitch);
          cd.setLimits(1, g.pitch - 1, 0.5);
          cd.set(g.cd);
          focus.set(0);
        }
        schedule();
      }
    });
    function edited() {
      var p = presetById(preset.get());
      if (p && (p.wavelength !== lambda.get() || p.na !== na.get() || p.n !== index.get())) preset.set("custom");
      schedule();
    }
    var lambda = rangeField({ label: "Wavelength λ", unit: "nm", min: 1, max: 500, step: 0.1, value: start.wavelength, onInput: edited });
    var index = rangeField({
      label: "Medium index n", min: 1, max: 1.9, step: 0.01, value: start.n,
      onInput: function () {
        na.setLimits(0.05, Math.round((index.get() - 0.01) * 100) / 100, 0.01);
        edited();
      }
    });
    var na = rangeField({ label: "Numerical aperture NA", min: 0.05, max: 0.99, step: 0.01, value: start.na, onInput: edited });
    var sigmaOut = rangeField({
      label: "Source σ outer", min: 0, max: 1, step: 0.01, value: 0.7, onInput: schedule,
      hint: "0 = coherent; extent of the 1-D source in pupil units"
    });
    var sigmaIn = rangeField({
      label: "Source σ inner", min: 0, max: 1, step: 0.01, value: 0, onInput: schedule,
      hint: "0 = conventional; above 0 = two poles (dipole)"
    });
    var g0 = presetGeometry(start);
    var pitch = rangeField({
      label: "Pitch p", unit: "nm", min: 10, max: 2000, step: 1, value: g0.pitch,
      onInput: function () {
        cd.setLimits(1, Math.max(2, pitch.get() - 1), 0.5);
        schedule();
      }
    });
    var cd = rangeField({ label: "Line width (opaque)", unit: "nm", min: 1, max: g0.pitch - 1, step: 0.5, value: g0.cd, onInput: schedule });
    var focus = rangeField({ label: "Defocus z", unit: "nm", min: -1000, max: 1000, step: 1, value: 0, onInput: schedule });
    var threshold = rangeField({
      label: "Resist threshold", min: 0.02, max: 1.2, step: 0.01, value: 0.3, onInput: schedule,
      hint: "intensity (clear field = 1) below which the line prints"
    });
    var coherent = checkField({ label: "Compare with coherent illumination (σ = 0)", value: true, onChange: schedule });

    var out = tiles([
      ["k1", "k₁ of the half-pitch"],
      ["contrast", "Image contrast"],
      ["nils", "NILS at the line edge"],
      ["cd", "Printed line width"],
      ["orders", "Orders through the pupil"]
    ]);
    var note = message();

    var legendMain = el("span", { className: "huv-legend__key", style: "--huv-key-color: var(--huv-series-1)" });
    var legendRef = el("span", { className: "huv-legend__key huv-legend__key--dashed", style: "--huv-key-color: var(--huv-series-2)" });
    var legendRefItem = el("li", null, [legendRef, el("span", { text: "Coherent (σ = 0)" })]);
    var legendMainText = el("span", { text: "Partially coherent" });
    var legend = el("ul", { className: "huv-legend", "aria-hidden": "true" }, [
      el("li", null, [legendMain, legendMainText]),
      legendRefItem,
      el("li", null, [el("span", { className: "huv-legend__key huv-legend__key--dashed", style: "--huv-key-color: var(--md-default-fg-color--light)" }), el("span", { text: "Resist threshold" })]),
      el("li", null, [el("span", { className: "huv-legend__key huv-legend__key--mask" }), el("span", { text: "Opaque mask lines" })])
    ]);
    var canvas = el("canvas", { role: "img", tabindex: "0", "aria-label": "Aerial image intensity plot" });
    var tooltip = el("div", { className: "huv-tooltip", hidden: true });
    var live = el("p", { className: "huv-sr-only", "aria-live": "polite" });
    var plot = el("div", { className: "huv-plot" }, [canvas, tooltip]);
    var dataTable = table("Aerial image samples over one period", ["x (nm)", "Partially coherent", "Coherent"]);
    var details = el("details", null, [el("summary", { text: "Show the image as a table" }), dataTable.node]);
    var keysHint = el("p", { className: "huv-field__hint", text: "Hover or focus the plot and use ← → to read values." });

    var speak = announcer();
    root.appendChild(controls("Aerial image inputs", [
      preset.node, lambda.node, index.node, na.node, sigmaOut.node, sigmaIn.node, pitch.node, cd.node,
      focus.node, threshold.node, coherent.node
    ]));
    root.appendChild(out.node);
    root.appendChild(speak.node);
    root.appendChild(note.node);
    root.appendChild(legend);
    root.appendChild(plot);
    root.appendChild(keysHint);
    root.appendChild(live);
    root.appendChild(details);

    var state = null; // last computed curves, for drawing and hover
    var hoverIndex = null;
    var geometry = null;

    update = function () {
      if (sigmaIn.get() > sigmaOut.get()) sigmaIn.set(sigmaOut.get());
      var dof = lambda.get() / (na.get() * na.get());
      var span = Math.max(50, Math.ceil((4 * dof) / 50) * 50);
      focus.setLimits(-span, span, 1);

      var params = {
        wavelength: lambda.get(), na: na.get(), n: index.get(), pitch: pitch.get(), cd: cd.get(),
        sigmaIn: sigmaIn.get(), sigmaOut: sigmaOut.get(), defocus: focus.get(), threshold: threshold.get()
      };
      var model = P.aerialModel(params);
      var metrics = P.imageMetrics(model, params);
      var samples = Math.min(801, Math.max(241, Math.round(canvas.clientWidth || 480)));
      var main = model.sample(samples, 2);
      var ref = null;
      var refModel = null;
      var refMetrics = null;
      if (coherent.get()) {
        var refParams = Object.assign({}, params, { sigmaIn: 0, sigmaOut: 0 });
        refModel = P.aerialModel(refParams);
        ref = refModel.sample(samples, 2);
        refMetrics = P.imageMetrics(refModel, refParams);
      }
      state = { params: params, model: model, main: main, ref: ref, metrics: metrics };

      var k1 = (pitch.get() / 2) * na.get() / lambda.get();
      out.set.k1(fixed(k1, 3), "(p/2)·NA/λ");
      out.set.contrast(fixed(metrics.contrast, 3), refMetrics ? "coherent: " + fixed(refMetrics.contrast, 3) : "");
      out.set.nils(isFinite(metrics.nils) ? fixed(metrics.nils, 2) : "—", "w·|d ln I/dx| at x = w/2");
      var printed = metrics.printedCd;
      out.set.cd(
        printed === 0 ? "does not print" : printed >= pitch.get() - 1e-9 ? "space closes" : nm(printed, 1),
        "mask line " + nm(cd.get(), 1)
      );
      out.set.orders(
        model.ordersOnAxis.length === 1 ? "0th only"
          : model.ordersOnAxis.map(function (m) { return m > 0 ? "+" + m : String(m).replace("-", "−"); }).join(", "),
        "for the on-axis source point; " + model.sourceCount + " source point" + (model.sourceCount === 1 ? "" : "s") + " in total");
      legendRefItem.hidden = !ref;
      legendMainText.textContent = sigmaOut.get() === 0 ? "Your settings (σ = 0, coherent)"
        : "Partially coherent (σ " + (sigmaIn.get() > 0 ? fixed(sigmaIn.get(), 2) + "–" : "≤ ") + fixed(sigmaOut.get(), 2) + ")";

      var cutoff = lambda.get() / ((1 + sigmaOut.get()) * na.get());
      if (!model.passesFirstOrder) {
        note.show("The pitch is below λ/((1 + σ)NA) = " + nm(cutoff) + ": no first diffraction order passes the pupil for any source point, so the image is flat and the pattern cannot be resolved.");
      } else if (na.get() >= index.get()) {
        note.show("NA must stay below the medium index n.");
      } else {
        note.show(null);
      }

      // Table view: one period, 21 samples.
      var rows = [];
      for (var i = 0; i <= 20; i++) {
        var x = -pitch.get() / 2 + (pitch.get() * i) / 20;
        rows.push({ cells: [fixed(x, 1), fixed(model.at(x), 4), refModel ? fixed(refModel.at(x), 4) : "—"] });
      }
      dataTable.rows(rows);

      speak.say("Contrast " + fixed(metrics.contrast, 2) + ", NILS " +
        (isFinite(metrics.nils) ? fixed(metrics.nils, 1) : "undefined") + ", printed line " +
        (printed === 0 ? "does not print" : printed >= pitch.get() - 1e-9 ? "fills the pitch" : nm(printed, 1)) + ".");
      canvas.setAttribute("aria-label",
        "Aerial image over two periods of " + nm(pitch.get(), 0) + ": intensity from " + fixed(metrics.imin, 2) +
        " to " + fixed(metrics.imax, 2) + " of the clear-field level, contrast " + fixed(metrics.contrast, 2) +
        (ref ? "; the coherent image reaches contrast " + fixed(refMetrics.contrast, 2) : "") + ".");
      draw();
    };

    draw = function () {
      if (!state) return;
      var ratio = window.devicePixelRatio || 1;
      var width = canvas.clientWidth || 480;
      var height = canvas.clientHeight || 272;
      if (canvas.width !== Math.round(width * ratio) || canvas.height !== Math.round(height * ratio)) {
        canvas.width = Math.round(width * ratio);
        canvas.height = Math.round(height * ratio);
      }
      var ctx = canvas.getContext("2d");
      ctx.setTransform(ratio, 0, 0, ratio, 0, 0);
      ctx.clearRect(0, 0, width, height);

      var muted = css(root, "--md-default-fg-color--light", "#777");
      var gridColor = css(root, "--huv-grid", "rgba(0,0,0,.1)");
      var axisColor = css(root, "--huv-axis", "rgba(0,0,0,.35)");
      var surface = css(root, "--md-default-bg-color", "#fff");
      var c1 = css(root, "--huv-series-1", "#2a78d6");
      var c2 = css(root, "--huv-series-2", "#eb6834");
      var font = "11px " + (css(document.body, "--md-text-font-family", "") ? css(document.body, "--md-text-font-family", "") + ", " : "") + "system-ui, sans-serif";

      var pad = { left: 44, right: 14, top: 12, bottom: 50 };
      var plotW = Math.max(10, width - pad.left - pad.right);
      var plotH = Math.max(10, height - pad.top - pad.bottom);
      var p = state.params.pitch;
      var xmin = -p;
      var xmax = p;
      var ymaxData = Math.max(state.metrics.imax, state.ref ? Math.max.apply(null, state.ref.intensity) : 0, state.params.threshold);
      var ymax = Math.max(1, ymaxData) * 1.08;
      var sx = function (x) { return pad.left + ((x - xmin) / (xmax - xmin)) * plotW; };
      var sy = function (y) { return pad.top + plotH - (y / ymax) * plotH; };
      geometry = { sx: sx, pad: pad, plotW: plotW, xmin: xmin, xmax: xmax, width: width };

      ctx.font = font;
      ctx.lineWidth = 1;

      // Horizontal gridlines and y tick labels.
      var yt = ticks(0, ymax, 5);
      ctx.fillStyle = muted;
      ctx.textAlign = "right";
      ctx.textBaseline = "middle";
      yt.values.forEach(function (v) {
        var y = Math.round(sy(v)) + 0.5;
        ctx.strokeStyle = v === 0 ? axisColor : gridColor;
        ctx.beginPath();
        ctx.moveTo(pad.left, y);
        ctx.lineTo(pad.left + plotW, y);
        ctx.stroke();
        ctx.fillText(yt.step < 0.1 ? v.toFixed(2) : v.toFixed(1), pad.left - 6, y);
      });

      // Mask strip under the plot: opaque lines dark, clear spaces light.
      var stripTop = pad.top + plotH + 6;
      var stripH = 7;
      ctx.fillStyle = gridColor;
      ctx.fillRect(pad.left, stripTop, plotW, stripH);
      ctx.fillStyle = muted;
      var w = state.params.cd;
      for (var k = Math.floor(xmin / p) - 1; k <= Math.ceil(xmax / p) + 1; k++) {
        var a = Math.max(xmin, k * p - w / 2);
        var b = Math.min(xmax, k * p + w / 2);
        if (b > a) ctx.fillRect(sx(a), stripTop, sx(b) - sx(a), stripH);
      }

      // X tick labels.
      var xt = ticks(xmin, xmax, Math.max(3, Math.floor(plotW / 90)));
      ctx.textAlign = "center";
      ctx.textBaseline = "top";
      ctx.fillStyle = muted;
      xt.values.forEach(function (v) {
        ctx.fillText(Math.round(v).toString().replace("-", "−"), sx(v), stripTop + stripH + 5);
      });
      ctx.textAlign = "right";
      ctx.fillText("x (nm)", pad.left + plotW, stripTop + stripH + 19);
      ctx.save();
      ctx.translate(11, pad.top + plotH / 2);
      ctx.rotate(-Math.PI / 2);
      ctx.textAlign = "center";
      ctx.textBaseline = "middle";
      ctx.fillText("Intensity (clear field = 1)", 0, 0);
      ctx.restore();

      // Threshold.
      ctx.save();
      ctx.strokeStyle = muted;
      ctx.setLineDash([4, 4]);
      ctx.beginPath();
      var ty = Math.round(sy(state.params.threshold)) + 0.5;
      ctx.moveTo(pad.left, ty);
      ctx.lineTo(pad.left + plotW, ty);
      ctx.stroke();
      ctx.restore();

      function curve(series, color, dashed) {
        ctx.save();
        ctx.strokeStyle = color;
        ctx.lineWidth = 2;
        ctx.lineJoin = "round";
        ctx.lineCap = "round";
        if (dashed) ctx.setLineDash([7, 5]);
        ctx.beginPath();
        for (var i = 0; i < series.x.length; i++) {
          var X = sx(series.x[i]);
          var Y = sy(series.intensity[i]);
          if (i === 0) ctx.moveTo(X, Y);
          else ctx.lineTo(X, Y);
        }
        ctx.stroke();
        ctx.restore();
      }
      if (state.ref) curve(state.ref, c2, true);
      curve(state.main, c1, false);

      // Crosshair and markers for the hovered sample.
      if (hoverIndex !== null && hoverIndex < state.main.x.length) {
        var hx = Math.round(sx(state.main.x[hoverIndex])) + 0.5;
        ctx.strokeStyle = axisColor;
        ctx.lineWidth = 1;
        ctx.beginPath();
        ctx.moveTo(hx, pad.top);
        ctx.lineTo(hx, pad.top + plotH);
        ctx.stroke();
        [[state.ref, c2], [state.main, c1]].forEach(function (pair) {
          if (!pair[0]) return;
          var y = sy(pair[0].intensity[hoverIndex]);
          ctx.beginPath();
          ctx.arc(hx, y, 4, 0, 2 * Math.PI);
          ctx.fillStyle = pair[1];
          ctx.strokeStyle = surface;
          ctx.lineWidth = 2;
          ctx.fill();
          ctx.stroke();
        });
      }
    };

    function showTooltip(index, announce) {
      if (!state || !geometry) return;
      hoverIndex = Math.max(0, Math.min(state.main.x.length - 1, index));
      draw();
      var x = state.main.x[hoverIndex];
      tooltip.textContent = "";
      tooltip.appendChild(el("div", { className: "huv-tooltip__x", text: "x = " + fixed(x, 1) + " nm" }));
      function row(value, color, dashed, name) {
        tooltip.appendChild(el("div", { className: "huv-tooltip__row" }, [
          el("span", { className: "huv-legend__key" + (dashed ? " huv-legend__key--dashed" : ""), style: "--huv-key-color: " + color }),
          el("strong", { text: fixed(value, 3) }),
          el("span", { text: name })
        ]));
      }
      row(state.main.intensity[hoverIndex], "var(--huv-series-1)", false, "partially coherent");
      if (state.ref) row(state.ref.intensity[hoverIndex], "var(--huv-series-2)", true, "coherent");
      tooltip.hidden = false;
      var left = geometry.sx(x) + 12;
      var tipWidth = tooltip.offsetWidth || 150;
      if (left + tipWidth > geometry.width) left = geometry.sx(x) - tipWidth - 12;
      tooltip.style.left = Math.max(0, left) + "px";
      tooltip.style.top = geometry.pad.top + "px";
      if (announce) {
        live.textContent = "x " + fixed(x, 1) + " nm: intensity " + fixed(state.main.intensity[hoverIndex], 3) +
          (state.ref ? ", coherent " + fixed(state.ref.intensity[hoverIndex], 3) : "");
      }
    }

    function hideTooltip() {
      hoverIndex = null;
      tooltip.hidden = true;
      draw();
    }

    function indexFromEvent(event) {
      if (!state || !geometry) return null;
      var rect = canvas.getBoundingClientRect();
      var px = event.clientX - rect.left;
      var x = geometry.xmin + ((px - geometry.pad.left) / geometry.plotW) * (geometry.xmax - geometry.xmin);
      var n = state.main.x.length;
      return Math.round(((x - geometry.xmin) / (geometry.xmax - geometry.xmin)) * (n - 1));
    }

    canvas.addEventListener("pointermove", function (event) {
      var i = indexFromEvent(event);
      if (i === null || i < 0 || i >= state.main.x.length) hideTooltip();
      else showTooltip(i, false);
    });
    canvas.addEventListener("pointerleave", hideTooltip);
    canvas.addEventListener("focus", function () { showTooltip(hoverIndex === null ? Math.floor(state.main.x.length / 2) : hoverIndex, true); });
    canvas.addEventListener("blur", hideTooltip);
    canvas.addEventListener("keydown", function (event) {
      if (!state) return;
      var n = state.main.x.length;
      var stepSize = event.shiftKey ? Math.max(1, Math.round(n / 20)) : Math.max(1, Math.round(n / 100));
      var current = hoverIndex === null ? Math.floor(n / 2) : hoverIndex;
      if (event.key === "ArrowLeft") current -= stepSize;
      else if (event.key === "ArrowRight") current += stepSize;
      else if (event.key === "Home") current = 0;
      else if (event.key === "End") current = n - 1;
      else if (event.key === "Escape") { hideTooltip(); return; }
      else return;
      event.preventDefault();
      showTooltip(current, true);
    });

    var resize = typeof ResizeObserver === "function" ? new ResizeObserver(scheduler(function () { update(); })) : null;
    if (resize) resize.observe(plot);
    var palette = new MutationObserver(scheduler(draw));
    palette.observe(document.body, { attributes: true, attributeFilter: ["data-md-color-scheme", "data-md-color-primary", "data-md-color-accent"] });

    update();
    return function () {
      if (resize) resize.disconnect();
      palette.disconnect();
    };
  }

  /* ---------------------------------------------------------------- mounting */

  var WIDGETS = { rayleigh: initRayleigh, shotnoise: initShotNoise, aerial: initAerial };
  var cleanups = [];

  function mountAll() {
    cleanups.forEach(function (fn) { fn(); });
    cleanups = [];
    document.querySelectorAll("[data-huv-calc]").forEach(function (root) {
      var init = WIDGETS[root.getAttribute("data-huv-calc")];
      if (!init || root.getAttribute("data-huv-mounted")) return;
      root.setAttribute("data-huv-mounted", "true");
      try {
        var cleanup = init(root);
        if (typeof cleanup === "function") cleanups.push(cleanup);
      } catch (error) {
        root.appendChild(el("p", { className: "huv-calc__message", text: "This calculator failed to start: " + error.message }));
        if (window.console) window.console.error(error);
      }
    });
  }

  if (typeof document$ !== "undefined") {
    document$.subscribe(mountAll);
  } else if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", mountAll);
  } else {
    mountAll();
  }
})();
