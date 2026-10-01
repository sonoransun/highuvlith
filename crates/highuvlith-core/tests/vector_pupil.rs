//! Validation of the vector (polarized) pupil model, `optics::vector`,
//! against closed-form results: two-beam TE/TM interference, the low-NA
//! scalar limit, norm preservation, transversality, the obliquity energy
//! statement, and Fresnel film-entrance transmission. Fixture numbers come
//! from an independent numpy reference that uses explicit (θ, φ) unit vectors
//! and a numerical boundary-condition solve instead of the closed forms.

use approx::assert_relative_eq;
use highuvlith_core::optics::vector::{
    columns_per_source_point, field_columns, radiometric_factor, FilmInterface,
    FresnelCoefficients, IlluminationPolarization, VectorSettings,
};
use highuvlith_core::types::Complex64;

type Field = [Complex64; 3];

fn c(re: f64, im: f64) -> Complex64 {
    Complex64::new(re, im)
}

fn one() -> Complex64 {
    c(1.0, 0.0)
}

/// Per-state image-side fields `(E_x, E_y, E_z)` of the order at pupil `p`.
fn fields(
    settings: &VectorSettings,
    pupil: Complex64,
    p: (f64, f64),
    s: (f64, f64),
    na: f64,
) -> Vec<Field> {
    let mut out = vec![c(0.0, 0.0); columns_per_source_point(settings)];
    field_columns(pupil, p.0, p.1, s.0, s.1, na, settings, &mut out);
    out.chunks(3).map(|v| [v[0], v[1], v[2]]).collect()
}

fn norm2(e: &Field) -> f64 {
    e.iter().map(|v| v.norm_sqr()).sum()
}

fn total_norm2(es: &[Field]) -> f64 {
    es.iter().map(norm2).sum()
}

fn settings(polarization: IlluminationPolarization) -> VectorSettings {
    VectorSettings {
        polarization,
        ..Default::default()
    }
}

/// Fringe contrast of the two-beam image formed by orders at `(±p0, 0)`,
/// from the intensity `Σ_k |E1_k e^{iKx} + E2_k e^{-iKx}|²` sampled over one
/// fringe period.
fn two_beam_contrast(settings: &VectorSettings, na: f64, p0: f64, s: (f64, f64)) -> f64 {
    let (imin, imax) = two_beam_extrema(settings, na, p0, s);
    (imax - imin) / (imax + imin)
}

fn two_beam_intensity(settings: &VectorSettings, na: f64, p0: f64, s: (f64, f64)) -> Vec<f64> {
    let e1 = fields(settings, one(), (p0, 0.0), s, na);
    let e2 = fields(settings, one(), (-p0, 0.0), s, na);
    (0..512)
        .map(|m| {
            let half_phase = std::f64::consts::PI * m as f64 / 512.0; // Kx over one period
            let (a, b) = (
                Complex64::from_polar(1.0, half_phase),
                Complex64::from_polar(1.0, -half_phase),
            );
            e1.iter()
                .zip(&e2)
                .map(|(u, v)| {
                    (0..3)
                        .map(|i| (u[i] * a + v[i] * b).norm_sqr())
                        .sum::<f64>()
                })
                .sum()
        })
        .collect()
}

fn two_beam_extrema(settings: &VectorSettings, na: f64, p0: f64, s: (f64, f64)) -> (f64, f64) {
    let i = two_beam_intensity(settings, na, p0, s);
    let imin = i.iter().cloned().fold(f64::INFINITY, f64::min);
    let imax = i.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    (imin, imax)
}

/// Deterministic sample of pupil points with `|p| ≤ 1` (rings × azimuths).
fn pupil_samples() -> Vec<(f64, f64)> {
    let mut pts = vec![(0.0, 0.0)];
    for ring in 1..=8 {
        let r = ring as f64 / 8.0;
        for k in 0..12 {
            let phi = 0.37 + k as f64 * std::f64::consts::TAU / 12.0;
            pts.push((r * phi.cos(), r * phi.sin()));
        }
    }
    pts
}

fn all_polarizations() -> Vec<IlluminationPolarization> {
    use IlluminationPolarization as P;
    vec![
        P::Unpolarized,
        P::X,
        P::Y,
        P::Te,
        P::Tm,
        P::Linear { angle_deg: 30.0 },
        P::Linear { angle_deg: -110.0 },
    ]
}

// ---------------------------------------------------------------------------
// Two-beam interference
// ---------------------------------------------------------------------------

#[test]
fn two_beam_te_contrast_is_unity() {
    // Orders along x: TE (s) = y-polarized. The `Te` illumination at a source
    // point on the x axis is exactly y-polarized.
    for &na in &[0.3, 0.8, 0.95] {
        for pol in [IlluminationPolarization::Y, IlluminationPolarization::Te] {
            let s = settings(pol);
            assert_relative_eq!(
                two_beam_contrast(&s, na, 1.0, (0.5, 0.0)),
                1.0,
                epsilon = 1e-9
            );
        }
    }
}

#[test]
fn two_beam_tm_contrast_is_abs_cos_two_theta() {
    // Orders at the pupil edge in air: sin θ = NA, contrast = |cos 2θ| = |1 − 2 NA²|
    // (0.82, 0.28, 0.805 at NA 0.3, 0.8, 0.95).
    for &(na, expected) in &[(0.3, 0.82), (0.8, 0.28), (0.95, 0.805)] {
        for pol in [IlluminationPolarization::X, IlluminationPolarization::Tm] {
            let s = settings(pol);
            let got = two_beam_contrast(&s, na, 1.0, (0.5, 0.0));
            assert_relative_eq!(got, expected, epsilon = 1e-9);
            assert_relative_eq!(got, (1.0 - 2.0 * na * na).abs(), epsilon = 1e-9);
        }
    }
    // Interior orders: sin θ = NA·p0.
    let s = settings(IlluminationPolarization::X);
    let (na, p0) = (0.9, 0.6);
    let st = na * p0;
    assert_relative_eq!(
        two_beam_contrast(&s, na, p0, (0.0, 0.0)),
        (1.0 - 2.0 * st * st).abs(),
        epsilon = 1e-9
    );
}

#[test]
fn two_beam_unpolarized_is_mean_of_te_and_tm() {
    for &na in &[0.3, 0.8, 0.95] {
        let te = two_beam_intensity(&settings(IlluminationPolarization::Y), na, 1.0, (0.5, 0.0));
        let tm = two_beam_intensity(&settings(IlluminationPolarization::X), na, 1.0, (0.5, 0.0));
        let un = two_beam_intensity(
            &settings(IlluminationPolarization::Unpolarized),
            na,
            1.0,
            (0.5, 0.0),
        );
        for ((u, a), b) in un.iter().zip(&te).zip(&tm) {
            assert_relative_eq!(*u, 0.5 * (a + b), epsilon = 1e-12);
        }
        // Contrast of the mean = (1 + cos 2θ)/2 = cos²θ = 1 − NA² (0.91, 0.36, 0.0975).
        let got = two_beam_contrast(
            &settings(IlluminationPolarization::Unpolarized),
            na,
            1.0,
            (0.5, 0.0),
        );
        assert_relative_eq!(got, 1.0 - na * na, epsilon = 1e-9);
    }
}

#[test]
fn two_beam_contrast_is_independent_of_obliquity() {
    // Symmetric orders share the same radiometric factor, so it cancels in
    // the contrast (but scales the intensity by A²).
    let na = 0.9;
    let on = settings(IlluminationPolarization::X);
    let off = VectorSettings {
        obliquity: false,
        ..on.clone()
    };
    assert_relative_eq!(
        two_beam_contrast(&on, na, 1.0, (0.0, 0.0)),
        two_beam_contrast(&off, na, 1.0, (0.0, 0.0)),
        epsilon = 1e-12
    );
    let (_, max_on) = two_beam_extrema(&on, na, 1.0, (0.0, 0.0));
    let (_, max_off) = two_beam_extrema(&off, na, 1.0, (0.0, 0.0));
    let a = radiometric_factor(na, 1.0, 4.0);
    assert_relative_eq!(max_on / max_off, a * a, epsilon = 1e-12);
}

#[test]
fn two_beam_tm_contrast_inside_film_uses_refracted_angle() {
    // Real film n₂ = 1.7 at NA 0.9: sin θ₂ = 0.9/1.7, contrast = |cos 2θ₂|
    // = 0.439446366782007 (numpy reference). TE stays at 1.
    let na = 0.9;
    let film = Some(FilmInterface { n: 1.7, k: 0.0 });
    let tm = VectorSettings {
        film,
        ..settings(IlluminationPolarization::X)
    };
    let got = two_beam_contrast(&tm, na, 1.0, (0.0, 0.0));
    assert_relative_eq!(got, 0.439_446_366_782_007, epsilon = 1e-9);
    let s2 = na / 1.7;
    assert_relative_eq!(got, (1.0 - 2.0 * s2 * s2).abs(), epsilon = 1e-9);
    let te = VectorSettings {
        film,
        ..settings(IlluminationPolarization::Y)
    };
    assert_relative_eq!(
        two_beam_contrast(&te, na, 1.0, (0.0, 0.0)),
        1.0,
        epsilon = 1e-9
    );
}

#[test]
fn two_beam_tm_contrast_in_immersion_medium() {
    // Water immersion n = 1.44, NA 1.35: sin θ = 0.9375 inside the fluid.
    let s = VectorSettings {
        image_index: 1.44,
        ..settings(IlluminationPolarization::X)
    };
    assert!(s.validate(1.35).is_ok());
    let st: f64 = 1.35 / 1.44;
    assert_relative_eq!(
        two_beam_contrast(&s, 1.35, 1.0, (0.0, 0.0)),
        (1.0 - 2.0 * st * st).abs(),
        epsilon = 1e-9
    );
}

// ---------------------------------------------------------------------------
// Limits, norms, symmetries
// ---------------------------------------------------------------------------

/// Largest deviation of the vector field from the scalar field `P·J`
/// (transverse part) and largest `|E_z|` over the pupil samples.
fn scalar_deviation(na: f64) -> (f64, f64, f64) {
    let pupil = Complex64::from_polar(0.8, 0.4);
    let mut max_perp: f64 = 0.0;
    let mut max_z: f64 = 0.0;
    let mut max_int: f64 = 0.0;
    for pol in [
        IlluminationPolarization::X,
        IlluminationPolarization::Y,
        IlluminationPolarization::Linear { angle_deg: 30.0 },
    ] {
        let a = match pol {
            IlluminationPolarization::Linear { angle_deg } => angle_deg.to_radians(),
            IlluminationPolarization::Y => std::f64::consts::FRAC_PI_2,
            _ => 0.0,
        };
        let j = [a.cos(), a.sin()];
        let s = settings(pol);
        for &p in &pupil_samples() {
            let e = fields(&s, pupil, p, (0.2, 0.1), na)[0];
            let dperp =
                ((e[0] - pupil * j[0]).norm_sqr() + (e[1] - pupil * j[1]).norm_sqr()).sqrt();
            max_perp = max_perp.max(dperp / pupil.norm());
            max_z = max_z.max(e[2].norm() / pupil.norm());
            max_int = max_int.max((norm2(&e) / pupil.norm_sqr() - 1.0).abs());
        }
    }
    (max_perp, max_z, max_int)
}

#[test]
fn low_na_limit_approaches_scalar_with_quadratic_error() {
    // Transverse error ≤ sin²θ/(1+cos θ) + (A − 1) < NA² for NA ≤ 0.3;
    // E_z is first order in NA; intensity error is second order.
    for &na in &[0.01, 0.03, 0.1, 0.3] {
        let (perp, z, int) = scalar_deviation(na);
        assert!(
            perp <= na * na,
            "NA {na}: transverse deviation {perp} > NA²"
        );
        assert!(z <= 1.05 * na, "NA {na}: |E_z| {z} > 1.05·NA");
        assert!(int <= na * na, "NA {na}: intensity deviation {int} > NA²");
    }
    // Scaling: ten times smaller NA → a hundred times smaller deviation.
    let (p1, _, i1) = scalar_deviation(0.1);
    let (p2, _, i2) = scalar_deviation(0.01);
    assert_relative_eq!(p1 / p2, 100.0, max_relative = 0.02);
    assert_relative_eq!(i1 / i2, 100.0, max_relative = 0.02);
}

#[test]
fn norm_is_preserved_without_obliquity_and_film() {
    for pol in all_polarizations() {
        let s = VectorSettings {
            obliquity: false,
            ..settings(pol)
        };
        for &na in &[0.5, 0.93] {
            for &p in &pupil_samples() {
                for &src in &[(0.0, 0.0), (0.6, -0.2), (-0.1, 0.9)] {
                    let pupil = Complex64::from_polar(0.7, -1.1);
                    let es = fields(&s, pupil, p, src, na);
                    assert_relative_eq!(total_norm2(&es), pupil.norm_sqr(), epsilon = 1e-12);
                }
            }
        }
    }
}

#[test]
fn field_is_transverse_to_its_order() {
    // E·k̂ = 0 in the image medium; inside a film E·k̂₂ = 0 with the complex
    // refracted direction k̂₂ = (sin θ₂ û, cos θ₂) (bilinear dot product).
    let na = 0.9;
    let film = FilmInterface { n: 1.68, k: 0.05 };
    for pol in all_polarizations() {
        for &with_film in &[false, true] {
            let s = VectorSettings {
                film: with_film.then_some(film),
                ..settings(pol)
            };
            for &p in &pupil_samples()[1..] {
                let (alpha, beta) = (na * p.0, na * p.1);
                let st = alpha.hypot(beta);
                let (u, gamma) = ((alpha / st, beta / st), (1.0 - st * st).sqrt());
                let k: [Complex64; 3] = if with_film {
                    let f = FresnelCoefficients::new(1.0, film.index(), st);
                    [f.sin_theta2 * u.0, f.sin_theta2 * u.1, f.cos_theta2]
                } else {
                    [c(alpha, 0.0), c(beta, 0.0), c(gamma, 0.0)]
                };
                for e in fields(&s, one(), p, (0.3, 0.4), na) {
                    let dot = e[0] * k[0] + e[1] * k[1] + e[2] * k[2];
                    assert!(
                        dot.norm() < 1e-12,
                        "E·k = {dot} for {pol:?}, film={with_film}"
                    );
                }
            }
        }
    }
}

#[test]
fn s_and_p_channels_do_not_mix() {
    // For an order at azimuth φ, J = ê_s (TE of that order) maps to ê_s and
    // J = ê_p,in (TM) maps to ê_p,out = (cos θ cos φ, cos θ sin φ, −sin θ).
    let na = 0.85;
    let s = VectorSettings {
        obliquity: false,
        ..Default::default()
    };
    for &p in &pupil_samples()[1..] {
        let phi = p.1.atan2(p.0);
        let st = na * p.0.hypot(p.1);
        let ct = (1.0 - st * st).sqrt();
        let te = VectorSettings {
            polarization: IlluminationPolarization::Linear {
                angle_deg: phi.to_degrees() + 90.0,
            },
            ..s.clone()
        };
        let tm = VectorSettings {
            polarization: IlluminationPolarization::Linear {
                angle_deg: phi.to_degrees(),
            },
            ..s.clone()
        };
        let e = fields(&te, one(), p, (0.0, 0.0), na)[0];
        let want = [-phi.sin(), phi.cos(), 0.0];
        for i in 0..3 {
            assert_relative_eq!(e[i].re, want[i], epsilon = 1e-12);
        }
        let e = fields(&tm, one(), p, (0.0, 0.0), na)[0];
        let want = [ct * phi.cos(), ct * phi.sin(), -st];
        for i in 0..3 {
            assert_relative_eq!(e[i].re, want[i], epsilon = 1e-12);
        }
    }
}

#[test]
fn field_is_continuous_at_pupil_center() {
    let na = 0.9;
    let film = Some(FilmInterface { n: 1.7, k: 0.03 });
    for pol in all_polarizations() {
        for f in [None, film] {
            let s = VectorSettings {
                film: f,
                ..settings(pol)
            };
            let e0 = fields(&s, one(), (0.0, 0.0), (0.4, -0.3), na);
            // On axis the lens transfer is the identity (times t at a film).
            let t = f.map_or(one(), |f| 2.0 / (1.0 + f.index()));
            let j = fields(
                &VectorSettings {
                    obliquity: false,
                    film: None,
                    ..s.clone()
                },
                one(),
                (0.0, 0.0),
                (0.4, -0.3),
                na,
            );
            for (a, b) in e0.iter().zip(&j) {
                for i in 0..3 {
                    assert!((a[i] - t * b[i]).norm() < 1e-15);
                }
                assert_eq!(a[2], c(0.0, 0.0));
            }
            for &eps in &[1e-3, 1e-6, 1e-9] {
                for k in 0..8 {
                    let phi = k as f64 * std::f64::consts::FRAC_PI_4 + 0.1;
                    let es = fields(
                        &s,
                        one(),
                        (eps * phi.cos(), eps * phi.sin()),
                        (0.4, -0.3),
                        na,
                    );
                    for (a, b) in es.iter().zip(&e0) {
                        for i in 0..3 {
                            assert!((a[i] - b[i]).norm() <= 2.0 * eps, "jump at |p| = {eps}");
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn inversion_symmetry_flips_only_the_longitudinal_component() {
    // E(−p) = diag(1, 1, −1)·E(p), with and without a film.
    let na = 0.92;
    for f in [None, Some(FilmInterface { n: 1.7, k: 0.04 })] {
        for pol in all_polarizations() {
            let s = VectorSettings {
                film: f,
                ..settings(pol)
            };
            for &p in &pupil_samples() {
                let a = fields(&s, one(), p, (0.3, 0.2), na);
                let b = fields(&s, one(), (-p.0, -p.1), (0.3, 0.2), na);
                for (u, v) in a.iter().zip(&b) {
                    assert!((u[0] - v[0]).norm() < 1e-12);
                    assert!((u[1] - v[1]).norm() < 1e-12);
                    assert!((u[2] + v[2]).norm() < 1e-12);
                }
            }
        }
    }
}

/// 3×3 cross-coherency `Σ_k E_k(p1) E_k(p2)ᴴ`.
fn cross_coherency(a: &[Field], b: &[Field]) -> [[Complex64; 3]; 3] {
    let mut w = [[c(0.0, 0.0); 3]; 3];
    for (u, v) in a.iter().zip(b) {
        for i in 0..3 {
            for j in 0..3 {
                w[i][j] += u[i] * v[j].conj();
            }
        }
    }
    w
}

#[test]
fn unpolarized_cross_coherency_is_basis_independent() {
    // Unpolarized light has coherency ½·I in any orthonormal basis, so the
    // TCC contribution Σ_k E_k(p1) E_k(p2)ᴴ must equal the average over any
    // orthogonal pair: (X, Y), (30°, 120°), or (TE, TM) at an off-axis source.
    let na = 0.9;
    let film = Some(FilmInterface { n: 1.7, k: 0.02 });
    let (p1, p2, src) = ((0.7, -0.2), (-0.35, 0.6), (0.45, 0.3));
    for f in [None, film] {
        let with = |pol| VectorSettings {
            film: f,
            ..settings(pol)
        };
        let un = with(IlluminationPolarization::Unpolarized);
        let w_un = cross_coherency(
            &fields(&un, one(), p1, src, na),
            &fields(&un, one(), p2, src, na),
        );
        use IlluminationPolarization as P;
        for (a, b) in [
            (P::X, P::Y),
            (
                P::Linear { angle_deg: 30.0 },
                P::Linear { angle_deg: 120.0 },
            ),
            (P::Te, P::Tm),
        ] {
            let (sa, sb) = (with(a), with(b));
            let wa = cross_coherency(
                &fields(&sa, one(), p1, src, na),
                &fields(&sa, one(), p2, src, na),
            );
            let wb = cross_coherency(
                &fields(&sb, one(), p1, src, na),
                &fields(&sb, one(), p2, src, na),
            );
            for i in 0..3 {
                for j in 0..3 {
                    assert!((w_un[i][j] - 0.5 * (wa[i][j] + wb[i][j])).norm() < 1e-12);
                }
            }
        }
    }
}

#[test]
fn te_tm_illumination_bases_and_on_axis_fallback() {
    let na = 0.9;
    let flat = |pol| VectorSettings {
        obliquity: false,
        ..settings(pol)
    };
    let (te, tm) = (
        flat(IlluminationPolarization::Te),
        flat(IlluminationPolarization::Tm),
    );
    // At p = 0 the field is the Jones vector itself.
    let jones = |s: &VectorSettings, src| fields(s, one(), (0.0, 0.0), src, na);
    for &(sx, sy) in &[
        (0.5, 0.0),
        (0.0, -0.7),
        (0.3, 0.4),
        (-0.6, 0.25),
        (1e-6, 0.0),
    ] {
        let r = f64::hypot(sx, sy);
        let (a, b) = (jones(&te, (sx, sy)), jones(&tm, (sx, sy)));
        // Azimuthal / radial, unit amplitude, mutually orthogonal.
        assert_relative_eq!(a[0][0].re, -sy / r, epsilon = 1e-12);
        assert_relative_eq!(a[0][1].re, sx / r, epsilon = 1e-12);
        assert_relative_eq!(b[0][0].re, sx / r, epsilon = 1e-12);
        assert_relative_eq!(b[0][1].re, sy / r, epsilon = 1e-12);
        let dot = a[0][0] * b[0][0] + a[0][1] * b[0][1];
        assert!(dot.norm() < 1e-12);
        // Off axis the second (fallback) state is identically zero.
        assert_eq!(a[1], [c(0.0, 0.0); 3]);
        assert_eq!(b[1], [c(0.0, 0.0); 3]);
    }
    // Rotating the source point by 90° rotates the TE vector by 90°.
    let a = jones(&te, (0.3, 0.4));
    let b = jones(&te, (-0.4, 0.3));
    assert_relative_eq!(b[0][0].re, -a[0][1].re, epsilon = 1e-12);
    assert_relative_eq!(b[0][1].re, a[0][0].re, epsilon = 1e-12);
    // On-axis source point: TE and TM both fall back to the unpolarized pair,
    // at every pupil point, with and without obliquity/film.
    for f in [None, Some(FilmInterface { n: 1.7, k: 0.03 })] {
        for &p in &pupil_samples() {
            let un = VectorSettings {
                film: f,
                ..settings(IlluminationPolarization::Unpolarized)
            };
            let want = fields(&un, one(), p, (0.0, 0.0), na);
            for pol in [IlluminationPolarization::Te, IlluminationPolarization::Tm] {
                let s = VectorSettings {
                    film: f,
                    ..settings(pol)
                };
                assert_eq!(fields(&s, one(), p, (0.0, 0.0), na), want);
            }
        }
    }
}

#[test]
fn orders_outside_pupil_or_evanescent_are_zero() {
    for pol in all_polarizations() {
        let s = settings(pol);
        let zero = vec![[c(0.0, 0.0); 3]; columns_per_source_point(&s) / 3];
        assert_eq!(fields(&s, one(), (1.0001, 0.0), (0.1, 0.0), 0.9), zero);
        assert_eq!(fields(&s, one(), (0.8, 0.7), (0.1, 0.0), 0.9), zero);
        assert_eq!(fields(&s, one(), (f64::NAN, 0.0), (0.1, 0.0), 0.9), zero);
        assert_eq!(fields(&s, c(0.0, 0.0), (0.2, 0.0), (0.1, 0.0), 0.9), zero);
        // NA 1.2 in air (rejected by `validate`): orders with NA·|p| ≥ 1 are
        // evanescent in the image medium and must vanish, the rest propagate.
        assert_eq!(fields(&s, one(), (0.9, 0.0), (0.1, 0.0), 1.2), zero);
        assert!(total_norm2(&fields(&s, one(), (0.8, 0.0), (0.1, 0.0), 1.2)) > 0.1);
    }
    assert!(settings(IlluminationPolarization::X).validate(1.2).is_err());
}

// ---------------------------------------------------------------------------
// Obliquity (radiometric) energy statement
// ---------------------------------------------------------------------------

#[test]
fn obliquity_conserves_power_per_pupil_area() {
    // Derived statement (module docs): |E(p)|² cos θ_img = |P|²|J|² cos θ_obj,
    // i.e. the z-directed power per pupil area is the same on both sides of
    // the lens (up to a pupil-independent constant, fixed to 1 on axis).
    let pupil = Complex64::from_polar(0.9, 0.3);
    for &(na, n_img, red) in &[
        (0.5, 1.0, 4.0),
        (0.9, 1.0, 4.0),
        (0.95, 1.0, 5.0),
        (1.35, 1.44, 4.0),
        (0.9, 1.0, f64::INFINITY),
    ] {
        for pol in all_polarizations() {
            let s = VectorSettings {
                image_index: n_img,
                reduction: red,
                ..settings(pol)
            };
            for &p in &pupil_samples() {
                let pna = na * p.0.hypot(p.1);
                let cos_img = (1.0 - (pna / n_img).powi(2)).sqrt();
                let cos_obj = (1.0 - (pna / red).powi(2)).sqrt();
                let es = fields(&s, pupil, p, (0.35, -0.5), na);
                assert_relative_eq!(
                    total_norm2(&es) * cos_img,
                    pupil.norm_sqr() * cos_obj,
                    max_relative = 1e-12
                );
            }
        }
    }
}

#[test]
fn obliquity_only_rescales_the_field() {
    let na = 0.93;
    for pol in all_polarizations() {
        let on = settings(pol);
        let off = VectorSettings {
            obliquity: false,
            ..on.clone()
        };
        for &p in &pupil_samples() {
            let a = radiometric_factor(na * p.0.hypot(p.1), 1.0, 4.0);
            let (u, v) = (
                fields(&on, one(), p, (0.2, 0.2), na),
                fields(&off, one(), p, (0.2, 0.2), na),
            );
            for (x, y) in u.iter().zip(&v) {
                for i in 0..3 {
                    assert!((x[i] - a * y[i]).norm() < 1e-12);
                }
            }
        }
    }
    // Pupil-edge fixture at NA 0.9, R = 4 (numpy): A = 1.4951027749973473.
    assert_relative_eq!(
        radiometric_factor(0.9, 1.0, 4.0),
        1.495_102_774_997_347_3,
        epsilon = 1e-12
    );
}

// ---------------------------------------------------------------------------
// Film entrance (Fresnel)
// ---------------------------------------------------------------------------

#[test]
fn film_normal_incidence_transmission() {
    // t = 2 n₁ / (n₁ + N) for both polarizations at p = 0.
    for &(n_img, n, k) in &[(1.0, 1.7, 0.0), (1.0, 1.7, 0.03), (1.44, 1.7, 0.02)] {
        let film = FilmInterface { n, k };
        let t = 2.0 * n_img / (n_img + film.index());
        for pol in all_polarizations() {
            let s = VectorSettings {
                image_index: n_img,
                film: Some(film),
                ..settings(pol)
            };
            let plain = VectorSettings {
                film: None,
                ..s.clone()
            };
            let (a, b) = (
                fields(&s, one(), (0.0, 0.0), (0.2, 0.5), 0.9),
                fields(&plain, one(), (0.0, 0.0), (0.2, 0.5), 0.9),
            );
            for (x, y) in a.iter().zip(&b) {
                for i in 0..3 {
                    assert!((x[i] - t * y[i]).norm() < 1e-14);
                }
            }
        }
    }
    // Numeric anchor: n = 1.7 → t = 2/2.7 = 0.740740…
    let f = FresnelCoefficients::new(1.0, c(1.7, 0.0), 0.0);
    assert_relative_eq!(f.t_s.re, 0.740_740_740_740_740_7, epsilon = 1e-15);
}

#[test]
fn film_brewster_angle_p_wave() {
    // Real film n₂ = 1.7 from air: tan θ_B = 1.7 → r_p = 0, T_p = 1, and the
    // in-film p field is (n₁/n₂)(cos θ₂, 0, −sin θ₂) with θ₂ = 90° − θ_B.
    let n2: f64 = 1.7;
    let theta_b = n2.atan();
    let na = 0.9;
    let p0 = theta_b.sin() / na;
    let s = VectorSettings {
        obliquity: false,
        film: Some(FilmInterface { n: n2, k: 0.0 }),
        ..settings(IlluminationPolarization::X)
    };
    let e = fields(&s, one(), (p0, 0.0), (0.0, 0.0), na)[0];
    assert_relative_eq!(norm2(&e), (1.0 / n2).powi(2), epsilon = 1e-12);
    assert_relative_eq!(e[0].re, theta_b.sin() / n2, epsilon = 1e-12);
    assert_relative_eq!(e[2].re, -theta_b.cos() / n2, epsilon = 1e-12);
    assert_relative_eq!(e[1].norm(), 0.0, epsilon = 1e-15);
    let f = FresnelCoefficients::new(1.0, c(n2, 0.0), theta_b.sin());
    assert!(f.r_p.norm() < 1e-12);
    assert_relative_eq!(f.transmittance_p(), 1.0, epsilon = 1e-12);
    // The s wave is still partially reflected at Brewster incidence.
    assert!(f.reflectance_s() > 0.1);
}

#[test]
fn film_transmitted_power_matches_textbook_fresnel() {
    // Lossless film, pure s or pure p incidence (relative to the order's own
    // plane of incidence). Power balance per pupil area:
    //   n₂ cos θ₂ |E_film|² = (1 − |r|²) · n₁ cos θ₁ |E_inc|²,
    // with r from the textbook Fresnel reflection formulas (computed here,
    // not taken from the implementation).
    let na = 0.9;
    for &(n1, n2) in &[(1.0, 1.7), (1.0, 1.45), (1.44, 1.7), (1.0, 2.4)] {
        for &p in &pupil_samples()[1..] {
            let phi = p.1.atan2(p.0);
            let s1 = na * p.0.hypot(p.1) / n1;
            let c1 = (1.0 - s1 * s1).sqrt();
            let s2 = n1 * s1 / n2;
            let c2 = (1.0 - s2 * s2).sqrt();
            let r_s = (n1 * c1 - n2 * c2) / (n1 * c1 + n2 * c2);
            let r_p = (n2 * c1 - n1 * c2) / (n2 * c1 + n1 * c2);
            for (angle, r) in [(phi.to_degrees() + 90.0, r_s), (phi.to_degrees(), r_p)] {
                let base = VectorSettings {
                    image_index: n1,
                    ..settings(IlluminationPolarization::Linear { angle_deg: angle })
                };
                let film = VectorSettings {
                    film: Some(FilmInterface { n: n2, k: 0.0 }),
                    ..base.clone()
                };
                let e_inc = fields(&base, one(), p, (0.0, 0.0), na)[0];
                let e_film = fields(&film, one(), p, (0.0, 0.0), na)[0];
                assert_relative_eq!(
                    n2 * c2 * norm2(&e_film),
                    (1.0 - r * r) * n1 * c1 * norm2(&e_inc),
                    max_relative = 1e-12
                );
            }
        }
    }
}

#[test]
fn absorbing_film_energy_balance() {
    // R + T = 1 at the interface for s and p, T from the Poynting flux just
    // inside the film (Re(N cos θ₂) for s, Re(N* cos θ₂) for p).
    for &(n, k) in &[(1.7, 0.03), (1.7, 0.4), (0.9, 0.2), (2.5, 1.2)] {
        for &s1 in &[0.0, 0.3, 0.6, 0.9, 0.97] {
            let f = FresnelCoefficients::new(1.0, c(n, k), s1);
            assert_relative_eq!(
                f.reflectance_s() + f.transmittance_s(),
                1.0,
                epsilon = 1e-12
            );
            assert_relative_eq!(
                f.reflectance_p() + f.transmittance_p(),
                1.0,
                epsilon = 1e-12
            );
        }
    }
}

#[test]
fn index_matched_film_is_transparent() {
    let na = 0.9;
    for pol in all_polarizations() {
        let plain = settings(pol);
        let matched = VectorSettings {
            film: Some(FilmInterface { n: 1.0, k: 0.0 }),
            ..plain.clone()
        };
        for &p in &pupil_samples() {
            let (a, b) = (
                fields(&plain, one(), p, (0.1, 0.6), na),
                fields(&matched, one(), p, (0.1, 0.6), na),
            );
            for (x, y) in a.iter().zip(&b) {
                for i in 0..3 {
                    assert!((x[i] - y[i]).norm() < 1e-12);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Fixtures from the independent numpy reference
// ---------------------------------------------------------------------------

#[allow(clippy::type_complexity)]
#[test]
fn fixtures_match_independent_reference() {
    use IlluminationPolarization as P;
    let e = |re: f64, im: f64| c(re, im);
    let cases: Vec<(
        VectorSettings,
        Complex64,
        (f64, f64),
        (f64, f64),
        f64,
        Vec<Field>,
    )> = vec![
        (
            settings(P::X),
            one(),
            (0.6, -0.3),
            (0.2, 0.7),
            0.9,
            vec![[
                e(9.328921486603449e-01, 0.0),
                e(9.034089581869399e-02, 0.0),
                e(-6.013299277607759e-01, 0.0),
            ]],
        ),
        (
            settings(P::Unpolarized),
            Complex64::from_polar(0.8, 0.3),
            (-0.45, 0.8),
            (0.0, 0.0),
            0.85,
            vec![
                [
                    e(6.157911195769706e-01, 1.904865154728511e-01),
                    e(1.08277570304233e-01, 3.349417751475375e-02),
                    e(2.588366992239614e-01, 8.006757379927427e-02),
                ],
                [
                    e(1.08277570304233e-01, 3.349417751475373e-02),
                    e(4.842037945544652e-01, 1.497817858542267e-01),
                    e(-4.601541319537092e-01, -1.423423534209321e-01),
                ],
            ],
        ),
        (
            VectorSettings {
                image_index: 1.44,
                ..settings(P::Te)
            },
            one(),
            (0.3, 0.55),
            (-0.4, 0.3),
            1.2,
            vec![
                [
                    e(-5.690628324944679e-01, 0.0),
                    e(-7.213465400354758e-01, 0.0),
                    e(5.544441614260653e-01, 0.0),
                ],
                [e(0.0, 0.0); 3],
            ],
        ),
        (
            VectorSettings {
                image_index: 1.44,
                ..settings(P::Tm)
            },
            one(),
            (0.3, 0.55),
            (-0.4, 0.3),
            1.2,
            vec![
                [
                    e(-8.693533831634022e-01, 0.0),
                    e(6.239621099046906e-01, 0.0),
                    e(-8.048382988442886e-02, 0.0),
                ],
                [e(0.0, 0.0); 3],
            ],
        ),
        (
            VectorSettings {
                film: Some(FilmInterface { n: 1.7, k: 0.04 }),
                ..settings(P::Linear { angle_deg: 30.0 })
            },
            one(),
            (0.7, 0.2),
            (0.1, 0.0),
            0.9,
            vec![[
                e(6.237139410013606e-01, -6.687220394107307e-03),
                e(3.657581223064352e-01, -5.408543844203539e-03),
                e(-2.921324761421464e-01, 1.137811992289116e-02),
            ]],
        ),
        (
            VectorSettings {
                obliquity: false,
                reduction: 5.0,
                film: Some(FilmInterface { n: 1.6, k: 0.1 }),
                ..settings(P::Te)
            },
            one(),
            (-0.2, -0.9),
            (0.0, 0.0),
            0.93,
            vec![
                [
                    e(3.873393585448766e-01, -2.376876960218281e-02),
                    e(-4.156114068426947e-03, 3.762886614610719e-03),
                    e(5.023519201588503e-02, -5.335137069972898e-03),
                ],
                [
                    e(-4.156114068426961e-03, 3.762886614610721e-03),
                    e(3.695604261410502e-01, -7.67197686190361e-03),
                    e(2.260583640714827e-01, -2.400811681487805e-02),
                ],
            ],
        ),
    ];
    for (s, pupil, p, src, na, want) in cases {
        let got = fields(&s, pupil, p, src, na);
        assert_eq!(got.len(), want.len());
        for (g, w) in got.iter().zip(&want) {
            for i in 0..3 {
                assert!(
                    (g[i] - w[i]).norm() < 1e-12,
                    "{s:?} p={p:?}: got {g:?}, want {w:?}"
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Serialization
// ---------------------------------------------------------------------------

#[test]
fn settings_round_trip_through_toml() {
    use IlluminationPolarization as P;
    for pol in [
        P::Unpolarized,
        P::X,
        P::Y,
        P::Te,
        P::Tm,
        P::Linear { angle_deg: 45.0 },
    ] {
        for film in [None, Some(FilmInterface { n: 1.7, k: 0.02 })] {
            let s = VectorSettings {
                polarization: pol,
                image_index: 1.44,
                obliquity: false,
                reduction: 5.0,
                film,
            };
            let text = toml::to_string(&s).expect("serialize");
            let back: VectorSettings = toml::from_str(&text).expect("deserialize");
            assert_eq!(back, s, "round trip of\n{text}");
        }
    }
}

#[test]
fn settings_parse_from_hand_written_toml() {
    let s: VectorSettings = toml::from_str(
        r#"
        image_index = 1.44
        obliquity = false
        polarization = { type = "linear", angle_deg = 45.0 }
        film = { n = 1.7, k = 0.02 }
        "#,
    )
    .unwrap();
    assert_eq!(
        s.polarization,
        IlluminationPolarization::Linear { angle_deg: 45.0 }
    );
    assert_eq!(s.film, Some(FilmInterface { n: 1.7, k: 0.02 }));
    assert_eq!(s.reduction, 4.0); // defaulted
    assert!(!s.obliquity);

    // Minimal tables: every omitted field takes its default; k defaults to 0.
    let s: VectorSettings =
        toml::from_str("polarization = { type = \"te\" }\nfilm = { n = 1.6 }").unwrap();
    assert_eq!(
        s,
        VectorSettings {
            polarization: IlluminationPolarization::Te,
            film: Some(FilmInterface { n: 1.6, k: 0.0 }),
            ..Default::default()
        }
    );
    let s: VectorSettings = toml::from_str("").unwrap();
    assert_eq!(s, VectorSettings::default());
    let s: VectorSettings = toml::from_str("[polarization]\ntype = \"tm\"\n").unwrap();
    assert_eq!(s.polarization, IlluminationPolarization::Tm);
    // Unknown polarization tags are rejected.
    assert!(toml::from_str::<VectorSettings>("polarization = { type = \"circular\" }").is_err());
}

#[test]
fn settings_reject_unknown_toml_keys() {
    // Misspelled keys must error, not be silently ignored.
    let err = |text: &str| {
        toml::from_str::<VectorSettings>(text)
            .expect_err(&format!("should reject:\n{text}"))
            .to_string()
    };
    assert!(err("polarisation = { type = \"te\" }").contains("polarisation"));
    assert!(err("film = { n = 1.7, kk = 0.1 }").contains("kk"));
    assert!(err("polarization = { type = \"linear\", angle = 45.0 }").contains("angle"));
    assert!(err("polarization = { type = \"te\", angle_deg = 3.0 }").contains("angle_deg"));
    assert!(err("polarization = { type = \"linear\" }").contains("requires `angle_deg`"));
    assert!(err("polarization = { type = \"circular\" }").contains("expected one of"));
    // Aliases used by the Python API are accepted.
    let s: VectorSettings = toml::from_str("polarization = { type = \"azimuthal\" }").unwrap();
    assert_eq!(s.polarization, IlluminationPolarization::Te);
    let s: VectorSettings = toml::from_str("polarization = { type = \"radial\" }").unwrap();
    assert_eq!(s.polarization, IlluminationPolarization::Tm);
    // Valid tables still parse.
    assert!(toml::from_str::<VectorSettings>("polarization = { type = \"te\" }").is_ok());
    assert!(toml::from_str::<VectorSettings>(
        "polarization = { type = \"linear\", angle_deg = 45.0 }\nfilm = { n = 1.7 }"
    )
    .is_ok());
}
