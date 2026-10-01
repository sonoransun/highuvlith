//! Imaging-engine benchmarks (run with `cargo bench -p highuvlith-core`).
//!
//! - `engine_creation`: source sampling + in-focus kernel set (VUV F2 at
//!   NA 0.75 on 2 nm pixels; EUV 13.5 nm through a Schwarzschild objective
//!   on 1 nm pixels).
//! - `aerial_compute`: one image with cached kernels.
//! - `focus_sweep_21`: 21 focus planes with exact (in-pupil) defocus,
//!   kernels rebuilt at every plane (cache disabled) and, for comparison,
//!   with the legacy kernel-phase model.
//! - `multiwavelength`: exact per-wavelength imaging of a 5-line comb
//!   (kernels rebuilt at every line, cache disabled).

use std::time::Duration;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use highuvlith_core::aerial::{AerialImageEngine, DefocusModel, ImagingSettings};
use highuvlith_core::mask::Mask;
use highuvlith_core::optics::schwarzschild::SchwarzschildObjective;
use highuvlith_core::optics::ProjectionOptics;
use highuvlith_core::source::{IlluminationShape, LithographySource, VuvSource};
use highuvlith_core::types::GridConfig;

fn vuv_source() -> VuvSource {
    VuvSource::f2_laser(0.7).unwrap()
}

fn euv_source() -> VuvSource {
    // A 13.5 nm source with a conventional σ = 0.7 fill (the source family is
    // irrelevant to the engine cost; only λ and the pupil fill matter).
    VuvSource {
        wavelength_nm: 13.5,
        bandwidth_pm: 0.0,
        spectral_samples: 1,
        illumination: IlluminationShape::Conventional { sigma: 0.7 },
        ..VuvSource::f2_laser(0.7).unwrap()
    }
}

/// Five-line comb around 13.5 nm (an HHG-like spectrum, ±10 %).
struct Comb;

impl LithographySource for Comb {
    fn wavelength_nm(&self) -> f64 {
        13.5
    }
    fn bandwidth_pm(&self) -> f64 {
        0.0
    }
    fn intensity_at(&self, x: f64, y: f64) -> f64 {
        if x.hypot(y) <= 0.5 {
            1.0
        } else {
            0.0
        }
    }
    fn spectral_weights(&self) -> Vec<(f64, f64)> {
        vec![
            (12.2, 0.1),
            (12.8, 0.2),
            (13.5, 0.4),
            (14.2, 0.2),
            (14.9, 0.1),
        ]
    }
}

fn bench_engine_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("engine_creation");
    group.sample_size(10);
    let optics = ProjectionOptics::new(0.75).unwrap();
    for size in [128, 256] {
        let source = vuv_source();
        group.bench_with_input(BenchmarkId::new("vuv_2nm", size), &size, |b, &size| {
            let grid = GridConfig {
                size,
                pixel_nm: 2.0,
            };
            b.iter(|| AerialImageEngine::new(&source, &optics, grid.clone(), 20).unwrap());
        });
    }
    let euv = SchwarzschildObjective::euv_standard();
    for size in [128, 256] {
        let source = euv_source();
        group.bench_with_input(BenchmarkId::new("euv_1nm", size), &size, |b, &size| {
            let grid = GridConfig {
                size,
                pixel_nm: 1.0,
            };
            b.iter(|| AerialImageEngine::new(&source, &euv, grid.clone(), 20).unwrap());
        });
    }
    group.finish();
}

fn bench_compute(c: &mut Criterion) {
    let mut group = c.benchmark_group("aerial_compute");
    let optics = ProjectionOptics::new(0.75).unwrap();
    let mask = Mask::line_space(65.0, 180.0).unwrap();
    for size in [128, 256, 512] {
        let grid = GridConfig {
            size,
            pixel_nm: 2.0,
        };
        let engine = AerialImageEngine::new(&vuv_source(), &optics, grid, 20).unwrap();
        group.bench_with_input(BenchmarkId::new("vuv_2nm", size), &size, |b, _| {
            b.iter(|| engine.compute(&mask, 0.0));
        });
    }
    let euv = SchwarzschildObjective::euv_standard();
    let euv_mask = Mask::line_space(16.0, 32.0).unwrap();
    let grid = GridConfig {
        size: 256,
        pixel_nm: 1.0,
    };
    let engine = AerialImageEngine::new(&euv_source(), &euv, grid, 20).unwrap();
    group.bench_function("euv_1nm/256", |b| b.iter(|| engine.compute(&euv_mask, 0.0)));
    group.finish();
}

fn bench_focus_sweep(c: &mut Criterion) {
    let mut group = c.benchmark_group("focus_sweep_21");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(8));
    let focus: Vec<f64> = (0..21).map(|i| -200.0 + 20.0 * i as f64).collect();
    let optics = ProjectionOptics::new(0.75).unwrap();
    let mask = Mask::line_space(65.0, 180.0).unwrap();
    for (name, model) in [
        ("exact", DefocusModel::Exact),
        ("kernel_phase", DefocusModel::KernelPhase),
    ] {
        let settings = ImagingSettings {
            max_kernels: 20,
            defocus_model: model,
            kernel_cache_capacity: 0,
            ..Default::default()
        };
        let grid = GridConfig {
            size: 256,
            pixel_nm: 2.0,
        };
        let engine =
            AerialImageEngine::with_settings(&vuv_source(), &optics, grid, settings).unwrap();
        group.bench_function(format!("vuv_2nm_256/{name}"), |b| {
            b.iter(|| engine.compute_through_focus(&mask, &focus))
        });
    }
    let euv = SchwarzschildObjective::euv_standard();
    let euv_mask = Mask::line_space(16.0, 32.0).unwrap();
    let euv_focus: Vec<f64> = (0..21).map(|i| -100.0 + 10.0 * i as f64).collect();
    let settings = ImagingSettings {
        max_kernels: 20,
        kernel_cache_capacity: 0,
        ..Default::default()
    };
    let grid = GridConfig {
        size: 256,
        pixel_nm: 1.0,
    };
    let engine = AerialImageEngine::with_settings(&euv_source(), &euv, grid, settings).unwrap();
    group.bench_function("euv_1nm_256/exact", |b| {
        b.iter(|| engine.compute_through_focus(&euv_mask, &euv_focus))
    });
    group.finish();
}

fn bench_multiwavelength(c: &mut Criterion) {
    let mut group = c.benchmark_group("multiwavelength");
    group.sample_size(10);
    let euv = SchwarzschildObjective::euv_standard();
    let mask = Mask::line_space(16.0, 32.0).unwrap();
    for size in [128, 256] {
        let settings = ImagingSettings {
            max_kernels: 20,
            kernel_cache_capacity: 0,
            ..Default::default()
        };
        let grid = GridConfig {
            size,
            pixel_nm: 1.0,
        };
        let engine = AerialImageEngine::with_settings(&Comb, &euv, grid, settings).unwrap();
        group.bench_with_input(BenchmarkId::new("comb5_euv_1nm", size), &size, |b, _| {
            b.iter(|| engine.compute_multiwavelength(&mask, 0.0).unwrap())
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_engine_creation,
    bench_compute,
    bench_focus_sweep,
    bench_multiwavelength
);
criterion_main!(benches);
