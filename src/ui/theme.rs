use libadwaita as adw;
use gtk4::{self as gtk, gdk};
use libadwaita::prelude::*;
use crate::models::Theme;

pub fn load_custom_css() {
    let provider = gtk::CssProvider::new();
    provider.load_from_data("
        .blockquote {
            border-left: 4px solid #3584e4;
            padding-left: 12px;
            font-style: italic;
            margin-bottom: 12px;
        }
        .code-block {
            font-family: monospace;
            background-color: rgba(255, 255, 255, 0.08);
            padding: 12px;
            border-radius: 6px;
            margin-bottom: 12px;
        }
        .dim-label {
            opacity: 0.7;
        }
        .theme-btn-overlay {
            margin: 4px;
        }
        button.theme-btn {
            min-width: 44px;
            min-height: 44px;
            border-radius: 9999px;
            border: 1px solid rgba(0, 0, 0, 0.15);
            padding: 0;
            margin: 0;
            background-color: transparent;
            background-image: none;
            box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
        }
        button.theme-btn-system {
            background-image: linear-gradient(135deg, #ffffff 48%, #2e2e2e 52%);
            background-clip: border-box;
            background-origin: border-box;
        }
        button.theme-btn-light {
            background-color: #ffffff;
            background-image: none;
        }
        button.theme-btn-sepia {
            background-color: #f4ecd8;
            background-image: none;
        }
        button.theme-btn-dark {
            background-color: #2e2e2e;
            background-image: none;
        }
        button.theme-btn-selected {
            box-shadow: 0 0 0 2px #3584e4;
        }
        .theme-badge {
            background-color: #3584e4;
            color: #ffffff;
            border-radius: 9999px;
            border: 1.5px solid #ffffff;
            min-width: 14px;
            min-height: 14px;
            padding: 2px;
            margin-bottom: -2px;
            margin-right: -2px;
        }
        .menu-item-btn {
            padding: 8px 12px;
            margin: 2px 0;
            border-radius: 6px;
            font-weight: normal;
        }
        .menu-item-btn label {
            font-weight: normal;
        }
        .sepia, 
        .sepia window, 
        .sepia scrolledwindow, 
        .sepia viewport, 
        .sepia .background, 
        .sepia statuspage {
            background-color: #f4ecd8;
            color: #433422;
        }
        .sepia headerbar, 
        .sepia .headerbar,
        .sepia actionbar,
        .sepia .actionbar {
            background-color: #e8ddb5;
            color: #433422;
            border-color: rgba(67, 52, 34, 0.15);
        }
        .sepia headerbar button:not(.theme-btn),
        .sepia actionbar button:not(.theme-btn) {
            background-color: transparent;
            color: #433422;
            border-color: rgba(67, 52, 34, 0.15);
        }
        .sepia headerbar button:hover:not(.theme-btn),
        .sepia actionbar button:hover:not(.theme-btn) {
            background-color: rgba(67, 52, 34, 0.1);
        }
        .sepia label,
        .sepia .title-1,
        .sepia .body {
            color: #433422;
        }
        .sepia .dim-label {
            color: #6e5e4f;
        }
        .sepia .blockquote {
            border-left: 4px solid #8c6239;
            color: #5c4a37;
            background-color: rgba(140, 98, 57, 0.05);
        }
        .sepia .code-block {
            background-color: rgba(140, 98, 57, 0.08);
            color: #433422;
            border: 1px solid rgba(140, 98, 57, 0.15);
        }
        .sepia label a, 
        .sepia a {
            color: #8c6239;
        }
        .sepia label a:hover, 
        .sepia a:hover {
            color: #704d2b;
        }
        .sepia progressbar progress {
            background-color: #8c6239;
        }
        .sepia progressbar trough {
            background-color: rgba(140, 98, 57, 0.15);
        }
        .sepia popover.background {
            background-color: transparent;
            background-image: none;
            box-shadow: none;
        }
        .sepia popover contents,
        .sepia popover arrow {
            background-color: #f4ecd8;
            color: #433422;
        }
    ");
    gtk::style_context_add_provider_for_display(
        &gdk::Display::default().expect("Failed to get default GDK display"),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

pub fn apply_theme(theme: Theme, window: &adw::ApplicationWindow) {
    let style_manager = adw::StyleManager::default();
    match theme {
        Theme::System => {
            style_manager.set_color_scheme(adw::ColorScheme::Default);
            window.remove_css_class("sepia");
        }
        Theme::Light => {
            style_manager.set_color_scheme(adw::ColorScheme::ForceLight);
            window.remove_css_class("sepia");
        }
        Theme::Dark => {
            style_manager.set_color_scheme(adw::ColorScheme::ForceDark);
            window.remove_css_class("sepia");
        }
        Theme::Sepia => {
            style_manager.set_color_scheme(adw::ColorScheme::ForceLight);
            window.add_css_class("sepia");
        }
    }
}

pub fn create_theme_button(
    theme_class: &str,
    tooltip: &str,
) -> (gtk::Overlay, gtk::Button, gtk::Image) {
    let button = gtk::Button::builder()
        .css_classes(vec![
            "theme-btn".to_string(),
            "circular".to_string(),
            theme_class.to_string(),
        ])
        .tooltip_text(tooltip)
        .build();
    
    let badge = gtk::Image::builder()
        .icon_name("object-select-symbolic")
        .pixel_size(10)
        .css_classes(vec!["theme-badge".to_string()])
        .halign(gtk::Align::End)
        .valign(gtk::Align::End)
        .visible(false)
        .build();

    let overlay = gtk::Overlay::builder()
        .child(&button)
        .css_classes(vec!["theme-btn-overlay".to_string()])
        .build();
    overlay.add_overlay(&badge);

    (overlay, button, badge)
}

pub fn update_theme_buttons_ui(selected_theme: Theme, buttons: &[(Theme, gtk::Button, gtk::Image)]) {
    for (theme, button, badge) in buttons {
        if *theme == selected_theme {
            button.add_css_class("theme-btn-selected");
            badge.set_visible(true);
        } else {
            button.remove_css_class("theme-btn-selected");
            badge.set_visible(false);
        }
    }
}
