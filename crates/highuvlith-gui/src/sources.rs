//! Source-family view model.
//!
//! One parameter struct per registered `SourceKind` family, holding the
//! family's main machine parameters (what the sliders edit); presets that
//! are read from the core factories, so no preset number is duplicated
//! here; the honesty badge of each family (mirroring the `**Status:**` line
//! of its `docs/sources/*.md` page); whether the family can drive
//! projection imaging or belongs to the LIGA view; and the mapping
//! parameters → [`SourceKind`].

use highuvlith_core::source::{
    sigma_from_coherence, BetatronSource, DppFuel, DppSource, EntangledPhotonSource, HhgGas,
    HhgSource, IcsSource, IlluminationShape, LithographySource, LpaFelSource, LppFuel, LppSource,
    SmithPurcellSource, SourceKind, SpectralShape, SsmbSource, SxrlScheme, SxrlSource,
    SynchrotronBeamline, SynchrotronSource, VuvSource, XfelMode, XfelSource, XrayAnode,
    XrayTubeSource,
};
use highuvlith_core::source_models::ics::IcsCollision;
use highuvlith_core::source_models::lpp::LppDriveLaser;
use highuvlith_core::source_models::physics::{self, ElectronBeamParams, UndulatorParams};
use highuvlith_core::source_models::ssmb::SsmbRadiator;
use highuvlith_core::source_models::synchrotron::StorageRingBeam;
use highuvlith_core::source_models::xfel::FelMachine;

/// Honesty badge (see `docs/capability-matrix.md`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Badge {
    /// ✅ computed by code, covered by tests, validated physics.
    Implemented,
    /// 🔶 runs end to end with documented approximations.
    Simplified,
    /// 🧪 runnable, but models speculative physics / research projections.
    Theoretical,
}

impl Badge {
    /// The badge emoji used across the docs.
    pub fn emoji(self) -> &'static str {
        match self {
            Badge::Implemented => "\u{2705}",
            Badge::Simplified => "\u{1f536}",
            Badge::Theoretical => "\u{1f9ea}",
        }
    }

    /// The badge word used across the docs.
    pub fn word(self) -> &'static str {
        match self {
            Badge::Implemented => "Implemented",
            Badge::Simplified => "Simplified",
            Badge::Theoretical => "Theoretical",
        }
    }
}

/// Family-level status: badges plus the one-line justification of the
/// docs page's Status line.
#[derive(Debug)]
pub struct StatusInfo {
    /// Badges in the order the Status line gives them.
    pub badges: &'static [Badge],
    /// Short justification.
    pub summary: &'static str,
    /// Docs page (repository-relative path).
    pub doc: &'static str,
}

/// Where a view sends the source: projection imaging (aerial image, process
/// window, volumetric resist) or LIGA shadow printing (depth dose).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Route {
    /// Usable with projection optics.
    Projection,
    /// Broadband, incoherent X-rays: LIGA / proximity printing only.
    Liga,
}

/// How the operating wavelength comes about.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LambdaOrigin {
    /// Set directly (a set-point the machine is tuned to).
    SetPoint,
    /// Fixed by an atomic / plasma line or a mirror band.
    FixedLine,
    /// Derived from the machine parameters (the formula is given).
    Derived(&'static str),
}

/// Every registered `SourceKind` family.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Family {
    Vuv,
    LpaFel,
    Lpp,
    Synchrotron,
    Hhg,
    Xfel,
    Ics,
    Ssmb,
    Entangled,
    XrayTube,
    Dpp,
    Sxrl,
    Betatron,
    SmithPurcell,
}

impl Family {
    /// All families, in menu order (heritage optical → plasma → accelerator
    /// → lab/compact → exotic).
    pub const ALL: [Family; 14] = [
        Family::Vuv,
        Family::Lpp,
        Family::Dpp,
        Family::Synchrotron,
        Family::Xfel,
        Family::LpaFel,
        Family::Hhg,
        Family::Sxrl,
        Family::Ics,
        Family::Ssmb,
        Family::SmithPurcell,
        Family::Entangled,
        Family::XrayTube,
        Family::Betatron,
    ];

    /// Menu label.
    pub fn label(self) -> &'static str {
        match self {
            Family::Vuv => "VUV / DUV / UV (excimer lasers, Hg lamps)",
            Family::LpaFel => "LPA-FEL (laser-plasma FEL)",
            Family::Lpp => "LPP (laser-produced plasma)",
            Family::Synchrotron => "Synchrotron (undulator / bending magnet)",
            Family::Hhg => "HHG (high-harmonic generation)",
            Family::Xfel => "XFEL / FEL (SASE, seeded, CW-SC, ERL)",
            Family::Ics => "Inverse Compton scattering",
            Family::Ssmb => "SSMB storage ring",
            Family::Entangled => "Entangled photons (NOON)",
            Family::XrayTube => "Hard X-ray tube",
            Family::Dpp => "DPP / LDP (discharge plasma)",
            Family::Sxrl => "Soft-X-ray laser (plasma)",
            Family::Betatron => "Laser-wakefield betatron",
            Family::SmithPurcell => "Smith\u{2013}Purcell grating",
        }
    }

    /// The `SourceKind::kind_label` / TOML `type` tag of the family.
    pub fn kind_label(self) -> &'static str {
        match self {
            Family::Vuv => "vuv",
            Family::LpaFel => "lpa_fel",
            Family::Lpp => "lpp",
            Family::Synchrotron => "synchrotron",
            Family::Hhg => "hhg",
            Family::Xfel => "xfel",
            Family::Ics => "ics",
            Family::Ssmb => "ssmb",
            Family::Entangled => "entangled",
            Family::XrayTube => "xray_tube",
            Family::Dpp => "dpp",
            Family::Sxrl => "sxrl",
            Family::Betatron => "betatron",
            Family::SmithPurcell => "smith_purcell",
        }
    }

    /// Status of the family (mirrors the Status line of its docs page).
    pub fn status(self) -> &'static StatusInfo {
        use Badge::*;
        match self {
            Family::Vuv => &StatusInfo {
                badges: &[Implemented],
                summary: "Line shape, pupil fill, pulse-energy \u{d7} rep-rate power and the derived \
                          photon budget are live and test-pinned.",
                doc: "docs/sources/vuv-excimer.md",
            },
            Family::LpaFel => &StatusInfo {
                badges: &[Simplified],
                summary: "Resonance and 1D / Ming-Xie FEL physics are derived from the undulator and \
                          beam; demonstrated LPA-FEL lasing is 420 nm at 1 Hz, the 25 nm point is a \
                          design projection.",
                doc: "docs/sources/lpa-fel.md",
            },
            Family::Lpp => &StatusInfo {
                badges: &[Implemented, Simplified],
                summary: "Drive power, CE and collection give the intermediate-focus power on the \
                          NXE:3400B parameters (250 W at IF); CE defaults and \u{e9}tendue are \
                          simplified; Gd/Tb in-band power is a projection.",
                doc: "docs/sources/lpp.md",
            },
            Family::Synchrotron => &StatusInfo {
                badges: &[Implemented, Simplified],
                summary: "Wavelength, sinc\u{b2} line, harmonic content and absolute flux/power are \
                          derived from the machine; the emittance-derived coherent fraction is \
                          simplified.",
                doc: "docs/sources/synchrotron.md",
            },
            Family::Hhg => &StatusInfo {
                badges: &[Implemented, Simplified],
                summary: "Harmonic comb, cutoff law, Keldysh parameter and critical ionization are \
                          exact; the conversion efficiency is an order-of-magnitude estimate.",
                doc: "docs/sources/hhg.md",
            },
            Family::Xfel => &StatusInfo {
                badges: &[Implemented],
                summary: "SASE bandwidth (Pierce parameter derived from the machine), longitudinal-\
                          mode statistics and dose jitter are live; gain length and saturation are \
                          1D + Ming-Xie estimates.",
                doc: "docs/sources/xfel.md",
            },
            Family::Ics => &StatusInfo {
                badges: &[Theoretical],
                summary: "Compton kinematics exact and the photon yield derived, but even the \
                          aggressive design point delivers ~71 \u{b5}W, millions of times below \
                          250 W.",
                doc: "docs/sources/inverse-compton.md",
            },
            Family::Ssmb => &StatusInfo {
                badges: &[Theoretical],
                summary: "Harmonic check and first-principles coherent power are live; the 1 kW \
                          point requires a bunching factor b \u{2248} 0.146 that has never been \
                          demonstrated.",
                doc: "docs/sources/ssmb.md",
            },
            Family::Entangled => &StatusInfo {
                badges: &[Theoretical],
                summary: "Runs end to end but is imaged classically; the opt-in quantum module \
                          sharpens as I^N at the classical period (\u{3bb}/N is only reported); no \
                          entangled sub-Rayleigh pattern has been recorded in a resist.",
                doc: "docs/sources/entangled-photon.md",
            },
            Family::XrayTube => &StatusInfo {
                badges: &[Implemented, Simplified],
                summary: "Kramers continuum, Duane\u{2013}Hunt cutoff, line energies and Be-window \
                          filtration are fixture-tested; absolute flux and line yields are \
                          empirical. Broadband and incoherent: LIGA / proximity only.",
                doc: "docs/sources/xray-tube.md",
            },
            Family::Dpp => &StatusInfo {
                badges: &[Simplified],
                summary: "The electrical power, in-band, collected and \
                          \u{e9}tendue-limited power chain is live; conversion efficiencies are \
                          order-of-magnitude and the pinch/collector geometry is assumed.",
                doc: "docs/sources/dpp.md",
            },
            Family::Sxrl => &StatusInfo {
                badges: &[Implemented, Simplified],
                summary: "Fixed atomic lasing lines, photon number and coherence length are exact; \
                          presets reproduce reported average powers, while pulse duration, \
                          linewidth and coherent fraction are order-of-magnitude assumptions.",
                doc: "docs/sources/soft-xray-laser.md",
            },
            Family::Betatron => &StatusInfo {
                badges: &[Theoretical],
                summary: "Plasma/betatron frequencies, K, critical energy and spectrum are textbook \
                          and fixture-tested; as a lithography source it is a research projection \
                          (\u{b5}W average power). Broadband: LIGA view.",
                doc: "docs/sources/betatron.md",
            },
            Family::SmithPurcell => &StatusInfo {
                badges: &[Theoretical],
                summary: "Dispersion relation, tuning and evanescent height are exact kinematics; \
                          the emitted power is an order-of-magnitude estimate (assumed coupling \
                          1e-3), pW\u{2013}nW.",
                doc: "docs/sources/smith-purcell.md",
            },
        }
    }

    /// How the family's operating wavelength comes about.
    pub fn lambda_origin(self) -> LambdaOrigin {
        match self {
            Family::Vuv => LambdaOrigin::FixedLine,
            Family::LpaFel => {
                LambdaOrigin::Derived("undulator resonance \u{3bb}u(1+K\u{b2}/2)/(2\u{3b3}\u{b2})")
            }
            Family::Lpp => LambdaOrigin::FixedLine,
            Family::Synchrotron => {
                LambdaOrigin::Derived("undulator resonance \u{3bb}u(1+K\u{b2}/2)/(2n\u{3b3}\u{b2})")
            }
            Family::Hhg => LambdaOrigin::Derived("driver wavelength / harmonic order q"),
            Family::Xfel => LambdaOrigin::SetPoint,
            Family::Ics => {
                LambdaOrigin::Derived("Compton upshift \u{3bb}L(1+a0\u{b2}/2)/(4\u{3b3}\u{b2})")
            }
            Family::Ssmb => LambdaOrigin::Derived("modulation wavelength / harmonic h"),
            Family::Entangled => LambdaOrigin::SetPoint,
            Family::XrayTube => LambdaOrigin::Derived(
                "photon-weighted mean of the filtered Kramers + line spectrum",
            ),
            Family::Dpp => LambdaOrigin::FixedLine,
            Family::Sxrl => LambdaOrigin::FixedLine,
            Family::Betatron => {
                LambdaOrigin::Derived("hc / mean photon energy of the synchrotron-like spectrum")
            }
            Family::SmithPurcell => {
                LambdaOrigin::Derived("dispersion \u{3bb} = (a/m)(1/\u{3b2} \u{2212} cos \u{3b8})")
            }
        }
    }

    /// Whether the family's default pupil fill is a conventional disk of the
    /// user's σ (incoherent sources); the others derive their pupil fill
    /// from the beam coherence.
    pub fn default_pupil_uses_sigma(self) -> bool {
        matches!(
            self,
            Family::Vuv | Family::LpaFel | Family::Lpp | Family::Dpp
        )
    }

    /// Presets of the family, in menu order.
    pub fn presets(self) -> impl Iterator<Item = &'static PresetInfo> {
        PRESETS.iter().filter(move |p| p.family == self)
    }

    /// The preset the family's parameters start from.
    pub fn default_preset(self) -> PresetId {
        self.presets()
            .next()
            .map(|p| p.id)
            .expect("every family has a preset")
    }
}

/// Identifier of a preset (each is built from a core factory).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PresetId {
    F2,
    ArF,
    KrF,
    HgI,
    HgH,
    HgG,
    Ar2,
    LpaFel25,
    LpaFel420,
    LppSnCo2,
    LppSn1um,
    LppSn2um,
    LppSn500W,
    LppGd,
    LppTb,
    UndulatorEuv,
    BendingMagnetLiga,
    HhgNe,
    HhgAr,
    XfelFlash,
    XfelFermi,
    XfelCwSc,
    XfelErl,
    IcsCompact,
    SsmbEuv,
    NoonF2,
    TubeW60,
    TubeMo50,
    TubeCu40,
    TubeRh50,
    DppSn,
    DppXe,
    SxrlAg,
    SxrlCd,
    SxrlMo,
    SxrlAr,
    BetatronLwfa,
    SmithPurcellEuv,
}

/// Menu entry of a preset.
#[derive(Debug)]
pub struct PresetInfo {
    pub id: PresetId,
    pub family: Family,
    pub label: &'static str,
    /// Preset-specific badge when it differs from the family's first badge.
    pub badge: Option<Badge>,
    /// Provenance / caveat shown under the preset.
    pub note: &'static str,
    /// Docs page when it is not the family's page.
    pub doc: Option<&'static str>,
}

/// Every preset. The first preset of a family is its default.
pub const PRESETS: &[PresetInfo] = &[
    PresetInfo {
        id: PresetId::F2,
        family: Family::Vuv,
        label: "F\u{2082} 157.63 nm",
        badge: None,
        note: "F\u{2082} excimer laser (10 mJ \u{d7} 4 kHz); the 157 nm program stopped in 2003.",
        doc: None,
    },
    PresetInfo {
        id: PresetId::ArF,
        family: Family::Vuv,
        label: "ArF 193.368 nm",
        badge: Some(Badge::Simplified),
        note:
            "Line-narrowed ArF scanner laser; bandwidth and 15 mJ \u{d7} 6 kHz are representative. \
               Pair with water immersion (193i) for NA > 1.",
        doc: Some("docs/sources/duv-heritage.md"),
    },
    PresetInfo {
        id: PresetId::KrF,
        family: Family::Vuv,
        label: "KrF 248.3 nm",
        badge: Some(Badge::Simplified),
        note:
            "Line-narrowed KrF scanner laser; bandwidth and 10 mJ \u{d7} 4 kHz are representative.",
        doc: Some("docs/sources/duv-heritage.md"),
    },
    PresetInfo {
        id: PresetId::HgI,
        family: Family::Vuv,
        label: "Hg i-line 365.0 nm",
        badge: Some(Badge::Simplified),
        note: "Filtered mercury-lamp line (NIST air wavelength), CW: lamp power is not modeled.",
        doc: Some("docs/sources/duv-heritage.md"),
    },
    PresetInfo {
        id: PresetId::HgH,
        family: Family::Vuv,
        label: "Hg h-line 404.7 nm",
        badge: Some(Badge::Simplified),
        note: "Filtered mercury-lamp line (NIST air wavelength), CW: lamp power is not modeled.",
        doc: Some("docs/sources/duv-heritage.md"),
    },
    PresetInfo {
        id: PresetId::HgG,
        family: Family::Vuv,
        label: "Hg g-line 435.8 nm",
        badge: Some(Badge::Simplified),
        note: "Filtered mercury-lamp line (NIST air wavelength), CW: lamp power is not modeled.",
        doc: Some("docs/sources/duv-heritage.md"),
    },
    PresetInfo {
        id: PresetId::Ar2,
        family: Family::Vuv,
        label: "Ar\u{2082} 126 nm (hypothetical)",
        badge: Some(Badge::Theoretical),
        note:
            "No line-narrowed Ar\u{2082} lithography laser exists; 126 nm lithography never left \
               laboratory discussion.",
        doc: None,
    },
    PresetInfo {
        id: PresetId::LpaFel25,
        family: Family::LpaFel,
        label: "25 nm, 500 MeV (projection)",
        badge: None,
        note: "Design projection on an illustrative LPA-class beam; its percent-level energy \
               spread exceeds the Pierce parameter, so it would not lase as is.",
        doc: None,
    },
    PresetInfo {
        id: PresetId::LpaFel420,
        family: Family::LpaFel,
        label: "420 nm, 100 MeV (BELLA, demonstrated)",
        badge: None,
        note: "Demonstrated LPA-FEL anchor: 420 nm SASE at 1 Hz (not useful for EUV lithography).",
        doc: None,
    },
    PresetInfo {
        id: PresetId::LppSnCo2,
        family: Family::Lpp,
        label: "Sn 13.5 nm, CO\u{2082} drive (NXE:3400B)",
        badge: None,
        note: "21.5 kW CO\u{2082}, 6 % CE, 50 kHz: 250 W in-band at intermediate focus.",
        doc: None,
    },
    PresetInfo {
        id: PresetId::LppSn1um,
        family: Family::Lpp,
        label: "Sn 13.5 nm, 1 \u{b5}m drive",
        badge: Some(Badge::Simplified),
        note: "Same 21.5 kW with the reported 1 \u{b5}m-laser CE (3 %).",
        doc: None,
    },
    PresetInfo {
        id: PresetId::LppSn2um,
        family: Family::Lpp,
        label: "Sn 13.5 nm, 2 \u{b5}m drive (projection)",
        badge: Some(Badge::Simplified),
        note: "2 \u{b5}m thulium drive; its 4.5 % CE is a projection, not a reported value.",
        doc: None,
    },
    PresetInfo {
        id: PresetId::LppSn500W,
        family: Family::Lpp,
        label: "Sn 13.5 nm, 500 W at IF (NXE:3800E class)",
        badge: Some(Badge::Simplified),
        note: "Only the 500 W is sourced (ASML, NXE:3800E); the 43 kW CO\u{2082} drive (2\u{d7} \
               NXE:3400B at the same CE and collection) is an assumed scaling.",
        doc: None,
    },
    PresetInfo {
        id: PresetId::LppGd,
        family: Family::Lpp,
        label: "Gd 6.7 nm BEUV (power projection)",
        badge: Some(Badge::Simplified),
        note:
            "Lab CE (0.7 % in the 0.6 % band) with Sn-calibrated collection; no watt-level 6.x nm \
               source has been demonstrated.",
        doc: None,
    },
    PresetInfo {
        id: PresetId::LppTb,
        family: Family::Lpp,
        label: "Tb 6.5 nm BEUV (power projection)",
        badge: Some(Badge::Simplified),
        note: "Assumed parameters similar to Gd; in-band power is a projection.",
        doc: None,
    },
    PresetInfo {
        id: PresetId::UndulatorEuv,
        family: Family::Synchrotron,
        label: "Compact EUV undulator 13.5 nm",
        badge: None,
        note:
            "538 MeV ring, 20 mm period, K = 1, 100 periods, illustrative compact-ring emittances.",
        doc: None,
    },
    PresetInfo {
        id: PresetId::BendingMagnetLiga,
        family: Family::Synchrotron,
        label: "LIGA bending magnet (2.5 GeV, 1.5 T)",
        badge: None,
        note: "White beam, E_c = 6.23 keV (KIT/ANKA class): a LIGA source, shown in the LIGA view.",
        doc: None,
    },
    PresetInfo {
        id: PresetId::HhgNe,
        family: Family::Hhg,
        label: "Ne, 800 nm, q = 59 (13.56 nm)",
        badge: None,
        note: "4e14 W/cm\u{b2}; an assumed 20 W driver at 10 kHz derives \u{2248} 0.9 \u{b5}W, \
               between the measured 13.5 nm records (0.43\u{2013}1 \u{b5}W).",
        doc: None,
    },
    PresetInfo {
        id: PresetId::HhgAr,
        family: Family::Hhg,
        label: "Ar, 800 nm, q = 27 (29.6 nm)",
        badge: None,
        note: "2e14 W/cm\u{b2}; an assumed 3 W driver at 100 kHz derives 30 \u{b5}W, below the \
               measured record at this photon energy.",
        doc: None,
    },
    PresetInfo {
        id: PresetId::XfelFlash,
        family: Family::Xfel,
        label: "FLASH-like SASE 13.5 nm",
        badge: None,
        note: "Illustrative FLASH-like machine; the Pierce parameter (\u{2248} 2.3e-3) is derived.",
        doc: None,
    },
    PresetInfo {
        id: PresetId::XfelFermi,
        family: Family::Xfel,
        label: "FERMI-like seeded 13.5 nm",
        badge: None,
        note: "Seeded (HGHG-class) 5e-5 bandwidth, 20 \u{b5}J at 50 Hz.",
        doc: None,
    },
    PresetInfo {
        id: PresetId::XfelCwSc,
        family: Family::Xfel,
        label: "CW superconducting linac (projection)",
        badge: Some(Badge::Theoretical),
        note:
            "LCLS-II-like architecture scaled to 13.5 nm at 1 MHz (~135 W); no such machine exists.",
        doc: None,
    },
    PresetInfo {
        id: PresetId::XfelErl,
        family: Family::Xfel,
        label: "ERL EUV-FEL (projection)",
        badge: Some(Badge::Theoretical),
        note: "Energy-recovery-linac design point in the class of KEK's EUV-FEL study (~10 kW).",
        doc: None,
    },
    PresetInfo {
        id: PresetId::IcsCompact,
        family: Family::Ics,
        label: "Compact EUV ICS 13.5 nm",
        badge: None,
        note:
            "1030 nm laser, a0 = 0.1, 100 pC / 10 mJ at 100 MHz (aggressive assumed design point).",
        doc: None,
    },
    PresetInfo {
        id: PresetId::SsmbEuv,
        family: Family::Ssmb,
        label: "1 kW EUV design point",
        badge: None,
        note: "1053 nm modulation, harmonic 78; b chosen so the derived power reproduces 1 kW.",
        doc: None,
    },
    PresetInfo {
        id: PresetId::NoonF2,
        family: Family::Entangled,
        label: "NOON N = 2 at 157.63 nm",
        badge: None,
        note: "Physical photon wavelength 157.63 nm; 1e6 pairs/s.",
        doc: None,
    },
    PresetInfo {
        id: PresetId::TubeW60,
        family: Family::XrayTube,
        label: "W anode, 60 kV / 30 mA",
        badge: None,
        note: "Lab-LIGA design point: hard continuum plus W L lines (no K lines below 69.5 keV).",
        doc: None,
    },
    PresetInfo {
        id: PresetId::TubeMo50,
        family: Family::XrayTube,
        label: "Mo anode, 50 kV / 40 mA",
        badge: None,
        note: "Continuum plus Mo K lines at 17.5 keV.",
        doc: None,
    },
    PresetInfo {
        id: PresetId::TubeCu40,
        family: Family::XrayTube,
        label: "Cu anode, 40 kV / 40 mA",
        badge: None,
        note: "XRD-class tube; Cu K\u{3b1} (8.05 keV) dominates.",
        doc: None,
    },
    PresetInfo {
        id: PresetId::TubeRh50,
        family: Family::XrayTube,
        label: "Rh anode, 50 kV / 40 mA",
        badge: None,
        note: "XRF-class operating point; Rh K lines at 20.2\u{2013}22.7 keV. Illustrative.",
        doc: None,
    },
    PresetInfo {
        id: PresetId::DppSn,
        family: Family::Dpp,
        label: "Sn LDP 13.5 nm",
        badge: None,
        note: "18 kW electrical at 2 % CE: 360 W into 2\u{3c0} (the demonstrated continuous \
               level, TRINITI 2010), \u{2248} 34 W at IF; geometry assumed.",
        doc: None,
    },
    PresetInfo {
        id: PresetId::DppXe,
        family: Family::Dpp,
        label: "Xe DPP 13.5 nm",
        badge: None,
        note: "2 kW electrical, 0.5 % CE: 10 W into 2\u{3c0}, the actinic metrology / \
               inspection class (Energetiq EQ-10 level).",
        doc: None,
    },
    PresetInfo {
        id: PresetId::SxrlAg,
        family: Family::Sxrl,
        label: "Ni-like Ag 13.9 nm",
        badge: None,
        note: "Transient-collisional; 10 \u{b5}J \u{d7} 10 Hz = 0.1 mW, the reported scale \
               (energy/rate split assumed).",
        doc: None,
    },
    PresetInfo {
        id: PresetId::SxrlCd,
        family: Family::Sxrl,
        label: "Ni-like Cd 13.2 nm",
        badge: None,
        note: "Transient-collisional; 0.1 \u{b5}J \u{d7} 10 Hz = 1 \u{b5}W, the reported scale \
               (energy/rate split assumed).",
        doc: None,
    },
    PresetInfo {
        id: PresetId::SxrlMo,
        family: Family::Sxrl,
        label: "Ni-like Mo 18.9 nm",
        badge: None,
        note: "Diode-pumped transient-collisional; 1 \u{b5}J \u{d7} 100 Hz = 0.1 mW \
               (Reagan et al., 2013).",
        doc: None,
    },
    PresetInfo {
        id: PresetId::SxrlAr,
        family: Family::Sxrl,
        label: "Ne-like Ar 46.9 nm (capillary)",
        badge: None,
        note: "Desk-top capillary discharge; 13 \u{b5}J \u{d7} 12 Hz = 0.16 mW (Heinbuch et \
               al., 2005).",
        doc: None,
    },
    PresetInfo {
        id: PresetId::BetatronLwfa,
        family: Family::Betatron,
        label: "100 TW-class LWFA",
        badge: None,
        note: "200 MeV, 1e19 /cm\u{b3}, r\u{3b2} = 1 \u{b5}m: E_c \u{2248} 8 keV; a LIGA source.",
        doc: None,
    },
    PresetInfo {
        id: PresetId::SmithPurcellEuv,
        family: Family::SmithPurcell,
        label: "30 keV, first order at 90\u{b0} (13.5 nm)",
        badge: None,
        note: "Grating period derived (a = 4.43 nm); 10 nA SEM-class beam; pW\u{2013}nW output.",
        doc: None,
    },
];

impl PresetId {
    /// Command-line name (`--preset`).
    pub fn cli_name(self) -> &'static str {
        match self {
            PresetId::F2 => "f2",
            PresetId::ArF => "arf",
            PresetId::KrF => "krf",
            PresetId::HgI => "hg-i",
            PresetId::HgH => "hg-h",
            PresetId::HgG => "hg-g",
            PresetId::Ar2 => "ar2",
            PresetId::LpaFel25 => "lpa-fel-25",
            PresetId::LpaFel420 => "lpa-fel-420",
            PresetId::LppSnCo2 => "lpp-sn",
            PresetId::LppSn1um => "lpp-sn-1um",
            PresetId::LppSn2um => "lpp-sn-2um",
            PresetId::LppSn500W => "lpp-sn-500w",
            PresetId::LppGd => "lpp-gd",
            PresetId::LppTb => "lpp-tb",
            PresetId::UndulatorEuv => "undulator",
            PresetId::BendingMagnetLiga => "bending-magnet",
            PresetId::HhgNe => "hhg-ne",
            PresetId::HhgAr => "hhg-ar",
            PresetId::XfelFlash => "xfel-flash",
            PresetId::XfelFermi => "xfel-fermi",
            PresetId::XfelCwSc => "xfel-cw-sc",
            PresetId::XfelErl => "xfel-erl",
            PresetId::IcsCompact => "ics",
            PresetId::SsmbEuv => "ssmb",
            PresetId::NoonF2 => "noon",
            PresetId::TubeW60 => "tube-w",
            PresetId::TubeMo50 => "tube-mo",
            PresetId::TubeCu40 => "tube-cu",
            PresetId::TubeRh50 => "tube-rh",
            PresetId::DppSn => "dpp-sn",
            PresetId::DppXe => "dpp-xe",
            PresetId::SxrlAg => "sxrl-ag",
            PresetId::SxrlCd => "sxrl-cd",
            PresetId::SxrlMo => "sxrl-mo",
            PresetId::SxrlAr => "sxrl-ar",
            PresetId::BetatronLwfa => "betatron",
            PresetId::SmithPurcellEuv => "smith-purcell",
        }
    }

    /// Parse a `--preset` value.
    pub fn from_cli_name(name: &str) -> Option<PresetId> {
        PRESETS
            .iter()
            .map(|p| p.id)
            .find(|id| id.cli_name() == name)
    }
}

/// Look up a preset's menu entry.
pub fn preset_info(id: PresetId) -> &'static PresetInfo {
    PRESETS
        .iter()
        .find(|p| p.id == id)
        .expect("every PresetId has a PRESETS entry")
}

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

fn positive(name: &str, v: f64) -> Result<(), String> {
    if v.is_finite() && v > 0.0 {
        Ok(())
    } else {
        Err(format!("{name} must be positive and finite (got {v})"))
    }
}

fn fraction(name: &str, v: f64) -> Result<(), String> {
    if v.is_finite() && v > 0.0 && v <= 1.0 {
        Ok(())
    } else {
        Err(format!("{name} must be in (0, 1] (got {v})"))
    }
}

// ---------------------------------------------------------------------------
// Per-family parameters
// ---------------------------------------------------------------------------

/// Excimer laser or filtered Hg lamp line (`VuvSource`).
#[derive(Clone, Debug, PartialEq)]
pub struct VuvParams {
    pub wavelength_nm: f64,
    pub bandwidth_pm: f64,
    pub gaussian_line: bool,
    pub pulse_energy_mj: f64,
    /// 0 = CW lamp.
    pub rep_rate_hz: f64,
    pub spectral_samples: usize,
}

impl VuvParams {
    fn from_core(s: &VuvSource) -> Self {
        Self {
            wavelength_nm: s.wavelength_nm,
            bandwidth_pm: s.bandwidth_pm,
            gaussian_line: matches!(s.spectral_shape, SpectralShape::Gaussian),
            pulse_energy_mj: s.pulse_energy_mj,
            rep_rate_hz: s.rep_rate_hz,
            spectral_samples: s.spectral_samples,
        }
    }

    fn to_core(&self, sigma: f64) -> Result<VuvSource, String> {
        positive("wavelength", self.wavelength_nm)?;
        if !(self.bandwidth_pm.is_finite() && self.bandwidth_pm >= 0.0) {
            return Err("bandwidth must be >= 0".into());
        }
        let mut s = VuvSource::f2_laser(sigma).map_err(err)?;
        s.wavelength_nm = self.wavelength_nm;
        s.bandwidth_pm = self.bandwidth_pm;
        s.spectral_shape = if self.gaussian_line {
            SpectralShape::Gaussian
        } else {
            SpectralShape::Lorentzian
        };
        s.pulse_energy_mj = self.pulse_energy_mj.max(0.0);
        s.rep_rate_hz = self.rep_rate_hz.max(0.0);
        s.spectral_samples = self.spectral_samples.max(1);
        Ok(s)
    }

    /// CW lamp (no pulse metadata).
    pub fn is_cw(&self) -> bool {
        self.rep_rate_hz <= 0.0
    }
}

/// Laser-plasma FEL with its undulator and beam (`LpaFelSource`); the
/// wavelength is DERIVED from the undulator resonance.
#[derive(Clone, Debug, PartialEq)]
pub struct LpaFelParams {
    pub electron_energy_mev: f64,
    pub undulator_period_mm: f64,
    pub undulator_k: f64,
    pub num_periods: usize,
    pub pulse_energy_uj: f64,
    pub rep_rate_hz: f64,
    pub pulse_duration_fs: f64,
    pub peak_current_a: f64,
    pub norm_emittance_um: f64,
    pub energy_spread_rel: f64,
    pub beta_m: f64,
    pub shot_to_shot: f64,
    pub coherence: f64,
    pub spectral_samples: usize,
}

impl LpaFelParams {
    fn from_core(s: &LpaFelSource) -> Self {
        let und = s.undulator.unwrap_or(UndulatorParams {
            period_mm: 20.0,
            k: 1.0,
            num_periods: 200,
        });
        let beam = s.electron_beam.unwrap_or(ElectronBeamParams {
            peak_current_a: 1000.0,
            norm_emittance_um: 0.5,
            energy_spread_rel: 0.01,
            beta_m: 1.0,
        });
        Self {
            electron_energy_mev: s.electron_energy_mev,
            undulator_period_mm: und.period_mm,
            undulator_k: und.k,
            num_periods: und.num_periods,
            pulse_energy_uj: s.pulse_energy_uj,
            rep_rate_hz: s.rep_rate_hz,
            pulse_duration_fs: s.pulse_duration_fs,
            peak_current_a: beam.peak_current_a,
            norm_emittance_um: beam.norm_emittance_um,
            energy_spread_rel: beam.energy_spread_rel,
            beta_m: beam.beta_m,
            shot_to_shot: s.shot_to_shot_stability,
            coherence: s.transverse_coherence_fraction,
            spectral_samples: s.spectral_samples,
        }
    }

    /// On-axis fundamental resonance `λu (1 + K²/2) / (2 γ²)` in nm.
    pub fn resonance_nm(&self) -> f64 {
        physics::undulator_resonance_nm(
            self.undulator_period_mm,
            self.undulator_k,
            physics::gamma_from_mev(self.electron_energy_mev),
            1,
        )
    }

    fn to_core(&self, sigma: f64) -> Result<LpaFelSource, String> {
        positive("electron energy", self.electron_energy_mev)?;
        let lambda = self.resonance_nm();
        let mut s = LpaFelSource::new(lambda, sigma).map_err(err)?;
        s.electron_energy_mev = self.electron_energy_mev;
        s.pulse_energy_uj = self.pulse_energy_uj;
        s.rep_rate_hz = self.rep_rate_hz;
        s.pulse_duration_fs = self.pulse_duration_fs;
        s.shot_to_shot_stability = self.shot_to_shot;
        s.transverse_coherence_fraction = self.coherence.clamp(0.0, 1.0);
        s.spectral_samples = self.spectral_samples.max(1);
        s.with_machine(
            UndulatorParams {
                period_mm: self.undulator_period_mm,
                k: self.undulator_k,
                num_periods: self.num_periods,
            },
            Some(ElectronBeamParams {
                peak_current_a: self.peak_current_a,
                norm_emittance_um: self.norm_emittance_um,
                energy_spread_rel: self.energy_spread_rel,
                beta_m: self.beta_m,
            }),
        )
        .map_err(err)
    }
}

/// Laser-produced plasma (`LppSource`); the fuel fixes the in-band line.
#[derive(Clone, Debug, PartialEq)]
pub struct LppParams {
    pub fuel: LppFuel,
    pub drive_laser: LppDriveLaser,
    pub drive_power_w: f64,
    pub conversion_efficiency: f64,
    pub transport_efficiency: f64,
    pub rep_rate_hz: f64,
    pub spectral_samples: usize,
}

impl LppParams {
    fn from_core(s: &LppSource) -> Self {
        Self {
            fuel: s.fuel,
            drive_laser: s.drive_laser.unwrap_or(LppDriveLaser::Co2),
            drive_power_w: s.drive_laser_power_w,
            conversion_efficiency: s.conversion_efficiency,
            transport_efficiency: s.transport_efficiency,
            rep_rate_hz: s.rep_rate_hz,
            spectral_samples: s.spectral_samples,
        }
    }

    fn to_core(&self, sigma: f64) -> Result<LppSource, String> {
        positive("drive power", self.drive_power_w)?;
        fraction("conversion efficiency", self.conversion_efficiency)?;
        fraction("2\u{3c0}-to-IF efficiency", self.transport_efficiency)?;
        positive("repetition rate", self.rep_rate_hz)?;
        let mut s = match self.fuel {
            LppFuel::Sn => LppSource::sn_with_drive_laser(sigma, self.drive_laser),
            LppFuel::Gd => LppSource::gd_6nm7(sigma),
            LppFuel::Tb => LppSource::tb_6nm5(sigma),
        }
        .map_err(err)?;
        s.drive_laser = Some(self.drive_laser);
        s.drive_laser_power_w = self.drive_power_w;
        s.conversion_efficiency = self.conversion_efficiency;
        s.transport_efficiency = self.transport_efficiency;
        s.rep_rate_hz = self.rep_rate_hz;
        s.spectral_samples = self.spectral_samples.max(1);
        Ok(s)
    }
}

/// Storage-ring source (`SynchrotronSource`): undulator (wavelength derived
/// from the resonance) or bending magnet (white beam → LIGA view).
#[derive(Clone, Debug, PartialEq)]
pub struct SynchrotronParams {
    pub bending_magnet: bool,
    pub energy_gev: f64,
    pub period_mm: f64,
    pub k: f64,
    pub num_periods: usize,
    pub harmonic: usize,
    pub field_t: f64,
    pub ring_current_ma: f64,
    /// Derive the coherent fraction from the stored-beam emittances.
    pub use_ring_beam: bool,
    pub emittance_x_nm_rad: f64,
    pub emittance_y_nm_rad: f64,
    pub energy_spread_rel: f64,
    pub spectral_samples: usize,
}

impl SynchrotronParams {
    fn from_core(s: &SynchrotronSource, into: Option<&SynchrotronParams>) -> Self {
        // Keep the other beamline's settings when switching presets.
        let mut p = into.cloned().unwrap_or(Self {
            bending_magnet: false,
            energy_gev: 0.538,
            period_mm: 20.0,
            k: 1.0,
            num_periods: 100,
            harmonic: 1,
            field_t: 1.5,
            ring_current_ma: 200.0,
            use_ring_beam: true,
            emittance_x_nm_rad: 10.0,
            emittance_y_nm_rad: 0.1,
            energy_spread_rel: 5e-4,
            spectral_samples: 5,
        });
        match s.beamline {
            SynchrotronBeamline::Undulator {
                electron_energy_gev,
                period_mm,
                k,
                num_periods,
                harmonic,
            } => {
                p.bending_magnet = false;
                p.energy_gev = electron_energy_gev;
                p.period_mm = period_mm;
                p.k = k;
                p.num_periods = num_periods;
                p.harmonic = harmonic;
            }
            SynchrotronBeamline::BendingMagnet {
                electron_energy_gev,
                field_t,
                ..
            } => {
                p.bending_magnet = true;
                p.energy_gev = electron_energy_gev;
                p.field_t = field_t;
            }
        }
        p.ring_current_ma = s.ring_current_ma;
        p.use_ring_beam = s.ring_beam.is_some();
        if let Some(beam) = s.ring_beam {
            p.emittance_x_nm_rad = beam.emittance_x_nm_rad;
            p.emittance_y_nm_rad = beam.emittance_y_nm_rad;
            p.energy_spread_rel = beam.energy_spread_rel;
        }
        p.spectral_samples = s.spectral_samples;
        p
    }

    fn to_core(&self) -> Result<SynchrotronSource, String> {
        positive("ring energy", self.energy_gev)?;
        positive("ring current", self.ring_current_ma)?;
        let mut s = if self.bending_magnet {
            positive("bending field", self.field_t)?;
            let mut s = SynchrotronSource::liga_bending_magnet();
            if let SynchrotronBeamline::BendingMagnet {
                electron_energy_gev,
                field_t,
                ..
            } = &mut s.beamline
            {
                *electron_energy_gev = self.energy_gev;
                *field_t = self.field_t;
            }
            s
        } else {
            SynchrotronSource::undulator(
                self.energy_gev,
                self.period_mm,
                self.k,
                self.num_periods,
                self.harmonic,
            )
            .map_err(err)?
        };
        s.ring_current_ma = self.ring_current_ma;
        s.spectral_samples = self.spectral_samples.max(1);
        if self.use_ring_beam && !self.bending_magnet {
            positive("horizontal emittance", self.emittance_x_nm_rad)?;
            positive("vertical emittance", self.emittance_y_nm_rad)?;
            s = s.with_ring_beam(StorageRingBeam {
                emittance_x_nm_rad: self.emittance_x_nm_rad,
                emittance_y_nm_rad: self.emittance_y_nm_rad,
                energy_spread_rel: self.energy_spread_rel.max(0.0),
            });
        }
        Ok(s)
    }
}

/// Generation gases, in menu order.
pub const HHG_GASES: [HhgGas; 5] = [
    HhgGas::Helium,
    HhgGas::Neon,
    HhgGas::Argon,
    HhgGas::Krypton,
    HhgGas::Xenon,
];

/// Display name of a generation gas.
pub fn gas_label(gas: HhgGas) -> &'static str {
    match gas {
        HhgGas::Helium => "He",
        HhgGas::Neon => "Ne",
        HhgGas::Argon => "Ar",
        HhgGas::Krypton => "Kr",
        HhgGas::Xenon => "Xe",
    }
}

/// High-harmonic generation (`HhgSource`); wavelength = driver / q.
#[derive(Clone, Debug, PartialEq)]
pub struct HhgParams {
    pub driver_wavelength_nm: f64,
    pub gas: HhgGas,
    pub intensity_w_cm2: f64,
    pub harmonic: usize,
    pub mono_bandwidth_pm: f64,
    /// Driver average power in W: `Some` DERIVES the harmonic power as
    /// `P_driver × η_q` (core default); `None` uses `pulse_energy_nj`.
    pub driver_power_w: Option<f64>,
    /// Stored in-band pulse energy, used only when `driver_power_w` is `None`.
    pub pulse_energy_nj: f64,
    pub rep_rate_hz: f64,
    /// Full comb (no monochromator) instead of one selected harmonic.
    pub full_comb: bool,
    /// Optional band-pass `[min, max]` nm applied to the full comb.
    pub passband_nm: Option<[f64; 2]>,
    pub spectral_samples: usize,
}

impl HhgParams {
    fn from_core(s: &HhgSource) -> Self {
        let (harmonic, bw) = s
            .monochromator
            .as_ref()
            .map(|m| (m.harmonic, m.bandwidth_pm))
            .unwrap_or((59, 15.0));
        Self {
            driver_wavelength_nm: s.driver_wavelength_nm,
            gas: s.gas,
            intensity_w_cm2: s.driver_intensity_w_cm2,
            harmonic,
            mono_bandwidth_pm: bw,
            driver_power_w: s.driver_average_power_w,
            pulse_energy_nj: s.pulse_energy_nj,
            rep_rate_hz: s.rep_rate_hz,
            full_comb: s.monochromator.is_none(),
            passband_nm: s.comb_passband_nm,
            spectral_samples: s.spectral_samples,
        }
    }

    fn to_core(&self) -> Result<HhgSource, String> {
        let mut s = HhgSource::new(
            self.driver_wavelength_nm,
            self.gas,
            self.intensity_w_cm2,
            self.harmonic,
            self.mono_bandwidth_pm,
        )
        .map_err(err)?;
        s.pulse_energy_nj = self.pulse_energy_nj;
        s.rep_rate_hz = self.rep_rate_hz;
        if let Some(p) = self.driver_power_w {
            positive("driver average power", p)?;
        }
        s.driver_average_power_w = self.driver_power_w;
        s.spectral_samples = self.spectral_samples.max(1);
        if self.full_comb {
            s.monochromator = None;
            s.comb_passband_nm = self.passband_nm;
            if s.comb_lines().is_empty() {
                return Err("no harmonic of the comb lies inside the pass band".into());
            }
        }
        Ok(s)
    }
}

/// FEL (`XfelSource`) with its machine: the wavelength is a set-point and
/// the undulator K is gap-tuned to it (derived).
#[derive(Clone, Debug, PartialEq)]
pub struct XfelParams {
    pub seeded: bool,
    pub wavelength_nm: f64,
    pub electron_energy_mev: f64,
    pub undulator_period_mm: f64,
    pub undulator_length_m: f64,
    pub peak_current_a: f64,
    pub norm_emittance_um: f64,
    pub energy_spread_rel: f64,
    pub beta_m: f64,
    pub pulse_energy_uj: f64,
    pub pulse_duration_fs: f64,
    pub rep_rate_hz: f64,
    /// Seeded relative bandwidth.
    pub seeded_rel_bandwidth: f64,
    /// Stored SASE Pierce parameter (fallback only; derived from the machine).
    pub sase_rho_fallback: f64,
    pub coherence: f64,
    pub spectral_samples: usize,
}

impl XfelParams {
    fn from_core(s: &XfelSource) -> Self {
        let m = s.machine.unwrap_or(FelMachine {
            electron_energy_mev: 680.0,
            undulator_period_mm: 31.4,
            undulator_length_m: 30.0,
            beam: ElectronBeamParams {
                peak_current_a: 2500.0,
                norm_emittance_um: 1.5,
                energy_spread_rel: 7e-4,
                beta_m: 6.0,
            },
        });
        let (seeded, seeded_rel_bandwidth, sase_rho_fallback) = match s.mode {
            XfelMode::Sase { pierce_parameter } => (false, 5e-5, pierce_parameter),
            XfelMode::SelfSeeded { rel_bandwidth } => (true, rel_bandwidth, 3e-3),
        };
        Self {
            seeded,
            wavelength_nm: s.wavelength_nm,
            electron_energy_mev: m.electron_energy_mev,
            undulator_period_mm: m.undulator_period_mm,
            undulator_length_m: m.undulator_length_m,
            peak_current_a: m.beam.peak_current_a,
            norm_emittance_um: m.beam.norm_emittance_um,
            energy_spread_rel: m.beam.energy_spread_rel,
            beta_m: m.beam.beta_m,
            pulse_energy_uj: s.pulse_energy_uj,
            pulse_duration_fs: s.pulse_duration_fs,
            rep_rate_hz: s.rep_rate_hz,
            seeded_rel_bandwidth,
            sase_rho_fallback,
            coherence: s.transverse_coherence_fraction,
            spectral_samples: s.spectral_samples,
        }
    }

    fn to_core(&self) -> Result<XfelSource, String> {
        positive("wavelength set-point", self.wavelength_nm)?;
        positive("electron energy", self.electron_energy_mev)?;
        positive("undulator period", self.undulator_period_mm)?;
        positive("undulator length", self.undulator_length_m)?;
        positive("peak current", self.peak_current_a)?;
        positive("normalized emittance", self.norm_emittance_um)?;
        positive("beta function", self.beta_m)?;
        let coherence = self.coherence.clamp(0.01, 1.0);
        let s = XfelSource {
            mode: if self.seeded {
                XfelMode::SelfSeeded {
                    rel_bandwidth: self.seeded_rel_bandwidth,
                }
            } else {
                XfelMode::Sase {
                    pierce_parameter: self.sase_rho_fallback,
                }
            },
            wavelength_nm: self.wavelength_nm,
            pulse_energy_uj: self.pulse_energy_uj,
            pulse_duration_fs: self.pulse_duration_fs,
            rep_rate_hz: self.rep_rate_hz,
            transverse_coherence_fraction: coherence,
            spectral_samples: self.spectral_samples.max(1),
            illumination: IlluminationShape::CoherentGaussian {
                sigma: sigma_from_coherence(coherence, 0.05),
            },
            sase_spike_seed: None,
            machine: Some(FelMachine {
                electron_energy_mev: self.electron_energy_mev,
                undulator_period_mm: self.undulator_period_mm,
                undulator_length_m: self.undulator_length_m,
                beam: ElectronBeamParams {
                    peak_current_a: self.peak_current_a,
                    norm_emittance_um: self.norm_emittance_um,
                    energy_spread_rel: self.energy_spread_rel.max(0.0),
                    beta_m: self.beta_m,
                },
            }),
        };
        if s.undulator_k().is_none() {
            return Err(format!(
                "the {:.2} nm set-point is shorter than the K = 0 resonance of this machine \
                 (raise the electron energy or shorten the period)",
                self.wavelength_nm
            ));
        }
        Ok(s)
    }
}

/// Inverse Compton scattering (`IcsSource`) with its collision; the
/// wavelength is DERIVED from the electron energy and laser.
#[derive(Clone, Debug, PartialEq)]
pub struct IcsParams {
    pub electron_energy_mev: f64,
    pub laser_wavelength_nm: f64,
    pub laser_a0: f64,
    pub collection_half_angle_mrad: f64,
    pub energy_spread_rel: f64,
    pub rep_rate_hz: f64,
    pub bunch_charge_pc: f64,
    pub laser_pulse_energy_mj: f64,
    pub electron_spot_um: f64,
    pub laser_spot_um: f64,
    pub spectral_samples: usize,
}

impl IcsParams {
    fn from_core(s: &IcsSource) -> Self {
        let c = s
            .collision
            .unwrap_or_else(IcsCollision::high_average_power_design);
        Self {
            electron_energy_mev: s.electron_energy_mev,
            laser_wavelength_nm: s.laser_wavelength_nm,
            laser_a0: s.laser_a0,
            collection_half_angle_mrad: s.collection_half_angle_mrad,
            energy_spread_rel: s.electron_energy_spread_rel,
            rep_rate_hz: s.rep_rate_hz,
            bunch_charge_pc: c.bunch_charge_pc,
            laser_pulse_energy_mj: c.laser_pulse_energy_mj,
            electron_spot_um: c.electron_spot_um,
            laser_spot_um: c.laser_spot_um,
            spectral_samples: s.spectral_samples,
        }
    }

    fn to_core(&self) -> Result<IcsSource, String> {
        positive("electron energy", self.electron_energy_mev)?;
        positive("laser wavelength", self.laser_wavelength_nm)?;
        positive("collection half-angle", self.collection_half_angle_mrad)?;
        positive("repetition rate", self.rep_rate_hz)?;
        if !(self.laser_a0.is_finite() && self.laser_a0 >= 0.0) {
            return Err("laser a0 must be >= 0".into());
        }
        let mut s = IcsSource::compact_euv_13nm5().map_err(err)?;
        s.electron_energy_mev = self.electron_energy_mev;
        s.laser_wavelength_nm = self.laser_wavelength_nm;
        s.laser_a0 = self.laser_a0;
        s.collection_half_angle_mrad = self.collection_half_angle_mrad;
        s.electron_energy_spread_rel = self.energy_spread_rel.max(0.0);
        s.rep_rate_hz = self.rep_rate_hz;
        s.spectral_samples = self.spectral_samples.max(1);
        s.with_collision(IcsCollision {
            bunch_charge_pc: self.bunch_charge_pc,
            laser_pulse_energy_mj: self.laser_pulse_energy_mj,
            electron_spot_um: self.electron_spot_um,
            laser_spot_um: self.laser_spot_um,
        })
        .map_err(err)
    }
}

/// Steady-state microbunching ring (`SsmbSource`); wavelength =
/// modulation wavelength / harmonic, power DERIVED from the radiator.
#[derive(Clone, Debug, PartialEq)]
pub struct SsmbParams {
    pub ring_energy_mev: f64,
    pub modulation_wavelength_nm: f64,
    pub harmonic: usize,
    pub average_current_a: f64,
    pub peak_current_a: f64,
    pub bunching_factor: f64,
    pub radiator_periods: usize,
    pub radiator_k: f64,
    /// Stored design power (superseded by the derived coherent power).
    pub stored_power_w: f64,
    pub spectral_samples: usize,
}

impl SsmbParams {
    fn from_core(s: &SsmbSource) -> Self {
        let r = s.radiator.unwrap_or(SsmbRadiator {
            average_current_a: 1.0,
            peak_current_a: 1.0,
            bunching_factor: 0.146,
            num_periods: 100,
            k: 1.6,
        });
        Self {
            ring_energy_mev: s.ring_energy_mev,
            modulation_wavelength_nm: s.modulation_wavelength_nm,
            harmonic: s.harmonic(),
            average_current_a: r.average_current_a,
            peak_current_a: r.peak_current_a,
            bunching_factor: r.bunching_factor,
            radiator_periods: r.num_periods,
            radiator_k: r.k,
            stored_power_w: s.average_power_w,
            spectral_samples: s.spectral_samples,
        }
    }

    /// Target wavelength (nm).
    pub fn wavelength_nm(&self) -> f64 {
        self.modulation_wavelength_nm / self.harmonic.max(1) as f64
    }

    fn to_core(&self) -> Result<SsmbSource, String> {
        positive("ring energy", self.ring_energy_mev)?;
        positive("modulation wavelength", self.modulation_wavelength_nm)?;
        let mut s = SsmbSource::new(
            self.ring_energy_mev,
            self.modulation_wavelength_nm,
            self.wavelength_nm(),
            self.stored_power_w,
        )
        .map_err(err)?;
        s.spectral_samples = self.spectral_samples.max(1);
        s.with_radiator(SsmbRadiator {
            average_current_a: self.average_current_a,
            peak_current_a: self.peak_current_a.max(self.average_current_a),
            bunching_factor: self.bunching_factor,
            num_periods: self.radiator_periods,
            k: self.radiator_k,
        })
        .map_err(err)
    }
}

/// N-photon NOON source (`EntangledPhotonSource`).
#[derive(Clone, Debug, PartialEq)]
pub struct EntangledParams {
    pub wavelength_nm: f64,
    pub n_photons: usize,
    pub fidelity: f64,
    pub pair_rate_hz: f64,
    pub spectral_samples: usize,
}

impl EntangledParams {
    fn from_core(s: &EntangledPhotonSource) -> Self {
        Self {
            wavelength_nm: s.wavelength_nm,
            n_photons: s.num_entangled_photons,
            fidelity: s.fidelity,
            pair_rate_hz: s.pair_rate_hz,
            spectral_samples: s.spectral_samples,
        }
    }

    fn to_core(&self) -> Result<EntangledPhotonSource, String> {
        let mut s = EntangledPhotonSource::noon(self.wavelength_nm, self.n_photons, self.fidelity)
            .map_err(err)?;
        s.pair_rate_hz = self.pair_rate_hz.max(0.0);
        s.spectral_samples = self.spectral_samples.max(1);
        Ok(s)
    }
}

/// Anodes, in menu order.
pub const XRAY_ANODES: [XrayAnode; 4] = [XrayAnode::W, XrayAnode::Mo, XrayAnode::Cu, XrayAnode::Rh];

/// Hard X-ray tube (`XrayTubeSource`) — a LIGA / proximity source.
#[derive(Clone, Debug, PartialEq)]
pub struct XrayTubeParams {
    pub anode: XrayAnode,
    pub kvp: f64,
    pub current_ma: f64,
    pub be_window_um: f64,
    pub spectral_samples: usize,
}

impl XrayTubeParams {
    fn from_core(s: &XrayTubeSource) -> Self {
        Self {
            anode: s.anode,
            kvp: s.kvp,
            current_ma: s.current_ma,
            be_window_um: s.be_window_um,
            spectral_samples: s.spectral_samples,
        }
    }

    pub(crate) fn to_core(&self) -> Result<XrayTubeSource, String> {
        let mut s = XrayTubeSource::new(self.anode, self.kvp, self.current_ma).map_err(err)?;
        s.be_window_um = self.be_window_um;
        s.spectral_samples = self.spectral_samples.max(1);
        s.validate().map_err(err)?;
        Ok(s)
    }
}

/// Discharge-produced / laser-assisted discharge plasma (`DppSource`).
#[derive(Clone, Debug, PartialEq)]
pub struct DppParams {
    pub fuel: DppFuel,
    pub electrical_power_w: f64,
    pub conversion_efficiency: f64,
    pub source_diameter_mm: f64,
    pub source_length_mm: f64,
    pub collector_solid_angle_sr: f64,
    pub collector_efficiency: f64,
    pub illuminator_etendue_mm2_sr: f64,
    pub rep_rate_hz: f64,
    pub shot_to_shot_rms: f64,
    pub spectral_samples: usize,
}

impl DppParams {
    fn from_core(s: &DppSource) -> Self {
        Self {
            fuel: s.fuel,
            electrical_power_w: s.electrical_power_w,
            conversion_efficiency: s.conversion_efficiency,
            source_diameter_mm: s.source_diameter_mm,
            source_length_mm: s.source_length_mm,
            collector_solid_angle_sr: s.collector_solid_angle_sr,
            collector_efficiency: s.collector_efficiency,
            illuminator_etendue_mm2_sr: s.illuminator_etendue_mm2_sr,
            rep_rate_hz: s.rep_rate_hz,
            shot_to_shot_rms: s.shot_to_shot_rms,
            spectral_samples: s.spectral_samples,
        }
    }

    fn to_core(&self, sigma: f64) -> Result<DppSource, String> {
        let mut s = match self.fuel {
            DppFuel::Xe => DppSource::xe_13nm5(sigma),
            DppFuel::Sn => DppSource::sn_13nm5(sigma),
        }
        .map_err(err)?;
        s.electrical_power_w = self.electrical_power_w;
        s.conversion_efficiency = self.conversion_efficiency;
        s.source_diameter_mm = self.source_diameter_mm;
        s.source_length_mm = self.source_length_mm;
        s.collector_solid_angle_sr = self.collector_solid_angle_sr;
        s.collector_efficiency = self.collector_efficiency;
        s.illuminator_etendue_mm2_sr = self.illuminator_etendue_mm2_sr;
        s.rep_rate_hz = self.rep_rate_hz;
        s.shot_to_shot_rms = self.shot_to_shot_rms;
        s.spectral_samples = self.spectral_samples.max(1);
        s.validate().map_err(err)?;
        Ok(s)
    }
}

/// Lasing schemes, in menu order.
pub const SXRL_SCHEMES: [SxrlScheme; 4] = [
    SxrlScheme::NiLikeAg,
    SxrlScheme::NiLikeCd,
    SxrlScheme::NiLikeMo,
    SxrlScheme::CapillaryNeLikeAr,
];

/// Plasma soft-X-ray laser (`SxrlSource`); the scheme fixes the line.
#[derive(Clone, Debug, PartialEq)]
pub struct SxrlParams {
    pub scheme: SxrlScheme,
    pub pulse_energy_uj: f64,
    pub rep_rate_hz: f64,
    pub pulse_duration_ps: f64,
    pub coherence: f64,
    pub rel_linewidth: f64,
    pub spectral_samples: usize,
}

impl SxrlParams {
    fn from_core(s: &SxrlSource) -> Self {
        Self {
            scheme: s.scheme,
            pulse_energy_uj: s.pulse_energy_uj,
            rep_rate_hz: s.rep_rate_hz,
            pulse_duration_ps: s.pulse_duration_ps,
            coherence: s.transverse_coherence_fraction,
            rel_linewidth: s.rel_linewidth,
            spectral_samples: s.spectral_samples,
        }
    }

    fn to_core(&self) -> Result<SxrlSource, String> {
        let mut s = SxrlSource::new(
            self.scheme,
            self.pulse_energy_uj,
            self.rep_rate_hz,
            self.pulse_duration_ps,
            self.coherence,
        )
        .map_err(err)?;
        s.rel_linewidth = self.rel_linewidth;
        s.spectral_samples = self.spectral_samples.max(1);
        s.validate().map_err(err)?;
        Ok(s)
    }
}

/// Laser-wakefield betatron source (`BetatronSource`) — a LIGA source.
#[derive(Clone, Debug, PartialEq)]
pub struct BetatronParams {
    pub electron_energy_mev: f64,
    pub plasma_density_cm3: f64,
    pub betatron_amplitude_um: f64,
    pub interaction_length_mm: f64,
    pub bunch_charge_pc: f64,
    pub rep_rate_hz: f64,
    pub spectral_samples: usize,
}

impl BetatronParams {
    fn from_core(s: &BetatronSource) -> Self {
        Self {
            electron_energy_mev: s.electron_energy_mev,
            plasma_density_cm3: s.plasma_density_cm3,
            betatron_amplitude_um: s.betatron_amplitude_um,
            interaction_length_mm: s.interaction_length_mm,
            bunch_charge_pc: s.bunch_charge_pc,
            rep_rate_hz: s.rep_rate_hz,
            spectral_samples: s.spectral_samples,
        }
    }

    pub(crate) fn to_core(&self) -> Result<BetatronSource, String> {
        let mut s = BetatronSource::new(
            self.electron_energy_mev,
            self.plasma_density_cm3,
            self.betatron_amplitude_um,
            self.interaction_length_mm,
            self.bunch_charge_pc,
            self.rep_rate_hz,
        )
        .map_err(err)?;
        s.spectral_samples = self.spectral_samples.max(1);
        Ok(s)
    }
}

/// Smith–Purcell free-electron grating (`SmithPurcellSource`); wavelength
/// DERIVED from the dispersion relation.
#[derive(Clone, Debug, PartialEq)]
pub struct SmithPurcellParams {
    pub electron_energy_kev: f64,
    pub grating_period_nm: f64,
    pub diffraction_order: usize,
    pub observation_angle_deg: f64,
    pub num_periods: usize,
    pub beam_current_na: f64,
    pub impact_height_nm: f64,
    pub coupling_efficiency: f64,
    pub spectral_samples: usize,
}

impl SmithPurcellParams {
    fn from_core(s: &SmithPurcellSource) -> Self {
        Self {
            electron_energy_kev: s.electron_energy_kev,
            grating_period_nm: s.grating_period_nm,
            diffraction_order: s.diffraction_order,
            observation_angle_deg: s.observation_angle_deg,
            num_periods: s.num_periods,
            beam_current_na: s.beam_current_na,
            impact_height_nm: s.impact_height_nm,
            coupling_efficiency: s.coupling_efficiency,
            spectral_samples: s.spectral_samples,
        }
    }

    fn to_core(&self) -> Result<SmithPurcellSource, String> {
        let mut s = SmithPurcellSource::new(
            self.electron_energy_kev,
            self.grating_period_nm,
            self.diffraction_order,
            self.observation_angle_deg,
        )
        .map_err(err)?;
        s.num_periods = self.num_periods;
        s.beam_current_na = self.beam_current_na;
        s.impact_height_nm = self.impact_height_nm;
        s.coupling_efficiency = self.coupling_efficiency;
        s.spectral_samples = self.spectral_samples.max(1);
        s.validate().map_err(err)?;
        Ok(s)
    }
}

// ---------------------------------------------------------------------------
// Illumination (pupil fill)
// ---------------------------------------------------------------------------

/// Pupil-fill choice.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum IllumMode {
    /// The family's own fill: a conventional disk of σ for incoherent
    /// sources, a coherence-derived Gaussian for beams.
    #[default]
    SourceDefault,
    Conventional,
    Annular,
    Dipole,
    Quadrupole,
    CoherentGaussian,
}

impl IllumMode {
    /// All modes, in menu order.
    pub const ALL: [IllumMode; 6] = [
        IllumMode::SourceDefault,
        IllumMode::Conventional,
        IllumMode::Annular,
        IllumMode::Dipole,
        IllumMode::Quadrupole,
        IllumMode::CoherentGaussian,
    ];

    /// Menu label.
    pub fn label(self) -> &'static str {
        match self {
            IllumMode::SourceDefault => "Source default",
            IllumMode::Conventional => "Conventional (disk)",
            IllumMode::Annular => "Annular",
            IllumMode::Dipole => "Dipole",
            IllumMode::Quadrupole => "Quadrupole",
            IllumMode::CoherentGaussian => "Coherent Gaussian",
        }
    }
}

/// Pupil-fill parameters (σ in units of NA).
#[derive(Clone, Debug, PartialEq)]
pub struct IlluminationParams {
    pub mode: IllumMode,
    /// Disk σ (also the default pupil of incoherent families) or Gaussian σ.
    pub sigma: f64,
    pub sigma_inner: f64,
    pub sigma_outer: f64,
    /// Pole centre (dipole / quadrupole).
    pub pole_center: f64,
    /// Pole radius (dipole / quadrupole).
    pub pole_radius: f64,
    /// Dipole orientation / quadrupole opening angle, degrees.
    pub angle_deg: f64,
}

impl Default for IlluminationParams {
    fn default() -> Self {
        Self {
            mode: IllumMode::SourceDefault,
            sigma: 0.7,
            sigma_inner: 0.5,
            sigma_outer: 0.8,
            pole_center: 0.7,
            pole_radius: 0.15,
            angle_deg: 0.0,
        }
    }
}

impl IlluminationParams {
    /// Explicit pupil fill for an override mode (`None` = source default).
    pub fn override_shape(&self) -> Result<Option<IlluminationShape>, String> {
        let in_pupil = |name: &str, v: f64| -> Result<(), String> {
            if v.is_finite() && v > 0.0 && v <= 1.0 {
                Ok(())
            } else {
                Err(format!("{name} must be in (0, 1] (got {v})"))
            }
        };
        Ok(match self.mode {
            IllumMode::SourceDefault => None,
            IllumMode::Conventional => {
                in_pupil("\u{3c3}", self.sigma)?;
                Some(IlluminationShape::Conventional { sigma: self.sigma })
            }
            IllumMode::CoherentGaussian => {
                in_pupil("\u{3c3}", self.sigma)?;
                Some(IlluminationShape::CoherentGaussian { sigma: self.sigma })
            }
            IllumMode::Annular => {
                in_pupil("\u{3c3} outer", self.sigma_outer)?;
                if !(self.sigma_inner >= 0.0 && self.sigma_inner < self.sigma_outer) {
                    return Err(
                        "annular fill needs 0 \u{2264} \u{3c3} inner < \u{3c3} outer".into(),
                    );
                }
                Some(IlluminationShape::Annular {
                    sigma_inner: self.sigma_inner,
                    sigma_outer: self.sigma_outer,
                })
            }
            IllumMode::Dipole => {
                in_pupil("pole centre", self.pole_center)?;
                in_pupil("pole radius", self.pole_radius)?;
                Some(IlluminationShape::Dipole {
                    sigma_center: self.pole_center,
                    sigma_radius: self.pole_radius,
                    orientation_deg: self.angle_deg,
                })
            }
            IllumMode::Quadrupole => {
                in_pupil("pole centre", self.pole_center)?;
                in_pupil("pole radius", self.pole_radius)?;
                Some(IlluminationShape::Quadrupole {
                    sigma_center: self.pole_center,
                    sigma_radius: self.pole_radius,
                    opening_angle_deg: self.angle_deg.clamp(1.0, 90.0),
                })
            }
        })
    }
}

fn set_illumination(kind: &mut SourceKind, shape: IlluminationShape) {
    match kind {
        SourceKind::Vuv(s) => s.illumination = shape,
        SourceKind::LpaFel(s) => s.illumination = shape,
        SourceKind::Lpp(s) => s.illumination = shape,
        SourceKind::Synchrotron(s) => s.illumination = shape,
        SourceKind::Hhg(s) => s.illumination = shape,
        SourceKind::Xfel(s) => s.illumination = shape,
        SourceKind::Ics(s) => s.illumination = shape,
        SourceKind::Ssmb(s) => s.illumination = shape,
        SourceKind::Entangled(s) => s.illumination = shape,
        SourceKind::XrayTube(s) => s.illumination = shape,
        SourceKind::Dpp(s) => s.illumination = shape,
        SourceKind::Sxrl(s) => s.illumination = shape,
        SourceKind::Betatron(s) => s.illumination = shape,
        SourceKind::SmithPurcell(s) => s.illumination = shape,
    }
}

fn set_spectral_samples(kind: &mut SourceKind, n: usize) {
    let n = n.max(1);
    match kind {
        SourceKind::Vuv(s) => s.spectral_samples = n,
        SourceKind::LpaFel(s) => s.spectral_samples = n,
        SourceKind::Lpp(s) => s.spectral_samples = n,
        SourceKind::Synchrotron(s) => s.spectral_samples = n,
        SourceKind::Hhg(s) => s.spectral_samples = n,
        SourceKind::Xfel(s) => s.spectral_samples = n,
        SourceKind::Ics(s) => s.spectral_samples = n,
        SourceKind::Ssmb(s) => s.spectral_samples = n,
        SourceKind::Entangled(s) => s.spectral_samples = n,
        SourceKind::XrayTube(s) => s.spectral_samples = n,
        SourceKind::Dpp(s) => s.spectral_samples = n,
        SourceKind::Sxrl(s) => s.spectral_samples = n,
        SourceKind::Betatron(s) => s.spectral_samples = n,
        SourceKind::SmithPurcell(s) => s.spectral_samples = n,
    }
}

// ---------------------------------------------------------------------------
// All source parameters
// ---------------------------------------------------------------------------

/// Everything the Source panel edits.
#[derive(Clone, Debug, PartialEq)]
pub struct SourceParams {
    pub family: Family,
    /// Preset last applied (presets are starting points: later slider edits
    /// are not tracked here).
    pub preset: PresetId,
    pub illumination: IlluminationParams,
    pub vuv: VuvParams,
    pub lpa_fel: LpaFelParams,
    pub lpp: LppParams,
    pub synchrotron: SynchrotronParams,
    pub hhg: HhgParams,
    pub xfel: XfelParams,
    pub ics: IcsParams,
    pub ssmb: SsmbParams,
    pub entangled: EntangledParams,
    pub xray_tube: XrayTubeParams,
    pub dpp: DppParams,
    pub sxrl: SxrlParams,
    pub betatron: BetatronParams,
    pub smith_purcell: SmithPurcellParams,
}

impl Default for SourceParams {
    fn default() -> Self {
        let sigma = IlluminationParams::default().sigma;
        let expect = "core preset is valid";
        Self {
            family: Family::Vuv,
            preset: PresetId::F2,
            illumination: IlluminationParams::default(),
            vuv: VuvParams::from_core(&VuvSource::f2_laser(sigma).expect(expect)),
            lpa_fel: LpaFelParams::from_core(
                &LpaFelSource::bella_target_25nm(sigma).expect(expect),
            ),
            lpp: LppParams::from_core(&LppSource::sn_13nm5(sigma).expect(expect)),
            synchrotron: SynchrotronParams::from_core(
                &SynchrotronSource::compact_euv_undulator().expect(expect),
                None,
            ),
            hhg: HhgParams::from_core(&HhgSource::ne_800nm_13nm5().expect(expect)),
            xfel: XfelParams::from_core(&XfelSource::flash_13nm5()),
            ics: IcsParams::from_core(&IcsSource::compact_euv_13nm5().expect(expect)),
            ssmb: SsmbParams::from_core(&SsmbSource::euv_1kw_13nm5().expect(expect)),
            entangled: EntangledParams::from_core(
                &EntangledPhotonSource::noon(157.63, 2, 1.0).expect(expect),
            ),
            xray_tube: XrayTubeParams::from_core(&XrayTubeSource::w_60kv().expect(expect)),
            dpp: DppParams::from_core(&DppSource::sn_13nm5(sigma).expect(expect)),
            sxrl: SxrlParams::from_core(&SxrlSource::ag_13nm9().expect(expect)),
            betatron: BetatronParams::from_core(&BetatronSource::lwfa_100tw().expect(expect)),
            smith_purcell: SmithPurcellParams::from_core(
                &SmithPurcellSource::euv_13nm5().expect(expect),
            ),
        }
    }
}

impl SourceParams {
    /// Switch to `family`, keeping every family's parameters.
    pub fn select_family(&mut self, family: Family) {
        if self.family != family {
            self.family = family;
            self.preset = family.default_preset();
            self.illumination.mode = IllumMode::SourceDefault;
        }
    }

    /// Load a preset (from its core factory) into its family's parameters
    /// and select that family. The illumination returns to the source
    /// default; the user's σ is kept.
    pub fn apply_preset(&mut self, id: PresetId) -> Result<(), String> {
        let sigma = self.illumination.sigma;
        match id {
            PresetId::F2 => {
                self.vuv = VuvParams::from_core(&VuvSource::f2_laser(sigma).map_err(err)?)
            }
            PresetId::ArF => {
                self.vuv = VuvParams::from_core(&VuvSource::arf_laser(sigma).map_err(err)?)
            }
            PresetId::KrF => {
                self.vuv = VuvParams::from_core(&VuvSource::krf_laser(sigma).map_err(err)?)
            }
            PresetId::HgI => {
                self.vuv = VuvParams::from_core(&VuvSource::hg_i_line(sigma).map_err(err)?)
            }
            PresetId::HgH => {
                self.vuv = VuvParams::from_core(&VuvSource::hg_h_line(sigma).map_err(err)?)
            }
            PresetId::HgG => {
                self.vuv = VuvParams::from_core(&VuvSource::hg_g_line(sigma).map_err(err)?)
            }
            PresetId::Ar2 => {
                self.vuv = VuvParams::from_core(&VuvSource::ar2_laser(sigma).map_err(err)?)
            }
            PresetId::LpaFel25 => {
                self.lpa_fel =
                    LpaFelParams::from_core(&LpaFelSource::bella_target_25nm(sigma).map_err(err)?)
            }
            PresetId::LpaFel420 => {
                self.lpa_fel =
                    LpaFelParams::from_core(&LpaFelSource::bella_baseline_100mev().map_err(err)?)
            }
            PresetId::LppSnCo2 => {
                self.lpp = LppParams::from_core(
                    &LppSource::sn_with_drive_laser(sigma, LppDriveLaser::Co2).map_err(err)?,
                )
            }
            PresetId::LppSn1um => {
                self.lpp = LppParams::from_core(
                    &LppSource::sn_with_drive_laser(sigma, LppDriveLaser::SolidState1um)
                        .map_err(err)?,
                )
            }
            PresetId::LppSn2um => {
                self.lpp = LppParams::from_core(
                    &LppSource::sn_with_drive_laser(sigma, LppDriveLaser::Thulium2um)
                        .map_err(err)?,
                )
            }
            PresetId::LppSn500W => {
                self.lpp = LppParams::from_core(&LppSource::sn_13nm5_500w(sigma).map_err(err)?)
            }
            PresetId::LppGd => {
                self.lpp = LppParams::from_core(&LppSource::gd_6nm7(sigma).map_err(err)?)
            }
            PresetId::LppTb => {
                self.lpp = LppParams::from_core(&LppSource::tb_6nm5(sigma).map_err(err)?)
            }
            PresetId::UndulatorEuv => {
                self.synchrotron = SynchrotronParams::from_core(
                    &SynchrotronSource::compact_euv_undulator().map_err(err)?,
                    Some(&self.synchrotron),
                )
            }
            PresetId::BendingMagnetLiga => {
                self.synchrotron = SynchrotronParams::from_core(
                    &SynchrotronSource::liga_bending_magnet(),
                    Some(&self.synchrotron),
                )
            }
            PresetId::HhgNe => {
                self.hhg = HhgParams::from_core(&HhgSource::ne_800nm_13nm5().map_err(err)?)
            }
            PresetId::HhgAr => {
                self.hhg = HhgParams::from_core(&HhgSource::ar_800nm_30nm().map_err(err)?)
            }
            PresetId::XfelFlash => self.xfel = XfelParams::from_core(&XfelSource::flash_13nm5()),
            PresetId::XfelFermi => {
                self.xfel = XfelParams::from_core(&XfelSource::fermi_seeded_13nm5())
            }
            PresetId::XfelCwSc => self.xfel = XfelParams::from_core(&XfelSource::cw_sc_13nm5()),
            PresetId::XfelErl => self.xfel = XfelParams::from_core(&XfelSource::erl_13nm5()),
            PresetId::IcsCompact => {
                self.ics = IcsParams::from_core(&IcsSource::compact_euv_13nm5().map_err(err)?)
            }
            PresetId::SsmbEuv => {
                self.ssmb = SsmbParams::from_core(&SsmbSource::euv_1kw_13nm5().map_err(err)?)
            }
            PresetId::NoonF2 => {
                self.entangled = EntangledParams::from_core(
                    &EntangledPhotonSource::noon(157.63, 2, 1.0).map_err(err)?,
                )
            }
            PresetId::TubeW60 => {
                self.xray_tube = XrayTubeParams::from_core(&XrayTubeSource::w_60kv().map_err(err)?)
            }
            PresetId::TubeMo50 => {
                self.xray_tube = XrayTubeParams::from_core(&XrayTubeSource::mo_50kv().map_err(err)?)
            }
            PresetId::TubeCu40 => {
                self.xray_tube = XrayTubeParams::from_core(&XrayTubeSource::cu_40kv().map_err(err)?)
            }
            PresetId::TubeRh50 => {
                self.xray_tube = XrayTubeParams::from_core(&XrayTubeSource::rh_50kv().map_err(err)?)
            }
            PresetId::DppSn => {
                self.dpp = DppParams::from_core(&DppSource::sn_13nm5(sigma).map_err(err)?)
            }
            PresetId::DppXe => {
                self.dpp = DppParams::from_core(&DppSource::xe_13nm5(sigma).map_err(err)?)
            }
            PresetId::SxrlAg => {
                self.sxrl = SxrlParams::from_core(&SxrlSource::ag_13nm9().map_err(err)?)
            }
            PresetId::SxrlCd => {
                self.sxrl = SxrlParams::from_core(&SxrlSource::cd_13nm2().map_err(err)?)
            }
            PresetId::SxrlMo => {
                self.sxrl = SxrlParams::from_core(&SxrlSource::mo_18nm9().map_err(err)?)
            }
            PresetId::SxrlAr => {
                self.sxrl = SxrlParams::from_core(&SxrlSource::ar_46nm9().map_err(err)?)
            }
            PresetId::BetatronLwfa => {
                self.betatron =
                    BetatronParams::from_core(&BetatronSource::lwfa_100tw().map_err(err)?)
            }
            PresetId::SmithPurcellEuv => {
                self.smith_purcell =
                    SmithPurcellParams::from_core(&SmithPurcellSource::euv_13nm5().map_err(err)?)
            }
        }
        let info = preset_info(id);
        self.family = info.family;
        self.preset = id;
        self.illumination.mode = IllumMode::SourceDefault;
        Ok(())
    }

    /// Projection imaging or the LIGA view.
    pub fn route(&self) -> Route {
        match self.family {
            Family::XrayTube | Family::Betatron => Route::Liga,
            Family::Synchrotron if self.synchrotron.bending_magnet => Route::Liga,
            _ => Route::Projection,
        }
    }

    /// Build the source. `spectral_samples` overrides the family's own
    /// spectral sampling when given.
    pub fn build(&self, spectral_samples: Option<usize>) -> Result<SourceKind, String> {
        let sigma = self.illumination.sigma;
        if self.family.default_pupil_uses_sigma() && !(sigma > 0.0 && sigma <= 1.0) {
            return Err(format!("\u{3c3} must be in (0, 1] (got {sigma})"));
        }
        let mut kind = match self.family {
            Family::Vuv => SourceKind::Vuv(self.vuv.to_core(sigma)?),
            Family::LpaFel => SourceKind::LpaFel(self.lpa_fel.to_core(sigma)?),
            Family::Lpp => SourceKind::Lpp(self.lpp.to_core(sigma)?),
            Family::Synchrotron => SourceKind::Synchrotron(self.synchrotron.to_core()?),
            Family::Hhg => SourceKind::Hhg(self.hhg.to_core()?),
            Family::Xfel => SourceKind::Xfel(self.xfel.to_core()?),
            Family::Ics => SourceKind::Ics(self.ics.to_core()?),
            Family::Ssmb => SourceKind::Ssmb(self.ssmb.to_core()?),
            Family::Entangled => SourceKind::Entangled(self.entangled.to_core()?),
            Family::XrayTube => SourceKind::XrayTube(self.xray_tube.to_core()?),
            Family::Dpp => SourceKind::Dpp(self.dpp.to_core(sigma)?),
            Family::Sxrl => SourceKind::Sxrl(self.sxrl.to_core()?),
            Family::Betatron => SourceKind::Betatron(self.betatron.to_core()?),
            Family::SmithPurcell => SourceKind::SmithPurcell(self.smith_purcell.to_core()?),
        };
        if let Some(shape) = self.illumination.override_shape()? {
            set_illumination(&mut kind, shape);
        }
        if let Some(n) = spectral_samples {
            set_spectral_samples(&mut kind, n);
        }
        let lambda = kind.wavelength_nm();
        if !(lambda.is_finite() && lambda > 0.0) {
            return Err(format!("source wavelength is not positive ({lambda} nm)"));
        }
        Ok(kind)
    }

    /// Badge of the active preset if it differs from the family's badges.
    pub fn preset_badge(&self) -> Option<Badge> {
        let info = preset_info(self.preset);
        (info.family == self.family).then_some(info.badge).flatten()
    }
}

/// One-line description of an illumination shape.
pub fn describe_illumination(shape: &IlluminationShape) -> String {
    match shape {
        IlluminationShape::Conventional { sigma } => format!("conventional \u{3c3} = {sigma:.2}"),
        IlluminationShape::Annular {
            sigma_inner,
            sigma_outer,
        } => format!("annular \u{3c3} {sigma_inner:.2}\u{2013}{sigma_outer:.2}"),
        IlluminationShape::Dipole {
            sigma_center,
            sigma_radius,
            orientation_deg,
        } => format!(
            "dipole \u{3c3}c {sigma_center:.2}, r {sigma_radius:.2}, {orientation_deg:.0}\u{b0}"
        ),
        IlluminationShape::Quadrupole {
            sigma_center,
            sigma_radius,
            opening_angle_deg,
        } => format!(
            "quadrupole \u{3c3}c {sigma_center:.2}, r {sigma_radius:.2}, {opening_angle_deg:.0}\u{b0}"
        ),
        IlluminationShape::CoherentGaussian { sigma } => {
            format!("coherent Gaussian \u{3c3} = {sigma:.3}")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, rel: f64) -> bool {
        (a - b).abs() <= rel * a.abs().max(b.abs())
    }

    #[test]
    fn every_family_builds_from_its_defaults() {
        let base = SourceParams::default();
        for family in Family::ALL {
            let mut p = base.clone();
            p.select_family(family);
            let kind = p
                .build(None)
                .unwrap_or_else(|e| panic!("{family:?} failed to build: {e}"));
            assert_eq!(kind.kind_label(), family.kind_label());
            assert!(kind.wavelength_nm() > 0.0);
            assert!(family.presets().count() >= 1);
        }
        // The fourteen registered families, each exactly once.
        let mut labels: Vec<&str> = Family::ALL.iter().map(|f| f.kind_label()).collect();
        labels.sort_unstable();
        labels.dedup();
        assert_eq!(labels.len(), 14);
    }

    #[test]
    fn every_preset_applies_and_builds() {
        for info in PRESETS {
            let mut p = SourceParams::default();
            p.apply_preset(info.id)
                .unwrap_or_else(|e| panic!("{:?}: {e}", info.id));
            assert_eq!(p.family, info.family);
            let kind = p
                .build(None)
                .unwrap_or_else(|e| panic!("{:?} failed to build: {e}", info.id));
            assert_eq!(kind.kind_label(), info.family.kind_label());
        }
    }

    #[test]
    fn presets_reproduce_the_core_factories() {
        let mut p = SourceParams::default();
        let cases = [
            (PresetId::ArF, highuvlith_core::source::ARF_WAVELENGTH_NM),
            (PresetId::KrF, highuvlith_core::source::KRF_WAVELENGTH_NM),
            (PresetId::HgI, highuvlith_core::source::HG_I_LINE_NM),
            (PresetId::F2, 157.63),
            (PresetId::LppGd, 6.7),
            (PresetId::SxrlAr, 46.9),
            (PresetId::HhgNe, 800.0 / 59.0),
            (PresetId::XfelErl, 13.5),
        ];
        for (id, lambda) in cases {
            p.apply_preset(id).unwrap();
            let kind = p.build(None).unwrap();
            assert!(
                close(kind.wavelength_nm(), lambda, 1e-12),
                "{id:?}: {} vs {lambda}",
                kind.wavelength_nm()
            );
        }
        // Power chain of the Sn LPP preset: 250 W at intermediate focus.
        p.apply_preset(PresetId::LppSnCo2).unwrap();
        let power = p.build(None).unwrap().average_power_w().unwrap();
        let core = LppSource::sn_13nm5(0.7).unwrap().average_power_w().unwrap();
        assert!(close(power, core, 1e-12));
        // Hg lamps are CW with one spectral sample.
        p.apply_preset(PresetId::HgG).unwrap();
        assert!(p.vuv.is_cw());
        assert_eq!(p.build(None).unwrap().spectral_samples(), 1);
    }

    #[test]
    fn lpa_fel_wavelength_is_the_undulator_resonance() {
        let mut p = SourceParams::default();
        p.apply_preset(PresetId::LpaFel25).unwrap();
        let lambda0 = p.build(None).unwrap().wavelength_nm();
        assert!(close(lambda0, 25.0, 1e-9), "preset resonance {lambda0}");
        // λ ∝ (1 + K²/2) / γ²: doubling γ (at fixed K) quarters λ.
        let gamma = physics::gamma_from_mev(p.lpa_fel.electron_energy_mev);
        p.lpa_fel.electron_energy_mev = (2.0 * gamma - 1.0) * physics::ELECTRON_REST_MEV;
        let lambda1 = p.build(None).unwrap().wavelength_nm();
        assert!(close(lambda1, lambda0 / 4.0, 1e-9));
        // Raising K lengthens the resonance.
        p.lpa_fel.undulator_k *= 1.2;
        assert!(p.build(None).unwrap().wavelength_nm() > lambda1);
    }

    #[test]
    fn synchrotron_undulator_wavelength_is_derived() {
        let mut p = SourceParams::default();
        p.apply_preset(PresetId::UndulatorEuv).unwrap();
        let s = &p.synchrotron;
        let expected = physics::undulator_resonance_nm(
            s.period_mm,
            s.k,
            physics::gamma_from_mev(s.energy_gev * 1e3),
            s.harmonic,
        );
        let lambda = p.build(None).unwrap().wavelength_nm();
        assert!(close(lambda, expected, 1e-6), "{lambda} vs {expected}");
        // Third harmonic: a third of the wavelength.
        p.synchrotron.harmonic = 3;
        let l3 = p.build(None).unwrap().wavelength_nm();
        assert!(close(l3, lambda / 3.0, 1e-9));
        // Even harmonics are rejected by the core.
        p.synchrotron.harmonic = 2;
        assert!(p.build(None).is_err());
    }

    #[test]
    fn broadband_x_ray_sources_route_to_the_liga_view() {
        let mut p = SourceParams::default();
        assert_eq!(p.route(), Route::Projection);
        for (id, route) in [
            (PresetId::TubeW60, Route::Liga),
            (PresetId::BetatronLwfa, Route::Liga),
            (PresetId::BendingMagnetLiga, Route::Liga),
            (PresetId::UndulatorEuv, Route::Projection),
            (PresetId::LppSnCo2, Route::Projection),
            (PresetId::SmithPurcellEuv, Route::Projection),
        ] {
            p.apply_preset(id).unwrap();
            assert_eq!(p.route(), route, "{id:?}");
        }
    }

    #[test]
    fn hhg_harmonic_beyond_cutoff_is_rejected_and_comb_mode_works() {
        let mut p = SourceParams::default();
        p.apply_preset(PresetId::HhgNe).unwrap();
        assert!(close(
            p.build(None).unwrap().wavelength_nm(),
            800.0 / 59.0,
            1e-12
        ));
        p.hhg.harmonic = 151; // far beyond the Ne cutoff at 4e14 W/cm²
        assert!(p.build(None).is_err());
        p.hhg.harmonic = 60; // even
        assert!(p.build(None).is_err());
        p.hhg.harmonic = 59;
        p.hhg.full_comb = true;
        let comb = p.build(Some(1)).unwrap();
        // One sample per harmonic of the plateau.
        assert!(comb.spectral_weights().len() > 5);
        p.hhg.passband_nm = Some([12.5, 14.5]);
        let band = p.build(Some(1)).unwrap();
        assert!(band
            .spectral_weights()
            .iter()
            .all(|(l, _)| (12.4..=14.6).contains(l)));
        p.hhg.passband_nm = Some([1.0, 2.0]);
        assert!(p.build(Some(1)).is_err());
    }

    #[test]
    fn xfel_set_point_must_be_reachable() {
        let mut p = SourceParams::default();
        p.apply_preset(PresetId::XfelFlash).unwrap();
        assert!(p.build(None).is_ok());
        p.xfel.wavelength_nm = 0.01; // below the K = 0 resonance of a 680 MeV machine
        assert!(p.build(None).is_err());
    }

    #[test]
    fn illumination_override_and_spectral_samples() {
        let mut p = SourceParams::default();
        // Source default of an incoherent family: a disk of the user's σ.
        p.illumination.sigma = 0.55;
        let kind = p.build(None).unwrap();
        assert!(
            matches!(kind.illumination(), IlluminationShape::Conventional { sigma } if *sigma == 0.55)
        );
        // Coherent families keep their coherence-derived Gaussian by default.
        p.apply_preset(PresetId::XfelFlash).unwrap();
        assert!(matches!(
            p.build(None).unwrap().illumination(),
            IlluminationShape::CoherentGaussian { .. }
        ));
        // Explicit dipole override.
        p.illumination.mode = IllumMode::Dipole;
        p.illumination.pole_center = 0.6;
        p.illumination.pole_radius = 0.2;
        p.illumination.angle_deg = 90.0;
        match p.build(Some(3)).unwrap() {
            k @ SourceKind::Xfel(_) => {
                assert!(matches!(
                    k.illumination(),
                    IlluminationShape::Dipole { sigma_center, sigma_radius, orientation_deg }
                        if *sigma_center == 0.6 && *sigma_radius == 0.2 && *orientation_deg == 90.0
                ));
                assert_eq!(k.spectral_samples(), 3);
            }
            other => panic!("unexpected {}", other.kind_label()),
        }
        // Invalid annulus.
        p.illumination.mode = IllumMode::Annular;
        p.illumination.sigma_inner = 0.9;
        p.illumination.sigma_outer = 0.8;
        assert!(p.build(None).is_err());
        // Selecting another family returns to the source default.
        p.select_family(Family::Lpp);
        assert_eq!(p.illumination.mode, IllumMode::SourceDefault);
    }

    #[test]
    fn statuses_mirror_existing_docs_pages() {
        let docs = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        for family in Family::ALL {
            let status = family.status();
            assert!(!status.badges.is_empty());
            assert!(
                docs.join(status.doc).is_file(),
                "{family:?}: missing {}",
                status.doc
            );
        }
        // The research-projection families carry the theoretical badge.
        for family in [
            Family::Ics,
            Family::Ssmb,
            Family::Entangled,
            Family::Betatron,
            Family::SmithPurcell,
        ] {
            assert_eq!(family.status().badges, &[Badge::Theoretical]);
        }
        let mut p = SourceParams::default();
        p.apply_preset(PresetId::Ar2).unwrap();
        assert_eq!(p.preset_badge(), Some(Badge::Theoretical));
        p.apply_preset(PresetId::F2).unwrap();
        assert_eq!(p.preset_badge(), None);
    }

    #[test]
    fn preset_cli_names_round_trip() {
        let mut names: Vec<&str> = PRESETS.iter().map(|p| p.id.cli_name()).collect();
        for p in PRESETS {
            assert_eq!(PresetId::from_cli_name(p.id.cli_name()), Some(p.id));
        }
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), PRESETS.len());
        assert_eq!(PresetId::from_cli_name("nope"), None);
    }

    #[test]
    fn derived_quantities_are_reported_for_every_family() {
        let base = SourceParams::default();
        for family in Family::ALL {
            let mut p = base.clone();
            p.select_family(family);
            let q = p.build(None).unwrap().derived_quantities();
            assert!(!q.is_empty(), "{family:?} reports no derived quantities");
            assert!(q.iter().all(|d| !d.name.is_empty()));
        }
    }

    #[test]
    fn hhg_power_is_derived_from_the_driver() {
        let mut p = SourceParams::default();
        p.apply_preset(PresetId::HhgNe).unwrap();
        // Preset: assumed 20 W driver -> ~0.9 µW (between the 0.43 and ~1 µW records).
        let core = HhgSource::ne_800nm_13nm5()
            .unwrap()
            .average_power_w()
            .unwrap();
        let p0 = p.build(None).unwrap().average_power_w().unwrap();
        assert!(close(p0, core, 1e-12), "{p0} vs {core}");
        assert!((0.43e-6..=1.0e-6).contains(&p0), "{p0}");
        // P = P_driver × η_q: doubling the driver doubles the harmonic power.
        p.hhg.driver_power_w = Some(40.0);
        let p1 = p.build(None).unwrap().average_power_w().unwrap();
        assert!(close(p1, 2.0 * p0, 1e-12));
        // The rep rate only splits the power into pulses.
        p.hhg.rep_rate_hz *= 10.0;
        assert!(close(
            p.build(None).unwrap().average_power_w().unwrap(),
            p1,
            1e-12
        ));
        // Without a driver power the stored pulse energy is used: 2 nJ × 1 kHz = 2 µW.
        p.hhg.driver_power_w = None;
        p.hhg.pulse_energy_nj = 2.0;
        p.hhg.rep_rate_hz = 1e3;
        let stored = p.build(None).unwrap().average_power_w().unwrap();
        assert!(close(stored, 2e-6, 1e-12), "{stored}");
        p.hhg.driver_power_w = Some(-1.0);
        assert!(p.build(None).is_err());
    }

    #[test]
    fn lpp_500w_and_rh_tube_presets_match_the_core() {
        let mut p = SourceParams::default();
        p.apply_preset(PresetId::LppSn500W).unwrap();
        let power = p.build(None).unwrap().average_power_w().unwrap();
        assert!(close(power, 500.0, 1e-9), "{power}");
        // Twice the NXE:3400B drive at the same CE and collection.
        p.apply_preset(PresetId::LppSnCo2).unwrap();
        let base = p.build(None).unwrap().average_power_w().unwrap();
        assert!(close(power, 2.0 * base, 1e-12));
        p.apply_preset(PresetId::TubeRh50).unwrap();
        assert_eq!(p.route(), Route::Liga);
        let kind = p.build(None).unwrap();
        let core = XrayTubeSource::rh_50kv().unwrap();
        assert!(close(kind.wavelength_nm(), core.wavelength_nm(), 1e-12));
    }
}
