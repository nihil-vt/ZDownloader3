//! Builtin theme engine. ponytail: CSS files baked into the binary +
//! libadwaita color-scheme switching. No theme marketplace, no hot-reload.

use gtk4::{gdk::Display, style_context_add_provider_for_display, CssProvider, STYLE_PROVIDER_PRIORITY_APPLICATION};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    AdwaitaLight,
    AdwaitaDark,
    Arc,
    Nord,
    CatppuccinMocha,
    CatppuccinLatte,
}

impl Theme {
    pub const ALL: [(&'static str, Theme); 6] = [
        ("Adwaita Light", Theme::AdwaitaLight),
        ("Adwaita Dark", Theme::AdwaitaDark),
        ("Arc", Theme::Arc),
        ("Nord", Theme::Nord),
        ("Catppuccin Mocha", Theme::CatppuccinMocha),
        ("Catppuccin Latte", Theme::CatppuccinLatte),
    ];

    fn css(self) -> Option<&'static str> {
        match self {
            Theme::AdwaitaLight | Theme::AdwaitaDark => None, // stock libadwaita
            Theme::Arc => Some(include_str!("../themes/arc.css")),
            Theme::Nord => Some(include_str!("../themes/nord.css")),
            Theme::CatppuccinMocha => Some(include_str!("../themes/catppuccin-mocha.css")),
            Theme::CatppuccinLatte => Some(include_str!("../themes/catppuccin-latte.css")),
        }
    }

    /// Dark themes need libadwaita's dark color scheme; light ones prefer-light.
    fn scheme(self) -> libadwaita::ColorScheme {
        use libadwaita::ColorScheme::*;
        match self {
            Theme::AdwaitaLight | Theme::CatppuccinLatte => PreferLight,
            _ => PreferDark,
        }
    }
}

/// Apply theme globally, removing the previously applied provider.
pub fn apply(theme: Theme, prev: &mut Option<CssProvider>) {
    libadwaita::StyleManager::default().set_color_scheme(theme.scheme());
    if let Some(old) = prev.take() {
        gtk4::style_context_remove_provider_for_display(
            &Display::default().expect("display"),
            &old,
        );
    }
    if let Some(css) = theme.css() {
        let provider = CssProvider::new();
        provider.load_from_string(css);
        style_context_add_provider_for_display(
            &Display::default().expect("display"),
            &provider,
            STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
        *prev = Some(provider);
    }
}
