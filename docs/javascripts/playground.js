/* playground.js (owner G0b; DESIGN §3.8, §5.6 R4, §7.10; contract + markup: docs-refactor/impl/G0b.md).
 * <div class="huv-calc" data-huv-calc="rayleigh|shotnoise|aerial"> placeholders become widgets
 * (data-huv-mounted="true"); physics = huv-physics.js. Site: window.HUVPlayground; node: api.install(win).
 * Exports: install(win), mountAll(), WIDGETS, parseLink(search) -> {calc, params}, formatLink(calc, params),
 *   fallbackFor(root) (the <figure> before a placeholder: hidden on mount as data-huv-fallback="hidden",
 *   shown for print), ticks, niceStep, logTicks, SERIES (the --huv-series-1…8 fallbacks).
 * Markup: fieldset.huv-calc__fieldset > legend.huv-calc__title · form.huv-calc__controls · ul.huv-tiles ·
 *   p.huv-calc__message · ul.huv-legend · div.huv-plot > canvas + div.huv-tooltip · div.huv-calc__row >
 *   div.huv-pupil > svg.huv-pupil__svg (the D5 live pupil) · div.huv-calc__tablewrap > table.huv-calc__table.
 * Deep links ?calc=<widget>&preset=…&<param>=…: applied on mount, rewritten (non-default values) after input.
 *   rayleigh: preset lambda na n k1 k2 · shotnoise: preset lambda dose shape size fabs target · aerial: preset
 *   lambda n na sigma sigmain pitch cd z t coh (+ k1 read-only: pitch = 2·k1·λ/NA, duty cycle kept).
 */
(function (root, factory) {
  "use strict";
  var api = factory();
  if (typeof module === "object" && module.exports) module.exports = api;
  else { root.HUVPlayground = api; api.install(root); }
})(typeof window !== "undefined" ? window : this, function () {
  "use strict";

  var P = null; // window.HUVPhysics (bound by install)
  var W = null; // window
  var D = null; // document

  /* ---- formatting */

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

  function minus(text) { return String(text).replace("-", "−"); }

  /* ---- DOM helpers */

  var uid = 0;

  function el(tag, props, children) {
    var node = D.createElement(tag);
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
      node.appendChild(typeof child === "string" ? D.createTextNode(child) : child);
    });
    return node;
  }

  function list(nodes) { return Array.prototype.slice.call(nodes || []); }

  function decimals(step) {
    var s = String(step);
    return s.indexOf(".") < 0 ? 0 : s.length - s.indexOf(".") - 1;
  }

  /* A slider paired with a number box (or a read-only <output> when opts.output); an out-of-range
     number is marked invalid until fixed or left (then clamped). */
  function rangeField(opts) {
    var id = "huv-field-" + ++uid;
    var hintId = opts.hint ? id + "-hint" : null;
    var min = opts.min;
    var max = opts.max;
    var step = opts.step;
    var label = el("label", { for: opts.output ? id + "-range" : id, id: id + "-label", text: opts.label + (opts.unit ? " (" + opts.unit + ")" : "") });
    var range = el("input", {
      type: "range", id: id + "-range", min: min, max: max, step: step, value: opts.value,
      "aria-labelledby": id + "-label", "aria-describedby": hintId
    });
    var number = opts.output ? el("output", { for: id + "-range", className: "huv-field__output" }) : el("input", {
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
      var text = String(Number(v.toFixed(Math.max(decimals(step), 0) + 2)));
      if (opts.output) number.textContent = text; else number.value = text;
      number.removeAttribute("aria-invalid");
    }
    if (opts.output) show(value);

    range.addEventListener("input", function () {
      value = Number(range.value);
      if (opts.output) number.textContent = range.value; else number.value = range.value;
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
      set: function (v) { if (opts.options.some(function (o) { return o[0] === v; })) select.value = v; }
    };
  }

  function checkField(opts) {
    var id = "huv-field-" + ++uid;
    var input = el("input", { type: "checkbox", id: id, checked: opts.value ? true : null });
    input.checked = !!opts.value;
    input.addEventListener("change", opts.onChange);
    return {
      node: el("div", { className: "huv-check" }, [input, el("label", { for: id, text: opts.label })]),
      get: function () { return input.checked; },
      set: function (v) { input.checked = !!v; }
    };
  }

  function tiles(specs) {
    var node = el("ul", { className: "huv-tiles" });
    var setters = {};
    specs.forEach(function (spec) {
      var value = el("span", { className: "huv-tile__value", text: "—" });
      var note = el("span", { className: "huv-tile__note" });
      node.appendChild(el("li", null, [el("span", { className: "huv-tile__label", text: spec[1] }), value, note]));
      setters[spec[0]] = function (text, noteText) {
        value.textContent = text;
        note.textContent = noteText || "";
      };
    });
    return { node: node, set: setters };
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

  function controls(label, children) {
    var form = el("form", { className: "huv-calc__controls", "aria-label": label }, children);
    form.addEventListener("submit", function (event) { event.preventDefault(); });
    return form;
  }

  function card(root, title) {
    var fieldset = el("fieldset", { className: "huv-calc__fieldset" }, [
      el("legend", { className: "huv-calc__title", text: root.getAttribute("data-huv-title") || title })
    ]);
    root.appendChild(fieldset);
    return fieldset;
  }

  function hint(text) { return el("p", { className: "huv-field__hint", text: text }); }
  var HINT = "hover or focus the plot and use ← → to read values.";

  /* Polite live region: one summary once the inputs settle, not every tile per step. */
  function announcer() {
    var node = el("p", { className: "huv-sr-only", "aria-live": "polite" });
    var timer = null;
    return {
      node: node,
      say: function (text) {
        W.clearTimeout(timer);
        timer = W.setTimeout(function () { node.textContent = text; }, 700);
      }
    };
  }

  function scheduler(fn) {
    var pending = false;
    return function () {
      if (pending) return;
      pending = true;
      W.requestAnimationFrame(function () {
        pending = false;
        fn();
      });
    };
  }

  /* items: [[color, label, "dashed"|"mask"|"dot"|null]] */
  function legend(items) {
    return el("ul", { className: "huv-legend", "aria-hidden": "true" }, items.map(function (item) {
      return el("li", null, [
        el("span", { className: "huv-legend__key" + (item[2] ? " huv-legend__key--" + item[2] : ""), style: item[0] ? "--huv-key-color: " + item[0] : null }),
        el("span", { text: item[1] })
      ]);
    }));
  }

  function presetById(id) {
    for (var i = 0; i < P.PRESETS.length; i++) if (P.PRESETS[i].id === id) return P.PRESETS[i];
    return null;
  }

  function presetOptions(presets) {
    return presets.map(function (p) { return [p.id, p.label + (p.illustrative ? " *" : "")]; })
      .concat([["custom", "Custom values"]]);
  }

  /* ---- colours */

  /* DESIGN §4.1 tokens.SERIES slots 1–8 (never cycled); --huv-series-N on .huv-calc overrides. */
  var SERIES = {
    light: ["#2a78d6", "#eb6834", "#1baf7a", "#eda100", "#e87ba4", "#008300", "#4a3aa7", "#e34948"],
    dark: ["#3987e5", "#d95926", "#199e70", "#c98500", "#d55181", "#008300", "#9085e9", "#e66767"]
  };

  function css(node, name, fallback) {
    var v = W.getComputedStyle(node).getPropertyValue(name).trim();
    return v || fallback;
  }

  function seriesColor(node, slot) {
    return css(node, "--huv-series-" + slot, SERIES[D.body.getAttribute("data-md-color-scheme") === "slate" ? "dark" : "light"][slot - 1]);
  }

  function palette(node) {
    var family = css(D.body, "--md-text-font-family", "");
    return {
      muted: css(node, "--md-default-fg-color--light", "#777"),
      faint: css(node, "--md-default-fg-color--lighter", "#aaa"),
      grid: css(node, "--huv-grid", "rgba(0,0,0,.1)"),
      axis: css(node, "--huv-axis", "rgba(0,0,0,.35)"),
      surface: css(node, "--md-default-bg-color", "#fff"),
      ink: css(node, "--md-default-fg-color", "#111"),
      font: "11px " + (family ? family + ", " : "") + "system-ui, sans-serif"
    };
  }

  /* ---- axes */

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

  function logTicks(lo, hi) {
    var major = [];
    var minor = [];
    for (var e = Math.floor(Math.log10(lo)); e <= Math.ceil(Math.log10(hi)); e++) {
      var d = Math.pow(10, e);
      if (d >= lo * (1 - 1e-9) && d <= hi * (1 + 1e-9)) major.push(d);
      [2, 5].forEach(function (k) { if (k * d >= lo && k * d <= hi) minor.push(k * d); });
    }
    return { major: major, minor: minor };
  }

  function tickLabel(v, step) {
    if (step !== undefined) return minus(step < 0.1 ? v.toFixed(2) : step < 1 ? v.toFixed(1) : String(Math.round(v)));
    return v >= 1 ? Math.round(v).toLocaleString("en-US") : minus(String(+v.toPrecision(2)));
  }

  function decade(v, up) { return Math.pow(10, (up ? Math.ceil : Math.floor)(Math.log10(v))); }

  /* ---- the line plot */

  /* One x axis, `panels` stacked with one y axis each (never two scales on one panel).
       linePlot({ root, label, height, xLabel, logX, xExtra, panels: [{ yLabel, logY }], extras(ctx, s, pal, data),
                  readout(i) -> { title, rows: [[color, dashed, value, name]] }, announce(i) -> text })
       .set({ x, xmin, xmax, marks, panels: [{ min, max, series: [{ y, color, dashed, width, alpha }], points }] }) */
  function linePlot(opts) {
    var canvas = el("canvas", { role: "img", tabindex: "0", "aria-label": opts.label });
    if (opts.height) canvas.style.height = opts.height;
    var tooltip = el("div", { className: "huv-tooltip", hidden: true });
    var live = el("p", { className: "huv-sr-only", "aria-live": "polite" });
    var node = el("div", { className: "huv-plot" }, [canvas, tooltip, live]);
    var data = null;
    var hover = null;
    var geom = null;
    var xExtra = opts.xExtra || 0;

    function scaleX(x) { return opts.logX ? Math.log10(x) : x; }

    function line(ctx, x0, y0, x1, y1, color, alpha) {
      ctx.strokeStyle = color;
      ctx.globalAlpha = alpha || 1;
      ctx.beginPath();
      ctx.moveTo(x0, y0);
      ctx.lineTo(x1, y1);
      ctx.stroke();
      ctx.globalAlpha = 1;
    }

    function dot(ctx, x, y, color, surface, r) {
      ctx.beginPath();
      ctx.arc(x, y, r, 0, 2 * Math.PI);
      ctx.fillStyle = color;
      ctx.strokeStyle = surface;
      ctx.lineWidth = 2;
      ctx.fill();
      ctx.stroke();
    }

    function draw() {
      if (!data) return;
      var ratio = W.devicePixelRatio || 1;
      var width = canvas.clientWidth || 480;
      var height = canvas.clientHeight || 272;
      if (canvas.width !== Math.round(width * ratio) || canvas.height !== Math.round(height * ratio)) {
        canvas.width = Math.round(width * ratio);
        canvas.height = Math.round(height * ratio);
      }
      var ctx = canvas.getContext("2d");
      ctx.setTransform(ratio, 0, 0, ratio, 0, 0);
      ctx.clearRect(0, 0, width, height);
      var pal = palette(opts.root);
      ctx.font = pal.font;
      ctx.lineWidth = 1;

      var yTicks = data.panels.map(function (panel, i) {
        return opts.panels[i].logY ? logTicks(panel.min, panel.max) : ticks(panel.min, panel.max, 5);
      });
      var widest = 0;
      yTicks.forEach(function (t, i) {
        var log = !!opts.panels[i].logY;
        (log ? t.major : t.values).forEach(function (v) { widest = Math.max(widest, ctx.measureText(tickLabel(v, log ? undefined : t.step)).width); });
      });
      var pad = { left: Math.max(50, Math.ceil(widest) + 24), right: 14, top: 12, bottom: 36 + xExtra };
      var n = data.panels.length;
      var gap = 26;
      var plotW = Math.max(10, width - pad.left - pad.right);
      var panelH = Math.max(10, (height - pad.top - pad.bottom - gap * (n - 1)) / n);
      var x0 = scaleX(data.xmin);
      var x1 = scaleX(data.xmax);
      var sx = function (x) { return pad.left + ((scaleX(x) - x0) / (x1 - x0)) * plotW; };
      var tops = [];
      var sy = data.panels.map(function (panel, i) {
        var top = pad.top + i * (panelH + gap);
        tops.push(top);
        var log = !!opts.panels[i].logY;
        var lo = log ? Math.log10(panel.min) : panel.min;
        var hi = log ? Math.log10(panel.max) : panel.max;
        return function (y) { return top + panelH - (((log ? Math.log10(y) : y) - lo) / (hi - lo)) * panelH; };
      });
      var bottom = tops[n - 1] + panelH;
      geom = { sx: sx, sy: sy, top: tops, bottom: tops.map(function (t) { return t + panelH; }), left: pad.left, right: pad.left + plotW, plotW: plotW, width: width, pad: pad };

      data.panels.forEach(function (panel, i) {
        var log = !!opts.panels[i].logY;
        var t = yTicks[i];
        ctx.textAlign = "right";
        ctx.textBaseline = "middle";
        (log ? t.minor : []).forEach(function (v) { var y = Math.round(sy[i](v)) + 0.5; line(ctx, pad.left, y, pad.left + plotW, y, pal.grid, 0.5); });
        (log ? t.major : t.values).forEach(function (v) {
          var y = Math.round(sy[i](v)) + 0.5;
          line(ctx, pad.left, y, pad.left + plotW, y, !log && v === 0 ? pal.axis : pal.grid);
          ctx.fillStyle = pal.muted;
          ctx.fillText(tickLabel(v, log ? undefined : t.step), pad.left - 6, y);
        });
        var base = Math.round(tops[i] + panelH) + 0.5;
        line(ctx, pad.left, base, pad.left + plotW, base, pal.axis);
        if (opts.panels[i].yLabel) {
          ctx.save();
          ctx.translate(11, tops[i] + panelH / 2);
          ctx.rotate(-Math.PI / 2);
          ctx.textAlign = "center";
          ctx.fillStyle = pal.muted;
          ctx.fillText(opts.panels[i].yLabel, 0, 0);
          ctx.restore();
        }
      });

      var xt = opts.logX ? logTicks(data.xmin, data.xmax) : ticks(data.xmin, data.xmax, Math.max(3, Math.floor(plotW / 90)));
      var xs = opts.logX ? xt.major.concat(xt.major.length < 3 ? xt.minor : []) : xt.values;
      ctx.textAlign = "center";
      ctx.textBaseline = "top";
      xs.forEach(function (v) {
        if (v < data.xmin || v > data.xmax) return;
        var x = sx(v);
        if (opts.logX) line(ctx, Math.round(x) + 0.5, tops[0], Math.round(x) + 0.5, bottom, pal.grid);
        ctx.fillStyle = pal.muted;
        ctx.fillText(tickLabel(v, opts.logX ? undefined : xt.step), x, bottom + 5 + xExtra);
      });
      ctx.textAlign = "right";
      ctx.fillText(opts.xLabel, pad.left + plotW, bottom + 19 + xExtra);

      if (opts.extras) opts.extras(ctx, geom, pal, data);

      (data.marks || []).forEach(function (x) {
        if (x < data.xmin || x > data.xmax) return;
        ctx.save();
        ctx.setLineDash([3, 3]);
        line(ctx, Math.round(sx(x)) + 0.5, tops[0], Math.round(sx(x)) + 0.5, bottom, pal.axis);
        ctx.restore();
      });

      // Series: gaps where a value is not finite or not drawable on a log axis.
      data.panels.forEach(function (panel, i) {
        var log = !!opts.panels[i].logY;
        panel.series.forEach(function (series) {
          ctx.save();
          ctx.strokeStyle = series.color;
          ctx.lineWidth = series.width || 2;
          ctx.globalAlpha = series.alpha === undefined ? 1 : series.alpha;
          ctx.lineJoin = ctx.lineCap = "round";
          if (series.dashed) ctx.setLineDash([7, 5]);
          ctx.beginPath();
          var pen = false;
          for (var k = 0; k < data.x.length; k++) {
            var y = series.y[k];
            if (!isFinite(y) || (log && y <= 0)) { pen = false; continue; }
            ctx[pen ? "lineTo" : "moveTo"](sx(data.x[k]), sy[i](y));
            pen = true;
          }
          ctx.stroke();
          ctx.restore();
        });
        (panel.points || []).forEach(function (pt) {
          if (isFinite(pt.y) && (!log || pt.y > 0)) dot(ctx, sx(pt.x), sy[i](pt.y), pt.color, pal.surface, pt.ring ? 6 : 4);
        });
      });

      if (hover !== null && hover < data.x.length) {
        var hx = Math.round(sx(data.x[hover])) + 0.5;
        line(ctx, hx, tops[0], hx, bottom, pal.axis);
        data.panels.forEach(function (panel, i) {
          panel.series.forEach(function (series) {
            var y = series.y[hover];
            if (isFinite(y) && (!opts.panels[i].logY || y > 0)) dot(ctx, hx, sy[i](y), series.color, pal.surface, 4);
          });
        });
      }
    }

    function showTooltip(index, announce) {
      if (!data || !geom) return;
      hover = Math.max(0, Math.min(data.x.length - 1, index));
      draw();
      var out = opts.readout(hover);
      tooltip.textContent = "";
      tooltip.appendChild(el("div", { className: "huv-tooltip__x", text: out.title }));
      out.rows.forEach(function (row) {
        tooltip.appendChild(el("div", { className: "huv-tooltip__row" }, [
          el("span", { className: "huv-legend__key" + (row[1] ? " huv-legend__key--dashed" : ""), style: "--huv-key-color: " + row[0] }),
          el("strong", { text: row[2] }),
          el("span", { text: row[3] })
        ]));
      });
      tooltip.hidden = false;
      var left = geom.sx(data.x[hover]) + 12;
      var tipWidth = tooltip.offsetWidth || 150;
      if (left + tipWidth > geom.width) left = geom.sx(data.x[hover]) - tipWidth - 12;
      tooltip.style.left = Math.max(0, left) + "px";
      tooltip.style.top = geom.pad.top + "px";
      if (announce) live.textContent = opts.announce ? opts.announce(hover) : out.title + ": " + out.rows.map(function (r) { return r[3] + " " + r[2]; }).join(", ");
    }

    function hideTooltip() {
      hover = null;
      tooltip.hidden = true;
      draw();
    }

    function indexFromEvent(event) {
      if (!data || !geom) return null;
      var frac = (event.clientX - canvas.getBoundingClientRect().left - geom.left) / geom.plotW;
      if (frac < -0.02 || frac > 1.02) return null;
      var target = scaleX(data.xmin) + frac * (scaleX(data.xmax) - scaleX(data.xmin));
      var best = 0;
      for (var i = 1; i < data.x.length; i++) if (Math.abs(scaleX(data.x[i]) - target) < Math.abs(scaleX(data.x[best]) - target)) best = i;
      return best;
    }

    canvas.addEventListener("pointermove", function (event) {
      var i = indexFromEvent(event);
      if (i === null) hideTooltip(); else showTooltip(i, false);
    });
    canvas.addEventListener("pointerleave", hideTooltip);
    canvas.addEventListener("focus", function () { if (data) showTooltip(hover === null ? Math.floor(data.x.length / 2) : hover, true); });
    canvas.addEventListener("blur", hideTooltip);
    canvas.addEventListener("keydown", function (event) {
      if (!data) return;
      var n = data.x.length;
      var stepSize = event.shiftKey ? Math.max(1, Math.round(n / 20)) : Math.max(1, Math.round(n / 100));
      var current = hover === null ? Math.floor(n / 2) : hover;
      if (event.key === "ArrowLeft") current -= stepSize;
      else if (event.key === "ArrowRight") current += stepSize;
      else if (event.key === "Home") current = 0;
      else if (event.key === "End") current = n - 1;
      else if (event.key === "Escape") { hideTooltip(); return; }
      else return;
      event.preventDefault();
      showTooltip(current, true);
    });

    return {
      node: node,
      canvas: canvas,
      set: function (d) { data = d; draw(); },
      redraw: draw,
      label: function (text) { canvas.setAttribute("aria-label", text); }
    };
  }

  function watch(plots, update) {
    var resize = typeof W.ResizeObserver === "function" ? new W.ResizeObserver(scheduler(update)) : null;
    plots.forEach(function (p) { if (resize) resize.observe(p.node); });
    var observer = new W.MutationObserver(scheduler(function () { plots.forEach(function (p) { p.redraw(); }); }));
    observer.observe(D.body, { attributes: true, attributeFilter: ["data-md-color-scheme"] });
    return function () {
      if (resize) resize.disconnect();
      observer.disconnect();
    };
  }

  /* ---- deep links */

  function parseLink(search) {
    var params = {};
    var calc = null;
    String(search || "").replace(/^\?/, "").split("&").forEach(function (pair) {
      if (!pair) return;
      var i = pair.indexOf("=");
      var k = decodeURIComponent(i < 0 ? pair : pair.slice(0, i));
      var v = i < 0 ? "" : decodeURIComponent(pair.slice(i + 1).replace(/\+/g, " "));
      if (k === "calc") calc = v; else params[k] = v;
    });
    return { calc: calc, params: params };
  }

  function formatLink(calc, params) {
    var parts = ["calc=" + encodeURIComponent(calc)];
    Object.keys(params || {}).forEach(function (k) {
      if (params[k] !== undefined && params[k] !== null) parts.push(encodeURIComponent(k) + "=" + encodeURIComponent(String(params[k])));
    });
    return "?" + parts.join("&");
  }

  function same(a, b) {
    return typeof a === "number" ? Math.abs(a - b) <= 1e-9 * Math.max(1, Math.abs(b)) : a === b;
  }

  /* Applies ?calc=<name>&… on mount; returns the writer (values differing from the preset defaults).
       spec = { preset: { field, apply(id) }, fields: [[name, field, defaultFn, parseFn]], after() } */
  function linkFields(name, link, spec) {
    var q = link && link.calc === name ? link.params : null;
    if (q) {
      if (spec.preset && q.preset !== undefined) spec.preset.apply(q.preset);
      spec.fields.forEach(function (f) {
        if (q[f[0]] === undefined) return;
        var v = f[3] ? f[3](q[f[0]]) : parseFloat(q[f[0]]);
        if (v !== undefined && !(typeof v === "number" && !isFinite(v))) f[1].set(v);
      });
      if (spec.after) spec.after();
    }
    return function () {
      var params = {};
      if (spec.preset) params.preset = spec.preset.field.get();
      spec.fields.forEach(function (f) {
        var v = f[1].get();
        if (v === undefined) return;
        var d = f[2] ? f[2]() : undefined;
        if (d === undefined || !same(v, d)) params[f[0]] = typeof v === "number" ? String(+v.toFixed(6)) : typeof v === "boolean" ? (v ? "1" : "0") : String(v);
      });
      link.write(name, params);
    };
  }

  function readLink() {
    var link = parseLink(W.location ? W.location.search : "");
    var timer = null;
    link.write = function (name, params) {
      W.clearTimeout(timer);
      timer = W.setTimeout(function () {
        try { W.history.replaceState(null, "", W.location.pathname + formatLink(name, params) + (W.location.hash || "")); } catch (e) { /* rate-limited */ }
      }, 250);
    };
    return link;
  }

  function flag(v) { return v === "1" || v === "true"; }

  function updater(fn) {
    var dirty = false;
    var out = { write: null };
    out.schedule = scheduler(function () { fn(); if (dirty) { dirty = false; out.write(); } });
    out.touch = function () { dirty = true; out.schedule(); };
    return out;
  }

  function presetDefault(preset, pick) {
    return function () { var p = presetById(preset.get()); return p ? pick(p) : undefined; };
  }

  /* ---- (a) Rayleigh */

  var NA_GRID = (function () {
    var out = [];
    for (var na = 0.05; na <= 1.7 + 1e-9; na += 0.0125) out.push(+na.toFixed(4));
    return out;
  })();

  function initRayleigh(root, link) {
    var box = card(root, "Resolution and depth of focus");
    var update;
    var run = updater(function () { update(); });
    var touch = run.touch;
    function usePreset(id) {
      var p = presetById(id);
      preset.set(id);
      if (p) {
        lambda.set(p.wavelength);
        na.set(p.na);
        index.set(p.n);
      }
    }
    var preset = selectField({
      label: "Wavelength preset (* = illustrative NA)",
      options: presetOptions(P.PRESETS),
      value: "arf",
      onChange: function (id) { usePreset(id); touch(); }
    });
    function edited() {
      var p = presetById(preset.get());
      if (p && (p.wavelength !== lambda.get() || p.na !== na.get() || p.n !== index.get())) preset.set("custom");
      touch();
    }
    var lambda = rangeField({ label: "Wavelength λ", unit: "nm", min: 1, max: 500, step: 0.1, value: 193, onInput: edited });
    var na = rangeField({ label: "Numerical aperture NA", min: 0.05, max: 1.7, step: 0.01, value: 0.93, onInput: edited });
    var index = rangeField({
      label: "Medium index n", min: 1, max: 1.9, step: 0.01, value: 1, onInput: edited,
      hint: "1 for air or vacuum; 1.437 for water at 193 nm"
    });
    var k1 = rangeField({
      label: "Process factor k₁", min: 0.2, max: 1, step: 0.01, value: 0.3, onInput: touch,
      hint: "0.25 is the single-exposure floor for dense lines"
    });
    var k2 = rangeField({
      label: "DOF factor k₂", min: 0.2, max: 1.5, step: 0.05, value: 1, onInput: touch,
      hint: "1 reproduces Rayleigh's quarter-wave criterion"
    });
    var out = tiles([
      ["r", "Half-pitch R = k₁λ/NA"],
      ["p", "Minimum pitch 2R"],
      ["dofp", "Depth of focus, paraxial"],
      ["dofe", "Depth of focus, exact"]
    ]);
    var note = message();
    var c1 = seriesColor(root, 1);
    var c2 = seriesColor(root, 2);
    var key = legend([
      [c1, "R = k₁λ/NA at your λ", null],
      [c2, "DOF exact (ends at NA = n)", null],
      [c2, "DOF paraxial k₂λ/NA²", "dashed"],
      [null, "dots: every preset at its own λ and NA", "dot"]
    ]);
    var plot = linePlot({
      root: root, label: "Resolution and depth of focus against numerical aperture", xLabel: "numerical aperture NA",
      panels: [{ yLabel: "nm (log scale)", logY: true }],
      readout: function (i) {
        var x = NA_GRID[i];
        var r = P.rayleigh({ wavelength: lambda.get(), na: x, n: index.get(), k1: k1.get(), k2: k2.get() });
        var rows = [[c1, false, nm(r.halfPitch), "half-pitch R"], [c2, true, nm(r.dofParaxial), "DOF paraxial"]];
        if (isFinite(r.dofExact)) rows.push([c2, false, nm(r.dofExact), "DOF exact"]);
        P.PRESETS.forEach(function (p) {
          if (Math.abs(p.na - x) <= 0.0063) rows.push([c1, false, nm(P.rayleigh({ wavelength: p.wavelength, na: p.na, n: p.n, k1: k1.get(), k2: k2.get() }).halfPitch), p.label.split(",")[0] + " preset, R"]);
        });
        return { title: "NA = " + fixed(x, 2) + " at λ = " + nm(lambda.get()), rows: rows };
      }
    });
    var tbl = table("Rayleigh resolution and depth of focus for every preset", [
      "Preset", "λ (nm)", "NA", "R (nm)", "Pitch (nm)", "DOF paraxial (nm)", "DOF exact (nm)"
    ]);
    var tableNote = hint("");

    var speak = announcer();
    [controls("Resolution inputs", [preset.node, lambda.node, na.node, index.node, k1.node, k2.node]), out.node, speak.node, note.node, key, plot.node,
      hint("H" + HINT.slice(1)), tbl.node, tableNote].forEach(function (n) { box.appendChild(n); });

    run.write = linkFields("rayleigh", link, {
      preset: { field: preset, apply: usePreset },
      fields: [
        ["lambda", lambda, presetDefault(preset, function (p) { return p.wavelength; })],
        ["na", na, presetDefault(preset, function (p) { return p.na; })],
        ["n", index, presetDefault(preset, function (p) { return p.n; })],
        ["k1", k1, function () { return 0.3; }],
        ["k2", k2, function () { return 1; }]
      ],
      after: function () { var p = presetById(preset.get()); if (p && (p.wavelength !== lambda.get() || p.na !== na.get() || p.n !== index.get())) preset.set("custom"); }
    });

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

      var rows = P.PRESETS.map(function (p) {
        var q = P.rayleigh({ wavelength: p.wavelength, na: p.na, n: p.n, k1: k1.get(), k2: k2.get() });
        return {
          current: p.id === preset.get(),
          cells: [p.label + (p.illustrative ? " *" : ""), sig(p.wavelength, 3), fixed(p.na, 2),
            sig(q.halfPitch, 3), sig(q.pitch, 3), sig(q.dofParaxial, 3), sig(q.dofExact, 3)],
          r: q
        };
      });
      tbl.rows(rows);
      var caption = "Every preset at k₁ = " + fixed(k1.get(), 2) + " and k₂ = " + fixed(k2.get(), 2) +
        ", each with its own NA and medium index. * Illustrative NA, not a specific tool.";
      tbl.caption(caption);
      tableNote.textContent = caption;

      // Curves at the current settings; dots for every preset at its own λ.
      var rCurve = [], dofP = [], dofE = [], lo = Infinity, hi = 0;
      NA_GRID.forEach(function (x) {
        var q = P.rayleigh({ wavelength: lambda.get(), na: x, n: index.get(), k1: k1.get(), k2: k2.get() });
        rCurve.push(q.halfPitch);
        dofP.push(q.dofParaxial);
        dofE.push(q.dofExact);
        lo = Math.min(lo, q.halfPitch);
        hi = Math.max(hi, q.dofParaxial);
      });
      var points = [{ x: na.get(), y: r.halfPitch, color: c1, ring: true }, { x: na.get(), y: r.dofExact, color: c2, ring: true }];
      P.PRESETS.forEach(function (p, i) {
        var q = rows[i].r;
        var dof = isFinite(q.dofExact) ? q.dofExact : q.dofParaxial;
        points.push({ x: p.na, y: q.halfPitch, color: c1, ring: p.id === preset.get() }, { x: p.na, y: dof, color: c2, ring: p.id === preset.get() });
        lo = Math.min(lo, q.halfPitch);
        hi = Math.max(hi, dof);
      });
      plot.set({
        x: NA_GRID, xmin: 0.05, xmax: 1.7, marks: [na.get()],
        panels: [{ min: decade(lo), max: decade(hi, true), points: points,
          series: [{ y: dofP, color: c2, dashed: true, width: 1.5 }, { y: dofE, color: c2 }, { y: rCurve, color: c1 }] }]
      });
      plot.label("Half-pitch and depth of focus against NA at " + nm(lambda.get()) + ", k₁ " + fixed(k1.get(), 2) + ", k₂ " + fixed(k2.get(), 2) +
        ": at NA " + fixed(na.get(), 2) + " R is " + nm(r.halfPitch) + " and the exact depth of focus " + (isFinite(r.dofExact) ? nm(r.dofExact) : "undefined") +
        "; dots mark the nine presets at their own wavelengths.");
    };
    update();
    return watch([plot], update);
  }

  /* ---- (b) shot noise */

  var WAVELENGTHS = [
    { id: "g", label: "436 nm, g-line", wavelength: 436 },
    { id: "i", label: "365 nm, i-line", wavelength: 365 },
    { id: "krf", label: "248 nm, KrF", wavelength: 248 },
    { id: "arf", label: "193 nm, ArF", wavelength: 193 },
    { id: "f2", label: "157 nm, F₂", wavelength: 157 },
    { id: "euv", label: "13.5 nm, EUV", wavelength: 13.5 },
    { id: "beuv", label: "6.7 nm, beyond-EUV", wavelength: 6.7 }
  ];

  var SIZE_GRID = (function () {
    var out = [];
    for (var i = 0; i <= 60; i++) out.push(+Math.pow(10, Math.log10(2) + (i / 60) * Math.log10(100)).toPrecision(5));
    return out;
  })();

  function wavelengthById(id) {
    for (var i = 0; i < WAVELENGTHS.length; i++) if (WAVELENGTHS[i].id === id) return WAVELENGTHS[i];
    return null;
  }

  function initShotNoise(root, link) {
    var box = card(root, "Photon shot noise");
    var update;
    var run = updater(function () { update(); });
    var touch = run.touch;
    function usePreset(id) {
      var p = wavelengthById(id);
      preset.set(id);
      if (p) lambda.set(p.wavelength);
    }
    var preset = selectField({
      label: "Wavelength preset",
      options: WAVELENGTHS.map(function (p) { return [p.id, p.label]; }).concat([["custom", "Custom wavelength"]]),
      value: "euv",
      onChange: function (id) { usePreset(id); touch(); }
    });
    function edited() {
      var p = wavelengthById(preset.get());
      if (p && p.wavelength !== lambda.get()) preset.set("custom");
    }
    var lambda = rangeField({
      label: "Wavelength λ", unit: "nm", min: 1, max: 500, step: 0.1, value: 13.5,
      onInput: function () { edited(); touch(); }
    });
    var dose = rangeField({ label: "Dose", unit: "mJ/cm²", min: 1, max: 200, step: 1, value: 30, onInput: touch });
    var shape = selectField({
      label: "Feature shape",
      options: [["square", "Square pixel (size = side)"], ["circle", "Round contact (size = diameter)"]],
      value: "square",
      onChange: touch
    });
    var size = rangeField({ label: "Feature size", unit: "nm", min: 2, max: 200, step: 1, value: 20, onInput: touch });
    var absorbed = rangeField({
      label: "Fraction absorbed in the resist", min: 0.01, max: 1, step: 0.01, value: 1, onInput: touch,
      hint: "1 counts every incident photon"
    });
    var target = rangeField({ label: "Target 1σ fluctuation", unit: "%", min: 0.2, max: 10, step: 0.1, value: 2, onInput: touch });
    var out = tiles([
      ["energy", "Photon energy"],
      ["density", "Photons per nm²"],
      ["count", "Photons in the feature"],
      ["sigma", "1σ fluctuation 1/√N"],
      ["dose", "Dose for the target"]
    ]);
    var key = legend(WAVELENGTHS.map(function (p, i) { return [seriesColor(root, i + 1), p.label, null]; }).concat([[seriesColor(root, 8), "custom λ", null]]));
    function curves() {
      var set = WAVELENGTHS.map(function (w, i) { return { id: w.id, label: w.label, wavelength: w.wavelength, slot: i + 1 }; });
      if (!wavelengthById(preset.get())) set.push({ id: "custom", label: "custom " + nm(lambda.get()), wavelength: lambda.get(), slot: 8 });
      return set;
    }
    var plot = linePlot({
      root: root, label: "Photons per feature and their fluctuation against feature size", xLabel: "feature size (nm, log scale)", logX: true, height: "24rem",
      panels: [{ yLabel: "incident photons × f_abs, Poisson only", logY: true }, { yLabel: "1σ = 1/√N (%)", logY: true }],
      readout: function (i) {
        var area = P.featureArea(shape.get(), SIZE_GRID[i]);
        return {
          title: "feature " + nm(SIZE_GRID[i], 0) + " (" + sig(area, 3) + " nm²) at " + sig(dose.get(), 3) + " mJ/cm²",
          rows: curves().map(function (c) {
            var q = P.shotNoise({ wavelength: c.wavelength, dose: dose.get(), area: area, absorbed: absorbed.get() });
            return [seriesColor(root, c.slot), false, sig(q.count, 3), c.label + " · 1σ " + percent(q.relSigma)];
          })
        };
      }
    });
    var tbl = table("Photon statistics at every preset wavelength", [
      "Wavelength", "E (eV)", "Photons/nm²", "Photons in feature", "1σ", "Dose for target (mJ/cm²)"
    ]);
    var tableNote = hint("");

    var speak = announcer();
    [controls("Shot-noise inputs", [preset.node, lambda.node, dose.node, shape.node, size.node, absorbed.node, target.node]), out.node, speak.node, key, plot.node,
      hint("Both panels share the size axis; " + HINT), tbl.node, tableNote].forEach(function (n) { box.appendChild(n); });

    run.write = linkFields("shotnoise", link, {
      preset: { field: preset, apply: usePreset },
      fields: [
        ["lambda", lambda, function () { var p = wavelengthById(preset.get()); return p ? p.wavelength : undefined; }],
        ["dose", dose, function () { return 30; }],
        ["shape", shape, function () { return "square"; }, function (v) { return v === "circle" ? "circle" : "square"; }],
        ["size", size, function () { return 20; }],
        ["fabs", absorbed, function () { return 1; }],
        ["target", target, function () { return 2; }]
      ],
      after: edited
    });

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

      // One curve per wavelength; the selected one bold.
      var selected = wavelengthById(preset.get()) ? preset.get() : "custom";
      var counts = [], sigmas = [], nmin = Infinity, nmax = 0, ringColor = seriesColor(root, 8);
      curves().forEach(function (c) {
        var ys = [], ss = [];
        SIZE_GRID.forEach(function (s) {
          var q = P.shotNoise({ wavelength: c.wavelength, dose: dose.get(), area: P.featureArea(shape.get(), s), absorbed: absorbed.get() });
          ys.push(q.count);
          ss.push(100 * q.relSigma);
          nmin = Math.min(nmin, q.count);
          nmax = Math.max(nmax, q.count);
        });
        var on = c.id === selected;
        var style = { color: seriesColor(root, c.slot), width: on ? 2.5 : 1.25, alpha: on ? 1 : 0.7 };
        if (on) ringColor = style.color;
        counts.push(Object.assign({ y: ys }, style));
        sigmas.push(Object.assign({ y: ss }, style));
      });
      plot.set({
        x: SIZE_GRID, xmin: 2, xmax: 200, marks: [size.get()],
        panels: [
          { min: decade(nmin), max: decade(nmax, true), series: counts, points: [{ x: size.get(), y: r.count, color: ringColor, ring: true }] },
          { min: decade(100 / Math.sqrt(nmax)), max: decade(100 / Math.sqrt(nmin), true), series: sigmas, points: [{ x: size.get(), y: 100 * r.relSigma, color: ringColor, ring: true }] }
        ]
      });
      plot.label("Photons in the feature (top) and their 1σ fluctuation (bottom) against feature size from 2 to 200 nm at " + sig(dose.get(), 3) +
        " mJ/cm², one curve per preset wavelength; at " + nm(size.get(), 0) + " the selected " + nm(lambda.get()) + " receives " + sig(r.count, 3) +
        " photons, a 1σ fluctuation of " + percent(r.relSigma) + ".");
    };
    update();
    return watch([plot], update);
  }

  /* ---- (c) aerial image */

  function presetGeometry(p) {
    // Half-pitch at k1 = 0.45, rounded: a pitch that images comfortably.
    var pitch = Math.max(10, Math.round((2 * 0.45 * p.wavelength) / p.na));
    return { pitch: pitch, cd: Math.round(pitch / 2) };
  }

  /* The live pupil (D5): rim of radius 1 (NA/λ), dashed support circle at 1 + σ, a copy of the
     source at every order m/(2k₁) tinted by |m|, the sampled diameter through each copy. */
  function pupilView(root) {
    function svg(tag, attrs, style) {
      var n = D.createElementNS("http://www.w3.org/2000/svg", tag);
      Object.keys(attrs).forEach(function (k) { n.setAttribute(k, String(attrs[k])); });
      Object.keys(style || {}).forEach(function (k) { n.style[k] = style[k]; });
      return picture.appendChild(n);
    }
    var picture = D.createElementNS("http://www.w3.org/2000/svg", "svg");
    ["class", "huv-pupil__svg", "viewBox", "-1.95 -1.55 3.9 3.1", "width", "260", "height", "207", "role", "img", "style", "max-width:100%;height:auto;display:block"]
      .forEach(function (v, i, a) { if (i % 2 === 0) picture.setAttribute(v, a[i + 1]); });
    var key = legend([
      [seriesColor(root, 1), "order 0", null], [seriesColor(root, 2), "orders ±1", null], [seriesColor(root, 3), "orders ±2", null],
      [null, "NA/λ rim · dashed (1+σ)NA/λ support", "dashed"]
    ]);
    var node = el("div", { className: "huv-pupil" }, [picture, key]);

    function update(geo, pitch) {
      var pal = palette(root);
      picture.textContent = "";
      svg("circle", { cx: 0, cy: 0, r: geo.support, fill: "none", "stroke-width": 0.02, "stroke-dasharray": "0.08 0.05" }, { stroke: pal.faint });
      svg("circle", { cx: 0, cy: 0, r: 1, fill: "none", "stroke-width": 0.03 }, { stroke: pal.ink });
      svg("line", { x1: -1.95, y1: 0, x2: 1.95, y2: 0, "stroke-width": 0.012 }, { stroke: pal.faint });
      var s = geo.source;
      var passing = [];
      geo.samples.forEach(function (sample) {
        sample.orders.forEach(function (o) { if (o.inside && passing.indexOf(o.m) < 0) passing.push(o.m); });
      });
      for (var m = -6; m <= 6; m++) {
        var cx = m * geo.spacing;
        if (Math.abs(cx) > 1.95 + s.outer) continue;
        var slot = Math.abs(m) + 1;
        var color = slot <= 3 ? seriesColor(root, slot) : pal.faint;
        var passes = passing.indexOf(m) >= 0;
        var copies = s.kind === "dipole" ? [cx - s.centre, cx + s.centre] : [cx];
        copies.forEach(function (c) {
          if (s.radius <= 0) return;
          svg("circle", { cx: c, cy: 0, r: s.radius, "stroke-width": 0.015 }, { fill: color, fillOpacity: passes ? "0.22" : "0.08", stroke: color, strokeOpacity: passes ? "0.9" : "0.45" });
          svg("line", { x1: c - s.radius, y1: 0, x2: c + s.radius, y2: 0, "stroke-width": 0.05, "stroke-linecap": "round" }, { stroke: color, strokeOpacity: passes ? "1" : "0.45" });
        });
        svg("circle", { cx: cx, cy: 0, r: 0.065, "stroke-width": 0.03 }, { fill: passes ? color : "none", stroke: color });
        if (Math.abs(m) <= 3 && Math.abs(cx) <= 1.7) svg("text", { x: cx, y: -(s.outer + 0.16), "text-anchor": "middle", "font-size": 0.17 }, { fill: pal.muted }).textContent = m === 0 ? "0" : (m > 0 ? "+" : "−") + Math.abs(m);
      }
      svg("text", { x: 0.74, y: 0.82, "font-size": 0.15 }, { fill: pal.muted }).textContent = "NA/λ";
      var passText = passing.length ? passing.sort(function (a, b) { return a - b; }).map(function (m) { return m > 0 ? "+" + m : minus(String(m)); }).join(", ") : "none";
      picture.setAttribute("aria-label", "Pupil diagram: the orders of the " + nm(pitch, 0) + " grating sit every " + fixed(geo.spacing, 2) +
        " pupil units (NA/λ = 1); a copy of the " + (s.kind === "coherent" ? "on-axis point source" : s.kind === "dipole" ? "two-pole source" : "σ " + fixed(s.outer, 2) + " source") +
        " is drawn at each order and only its horizontal diameter is sampled; orders reaching the pupil for some source point: " + passText +
        "; support circle at " + fixed(geo.support, 2) + ".");
    }
    return { node: node, update: update };
  }

  function initAerial(root, link) {
    var box = card(root, "Partially coherent aerial image (1-D toy)");
    var update;
    var run = updater(function () { update(); });
    var touch = run.touch;
    var start = presetById("arf");
    function usePreset(id) {
      var p = presetById(id);
      preset.set(id);
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
    }
    var preset = selectField({
      label: "Wavelength preset (* = illustrative NA)",
      options: presetOptions(P.PRESETS),
      value: "arf",
      onChange: function (id) { usePreset(id); touch(); }
    });
    function edited() {
      var p = presetById(preset.get());
      if (p && (p.wavelength !== lambda.get() || p.na !== na.get() || p.n !== index.get())) preset.set("custom");
    }
    function changed() { edited(); touch(); }
    var lambda = rangeField({ label: "Wavelength λ", unit: "nm", min: 1, max: 500, step: 0.1, value: start.wavelength, onInput: changed });
    var index = rangeField({
      label: "Medium index n", min: 1, max: 1.9, step: 0.01, value: start.n,
      onInput: function () {
        na.setLimits(0.05, Math.round((index.get() - 0.01) * 100) / 100, 0.01);
        changed();
      }
    });
    var na = rangeField({ label: "Numerical aperture NA", min: 0.05, max: 0.99, step: 0.01, value: start.na, onInput: changed });
    /* k₁ = (p/2)·NA/λ: drives the pitch (duty cycle kept) and follows it. */
    var k1Slider = rangeField({
      label: "k₁ of the half-pitch (sets the pitch)", min: 0.2, max: 2, step: 0.01, value: 0.45, output: true,
      onInput: function () { setK1(k1Slider.get()); touch(); },
      hint: "0.5 coherent limit · 1/(2(1+σ)) partially coherent · 0.25 dipole floor"
    });
    function setK1(v) {
      var p = Math.max(10, Math.round((2 * v * lambda.get()) / na.get()));
      var duty = cd.get() / pitch.get();
      pitch.set(p);
      cd.setLimits(1, Math.max(2, p - 1), 0.5);
      cd.set(Math.round(duty * p));
    }
    var sigmaOut = rangeField({
      label: "Source σ outer", min: 0, max: 1, step: 0.01, value: 0.7, onInput: touch,
      hint: "0 = coherent; extent of the 1-D source in pupil units"
    });
    var sigmaIn = rangeField({
      label: "Source σ inner", min: 0, max: 1, step: 0.01, value: 0, onInput: touch,
      hint: "0 = conventional; above 0 = two poles (dipole)"
    });
    var g0 = presetGeometry(start);
    var pitch = rangeField({
      label: "Pitch p", unit: "nm", min: 10, max: 2000, step: 1, value: g0.pitch,
      onInput: function () {
        cd.setLimits(1, Math.max(2, pitch.get() - 1), 0.5);
        touch();
      }
    });
    var cd = rangeField({ label: "Line width (opaque)", unit: "nm", min: 1, max: g0.pitch - 1, step: 0.5, value: g0.cd, onInput: touch });
    var focus = rangeField({ label: "Defocus z", unit: "nm", min: -1000, max: 1000, step: 1, value: 0, onInput: touch });
    var threshold = rangeField({
      label: "Resist threshold", min: 0.02, max: 1.2, step: 0.01, value: 0.3, onInput: touch,
      hint: "intensity (clear field = 1) below which the line prints"
    });
    var coherent = checkField({ label: "Compare with coherent illumination (σ = 0)", value: true, onChange: touch });

    var out = tiles([
      ["k1", "k₁ of the half-pitch"],
      ["contrast", "Image contrast"],
      ["nils", "NILS at the line edge"],
      ["cd", "Printed line width"],
      ["orders", "Orders through the pupil"]
    ]);
    var note = message();
    var c1 = seriesColor(root, 1);
    var c2 = seriesColor(root, 2);
    var key = legend([[c1, "Partially coherent", null], [c2, "Coherent (σ = 0)", "dashed"], ["var(--md-default-fg-color--light)", "Resist threshold", "dashed"], [null, "Opaque mask lines", "mask"]]);
    var legendMainText = key.children[0].lastElementChild;
    var legendRefItem = key.children[1];
    var state = null; // last computed curves, for the readout
    var plot = linePlot({
      root: root, label: "Aerial image intensity plot", xLabel: "x (nm)", xExtra: 14,
      panels: [{ yLabel: "Intensity (clear field = 1)" }],
      extras: function (ctx, s, pal, d) {
        // Mask strip (opaque lines dark) under the plot, then the threshold.
        var stripTop = s.bottom[0] + 6;
        ctx.fillStyle = pal.grid;
        ctx.fillRect(s.left, stripTop, s.plotW, 7);
        ctx.fillStyle = pal.muted;
        for (var k = Math.floor(d.xmin / d.pitch) - 1; k <= Math.ceil(d.xmax / d.pitch) + 1; k++) {
          var a = Math.max(d.xmin, k * d.pitch - d.cd / 2);
          var b = Math.min(d.xmax, k * d.pitch + d.cd / 2);
          if (b > a) ctx.fillRect(s.sx(a), stripTop, s.sx(b) - s.sx(a), 7);
        }
        ctx.save();
        ctx.setLineDash([4, 4]);
        ctx.strokeStyle = pal.muted;
        ctx.beginPath();
        ctx.moveTo(s.left, Math.round(s.sy[0](d.threshold)) + 0.5);
        ctx.lineTo(s.right, Math.round(s.sy[0](d.threshold)) + 0.5);
        ctx.stroke();
        ctx.restore();
      },
      readout: function (i) {
        var rows = [[c1, false, fixed(state.main.intensity[i], 3), "partially coherent"]];
        if (state.ref) rows.push([c2, true, fixed(state.ref.intensity[i], 3), "coherent"]);
        return { title: "x = " + fixed(state.main.x[i], 1) + " nm", rows: rows };
      },
      announce: function (i) {
        return "x " + fixed(state.main.x[i], 1) + " nm: intensity " + fixed(state.main.intensity[i], 3) +
          (state.ref ? ", coherent " + fixed(state.ref.intensity[i], 3) : "");
      }
    });
    var pupil = pupilView(root);
    var row = el("div", { className: "huv-calc__row", style: "display:flex;flex-wrap:wrap;gap:1rem;align-items:flex-start" }, [pupil.node, plot.node]);
    pupil.node.style.flex = "0 1 16rem";
    plot.node.style.flex = "1 1 18rem";
    plot.node.style.minWidth = "0";
    var dataTable = table("Aerial image samples over one period", ["x (nm)", "Partially coherent", "Coherent"]);

    var speak = announcer();
    [controls("Aerial image inputs", [preset.node, lambda.node, index.node, na.node, k1Slider.node, sigmaOut.node, sigmaIn.node, pitch.node, cd.node, focus.node, threshold.node, coherent.node]),
      out.node, speak.node, note.node, key, row,
      hint("Left: the pupil with a copy of the source at every order. Right: the image; " + HINT),
      el("details", null, [el("summary", { text: "Show the image as a table" }), dataTable.node])].forEach(function (n) { box.appendChild(n); });

    var k1Field = { get: function () { return undefined; }, set: setK1 };
    run.write = linkFields("aerial", link, {
      preset: { field: preset, apply: usePreset },
      fields: [
        ["lambda", lambda, presetDefault(preset, function (p) { return p.wavelength; })],
        ["n", index, presetDefault(preset, function (p) { return p.n; })],
        ["na", na, presetDefault(preset, function (p) { return p.na; })],
        ["k1", k1Field],
        ["pitch", pitch, presetDefault(preset, function (p) { return presetGeometry(p).pitch; })],
        ["cd", cd, presetDefault(preset, function (p) { return presetGeometry(p).cd; })],
        ["sigma", sigmaOut, function () { return 0.7; }],
        ["sigmain", sigmaIn, function () { return 0; }],
        ["z", focus, function () { return 0; }],
        ["t", threshold, function () { return 0.3; }],
        ["coh", coherent, function () { return true; }, flag]
      ],
      after: edited
    });

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
      var samples = Math.min(801, Math.max(241, Math.round(plot.canvas.clientWidth || 480)));
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
      k1Slider.set(k1);
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
          : model.ordersOnAxis.map(function (m) { return m > 0 ? "+" + m : minus(String(m)); }).join(", "),
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

      var rows = [];
      for (var i = 0; i <= 20; i++) {
        var x = -pitch.get() / 2 + (pitch.get() * i) / 20;
        rows.push({ cells: [fixed(x, 1), fixed(model.at(x), 4), refModel ? fixed(refModel.at(x), 4) : "—"] });
      }
      dataTable.rows(rows);

      speak.say("Contrast " + fixed(metrics.contrast, 2) + ", NILS " +
        (isFinite(metrics.nils) ? fixed(metrics.nils, 1) : "undefined") + ", printed line " +
        (printed === 0 ? "does not print" : printed >= pitch.get() - 1e-9 ? "fills the pitch" : nm(printed, 1)) + ".");
      plot.label(
        "Aerial image over two periods of " + nm(pitch.get(), 0) + ": intensity from " + fixed(metrics.imin, 2) +
        " to " + fixed(metrics.imax, 2) + " of the clear-field level, contrast " + fixed(metrics.contrast, 2) +
        (ref ? "; the coherent image reaches contrast " + fixed(refMetrics.contrast, 2) : "") + ".");
      var series = [{ y: main.intensity, color: c1 }];
      if (ref) series.unshift({ y: ref.intensity, color: c2, dashed: true });
      plot.set({
        x: main.x, xmin: -pitch.get(), xmax: pitch.get(), pitch: pitch.get(), cd: cd.get(), threshold: params.threshold,
        panels: [{ min: 0, max: Math.max(1, metrics.imax, ref ? Math.max.apply(null, ref.intensity) : 0, params.threshold) * 1.08, series: series }]
      });
      pupil.update(P.pupilOrders({ k1: k1, sigma: sigmaOut.get(), fill: sigmaIn.get() > 0 ? "dipole" : "conventional", poles: sigmaIn.get() }), pitch.get());
    };
    update();
    return watch([plot], update);
  }

  /* ---- mounting */

  var WIDGETS = { rayleigh: initRayleigh, shotnoise: initShotNoise, aerial: initAerial };
  var cleanups = [];
  var scrolled = false;

  function fallbackFor(root) {
    var prev = root.previousElementSibling;
    return prev && prev.tagName === "FIGURE" ? prev : null;
  }

  function showFallbacks(show) {
    list(D.querySelectorAll("figure[data-huv-fallback]")).forEach(function (fig) {
      fig.hidden = !show;
      fig.style.display = show ? "" : "none";
    });
  }

  function mountAll() {
    cleanups.forEach(function (fn) { fn(); });
    cleanups = [];
    var link = readLink();
    list(D.querySelectorAll("[data-huv-calc]")).forEach(function (root) {
      var init = WIDGETS[root.getAttribute("data-huv-calc")];
      if (!init || root.getAttribute("data-huv-mounted")) return;
      root.setAttribute("data-huv-mounted", "true");
      var fig = fallbackFor(root);
      if (fig) {
        fig.setAttribute("data-huv-fallback", "hidden");
        fig.hidden = true;
        fig.style.display = "none";
      }
      try {
        var cleanup = init(root, link);
        if (typeof cleanup === "function") cleanups.push(cleanup);
        if (typeof W.huvMarkOverflow === "function") W.huvMarkOverflow(root);
      } catch (error) {
        root.appendChild(el("p", { className: "huv-calc__message", text: "This calculator failed to start: " + error.message }));
        if (W.console) W.console.error(error);
      }
    });
    if (link.calc && !scrolled) {
      scrolled = true;
      var target = D.querySelector('[data-huv-calc="' + link.calc + '"][data-huv-mounted]');
      if (target && !(W.location && W.location.hash) && target.scrollIntoView) target.scrollIntoView();
    }
  }

  function install(win) {
    W = win;
    D = win.document;
    P = win.HUVPhysics || (typeof require === "function" ? require("./huv-physics.js") : null);
    if (!P) return false;
    win.addEventListener("beforeprint", function () { showFallbacks(true); });
    win.addEventListener("afterprint", function () { showFallbacks(false); });
    if (win.document$) win.document$.subscribe(mountAll);
    else if (D.readyState === "loading") D.addEventListener("DOMContentLoaded", mountAll);
    else mountAll();
    return true;
  }

  return {
    install: install,
    mountAll: mountAll,
    WIDGETS: WIDGETS,
    parseLink: parseLink,
    formatLink: formatLink,
    fallbackFor: fallbackFor,
    ticks: ticks,
    niceStep: niceStep,
    logTicks: logTicks,
    SERIES: SERIES
  };
});
