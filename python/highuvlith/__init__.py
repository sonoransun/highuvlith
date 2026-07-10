"""highuvlith: lithography simulation framework spanning VUV (120-160 nm)
through EUV (13.5 nm) to soft X-ray (1-10 nm), with pluggable illumination
sources and optical systems backed by a Rust physics engine."""

from highuvlith._native import (
    SourceConfig,
    OpticsConfig,
    MaskConfig,
    ResistConfig,
    FilmStackConfig,
    ProcessConfig,
    GridConfig,
    SimulationEngine,
    AerialImageResult,
    ResistProfileResult,
    BatchSimulator,
    ProcessWindowResult,
    # MNSL classes
    PyMnslConfig as MnslConfig,
    PyMnslEngine as MnslEngine,
    PyMnslResult as MnslResult,
    PyNanosphereArrayConfig as NanosphereArrayConfig,
    PySpherePacking as SpherePacking,
    PySubstrateCoupling as SubstrateCoupling,
    py_simulate_moire_emission,
    # Volumetric / deep-layer classes and helpers
    VolumetricResult,
    HeightMapResult,
    LigaResult,
    develop_fast_marching,
    height_map_from_times,
    grayscale_height_map,
    grayscale_transmittance_for_target,
    blazed_grating,
    microlens_array,
    quantum_aerial_image,
)

from highuvlith.api import (
    FullResult,
    simulate_line_space,
    simulate_contact_hole,
    sweep_focus,
    expose_volumetric,
    simulate_liga,
    simulate_interference,
    simulate_grayscale,
    simulate_quantum_line_space,
    GrayscaleResult,
    QuantumResult,
)

from highuvlith.mnsl import (
    MnslSimResult,
    simulate_moire_emission,
    create_nanosphere_array,
    sweep_rotation_angle,
    sweep_separation,
    optimize_moire_parameters,
)

__version__ = "0.1.0"

__all__ = [
    # Configuration
    "SourceConfig",
    "OpticsConfig",
    "MaskConfig",
    "ResistConfig",
    "FilmStackConfig",
    "ProcessConfig",
    "GridConfig",
    # Simulation
    "SimulationEngine",
    "BatchSimulator",
    # Results
    "AerialImageResult",
    "ResistProfileResult",
    "ProcessWindowResult",
    # High-level API
    "FullResult",
    "simulate_line_space",
    "simulate_contact_hole",
    "sweep_focus",
    # Volumetric / deep-layer results
    "VolumetricResult",
    "HeightMapResult",
    "LigaResult",
    # Volumetric / deep-layer API
    "expose_volumetric",
    "develop_fast_marching",
    "height_map_from_times",
    "simulate_liga",
    "simulate_interference",
    "simulate_grayscale",
    "simulate_quantum_line_space",
    "grayscale_height_map",
    "grayscale_transmittance_for_target",
    "blazed_grating",
    "microlens_array",
    "quantum_aerial_image",
    "GrayscaleResult",
    "QuantumResult",
    # MNSL Configuration
    "MnslConfig",
    "MnslEngine",
    "MnslResult",
    "NanosphereArrayConfig",
    "SpherePacking",
    "SubstrateCoupling",
    "py_simulate_moire_emission",
    # MNSL High-level API
    "MnslSimResult",
    "simulate_moire_emission",
    "create_nanosphere_array",
    "sweep_rotation_angle",
    "sweep_separation",
    "optimize_moire_parameters",
]
