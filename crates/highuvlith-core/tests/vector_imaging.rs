//! Engine-level vector-imaging tests (WP-A2), written against contract C1.
//!
//! POST-MERGE FILE: copy to `crates/highuvlith-core/tests/vector_imaging.rs`
//! once WP-A1's `ImagingSettings` / `ImagingModel::Vector` API is merged.
//!
//! Expected numbers come from an independent numpy vector-Abbe model
//! (31×31 cell-centred source grid over [−1, 1]² in σ, analytic L/S Fourier
//! orders, pupil field from explicit (θ, φ) unit vectors). The script is
//! `a2work/engine_ref.py` in the orchestration scratchpad. Assertions compare
//! contrasts, and ratios of intensities from the same engine configuration,
//! so they hold whether or not the engine renormalizes vector images to the
//! clear field. The two `clear_field_*` tests are the exception and pin the
//! C1 invariant.

use highuvlith_core::aerial::{AerialImageEngine, ImagingModel, ImagingSettings};
use highuvlith_core::mask::Mask;
use highuvlith_core::optics::vector::{FilmInterface, IlluminationPolarization, VectorSettings};
use highuvlith_core::optics::ProjectionOptics;
use highuvlith_core::source::{IlluminationShape, VuvSource};
use highuvlith_core::types::{Complex64, GridConfig};
use ndarray::Array2;

const F2_NM: f64 = 157.63;

fn dipole_x() -> IlluminationShape {
    IlluminationShape::Dipole {
        sigma_center: 0.7,
        sigma_radius: 0.15,
        orientation_deg: 0.0,
    }
}

fn annular(inner: f64, outer: f64) -> IlluminationShape {
    IlluminationShape::Annular {
        sigma_inner: inner,
        sigma_outer: outer,
    }
}

/// Engine at the F2 line with zero flare, no kernel truncation, and a fixed
/// 31×31 source grid. `model = None` → scalar imaging.
fn engine(
    na: f64,
    illumination: IlluminationShape,
    model: Option<VectorSettings>,
    grid: GridConfig,
) -> AerialImageEngine {
    let source = VuvSource {
        illumination,
        ..VuvSource::f2_laser(0.5).unwrap()
    };
    assert!((source.wavelength_nm - F2_NM).abs() < 1e-9);
    let optics = ProjectionOptics {
        flare_fraction: 0.0,
        ..ProjectionOptics::new(na).unwrap()
    };
    let settings = ImagingSettings {
        max_kernels: 256,
        kernel_energy_fraction: 1.0 - 1e-12,
        source_points_per_axis: Some(31),
        imaging_model: match model {
            Some(v) => ImagingModel::Vector(v),
            None => ImagingModel::Scalar,
        },
        ..Default::default()
    };
    AerialImageEngine::with_settings(&source, &optics, grid, settings).unwrap()
}

fn vector(polarization: IlluminationPolarization) -> Option<VectorSettings> {
    Some(VectorSettings {
        polarization,
        ..Default::default()
    })
}

fn contrast(img: &Array2<f64>) -> f64 {
    let max = img.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let min = img.iter().cloned().fold(f64::INFINITY, f64::min);
    (max - min) / (max + min)
}

/// Aerial image of vertical L/S (orders along x) or, with `horizontal`, of
/// the same pattern transposed (orders along y).
fn ls_image(e: &AerialImageEngine, cd: f64, pitch: f64, horizontal: bool) -> Array2<f64> {
    let mask = Mask::line_space(cd, pitch).unwrap();
    if horizontal {
        // Standard (C-order) layout: the FFT needs contiguous rows.
        let t = mask
            .rasterize(e.grid())
            .t()
            .as_standard_layout()
            .into_owned();
        e.compute_from_transmittance(&t, 0.0).data
    } else {
        e.compute(&mask, 0.0).data
    }
}

// 64 px × 2 nm = 128 nm = one pitch of the dense 64/128 L/S.
fn grid_128() -> GridConfig {
    GridConfig::new(64, 2.0).unwrap()
}

#[test]
fn dense_ls_te_beats_tm_at_na_0p9_dipole_x() {
    // Orders along x: TE = y-polarized, TM = x-polarized. Two-beam imaging
    // with first order at |f|/f_c ≈ 1.37 (needs support up to (1+σ)·NA/λ).
    // Reference contrasts: Y 0.898, X 0.211, TE 0.889, TM 0.221,
    // unpolarized 0.555, scalar 0.906.
    use IlluminationPolarization as P;
    let c = |model| {
        contrast(&ls_image(
            &engine(0.9, dipole_x(), model, grid_128()),
            64.0,
            128.0,
            false,
        ))
    };
    let (c_y, c_x) = (c(vector(P::Y)), c(vector(P::X)));
    let (c_te, c_tm) = (c(vector(P::Te)), c(vector(P::Tm)));
    let (c_un, c_sc) = (c(vector(P::Unpolarized)), c(None));
    assert!(c_y > 0.80, "TE (Y) contrast {c_y}");
    assert!(c_x < 0.35, "TM (X) contrast {c_x}");
    assert!(c_y - c_x > 0.5, "TE {c_y} vs TM {c_x}");
    assert!(
        (c_te - c_y).abs() < 0.05,
        "azimuthal ≈ Y for an x dipole: {c_te} vs {c_y}"
    );
    assert!(
        (c_tm - c_x).abs() < 0.05,
        "radial ≈ X for an x dipole: {c_tm} vs {c_x}"
    );
    assert!(
        c_x < c_un && c_un < c_y,
        "unpolarized {c_un} between TM {c_x} and TE {c_y}"
    );
    assert!(
        c_sc >= c_un + 0.2,
        "scalar {c_sc} must overstate unpolarized vector {c_un}"
    );
    assert!(
        (c_sc - 0.906).abs() < 0.05,
        "scalar reference 0.906, got {c_sc}"
    );
    assert!(
        (c_un - 0.555).abs() < 0.05,
        "unpolarized reference 0.555, got {c_un}"
    );
}

#[test]
fn unpolarized_image_is_mean_of_orthogonal_states() {
    // The TCC is linear in the illumination coherency matrix, and unpolarized
    // light is ½·I in any basis: I_un = ½(I_X + I_Y) = ½(I_TE + I_TM).
    use IlluminationPolarization as P;
    let img = |pol| {
        ls_image(
            &engine(0.9, annular(0.5, 0.8), vector(pol), grid_128()),
            64.0,
            128.0,
            false,
        )
    };
    let un = img(P::Unpolarized);
    let (x, y) = (img(P::X), img(P::Y));
    let (te, tm) = (img(P::Te), img(P::Tm));
    let scale = un.iter().cloned().fold(0.0, f64::max);
    for (((u, a), b), (t, m)) in un.iter().zip(&x).zip(&y).zip(te.iter().zip(&tm)) {
        assert!((u - 0.5 * (a + b)).abs() < 1e-6 * scale, "X/Y mean");
        assert!((u - 0.5 * (t + m)).abs() < 1e-6 * scale, "TE/TM mean");
    }
}

/// max |I_vector − I_scalar| / max I_scalar for 50 % L/S at fixed k₁.
fn low_na_deviation(na: f64, pitch: f64) -> f64 {
    // 64 px spanning exactly one pitch.
    let grid = GridConfig::new(64, pitch / 64.0).unwrap();
    let conv = IlluminationShape::Conventional { sigma: 0.5 };
    let s = ls_image(
        &engine(na, conv.clone(), None, grid.clone()),
        pitch / 2.0,
        pitch,
        false,
    );
    let v = ls_image(
        &engine(
            na,
            conv,
            vector(IlluminationPolarization::Unpolarized),
            grid,
        ),
        pitch / 2.0,
        pitch,
        false,
    );
    let smax = s.iter().cloned().fold(0.0, f64::max);
    s.iter()
        .zip(&v)
        .map(|(a, b)| (a - b).abs())
        .fold(0.0, f64::max)
        / smax
}

#[test]
fn low_na_vector_matches_scalar_within_one_percent() {
    // Reference: 0.56 % at NA 0.3 (0.40 % if the engine renormalizes to the
    // clear field), and the deviation scales as NA² at fixed pupil geometry
    // (NA 0.15, pitch 2000 → 0.14 %; ratio ≈ 4.1).
    let d30 = low_na_deviation(0.3, 1000.0);
    let d15 = low_na_deviation(0.15, 2000.0);
    assert!(d30 < 0.01, "NA 0.3 vector–scalar deviation {d30}");
    assert!(
        d30 > 1e-4,
        "vector model must differ from scalar at NA 0.3: {d30}"
    );
    let ratio = d30 / d15;
    assert!(
        (3.0..5.0).contains(&ratio),
        "deviation should scale as NA²: ratio {ratio}"
    );
}

fn clear_field(model: VectorSettings) -> Array2<f64> {
    let e = engine(0.9, dipole_x(), Some(model), grid_128());
    let ones = Array2::from_elem((64, 64), Complex64::new(1.0, 0.0));
    e.compute_from_transmittance(&ones, 0.0).data
}

#[test]
fn clear_field_is_unity_at_na_0p9_without_obliquity() {
    // With obliquity off and no film the TE/TM transfer is unitary
    // (|M·J| = |J|), so a clear mask images to exactly 1.
    use IlluminationPolarization as P;
    for pol in [
        P::Unpolarized,
        P::X,
        P::Y,
        P::Te,
        P::Tm,
        P::Linear { angle_deg: 20.0 },
    ] {
        let img = clear_field(VectorSettings {
            polarization: pol,
            obliquity: false,
            ..Default::default()
        });
        for &v in img.iter() {
            assert!((v - 1.0).abs() < 1e-6, "{pol:?}: clear field {v}");
        }
    }
}

#[test]
fn clear_field_with_obliquity_obeys_c1_invariant() {
    // C1: a clear mask images to 1.0 before flare. With obliquity on, the raw
    // vector clear field Σ_s w_s A(s)² of this two-pole dipole is 1.311 at
    // this test's Some(31) sampling (1.291 adaptive, 1.287 in the numpy
    // reference); the engine divides every image by it, so the result is 1.
    let img = clear_field(VectorSettings::default());
    for &v in img.iter() {
        assert!(
            (v - 1.0).abs() < 1e-6,
            "clear field {v}: the engine no longer renormalizes vector images to the \
             clear field (raw Σ w·A² ≈ 1.31 here); see notes/A2.md §3"
        );
    }
}

#[test]
fn x_polarization_favours_lines_whose_orders_run_along_y() {
    // X-pol is TM for vertical lines (orders along x) and TE for horizontal
    // lines. Reference (NA 0.9, annular 0.5–0.8, 90/180): 0.447 vs 0.750.
    let grid = GridConfig::new(64, 2.8125).unwrap(); // 180 nm = one pitch
    let e = engine(
        0.9,
        annular(0.5, 0.8),
        vector(IlluminationPolarization::X),
        grid,
    );
    let vertical = contrast(&ls_image(&e, 90.0, 180.0, false));
    let horizontal = contrast(&ls_image(&e, 90.0, 180.0, true));
    assert!(
        horizontal > vertical + 0.2,
        "horizontal {horizontal} vs vertical {vertical}"
    );
    assert!(
        (vertical - 0.447).abs() < 0.05,
        "vertical reference 0.447, got {vertical}"
    );
    assert!(
        (horizontal - 0.750).abs() < 0.05,
        "horizontal reference 0.750, got {horizontal}"
    );
}

#[test]
fn azimuthal_and_radial_illumination_are_rotation_symmetric() {
    // TE/TM are defined per source point, so a 90° rotation of the pattern
    // under an annular source leaves the contrast unchanged
    // (reference 0.621 TE, 0.576 TM, 0.599 unpolarized).
    use IlluminationPolarization as P;
    let grid = GridConfig::new(64, 2.8125).unwrap();
    for (pol, want) in [(P::Te, 0.621), (P::Tm, 0.576), (P::Unpolarized, 0.599)] {
        let e = engine(0.9, annular(0.5, 0.8), vector(pol), grid.clone());
        let v = contrast(&ls_image(&e, 90.0, 180.0, false));
        let h = contrast(&ls_image(&e, 90.0, 180.0, true));
        assert!(
            (v - h).abs() < 1e-3,
            "{pol:?}: vertical {v} vs horizontal {h}"
        );
        assert!(
            (v - want).abs() < 0.05,
            "{pol:?}: reference {want}, got {v}"
        );
    }
}

#[test]
fn high_index_film_restores_tm_contrast() {
    // Inside a film of index 1.7 the orders travel at smaller angles, so
    // the TM contrast loss shrinks: X-pol dense L/S 0.211 (air) → 0.666
    // (film n = 1.7, k = 0.03), while TE stays at ≈ 0.90.
    let air = contrast(&ls_image(
        &engine(
            0.9,
            dipole_x(),
            vector(IlluminationPolarization::X),
            grid_128(),
        ),
        64.0,
        128.0,
        false,
    ));
    let film = Some(VectorSettings {
        polarization: IlluminationPolarization::X,
        film: Some(FilmInterface { n: 1.7, k: 0.03 }),
        ..Default::default()
    });
    let in_film = contrast(&ls_image(
        &engine(0.9, dipole_x(), film, grid_128()),
        64.0,
        128.0,
        false,
    ));
    assert!(in_film > air + 0.3, "film {in_film} vs air {air}");
    assert!(
        (in_film - 0.666).abs() < 0.05,
        "film reference 0.666, got {in_film}"
    );
}
