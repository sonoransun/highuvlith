"""Visualization utilities for highuvlith (requires matplotlib; plotly optional).

Every helper takes ``theme="light" | "dark"`` (default: the current theme,
see :func:`use_theme`) and ``ax=`` (or ``fig=`` for multi-panel figures), and
returns the Axes (or Figure) without showing it. The shared palette and
chart chrome live in :mod:`highuvlith.viz.style`; :func:`export_light_dark`
renders any helper in both themes for documentation pages.
"""

from highuvlith.viz.aerial import plot_aerial, plot_cross_section
from highuvlith.viz.deep_litho import plot_liga_edge_profile, plot_talbot_carpet
from highuvlith.viz.mnsl import (
    plot_emission_pattern,
    plot_moire_analysis,
    plot_optimization_heatmap,
    plot_rotation_sweep,
)
from highuvlith.viz.optimization import plot_ilt_evolution, plot_sraf_dof
from highuvlith.viz.process_window import plot_bossung, plot_ed_window, plot_el_vs_dof
from highuvlith.viz.resist import plot_resist_profile
from highuvlith.viz.sources import plot_source, plot_source_landscape
from highuvlith.viz.style import (
    THEMES,
    categorical,
    current_theme,
    diverging_cmap,
    export_light_dark,
    ordinal_colors,
    sequential_cmap,
    use_theme,
)
from highuvlith.viz.vector import plot_polarization_contrast
from highuvlith.viz.volumetric import (
    plot_depth_dose,
    plot_development_front,
    plot_height_map,
    plot_isosurface,
    plot_xz_slice,
)

__all__ = [
    # Style
    "THEMES",
    "categorical",
    "current_theme",
    "diverging_cmap",
    "export_light_dark",
    "ordinal_colors",
    "sequential_cmap",
    "use_theme",
    # Imaging
    "plot_aerial",
    "plot_cross_section",
    "plot_polarization_contrast",
    # Sources
    "plot_source",
    "plot_source_landscape",
    # Process window / resist
    "plot_bossung",
    "plot_ed_window",
    "plot_el_vs_dof",
    "plot_resist_profile",
    # Volumetric / deep-layer
    "plot_xz_slice",
    "plot_depth_dose",
    "plot_height_map",
    "plot_development_front",
    "plot_isosurface",
    "plot_liga_edge_profile",
    "plot_talbot_carpet",
    # Optimization
    "plot_ilt_evolution",
    "plot_sraf_dof",
    # MNSL
    "plot_emission_pattern",
    "plot_moire_analysis",
    "plot_rotation_sweep",
    "plot_optimization_heatmap",
]
