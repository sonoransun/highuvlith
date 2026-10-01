# Data used by the figure scripts

## `source_power_vs_wavelength.csv` (49 rows)

Published average powers of lithography-relevant light sources versus wavelength,
compiled and fact-checked for this project on 2026-09-30. Every row carries its
citation and a URL (DOIs resolved via doi.org / Crossref, publisher or company
pages, EUV Litho workshop PDFs); `conversion_note` shows any arithmetic used
(E_photon = 1239.84 / λ[nm] eV). Used by `fig_sources.py` (source landscape).

Columns: `id, class, machine_or_paper, wavelength_nm, power_w_low, power_w_high,
plane_or_definition, status, year, citation, url, conversion_note`.

Caveats that the chart reflects:

- **Plane of definition differs.** Sn LPP powers are at intermediate focus (IF);
  laser powers are laser output; rows marked `AT THE SOURCE` (discharge sources,
  ids 20–22 and 24–26) are into 2π sr at the plasma and are drawn with a distinct
  marker — the power at IF is several times lower.
- In-burst powers (ASML 375/450/500 W at 3 % duty, Gigaphoton "250 W in-burst") are
  excluded; only time-averaged values are listed.
- id 9 vs 10: 500 W (investor deck) vs 600 W (press) for the current product
  source; id 32: the paper states 20 mW although 70 µJ × 700 Hz = 49 mW (plotted as
  stated); ids 41–44 are HHG at 21.7–30 eV (41–57 nm), not 13.5 nm; id 38 (SSMB) is a
  per-tool design figure.
- Status `PRODUCTION*` / `DEMONSTRATED*` are drawn as measured or shipping;
  `PROJECTED*` as projections or designs.

HVM requirement used for the band in the chart (13.5 nm, at IF, 2 % bandwidth):
about 250 W (NXE:3400B, ≥ 125 wph at 20 mJ/cm²) rising to 500–600 W (NXE:3800E
class) and a stated 1 kW target around 2030 (ids 8–10, 14).

Not verified, therefore not in the file: F₂ 157 nm lithography-laser powers, Hg
lamp output, Gd/Tb average powers (only conversion efficiencies were found).
