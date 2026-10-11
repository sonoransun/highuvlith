/* Physics for the playground calculators (docs/playground/index.md) and the calculators
 * embedded on nodes/litho-math.md and nodes/7-and-5nm.md.
 *
 * Closed-form textbook formulas only: Rayleigh resolution and depth of focus, photon
 * counting statistics, a 1-D scalar Abbe image of a line/space grating and the pupil
 * geometry of that grating (the site's D5 pupil grammar). This is a teaching toy, not
 * the Rust engine. Pure functions, no DOM: loaded by the site as window.HUVPhysics and
 * by `node --test docs/javascripts/tests/*.test.mjs` (physics.test.mjs).
 *
 * Exports (owner G0b; DESIGN §5.6 R3/R4, §5.2 I2):
 *   PRESETS                      wavelength presets {id, label, wavelength, na, n, illustrative}
 *   rayleigh(p)                  {halfPitch, pitch, dofParaxial, dofExact, minHalfPitch}
 *   photonEnergy(nm)             {eV, J}
 *   featureArea(shape, size)     nm² of a square or round feature
 *   shotNoise(p) / doseForNoise(p)
 *   maskCoefficient(m, cd, p)    Fourier coefficient of the opaque-line grating
 *   sourcePoints(σin, σout, n)   the 1-D source samples the toy integrates over
 *   aerialModel(p) / imageMetrics(model, p)
 *   pupilOrders({k1, sigma, fill, poles})   pupil geometry in units of NA/λ (R4 live pupil)
 *
 * Pinned sample points (tests/physics.test.mjs asserts them; other scripts copy them):
 *   I2 `pupil-orders-anim` (V1c, make_physics_schematics.py) — the 1:1 grating under the
 *   toy's defaults (41 midpoint source samples, thin mask, in focus):
 *     k₁ 0.40, conventional σ 0.7      contrast 0.6423340  (analytic: two-beam for |s| ≥ 0.25,
 *                                        26/41 of the samples → (2f/π) / (0.5 + 2f/π²), f = 26/41)
 *     k₁ 0.30, dipole σin = σout = 1/(4k₁) = 0.8333   contrast 0.9060367 ((2/π)/(0.5 + 2/π²))
 *     k₁ 0.25, conventional σ 0.7      contrast 0 (below the partial cutoff k₁ = 1/(2(1+σ)) = 0.2941)
 *   R3 `playground-*` fallbacks (V5, make_playground_figures.py), three points per calculator:
 *     rayleigh  λ 13.5 NA 0.33 k₁ 0.3 k₂ 1 → R 12.2727 nm, DOF exact 120.4947 nm (paraxial 123.9669)
 *               λ 193 NA 1.35 n 1.437 k₁ 0.27 k₂ 1 → R 38.6 nm, DOF exact 102.1609 nm (paraxial 105.8985)
 *               λ 13.5 NA 0.55 k₁ 0.3 k₂ 1 → R 7.3636 nm, DOF exact 40.9500 nm (paraxial 44.6281)
 *     shotnoise 13.5 nm, 30 mJ/cm², 20 nm square, f 1 → 8155.27 photons, 1σ 1.107 %
 *               193 nm, 30 mJ/cm², 20 nm square, f 1 → 116 590 photons, 1σ 0.2929 %
 *               13.5 nm, 30 mJ/cm², 20 nm circle, f 0.3 → 1921.54 photons, 1σ 2.281 %
 *     aerial    the three I2 contrasts above (the fallback is I2's still + the 1-D cut at the
 *               default preset: ArF 193 nm, NA 0.93, p 187 nm, w 94 nm, σ 0.7 → contrast 0.7923451)
 */
(function (root, factory) {
  "use strict";
  var api = factory();
  if (typeof module === "object" && module.exports) {
    module.exports = api;
  } else {
    root.HUVPhysics = api;
  }
})(typeof self !== "undefined" ? self : this, function () {
  "use strict";

  var HC_EV_NM = 1239.84193; // h·c in eV·nm (CODATA)
  var EV_IN_J = 1.602176634e-19; // exact since the 2019 SI
  var MJ_PER_CM2_IN_J_PER_NM2 = 1e-17; // 1e-3 J / 1e14 nm²

  /* Wavelength presets. `na` is an example numerical aperture and `n` the index of the
     medium between lens and wafer. KrF, ArF, ArF immersion and both EUV NAs follow ASML
     product pages (NXT:870, NXT:1470, NXT:2000i, NXE and EXE); the g-line, i-line, F2
     and 6.7 nm values are illustrative round numbers, flagged with `illustrative`. */
  var PRESETS = [
    { id: "g", label: "g-line, 436 nm (Hg lamp)", wavelength: 436, na: 0.45, n: 1, illustrative: true },
    { id: "i", label: "i-line, 365 nm (Hg lamp)", wavelength: 365, na: 0.6, n: 1, illustrative: true },
    { id: "krf", label: "KrF, 248 nm", wavelength: 248, na: 0.8, n: 1 },
    { id: "arf", label: "ArF dry, 193 nm", wavelength: 193, na: 0.93, n: 1 },
    { id: "arfi", label: "ArF immersion, 193 nm (water)", wavelength: 193, na: 1.35, n: 1.437 },
    { id: "f2", label: "F₂, 157 nm", wavelength: 157, na: 0.85, n: 1, illustrative: true },
    { id: "euv", label: "EUV, 13.5 nm", wavelength: 13.5, na: 0.33, n: 1 },
    { id: "hna", label: "High-NA EUV, 13.5 nm", wavelength: 13.5, na: 0.55, n: 1 },
    { id: "beuv", label: "Beyond-EUV, 6.7 nm", wavelength: 6.7, na: 0.55, n: 1, illustrative: true }
  ];

  /* Rayleigh scaling. Lengths in the unit of `wavelength` (nm).
     dofExact applies the quarter-wave criterion to the non-paraxial defocus path
     difference n·z·(1 − cos θ), sin θ = NA/n, scaled so that it equals k2·λ/NA² in the
     paraxial limit with n = 1. 1 − cos θ is evaluated as sin²θ / (1 + cos θ) to stay
     accurate at small NA. */
  function rayleigh(p) {
    var n = p.n === undefined ? 1 : p.n;
    var halfPitch = (p.k1 * p.wavelength) / p.na;
    var dofParaxial = (p.k2 * p.wavelength) / (p.na * p.na);
    var dofExact = NaN;
    if (p.na < n) {
      var sin = p.na / n;
      var cos = Math.sqrt(1 - sin * sin);
      var oneMinusCos = (sin * sin) / (1 + cos);
      dofExact = (p.k2 * p.wavelength) / (2 * n * oneMinusCos);
    }
    return {
      halfPitch: halfPitch,
      pitch: 2 * halfPitch,
      dofParaxial: dofParaxial,
      dofExact: dofExact,
      minHalfPitch: (0.25 * p.wavelength) / p.na
    };
  }

  function photonEnergy(wavelengthNm) {
    var eV = HC_EV_NM / wavelengthNm;
    return { eV: eV, J: eV * EV_IN_J };
  }

  function featureArea(shape, sizeNm) {
    return shape === "circle" ? (Math.PI * sizeNm * sizeNm) / 4 : sizeNm * sizeNm;
  }

  /* Mean photon count in a feature and its Poisson fluctuation. dose in mJ/cm²,
     area in nm², absorbed = fraction of incident photons that is counted. */
  function shotNoise(p) {
    var absorbed = p.absorbed === undefined ? 1 : p.absorbed;
    var energy = photonEnergy(p.wavelength);
    var perNm2 = (p.dose * MJ_PER_CM2_IN_J_PER_NM2) / energy.J;
    var count = perNm2 * p.area * absorbed;
    return {
      energyEV: energy.eV,
      energyJ: energy.J,
      perNm2: perNm2,
      count: count,
      relSigma: count > 0 ? 1 / Math.sqrt(count) : Infinity
    };
  }

  /* Dose (mJ/cm²) at which the 1σ relative fluctuation equals relSigma. */
  function doseForNoise(p) {
    var absorbed = p.absorbed === undefined ? 1 : p.absorbed;
    var count = 1 / (p.relSigma * p.relSigma);
    return (count * photonEnergy(p.wavelength).J) / (p.area * absorbed * MJ_PER_CM2_IN_J_PER_NM2);
  }

  /* Fourier coefficient c_m of a bright-field line/space mask: opaque lines of width
     `cd` centred at x = 0 on a clear background, period `pitch`. */
  function maskCoefficient(m, cd, pitch) {
    if (m === 0) return 1 - cd / pitch;
    return -Math.sin((Math.PI * m * cd) / pitch) / (Math.PI * m);
  }

  /* 1-D source: `count` points spread evenly (midpoint rule) over σin ≤ |s| ≤ σout,
     i.e. one segment [−σout, σout] when σin = 0 (conventional) or two poles otherwise.
     σout = 0 gives the single on-axis point of coherent illumination. */
  function sourcePoints(sigmaIn, sigmaOut, count) {
    var lo = Math.max(0, Math.min(sigmaIn, sigmaOut));
    var hi = Math.max(0, sigmaOut);
    if (hi === 0) return [0];
    var segments = lo === 0 ? [[-hi, hi]] : [[-hi, -lo], [lo, hi]];
    var points = [];
    var per = Math.max(1, Math.round(count / segments.length));
    segments.forEach(function (seg) {
      var width = seg[1] - seg[0];
      if (width === 0) {
        points.push(seg[0]);
        return;
      }
      for (var j = 0; j < per; j++) points.push(seg[0] + ((j + 0.5) * width) / per);
    });
    return points;
  }

  /* Partially coherent 1-D aerial image by Abbe's method (scalar, thin mask).
       f_mj = m/p + s_j·NA/λ, order m passes when |f_mj| ≤ NA/λ,
       φ_mj = 2π z (sqrt((n/λ)² − f_mj²) − n/λ)      (exact defocus phase),
       I(x) = mean_j |Σ_m c_m e^{iφ_mj} e^{2πi m x/p}|²  (clear field = 1).
     Returns a model with `at(x)` for any x plus summary data. */
  function aerialModel(p) {
    var lambda = p.wavelength;
    var n = p.n === undefined ? 1 : p.n;
    var fc = p.na / lambda;
    var pitch = p.pitch;
    var z = p.defocus || 0;
    var sources = sourcePoints(p.sigmaIn || 0, p.sigmaOut || 0, p.sourceCount || 41);
    var sMax = Math.max(Math.abs(p.sigmaIn || 0), Math.abs(p.sigmaOut || 0));
    var mMax = Math.floor((1 + sMax) * fc * pitch) + 1;
    var tol = 1e-12 * fc;
    var kMedium = n / lambda;

    var fields = sources.map(function (s) {
      var shift = s * fc;
      var orders = [];
      for (var m = -mMax; m <= mMax; m++) {
        var f = m / pitch + shift;
        if (Math.abs(f) > fc + tol) continue;
        var c = maskCoefficient(m, p.cd, pitch);
        var phase = z === 0 ? 0 : 2 * Math.PI * z * (Math.sqrt(kMedium * kMedium - f * f) - kMedium);
        orders.push({ m: m, slot: m + mMax, re: c * Math.cos(phase), im: c * Math.sin(phase) });
      }
      return orders;
    });

    // e^{2πi m x/p} for every m, shared by all source points (built by recurrence).
    var tabRe = new Float64Array(2 * mMax + 1);
    var tabIm = new Float64Array(2 * mMax + 1);

    function at(x) {
      var k = (2 * Math.PI * x) / pitch;
      var c1 = Math.cos(k);
      var s1 = Math.sin(k);
      var wr = 1;
      var wi = 0;
      tabRe[mMax] = 1;
      tabIm[mMax] = 0;
      for (var m = 1; m <= mMax; m++) {
        var nr = wr * c1 - wi * s1;
        wi = wr * s1 + wi * c1;
        wr = nr;
        tabRe[mMax + m] = wr;
        tabIm[mMax + m] = wi;
        tabRe[mMax - m] = wr;
        tabIm[mMax - m] = -wi;
      }
      var total = 0;
      for (var j = 0; j < fields.length; j++) {
        var re = 0;
        var im = 0;
        var orders = fields[j];
        for (var q = 0; q < orders.length; q++) {
          var o = orders[q];
          var er = tabRe[o.slot];
          var ei = tabIm[o.slot];
          re += o.re * er - o.im * ei;
          im += o.re * ei + o.im * er;
        }
        total += re * re + im * im;
      }
      return total / fields.length;
    }

    /* Sample `samples` points over `periods` periods centred on the line at x = 0. */
    function sample(samples, periods) {
      periods = periods || 2;
      var xs = new Float64Array(samples);
      var ys = new Float64Array(samples);
      var half = (periods * pitch) / 2;
      for (var i = 0; i < samples; i++) {
        var x = -half + (2 * half * i) / (samples - 1);
        xs[i] = x;
        ys[i] = at(x);
      }
      return { x: xs, intensity: ys };
    }

    var onAxis = [];
    for (var m = -mMax; m <= mMax; m++) {
      if (Math.abs(m / pitch) <= fc + tol) onAxis.push(m);
    }

    return {
      at: at,
      sample: sample,
      sourceCount: sources.length,
      ordersOnAxis: onAxis,
      passesFirstOrder: fields.some(function (orders) {
        return orders.some(function (o) { return o.m !== 0; });
      })
    };
  }

  /* Contrast, NILS at the nominal line edge (x = cd/2), and the printed line width for a
     constant-threshold resist: the length per period where I < threshold (for a normal
     image, the width of the dark line around x = 0; 0 = the line does not print,
     = pitch = the space never opens). Crossings are refined by bisection. */
  function imageMetrics(model, p, samples) {
    var pitch = p.pitch;
    var n = samples || 801;
    var xs = new Float64Array(n);
    var ys = new Float64Array(n);
    var imax = -Infinity;
    var imin = Infinity;
    for (var i = 0; i < n; i++) {
      xs[i] = -pitch / 2 + (pitch * i) / (n - 1);
      ys[i] = model.at(xs[i]);
      if (ys[i] > imax) imax = ys[i];
      if (ys[i] < imin) imin = ys[i];
    }
    var contrast = imax + imin > 0 ? (imax - imin) / (imax + imin) : 0;

    var edge = p.cd / 2;
    var h = pitch * 1e-5;
    var iEdge = model.at(edge);
    var slope = (model.at(edge + h) - model.at(edge - h)) / (2 * h);
    var nils = iEdge > 0 ? (p.cd * Math.abs(slope)) / iEdge : NaN;

    var printedCd = null;
    var t = p.threshold;
    if (t !== undefined) {
      var dark = 0;
      var start = ys[0] < t ? xs[0] : null;
      for (var k = 1; k < n; k++) {
        var wasDark = ys[k - 1] < t;
        var isDark = ys[k] < t;
        if (wasDark === isDark) continue;
        var a = xs[k - 1];
        var b = xs[k];
        for (var it = 0; it < 50; it++) {
          var mid = 0.5 * (a + b);
          if (model.at(mid) < t === wasDark) a = mid;
          else b = mid;
        }
        var crossing = 0.5 * (a + b);
        if (isDark) {
          start = crossing;
        } else {
          dark += crossing - start;
          start = null;
        }
      }
      if (start !== null) dark += xs[n - 1] - start;
      printedCd = dark;
    }

    return {
      imax: imax,
      imin: imin,
      contrast: contrast,
      nils: nils,
      intensityAtEdge: iEdge,
      printedCd: printedCd
    };
  }

  /* Pupil geometry of a line/space grating in pupil units (NA/λ ≡ 1): the site's D5
     drawing grammar — pupil circle of radius 1, the source fill, diffraction orders
     coloured by |m| and the dashed support circle at 1 + σ. Geometry only, no intensities.
       k1     half-pitch k₁ = (p/2)·NA/λ; the grating's orders sit at m/(2k₁) (any duty cycle)
       sigma  outer radius σ of the source (0 = coherent, on-axis point)
       fill   "conventional" (0 ≤ |s| ≤ σ, default) or "dipole" (two poles σin ≤ |s| ≤ σ,
              the toy's σ_in > 0 case; "annular" is accepted as a synonym)
       poles  σin of the poles (dipole only); a pole is a point when poles = sigma
     Returns {spacing, support, source, samples, cutoffs, resolved}:
       spacing   1/(2k₁), the order spacing in pupil units
       support   1 + σ, the radius of the support circle the engine uses ((1+σ)·NA/λ)
       source    {kind, outer, inner, centre, radius} — centre/radius of each pole (dipole)
       samples   one entry per sampled source point s (on-axis; the σ extremes; for a dipole
                 the pole centres and both edges of each pole): {s, label, orders:[{m, f,
                 inside}]} for m = −N…N, f = m·spacing + s, inside = |f| ≤ 1 (1e-9 tolerance)
       cutoffs   coherent 0.5 (the ±1 orders reach the rim on axis), partial 1/(2(1+σ)) (the
                 ±1 orders leave the support circle: no source point resolves the grating),
                 dipoleFloor 0.25 (pole centre and +1 order sit on opposite rims when the
                 pole is at σ_c = 1/(4k₁)), dipoleCentre 1/(4k₁) = λ/(2pNA)
       resolved  some sampled source point passes the zeroth and a first order together */
  function pupilOrders(p) {
    var k1 = p.k1;
    var sigma = Math.max(0, p.sigma || 0);
    var kind = sigma === 0 ? "coherent" : (p.fill === "dipole" || p.fill === "annular") ? "dipole" : "conventional";
    var inner = kind === "dipole" ? Math.min(sigma, Math.max(0, p.poles === undefined ? sigma : p.poles)) : 0;
    var spacing = 1 / (2 * k1);
    var support = 1 + sigma;
    var tol = 1e-9;
    var n = Math.max(1, Math.ceil((1 + sigma) / spacing) + 1);
    var points = [];
    function add(s, label) {
      for (var i = 0; i < points.length; i++) if (Math.abs(points[i].s - s) < 1e-12) return;
      points.push({ s: s, label: label });
    }
    if (kind === "coherent") add(0, "on-axis");
    else if (kind === "conventional") { add(0, "on-axis"); add(-sigma, "edge −σ"); add(sigma, "edge +σ"); }
    else {
      var c = (inner + sigma) / 2;
      add(-c, "pole −σc"); add(c, "pole +σc");
      add(-sigma, "pole edge −σ"); add(sigma, "pole edge +σ");
      add(-inner, "pole edge −σin"); add(inner, "pole edge +σin");
    }
    points.sort(function (a, b) { return a.s - b.s; });
    var resolved = false;
    var samples = points.map(function (pt) {
      var orders = [];
      var zero = false;
      var first = false;
      for (var m = -n; m <= n; m++) {
        var f = m * spacing + pt.s;
        var inside = Math.abs(f) <= 1 + tol;
        orders.push({ m: m, f: f, inside: inside });
        if (inside && m === 0) zero = true;
        if (inside && Math.abs(m) === 1) first = true;
      }
      if (zero && first) resolved = true;
      return { s: pt.s, label: pt.label, orders: orders };
    });
    return {
      spacing: spacing,
      support: support,
      source: { kind: kind, outer: sigma, inner: inner, centre: kind === "dipole" ? (inner + sigma) / 2 : 0,
        radius: kind === "dipole" ? (sigma - inner) / 2 : sigma },
      samples: samples,
      cutoffs: { coherent: 0.5, partial: 1 / (2 * (1 + sigma)), dipoleFloor: 0.25, dipoleCentre: 1 / (4 * k1) },
      resolved: resolved
    };
  }

  return {
    HC_EV_NM: HC_EV_NM,
    EV_IN_J: EV_IN_J,
    PRESETS: PRESETS,
    rayleigh: rayleigh,
    photonEnergy: photonEnergy,
    featureArea: featureArea,
    shotNoise: shotNoise,
    doseForNoise: doseForNoise,
    maskCoefficient: maskCoefficient,
    sourcePoints: sourcePoints,
    aerialModel: aerialModel,
    imageMetrics: imageMetrics,
    pupilOrders: pupilOrders
  };
});
