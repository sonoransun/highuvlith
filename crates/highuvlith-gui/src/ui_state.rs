//! Pure UI state: which views exist for the selected source, how the
//! current view follows a family change, and which background job a view
//! needs. Kept free of egui so it can be unit-tested.

use crate::sources::Route;
use crate::volume::DatasetKind;

/// A central-panel view.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Tab {
    Aerial,
    CrossSection,
    ProcessWindow,
    Volume,
    Liga,
    Source,
}

impl Tab {
    /// Every view, in tab-bar order.
    pub const ALL: [Tab; 6] = [
        Tab::Aerial,
        Tab::CrossSection,
        Tab::ProcessWindow,
        Tab::Volume,
        Tab::Liga,
        Tab::Source,
    ];

    /// Tab-bar label.
    pub fn label(self) -> &'static str {
        match self {
            Tab::Aerial => "Aerial image",
            Tab::CrossSection => "Cross-section",
            Tab::ProcessWindow => "Process window",
            Tab::Volume => "Volume viewer",
            Tab::Liga => "LIGA depth dose",
            Tab::Source => "Source",
        }
    }

    /// Command-line name (`--tab`).
    pub fn cli_name(self) -> &'static str {
        match self {
            Tab::Aerial => "aerial",
            Tab::CrossSection => "cross-section",
            Tab::ProcessWindow => "process-window",
            Tab::Volume => "volume",
            Tab::Liga => "liga",
            Tab::Source => "source",
        }
    }

    /// Parse a `--tab` value.
    pub fn from_cli_name(name: &str) -> Option<Tab> {
        Tab::ALL.into_iter().find(|t| t.cli_name() == name)
    }

    /// Whether the view exists for sources on `route`.
    pub fn available(self, route: Route) -> bool {
        match route {
            Route::Projection => !matches!(self, Tab::Liga),
            Route::Liga => matches!(self, Tab::Liga | Tab::Volume | Tab::Source),
        }
    }
}

/// The view to show after the source route changed: the current one if it
/// still exists, else the route's main view.
pub fn conform_tab(tab: Tab, route: Route) -> Tab {
    if tab.available(route) {
        tab
    } else {
        match route {
            Route::Projection => Tab::Aerial,
            Route::Liga => Tab::Liga,
        }
    }
}

/// Volume datasets a route can produce.
pub fn dataset_available(kind: DatasetKind, route: Route) -> bool {
    match route {
        Route::Projection => kind != DatasetKind::LigaDose,
        Route::Liga => kind == DatasetKind::LigaDose,
    }
}

/// The volume dataset to show after the route changed.
pub fn conform_dataset(kind: DatasetKind, route: Route) -> DatasetKind {
    if dataset_available(kind, route) {
        kind
    } else {
        match route {
            Route::Projection => DatasetKind::Latent,
            Route::Liga => DatasetKind::LigaDose,
        }
    }
}

/// Background computation a view depends on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Need {
    Aerial,
    ProcessWindow,
    Volumetric,
    Liga,
    Talbot,
}

/// What the visible view needs computed (only visible views compute).
pub fn needs(tab: Tab, dataset: DatasetKind, route: Route) -> Vec<Need> {
    match (route, tab) {
        (Route::Projection, Tab::Aerial | Tab::CrossSection | Tab::Source) => vec![Need::Aerial],
        (Route::Projection, Tab::ProcessWindow) => vec![Need::ProcessWindow],
        (Route::Projection, Tab::Volume) => match dataset {
            DatasetKind::TalbotCarpet => vec![Need::Talbot],
            _ => vec![Need::Volumetric],
        },
        (Route::Liga, Tab::Liga | Tab::Volume) => vec![Need::Liga],
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn liga_route_has_its_own_views() {
        assert!(Tab::Liga.available(Route::Liga));
        assert!(!Tab::Aerial.available(Route::Liga));
        assert!(!Tab::ProcessWindow.available(Route::Liga));
        assert!(Tab::Volume.available(Route::Liga));
        assert!(!Tab::Liga.available(Route::Projection));
        for tab in Tab::ALL {
            assert!(tab.available(Route::Projection) || tab.available(Route::Liga));
            assert_eq!(Tab::from_cli_name(tab.cli_name()), Some(tab));
        }
        assert_eq!(Tab::from_cli_name("nonsense"), None);
    }

    #[test]
    fn switching_to_an_x_ray_tube_routes_to_liga_and_back() {
        // Aerial image → X-ray tube: the aerial view does not exist, go to LIGA.
        assert_eq!(conform_tab(Tab::Aerial, Route::Liga), Tab::Liga);
        assert_eq!(conform_tab(Tab::ProcessWindow, Route::Liga), Tab::Liga);
        // Views that exist for both stay.
        assert_eq!(conform_tab(Tab::Volume, Route::Liga), Tab::Volume);
        assert_eq!(conform_tab(Tab::Source, Route::Liga), Tab::Source);
        // Back to a projection source from the LIGA view.
        assert_eq!(conform_tab(Tab::Liga, Route::Projection), Tab::Aerial);
        assert_eq!(conform_tab(Tab::Volume, Route::Projection), Tab::Volume);
    }

    #[test]
    fn datasets_follow_the_route() {
        assert_eq!(
            conform_dataset(DatasetKind::Latent, Route::Liga),
            DatasetKind::LigaDose
        );
        assert_eq!(
            conform_dataset(DatasetKind::LigaDose, Route::Projection),
            DatasetKind::Latent
        );
        assert_eq!(
            conform_dataset(DatasetKind::TalbotCarpet, Route::Projection),
            DatasetKind::TalbotCarpet
        );
        assert!(!dataset_available(DatasetKind::Developed, Route::Liga));
    }

    #[test]
    fn only_the_visible_view_computes() {
        use DatasetKind::*;
        assert_eq!(
            needs(Tab::Aerial, Latent, Route::Projection),
            vec![Need::Aerial]
        );
        assert_eq!(
            needs(Tab::Source, Latent, Route::Projection),
            vec![Need::Aerial]
        );
        assert_eq!(
            needs(Tab::ProcessWindow, Latent, Route::Projection),
            vec![Need::ProcessWindow]
        );
        assert_eq!(
            needs(Tab::Volume, Developed, Route::Projection),
            vec![Need::Volumetric]
        );
        assert_eq!(
            needs(Tab::Volume, TalbotCarpet, Route::Projection),
            vec![Need::Talbot]
        );
        assert_eq!(needs(Tab::Liga, LigaDose, Route::Liga), vec![Need::Liga]);
        assert_eq!(needs(Tab::Volume, LigaDose, Route::Liga), vec![Need::Liga]);
        assert!(needs(Tab::Source, LigaDose, Route::Liga).is_empty());
    }
}
