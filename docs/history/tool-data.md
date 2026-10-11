---
description: "The tool data behind the history section: the 86-tool sample, how it was assembled, and its sources."
---

# The tool data

<p class="huv-lead">Every chart in the history section is drawn from one file, <code>litho_tools.csv</code>: 86 exposure-tool models from the 1973 Perkin-Elmer Micralign to ASML's 2025 EXE:5200B, each with its year, wavelength, numerical aperture and vendor-stated resolution, and every value tied to a numbered source. This page is the data itself, how it was assembled, and what it is not: a sample of the tool models that mark the turning points, not a catalogue of everything that shipped.</p>

## The sample

The 86 rows cover Nikon, Canon, ASML, GCA, Perkin-Elmer and SVG Lithography models from
nine exposure technologies: the 1:1 scanning projection aligner, g-line and i-line steppers,
KrF, dry ArF, the 157 nm prototype, ArF immersion, EUV at NA 0.25–0.33 and High-NA EUV. The
selection follows the milestones the histories cite (first tool of a kind, first below a
resolution mark, the long-running production platforms) and the vendor archives that state
NA and resolution per model, so it is deliberately thick where the attack on R = k₁λ/NA
changed lever and thin elsewhere. A gap in a lane of the [tool-lanes
chart](index.md#old-wavelengths-never-die) means only that no model from those years was
included, and band ends on the era ribbon are sampling artefacts, not retirement dates.

What the sample is not: it is not every model a vendor shipped, it holds no third-party or
packaging-lithography tools before the 2025 XT:260, and it carries no throughput, overlay or
price columns, even where the chapters quote such numbers from the same sources.

## Methodology

- **Year** is the year of introduction or first shipment; the CSV notes which. Where sources
  disagree by a year (ASML's PAS 2000, the TWINSCAN launch) the chapter text gives both and the
  table keeps the vendor's own date.
- **R** is the vendor's stated resolution, not a measured half-pitch, and vendors define it
  differently: for the NXT:1980Di, for example, ASML quotes 38 nm with dipole illumination and
  40 nm with C-quad (S30). The derived **k₁ = R·NA/λ** inherits that convention and is an
  indicator of the trend, not a definition.
- **Blank cells** (–) mean no source was found for that value, so the charts skip the point.
  The PAS 5500/900's NA is blank because its sources disagree: ASML's product description gives
  an NA adjustable from 0.45 to below 0.6 (S41), while Bruning's table of Zeiss 193 nm lenses
  lists the "900" lens at 0.63 (S3).
- **Wavelength** is the nominal line (436, 365, 248, 193, 157, 13.5 nm); the two Micrascan
  points near 250 nm used a filtered mercury-lamp band rather than a KrF laser, and the 157 nm
  row is the Micrascan VII prototype, which never shipped as a production tool.
- **Reproducing the charts**: [make_history_figures.py](../figures/history/make_history_figures.py)
  reads the CSV through [history_data.py](../figures/history/history_data.py) and draws every
  history chart; `make_history_figures.py --table` prints the table below, which is pasted
  here verbatim and checked against the CSV on every build.

## Sources

Every row cites its sources by number; the list follows the table.

<!-- tool-table:start -->

| Year | Vendor | Model | λ (nm) | NA | R (nm) | k₁ | Sources |
|---:|---|---|---:|---:|---:|---:|---|
| 1973 | Perkin-Elmer | Micralign | – | 0.167 | – | – | S1, S2 |
| 1978 | GCA | DSW 4800 | 436 | 0.28 | – | – | S1, S2 |
| 1980 | Nikon | NSR-1010G | 436 | – | 1000 | – | S1, S5 |
| 1981 | Nikon | NSR-1505G | 436 | 0.3 | 1200 | 0.83 | S4, S5 |
| 1984 | Canon | FPA-1500FA | 436 | – | – | – | S1, S2 |
| 1984 | Nikon | NSR-1010i3 | 365 | – | 800 | – | S1, S2, S5 |
| 1986 | Canon | FPA-1550MII | 436 | 0.43 | 800 | 0.79 | S4 |
| 1987 | Nikon | NSR-1505G4B | 436 | – | 900 | – | S1, S5 |
| 1987 | Nikon | NSR-1505G4D | 436 | 0.45 | 750 | 0.77 | S4 |
| 1987 | ASML | PAS 2500/40 | 365 | 0.4 | 700 | 0.77 | S1, S2 |
| 1988 | Nikon | NSR-1505EX | 248 | 0.42 | 500 | 0.85 | S1, S5 |
| 1988 | Nikon | NSR-1505G6E | 436 | – | 650 | – | S5 |
| 1989 | Nikon | NSR-1505i6A | 365 | – | 650 | – | S5 |
| 1989 | Nikon | NSR-1755G7A | 436 | – | 650 | – | S5 |
| 1990 | Canon | FPA-2000i1 | 365 | – | – | – | S1, S2 |
| 1990 | SVG Lithography | Micrascan I | 250 | 0.35 | 350 | 0.49 | S1, S2, S3 |
| 1990 | Nikon | NSR-2005G8C | 436 | – | 550 | – | S5 |
| 1991 | Nikon | NSR-1755EX8A | 248 | – | 450 | – | S5 |
| 1991 | ASML | PAS 5000/70 | 248 | 0.42 | – | – | S1, S2 |
| 1992 | SVG Lithography | Micrascan II | 250 | 0.5 | 250 | 0.50 | S2, S3 |
| 1993 | Nikon | NSR-2005i10C | 365 | – | 450 | – | S5 |
| 1994 | Canon | FPA-3000i4 | 365 | 0.63 | 350 | 0.60 | S8 |
| 1994 | Nikon | NSR-2205i11D | 365 | 0.63 | 350 | 0.60 | S5, S6 |
| 1995 | Nikon | NSR-S201A | 248 | 0.6 | 250 | 0.60 | S2, S5, S6 |
| 1996 | Canon | FPA-3000EX3 | 248 | – | – | – | S8 |
| 1996 | SVG Lithography | Micrascan III | 248 | 0.6 | 250 | 0.60 | S1, S2 |
| 1996 | Nikon | NSR-2205EX12B | 248 | 0.55 | 280 | 0.62 | S5, S6 |
| 1996 | Nikon | NSR-2205i12D | 365 | 0.63 | 350 | 0.60 | S5, S6 |
| 1997 | Canon | FPA-4000ES1 | 248 | – | – | – | S1, S2 |
| 1997 | Nikon | NSR-2205EX14C | 248 | 0.6 | 250 | 0.60 | S5, S6 |
| 1997 | Nikon | NSR-2205i14E | 365 | 0.63 | 350 | 0.60 | S5, S6 |
| 1997 | ASML | PAS 5500/500 | 248 | 0.63 | 220 | 0.56 | S1, S2 |
| 1998 | Nikon | NSR-S203B | 248 | 0.68 | 180 | 0.49 | S5, S6 |
| 1998 | ASML | PAS 5500/900 | 193 | – | 130 | – | S1, S2, S3, S41 |
| 1999 | Canon | FPA-3000EX6 | 248 | 0.65 | 150 | 0.39 | S9, S10 |
| 1999 | Canon | FPA-5000AS1 | 193 | – | – | – | S1, S2 |
| 1999 | SVG Lithography | Micrascan 193 | 193 | 0.6 | – | – | S2 |
| 1999 | Nikon | NSR-2205i14E2 | 365 | 0.63 | 350 | 0.60 | S5, S6 |
| 1999 | Nikon | NSR-S204B | 248 | 0.68 | 150 | 0.41 | S5, S6 |
| 1999 | Nikon | NSR-S302A | 193 | – | 180 | – | S1, S5 |
| 1999 | Nikon | NSR-S305B | 193 | 0.68 | 110 | 0.39 | S5, S6 |
| 1999 | Nikon | NSR-SF100 | 365 | 0.52 | 400 | 0.57 | S5, S6 |
| 2000 | Nikon | NSR-S205C | 248 | 0.75 | 130 | 0.39 | S5, S6 |
| 2001 | SVG Lithography | Micrascan V | 193 | 0.75 | 130 | 0.51 | S3 |
| 2001 | Nikon | NSR-S306C | 193 | 0.78 | 100 | 0.40 | S5, S6 |
| 2002 | Nikon | NSR-S206D | 248 | 0.82 | 110 | 0.36 | S5, S6 |
| 2003 | SVG Lithography / ASML | Micrascan VII | 157 | 0.75 | 100 | 0.48 | S1, S2, S3 |
| 2003 | Nikon | NSR-S307E | 193 | 0.85 | 80 | 0.35 | S5, S6 |
| 2004 | Nikon | NSR-S208D | 248 | 0.82 | 110 | 0.36 | S5, S6 |
| 2004 | Nikon | NSR-S308F | 193 | 0.92 | 65 | 0.31 | S5, S6 |
| 2004 | ASML | TWINSCAN XT:1250i | 193 | 0.85 | 70 | 0.31 | S1, S19 |
| 2004 | ASML | TWINSCAN XT:1400 | 193 | 0.93 | – | – | S19 |
| 2005 | Nikon | NSR-S609B | 193 | 1.07 | 55 | 0.30 | S1, S5, S6 |
| 2005 | Nikon | NSR-SF140 | 365 | 0.62 | 280 | 0.48 | S5, S6 |
| 2006 | ASML | EUV Alpha Demo Tool | 13.5 | 0.25 | – | – | S18, S20 |
| 2006 | Nikon | NSR-S610C | 193 | 1.3 | 45 | 0.30 | S1, S5, S6 |
| 2006 | ASML | TWINSCAN XT:1700i | 193 | 1.2 | 45 | 0.28 | S1, S21 |
| 2007 | Nikon | NSR-S210D | 248 | 0.82 | 110 | 0.36 | S5, S6 |
| 2007 | Nikon | NSR-S310F | 193 | 0.92 | 65 | 0.31 | S5, S6 |
| 2007 | ASML | TWINSCAN XT:1900i | 193 | 1.35 | 40 | 0.28 | S15, S21 |
| 2008 | Nikon | NSR-S620D | 193 | 1.35 | 38 | 0.27 | S5, S6 |
| 2008 | ASML | TWINSCAN NXT:1950i | 193 | 1.35 | 38 | 0.27 | S16, S21 |
| 2010 | ASML | TWINSCAN NXE:3100 | 13.5 | 0.25 | 27 | 0.50 | S21, S22 |
| 2011 | Nikon | NSR-S320F | 193 | 0.92 | 65 | 0.31 | S5, S6 |
| 2012 | Canon | FPA-6300ES6a | 248 | 0.86 | 90 | 0.31 | S11, S12 |
| 2012 | Nikon | NSR-S621D | 193 | 1.35 | 38 | 0.27 | S5, S6 |
| 2013 | Nikon | NSR-S622D | 193 | 1.35 | 38 | 0.27 | S5, S6 |
| 2013 | ASML | TWINSCAN NXE:3300B | 13.5 | 0.33 | 22 | 0.54 | S22, S23 |
| 2014 | Nikon | NSR-S630D | 193 | 1.35 | 38 | 0.27 | S5, S6 |
| 2015 | ASML | TWINSCAN NXE:3350B | 13.5 | 0.33 | 16 | 0.39 | S23 |
| 2015 | ASML | TWINSCAN NXT:1980Di | 193 | 1.35 | 38 | 0.27 | S30 |
| 2016 | Canon | FPA-5550iZ2 | 365 | 0.57 | 350 | 0.55 | S13, S14 |
| 2016 | Nikon | NSR-S631E | 193 | 1.35 | 38 | 0.27 | S5, S6 |
| 2017 | ASML | TWINSCAN NXE:3400B | 13.5 | 0.33 | 13 | 0.32 | S24, S35 |
| 2019 | ASML | TWINSCAN NXE:3400C | 13.5 | 0.33 | 13 | 0.32 | S36 |
| 2020 | ASML | TWINSCAN NXT:1470 | 193 | 0.93 | 57 | 0.27 | S16, S32 |
| 2021 | ASML | TWINSCAN NXE:3600D | 13.5 | 0.33 | 13 | 0.32 | S25, S37 |
| 2021 | ASML | TWINSCAN XT:860N | 248 | 0.8 | 110 | 0.35 | S26, S33 |
| 2022 | ASML | TWINSCAN NXT:2100i | 193 | 1.35 | 38 | 0.27 | S17, S31 |
| 2023 | Nikon | NSR-2205iL1 | 365 | 0.45 | 350 | 0.43 | S5, S7 |
| 2023 | Nikon | NSR-S636E | 193 | 1.35 | 38 | 0.27 | S5, S7 |
| 2023 | ASML | TWINSCAN EXE:5000 | 13.5 | 0.55 | 8 | 0.33 | S27, S39 |
| 2024 | ASML | TWINSCAN NXE:3800E | 13.5 | 0.33 | 13 | 0.32 | S28, S38 |
| 2025 | Nikon | NSR-S333F | 193 | 0.92 | 65 | 0.31 | S5, S7 |
| 2025 | ASML | TWINSCAN EXE:5200B | 13.5 | 0.55 | 8 | 0.33 | S29, S40 |
| 2025 | ASML | TWINSCAN XT:260 | 365 | 0.35 | 400 | 0.38 | S15, S34 |

- **S1** [A. Kato, 'Chronology of Lithography Milestones', v0.9, May 2007](https://www.lithoguru.com/scientist/litho_history/Kato_Litho_History.pdf)
- **S2** [C. A. Mack, 'Milestones in Optical Lithography Tool Suppliers', 2005](https://lithoguru.com/scientist/litho_history/milestones_tools.pdf)
- **S3** [J. H. Bruning, 'Optical lithography ... 40 years and holding', Proc. SPIE 6520, 652004 (2007)](https://doi.org/10.1117/12.720631)
- **S4** [Semiconductor History Museum of Japan, stepper exhibit](https://www.shmj.or.jp/museum2010/exhibi2446.html)
- **S5** [Nikon, history of semiconductor lithography systems](https://www.nikon.com/business/semi/history/)
- **S6** [Nikon, lithography system archive (NA, resolution per model)](https://www.nikon.com/business/semi/lineup/archives/)
- **S7** [Nikon, current lithography line-up](https://www.nikon.com/business/semi/lineup/)
- **S8** [Canon, history of industrial equipment (IP history pages)](https://global.canon/en/intellectual-property/history/industrial.html)
- **S9** [Canon, FPA-3030EX6 product page](https://global.canon/en/product/indtech/semicon/fpa3030ex6.html)
- **S10** [Photonics Online, 'Canon introduces the first 0.65-NA DUV stepper'](https://www.photonicsonline.com/doc/canon-introduces-the-first-065-na-duv-stepper-0001)
- **S11** [Canon, FPA-6300ES6a product page](https://global.canon/en/product/indtech/semicon/fpa6300es6a.html)
- **S12** [Canon news release, 5 Apr 2012](https://global.canon/en/news/2012/apr05e.html)
- **S13** [Canon, FPA-5550iZ2 product page](https://global.canon/en/product/indtech/semicon/fpa5550iz2.html)
- **S14** [Canon news release, 11 Dec 2017](https://global.canon/en/news/2017/20171211.html)
- **S15** [ASML, company history](https://www.asml.com/en/company/about-asml/history)
- **S16** [ASML, 'TWINSCAN: 20 years of lithography innovation' (2021)](https://www.asml.com/en/company/stories/2021/twinscan-20-years-innovation)
- **S17** [ASML, 'How immersion lithography saved Moore's Law' (2023)](https://www.asml.com/en/company/stories/2023/how-immersion-lithography-saved-moores-law)
- **S18** [ASML, 'Making EUV: from lab to fab' (2022)](https://www.asml.com/en/company/stories/2022/making-euv-lab-to-fab)
- **S19** [ASML Form 20-F, fiscal 2004](https://www.sec.gov/Archives/edgar/data/937966/000115697305000129/u48286e20vf.htm)
- **S20** [ASML Form 20-F, fiscal 2006](https://www.sec.gov/Archives/edgar/data/937966/000095012307000854/u51076e20vf.htm)
- **S21** [ASML Form 20-F, fiscal 2010](https://www.sec.gov/Archives/edgar/data/937966/000095012311013996/u09689e20vf.htm)
- **S22** [ASML Form 20-F, fiscal 2013](https://www.sec.gov/Archives/edgar/data/937966/000119312514046822/d546896d20f.htm)
- **S23** [ASML Form 20-F, fiscal 2015](https://www.sec.gov/Archives/edgar/data/937966/000093796616000017/a20-fasml2015.htm)
- **S24** [ASML Form 20-F / integrated report, fiscal 2017](https://www.sec.gov/Archives/edgar/data/937966/000093796618000007/a2017integratedreportbased.htm)
- **S25** [ASML Q2 2021 results press release](https://www.sec.gov/Archives/edgar/data/937966/000093796621000016/pressreleasequarterlyresul.htm)
- **S26** [ASML Q4 2021 results press release](https://www.sec.gov/Archives/edgar/data/937966/000093796622000004/pressreleasequarterlyresul.htm)
- **S27** [ASML Q4 2023 results press release](https://www.sec.gov/Archives/edgar/data/937966/000093796624000003/pressreleasequarterlyresul.htm)
- **S28** [ASML Q1 2024 investor presentation](https://www.sec.gov/Archives/edgar/data/937966/000093796624000013/presentationinvestorrela.htm)
- **S29** [ASML Q2 2025 results press release](https://www.sec.gov/Archives/edgar/data/937966/000162828025034992/pressreleasequarterlyresul.htm)
- **S30** [ASML, TWINSCAN NXT:1980Di product page](https://www.asml.com/en/products/duv-lithography-systems/twinscan-nxt1980di)
- **S31** [ASML, TWINSCAN NXT:2100i product page](https://www.asml.com/en/products/duv-lithography-systems/twinscan-nxt2100i)
- **S32** [ASML, TWINSCAN NXT:1470 product page](https://www.asml.com/en/products/duv-lithography-systems/twinscan-nxt1470)
- **S33** [ASML, TWINSCAN XT:860N product page](https://www.asml.com/en/products/duv-lithography-systems/twinscan-xt860n)
- **S34** [ASML, TWINSCAN XT:260 product page](https://www.asml.com/en/products/duv-lithography-systems/twinscan-xt-260)
- **S35** [ASML, TWINSCAN NXE:3400B product page](https://www.asml.com/en/products/euv-lithography-systems/twinscan-nxe3400b)
- **S36** [ASML, TWINSCAN NXE:3400C product page](https://www.asml.com/en/products/euv-lithography-systems/twinscan-nxe3400c)
- **S37** [ASML, TWINSCAN NXE:3600D product page](https://www.asml.com/en/products/euv-lithography-systems/twinscan-nxe-3600d)
- **S38** [ASML, TWINSCAN NXE:3800E product page](https://www.asml.com/en/products/euv-lithography-systems/twinscan-nxe-3800e)
- **S39** [ASML, TWINSCAN EXE:5000 product page](https://www.asml.com/en/products/euv-lithography-systems/twinscan-exe-5000)
- **S40** [ASML, TWINSCAN EXE:5200B product page](https://www.asml.com/en/products/euv-lithography-systems/twinscan-exe-5200b)
- **S41** [Semiconductor Online, 'PAS 5500/900 193 nm Step and Scan System' (ASML product description: NA 0.45 to <0.6, linewidths 150-130 nm)](https://www.semiconductoronline.com/doc/pas-5500900-193-nm-step-and-scan-system-0001)

<!-- tool-table:end -->

**Where next:** [History](index.md) · [The road to EUV](euv.md) · [Process nodes](../nodes/index.md)
