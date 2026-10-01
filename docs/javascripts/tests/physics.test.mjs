// Tests for docs/javascripts/huv-physics.js. Run: node --test docs/javascripts/tests/*.test.mjs
// Reference numbers were computed independently (by hand / Python), not with the module.
import { test } from "node:test";
import assert from "node:assert/strict";
import { createRequire } from "node:module";

const require = createRequire(import.meta.url);
const P = require("../huv-physics.js");

const close = (actual, expected, rel = 1e-9, label = "") =>
  assert.ok(
    Math.abs(actual - expected) <= rel * Math.max(1, Math.abs(expected)),
    `${label} expected ${expected}, got ${actual}`
  );

test("Rayleigh half-pitch, pitch and paraxial DOF", () => {
  const r = P.rayleigh({ wavelength: 193, na: 1.35, k1: 0.27, k2: 1, n: 1.44 });
  close(r.halfPitch, (0.27 * 193) / 1.35);
  close(r.pitch, (2 * 0.27 * 193) / 1.35);
  close(r.dofParaxial, 193 / 1.35 ** 2);
  close(r.minHalfPitch, (0.25 * 193) / 1.35);
});

test("exact DOF: quarter-wave criterion without the paraxial approximation", () => {
  const euv = P.rayleigh({ wavelength: 13.5, na: 0.33, k1: 0.3, k2: 1 });
  close(euv.dofExact, 120.49468586843875, 1e-12, "EUV NA 0.33");
  close(euv.dofParaxial, 123.96694214876032, 1e-12);
  const arfi = P.rayleigh({ wavelength: 193, na: 1.35, k1: 0.3, k2: 1, n: 1.44 });
  close(arfi.dofExact, 102.77971659323931, 1e-12, "ArF immersion NA 1.35");
});

test("exact DOF reduces to the paraxial formula at small NA (and gains n in a medium)", () => {
  const dry = P.rayleigh({ wavelength: 248, na: 0.01, k1: 0.5, k2: 0.8 });
  close(dry.dofExact / dry.dofParaxial, 1, 1e-4);
  const wet = P.rayleigh({ wavelength: 193, na: 0.01, k1: 0.5, k2: 1, n: 1.44 });
  close(wet.dofExact / wet.dofParaxial, 1.44, 1e-4);
});

test("exact DOF reproduces Lin's 10 % (NA 0.6) and 20 % (NA 0.8) paraxial overestimate", () => {
  // B. J. Lin, JM3 1(1) 7-12 (2002): exact/paraxial = 2(1 − cos θ)/NA² = 0.9 and 0.8.
  for (const [na, ratio] of [[0.6, 0.9], [0.8, 0.8]]) {
    const r = P.rayleigh({ wavelength: 248, na, k1: 0.4, k2: 1 });
    close(r.dofExact / r.dofParaxial, ratio, 1e-12, `NA ${na}`);
  }
});

test("exact DOF is undefined when NA reaches the medium index", () => {
  assert.ok(Number.isNaN(P.rayleigh({ wavelength: 193, na: 1.35, k1: 0.3, k2: 1, n: 1 }).dofExact));
});

test("photon energy and photon counts (CODATA hc)", () => {
  close(P.photonEnergy(13.5).eV, 91.84014296296297, 1e-12);
  close(P.photonEnergy(13.5).J, 1.471441311184788e-17, 1e-12);
  const euv = P.shotNoise({ wavelength: 13.5, dose: 30, area: 400 });
  close(euv.perNm2, 20.388172991992686, 1e-12);
  close(euv.count, 8155.269196797074, 1e-12);
  close(euv.relSigma, 0.011073396464743734, 1e-12);
  const arf = P.shotNoise({ wavelength: 193, dose: 30, area: 400 });
  close(arf.perNm2 / euv.perNm2, 193 / 13.5, 1e-12, "photon-count ratio = wavelength ratio");
});

test("absorbed fraction scales the count; dose-for-noise inverts shotNoise", () => {
  const half = P.shotNoise({ wavelength: 13.5, dose: 30, area: 400, absorbed: 0.5 });
  close(half.count, 8155.269196797074 / 2, 1e-12);
  close(P.doseForNoise({ wavelength: 13.5, area: 400, relSigma: 0.02 }), 9.196508194904924, 1e-12);
  const dose = P.doseForNoise({ wavelength: 193, area: 250, absorbed: 0.3, relSigma: 0.015 });
  close(P.shotNoise({ wavelength: 193, dose, area: 250, absorbed: 0.3 }).relSigma, 0.015, 1e-12);
});

test("feature areas", () => {
  close(P.featureArea("square", 20), 400);
  close(P.featureArea("circle", 20), Math.PI * 100);
});

test("mask Fourier series rebuilds the binary grating", () => {
  const pitch = 100;
  const cd = 30;
  const series = (x) => {
    let t = P.maskCoefficient(0, cd, pitch);
    for (let m = 1; m <= 4000; m++) {
      t += 2 * P.maskCoefficient(m, cd, pitch) * Math.cos((2 * Math.PI * m * x) / pitch);
    }
    return t;
  };
  close(series(0), 0, 2e-3, "centre of the opaque line");
  close(series(50), 1, 2e-3, "centre of the clear space");
  close(series(15), 0.5, 2e-3, "edge: Gibbs midpoint");
});

test("source sampling: conventional, dipole and coherent", () => {
  assert.deepEqual(P.sourcePoints(0, 0, 41), [0]);
  const conv = P.sourcePoints(0, 0.5, 5);
  assert.deepEqual(conv.map((s) => +s.toFixed(12)), [-0.4, -0.2, 0, 0.2, 0.4]);
  const dipole = P.sourcePoints(0.6, 0.6, 41);
  assert.deepEqual(dipole, [-0.6, 0.6]);
  const annular = P.sourcePoints(0.5, 0.9, 40);
  assert.equal(annular.length, 40);
  assert.ok(annular.every((s) => Math.abs(s) >= 0.5 && Math.abs(s) <= 0.9));
});

test("a nearly clear mask images to the clear-field intensity", () => {
  const model = P.aerialModel({ wavelength: 193, na: 0.93, pitch: 400, cd: 1e-9, sigmaOut: 0.7 });
  for (const x of [-150, 0, 37, 199]) close(model.at(x), 1, 1e-9);
});

test("coherent three-beam image matches [c0 + 2 c1 cos(2πx/p)]²", () => {
  // λ = 193, NA = 0.93: 1/p = 1/300 is inside the pupil (NA/λ = 1/207.5), 2/p is not.
  const model = P.aerialModel({ wavelength: 193, na: 0.93, pitch: 300, cd: 150 });
  assert.deepEqual(model.ordersOnAxis, [-1, 0, 1]);
  close(model.at(0), 0.018664962201769754, 1e-12);
  close(model.at(37.5), 0.0024842092061225034, 1e-9);
  close(model.at(75), 0.25, 1e-12);
  close(model.at(150), 1.2919045069369328, 1e-12);
});

test("below λ/((1+σ)NA) no first order passes and the image is flat", () => {
  const p = { wavelength: 193, na: 0.93, pitch: 100, cd: 50, sigmaOut: 0.7, threshold: 0.3 };
  const model = P.aerialModel(p);
  assert.equal(model.passesFirstOrder, false);
  const m = P.imageMetrics(model, p);
  close(m.contrast, 0, 1e-12);
  close(m.imax, 0.25, 1e-12); // (1 − w/p)² with only the zeroth order
});

test("symmetric two-beam (dipole) imaging is independent of focus", () => {
  const base = { wavelength: 193, na: 0.93, pitch: 150, cd: 75 };
  const sigma = 193 / (2 * 150 * 0.93);
  const inFocus = P.aerialModel({ ...base, sigmaIn: sigma, sigmaOut: sigma });
  const defocused = P.aerialModel({ ...base, sigmaIn: sigma, sigmaOut: sigma, defocus: 500 });
  for (const x of [0, 20, 40, 75, 110]) close(defocused.at(x), inFocus.at(x), 1e-9, `x=${x}`);
  const conventional = { ...base, sigmaOut: 0.5 };
  const c0 = P.imageMetrics(P.aerialModel(conventional), conventional).contrast;
  const c1 = P.imageMetrics(P.aerialModel({ ...conventional, defocus: 150 }), conventional).contrast;
  assert.ok(c1 < c0, `defocus should cut contrast for conventional illumination: ${c1} vs ${c0}`);
});

test("images of symmetric masks and sources are symmetric, also out of focus", () => {
  const model = P.aerialModel({
    wavelength: 13.5, na: 0.33, pitch: 64, cd: 28, sigmaIn: 0.3, sigmaOut: 0.8, defocus: 40
  });
  for (const x of [3, 11, 17.5, 29]) close(model.at(x), model.at(-x), 1e-10, `x=${x}`);
});

test("printed CD at threshold matches the analytic coherent crossing", () => {
  const p = { wavelength: 193, na: 0.93, pitch: 300, cd: 150, threshold: 0.25 };
  const m = P.imageMetrics(P.aerialModel(p), p);
  // [c0 + 2 c1 cos(2πx/p)]² = 0.25 at x = 75 (see the three-beam test) → CD = 150.
  close(m.printedCd, 150, 1e-9);
  const all = P.imageMetrics(P.aerialModel(p), { ...p, threshold: 2 });
  close(all.printedCd, 300, 1e-12, "everything stays dark when the threshold is above Imax");
  // Coherent ringing: the amplitude A = c0 + 2 c1 cos(kx) = 0.5 − (2/π) cos(kx) crosses
  // zero, so a low threshold t prints two thin bands where |A| < √t, i.e. where
  // cos(kx) lies between (0.5 − √t)·π/2 and (0.5 + √t)·π/2.
  const k = (2 * Math.PI) / 300;
  for (const t of [0.001, 0.01]) {
    const inner = Math.acos((0.5 - Math.sqrt(t)) * (Math.PI / 2)) / k;
    const outer = Math.acos((0.5 + Math.sqrt(t)) * (Math.PI / 2)) / k;
    const ring = P.imageMetrics(P.aerialModel(p), { ...p, threshold: t });
    close(ring.printedCd, 2 * (inner - outer), 1e-9, `two thin bands at t=${t}`);
  }
  const flat = { wavelength: 193, na: 0.93, pitch: 100, cd: 50, sigmaOut: 0.7, threshold: 0.2 };
  const none = P.imageMetrics(P.aerialModel(flat), flat);
  assert.equal(none.printedCd, 0, "nothing prints when the whole image is above threshold");
});

test("NILS of the coherent three-beam image at the line edge", () => {
  const p = { wavelength: 193, na: 0.93, pitch: 300, cd: 150 };
  const m = P.imageMetrics(P.aerialModel(p), p);
  // I = A², A = c0 + 2 c1 cos(kx), dI/dx = −4 A c1 k sin(kx); at x = 75: A = 0.5, sin = 1.
  const c1 = -1 / Math.PI;
  const k = (2 * Math.PI) / 300;
  const slope = Math.abs(-4 * 0.5 * c1 * k);
  close(m.nils, (150 * slope) / 0.25, 1e-6);
});

test("presets are well formed", () => {
  const ids = new Set();
  for (const preset of P.PRESETS) {
    assert.ok(!ids.has(preset.id));
    ids.add(preset.id);
    assert.ok(preset.wavelength > 0 && preset.na > 0 && preset.na < preset.n, preset.id);
  }
});
