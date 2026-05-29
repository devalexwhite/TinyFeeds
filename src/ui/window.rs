use std::rc::Rc;
use std::cell::RefCell;
use libadwaita as adw;
use gtk4::{self as gtk, gdk, glib};
use libadwaita::prelude::*;
use webkit6::prelude::*;
use webkit6::{PolicyDecisionType, NavigationPolicyDecision, NavigationType};
use crate::models::{AppState, AppMessage, Theme};
use crate::config::{save_theme, add_story_read, open_feeds_file};
use crate::fetcher::trigger_fetch;
use crate::ui::theme::{load_custom_css, apply_theme, create_theme_button, update_theme_buttons_ui};

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
     .replace('<', "&lt;")
     .replace('>', "&gt;")
     .replace('"', "&quot;")
     .replace('\'', "&apos;")
}

#[allow(dead_code)]
#[derive(Clone)]
pub struct AppUi {
    pub window: adw::ApplicationWindow,
    pub stack: gtk::Stack,
    pub open_browser_btn: gtk::Button,
    pub email_author_btn: gtk::Button,
    pub empty_edit_btn: gtk::Button,
    pub loading_spinner: gtk::Spinner,
    pub progress_bar: gtk::ProgressBar,
    pub webview: webkit6::WebView,
    pub close_btn: gtk::Button,
    pub title_widget: adw::WindowTitle,
}

pub fn build_ui(app: &adw::Application, state: Rc<RefCell<AppState>>) {
    load_custom_css();

    let saved_theme = state.borrow().theme;
    
    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("TinyFeeds")
        .default_width(800)
        .default_height(700)
        .build();

    apply_theme(saved_theme, &window);

    let header_bar = adw::HeaderBar::new();
    
    let title_widget = adw::WindowTitle::new("TinyFeeds", "");
    header_bar.set_title_widget(Some(&title_widget));
    
    let open_browser_btn = gtk::Button::builder()
        .icon_name("external-link-symbolic")
        .tooltip_text("Open in Browser")
        .visible(false)
        .build();
        
    let email_author_btn = gtk::Button::builder()
        .icon_name("mail-send-symbolic")
        .tooltip_text("Email Author")
        .visible(false)
        .build();
        
    let menu_btn = gtk::MenuButton::builder()
        .icon_name("open-menu-symbolic")
        .tooltip_text("Menu")
        .build();

    let popover = gtk::Popover::new();
    let popover_box = gtk::Box::new(gtk::Orientation::Vertical, 6);
    popover_box.set_margin_top(8);
    popover_box.set_margin_bottom(8);
    popover_box.set_margin_start(8);
    popover_box.set_margin_end(8);
    popover.set_child(Some(&popover_box));
    menu_btn.set_popover(Some(&popover));

    let theme_box = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    theme_box.set_halign(gtk::Align::Center);
    theme_box.set_margin_bottom(8);
    popover_box.append(&theme_box);

    let (system_overlay, system_btn, system_badge) = create_theme_button("theme-btn-system", "Follow System");
    let (light_overlay, light_btn, light_badge) = create_theme_button("theme-btn-light", "Light");
    let (sepia_overlay, sepia_btn, sepia_badge) = create_theme_button("theme-btn-sepia", "Sepia");
    let (dark_overlay, dark_btn, dark_badge) = create_theme_button("theme-btn-dark", "Dark");

    theme_box.append(&system_overlay);
    theme_box.append(&light_overlay);
    theme_box.append(&sepia_overlay);
    theme_box.append(&dark_overlay);

    let theme_buttons = vec![
        (Theme::System, system_btn.clone(), system_badge.clone()),
        (Theme::Light, light_btn.clone(), light_badge.clone()),
        (Theme::Sepia, sepia_btn.clone(), sepia_badge.clone()),
        (Theme::Dark, dark_btn.clone(), dark_badge.clone()),
    ];

    update_theme_buttons_ui(saved_theme, &theme_buttons);

    let theme_buttons_rc = Rc::new(theme_buttons);

    let webview = webkit6::WebView::builder()
        .vexpand(true)
        .hexpand(true)
        .build();
    webview.set_background_color(&gdk::RGBA::new(0.0, 0.0, 0.0, 0.0));
    
    // Intercept clicks to external links to open them in system browser instead
    webview.connect_decide_policy(move |_, decision, decision_type| {
        if decision_type == PolicyDecisionType::NavigationAction {
            if let Some(nav_decision) = decision.downcast_ref::<NavigationPolicyDecision>() {
                if let Some(action) = nav_decision.navigation_action() {
                    if action.navigation_type() == NavigationType::LinkClicked {
                        if let Some(request) = action.request() {
                            if let Some(uri) = request.uri() {
                                webbrowser::open(uri.as_str()).ok();
                            }
                        }
                        nav_decision.ignore();
                        return true;
                    }
                }
            }
        }
        false
    });

    let state_c = state.clone();
    let window_c = window.clone();
    let theme_buttons_c = theme_buttons_rc.clone();
    let webview_c = webview.clone();
    system_btn.connect_clicked(move |_| {
        let mut s = state_c.borrow_mut();
        s.theme = Theme::System;
        drop(s);
        apply_theme(Theme::System, &window_c);
        save_theme(Theme::System);
        update_theme_buttons_ui(Theme::System, &theme_buttons_c);
        
        let theme_class = if adw::StyleManager::default().is_dark() {
            "theme-dark"
        } else {
            "theme-light"
        };
        let script = format!("document.body.className = '{}';", theme_class);
        webview_c.evaluate_javascript(&script, None, None, None::<&gtk::gio::Cancellable>, |_| {});
    });

    let state_c = state.clone();
    let window_c = window.clone();
    let theme_buttons_c = theme_buttons_rc.clone();
    let webview_c = webview.clone();
    light_btn.connect_clicked(move |_| {
        let mut s = state_c.borrow_mut();
        s.theme = Theme::Light;
        drop(s);
        apply_theme(Theme::Light, &window_c);
        save_theme(Theme::Light);
        update_theme_buttons_ui(Theme::Light, &theme_buttons_c);
        
        let script = "document.body.className = 'theme-light';";
        webview_c.evaluate_javascript(script, None, None, None::<&gtk::gio::Cancellable>, |_| {});
    });

    let state_c = state.clone();
    let window_c = window.clone();
    let theme_buttons_c = theme_buttons_rc.clone();
    let webview_c = webview.clone();
    sepia_btn.connect_clicked(move |_| {
        let mut s = state_c.borrow_mut();
        s.theme = Theme::Sepia;
        drop(s);
        apply_theme(Theme::Sepia, &window_c);
        save_theme(Theme::Sepia);
        update_theme_buttons_ui(Theme::Sepia, &theme_buttons_c);
        
        let script = "document.body.className = 'theme-sepia';";
        webview_c.evaluate_javascript(script, None, None, None::<&gtk::gio::Cancellable>, |_| {});
    });

    let state_c = state.clone();
    let window_c = window.clone();
    let theme_buttons_c = theme_buttons_rc.clone();
    let webview_c = webview.clone();
    dark_btn.connect_clicked(move |_| {
        let mut s = state_c.borrow_mut();
        s.theme = Theme::Dark;
        drop(s);
        apply_theme(Theme::Dark, &window_c);
        save_theme(Theme::Dark);
        update_theme_buttons_ui(Theme::Dark, &theme_buttons_c);
        
        let script = "document.body.className = 'theme-dark';";
        webview_c.evaluate_javascript(script, None, None, None::<&gtk::gio::Cancellable>, |_| {});
    });

    // Listen for system theme preference changes to update the webview dynamically
    let state_c = state.clone();
    let webview_c = webview.clone();
    adw::StyleManager::default().connect_dark_notify(move |style_manager| {
        let s = state_c.borrow();
        if s.theme == Theme::System {
            let is_dark = style_manager.is_dark();
            let theme_class = if is_dark { "theme-dark" } else { "theme-light" };
            let script = format!("document.body.className = '{}';", theme_class);
            webview_c.evaluate_javascript(&script, None, None, None::<&gtk::gio::Cancellable>, |_| {});
        }
    });

    let separator = gtk::Separator::new(gtk::Orientation::Horizontal);
    popover_box.append(&separator);

    let edit_feeds_item = gtk::Button::builder()
        .css_classes(vec!["flat".to_string(), "menu-item-btn".to_string()])
        .build();
    let edit_label = gtk::Label::builder()
        .label("Edit Feeds")
        .halign(gtk::Align::Start)
        .hexpand(true)
        .build();
    edit_feeds_item.set_child(Some(&edit_label));
    popover_box.append(&edit_feeds_item);

    let popover_c = popover.clone();
    edit_feeds_item.connect_clicked(move |_| {
        popover_c.popdown();
        open_feeds_file();
    });

    let about_item = gtk::Button::builder()
        .css_classes(vec!["flat".to_string(), "menu-item-btn".to_string()])
        .build();
    let about_label = gtk::Label::builder()
        .label("About TinyFeeds")
        .halign(gtk::Align::Start)
        .hexpand(true)
        .build();
    about_item.set_child(Some(&about_label));
    popover_box.append(&about_item);

    let popover_c = popover.clone();
    let window_c = window.clone();
    about_item.connect_clicked(move |_| {
        popover_c.popdown();
        let about = adw::AboutDialog::builder()
            .application_name("TinyFeeds")
            .version("0.1.0")
            .website("https://thatalexguy.dev")
            .developer_name("Alex White")
            .comments("With love and thanks to my kids and wife.")
            .license_type(gtk::License::Gpl30Only)
            .build();
        about.present(Some(&window_c));
    });

    header_bar.pack_start(&open_browser_btn);
    header_bar.pack_start(&email_author_btn);
    header_bar.pack_end(&menu_btn);

    let stack = gtk::Stack::builder()
        .transition_type(gtk::StackTransitionType::SlideLeftRight)
        .build();

    // Page 1: Empty state
    let empty_status = adw::StatusPage::builder()
        .title("Add Feeds to Get Started")
        .description("Edit your feeds.txt configuration file to add RSS feed URLs.")
        .icon_name("document-open-symbolic")
        .build();
    let empty_btn = gtk::Button::builder()
        .label("Open feeds.txt")
        .halign(gtk::Align::Center)
        .css_classes(vec!["pill".to_string(), "suggested-action".to_string()])
        .build();
    let empty_box = gtk::Box::new(gtk::Orientation::Vertical, 12);
    empty_box.set_halign(gtk::Align::Center);
    empty_box.append(&empty_btn);
    empty_status.set_child(Some(&empty_box));
    stack.add_named(&empty_status, Some("empty"));

    // Page 2: Loading state
    let loading_spinner = gtk::Spinner::new();
    loading_spinner.set_halign(gtk::Align::Center);
    loading_spinner.set_valign(gtk::Align::Center);
    loading_spinner.set_size_request(40, 40);
    
    let loading_status = adw::StatusPage::builder()
        .title("Checking for Stories...")
        .description("Fetching the latest stories from your configured feeds.")
        .build();
    loading_status.set_child(Some(&loading_spinner));
    stack.add_named(&loading_status, Some("loading"));

    // Page 3: Reader Page
    let reader_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
    
    let progress_bar = gtk::ProgressBar::builder()
        .css_classes(vec!["osd".to_string()])
        .build();
    reader_box.append(&progress_bar);
    reader_box.append(&webview);

    let action_bar = gtk::ActionBar::new();
    let next_btn = gtk::Button::builder()
        .label("Next Story")
        .css_classes(vec!["suggested-action".to_string(), "pill".to_string()])
        .hexpand(true)
        .build();
    action_bar.set_center_widget(Some(&next_btn));
    reader_box.append(&action_bar);

    stack.add_named(&reader_box, Some("reader"));

    // Page 4: Done page (Out of stories)
    let done_status = adw::StatusPage::builder()
        .title("That's it, check back later.")
        .description("You've read all the available stories!")
        .icon_name("face-smile-symbolic")
        .build();
    let close_btn = gtk::Button::builder()
        .label("See ya!")
        .halign(gtk::Align::Center)
        .css_classes(vec!["pill".to_string()])
        .build();
    let done_box = gtk::Box::new(gtk::Orientation::Vertical, 12);
    done_box.set_halign(gtk::Align::Center);
    done_box.append(&close_btn);
    done_status.set_child(Some(&done_box));
    stack.add_named(&done_status, Some("done"));

    let main_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
    main_box.append(&header_bar);
    main_box.append(&stack);
    window.set_content(Some(&main_box));

    let ui = AppUi {
        window: window.clone(),
        stack,
        open_browser_btn,
        email_author_btn,
        empty_edit_btn: empty_btn.clone(),
        loading_spinner,
        progress_bar,
        webview: webview.clone(),
        close_btn: close_btn.clone(),
        title_widget,
    };

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<AppMessage>();

    let ui_clone = ui.clone();
    let state_clone = state.clone();
    let tx_clone = tx.clone();
    
    glib::MainContext::default().spawn_local(async move {
        while let Some(msg) = rx.recv().await {
            match msg {
                AppMessage::StoriesFetched(stories) => {
                    let mut s = state_clone.borrow_mut();
                    s.stories = stories;
                    s.org_story_count = s.stories.len();
                    s.loading = false;
                    drop(s);
                    update_ui(&state_clone, &ui_clone, &tx_clone);
                }
                AppMessage::FetchFailed(err) => {
                    eprintln!("Failed to fetch stories: {}", err);
                    let mut s = state_clone.borrow_mut();
                    s.loading = false;
                    drop(s);
                    update_ui(&state_clone, &ui_clone, &tx_clone);
                }
            }
        }
    });

    empty_btn.connect_clicked(move |_| {
        open_feeds_file();
    });

    close_btn.connect_clicked(move |_| {
        std::process::exit(0);
    });

    let state_c = state.clone();
    ui.open_browser_btn.connect_clicked(move |_| {
        let s = state_c.borrow();
        if !s.stories.is_empty() {
            let url = &s.stories[0].url;
            if !url.is_empty() {
                webbrowser::open(url).ok();
            }
        }
    });

    let state_c = state.clone();
    ui.email_author_btn.connect_clicked(move |_| {
        let s = state_c.borrow();
        if !s.stories.is_empty() {
            if let Some(ref email) = s.stories[0].contact {
                webbrowser::open(&format!("mailto:{}", email)).ok();
            }
        }
    });

    let state_c = state.clone();
    let ui_c = ui.clone();
    let tx_c = tx.clone();
    next_btn.connect_clicked(move |_| {
        let mut s = state_c.borrow_mut();
        if !s.stories.is_empty() {
            let story = s.stories.remove(0);
            add_story_read(&story.url);
        }
        drop(s);
        update_ui(&state_c, &ui_c, &tx_c);
    });

    window.present();

    let state_c = state.clone();
    let ui_c = ui.clone();
    let tx_c = tx.clone();
    update_ui(&state_c, &ui_c, &tx_c);
    
    if !state_c.borrow().feeds.is_empty() {
        trigger_fetch(&state_c, tx_c);
    }
}

pub fn update_ui(
    app_state: &Rc<RefCell<AppState>>, 
    ui: &AppUi, 
    _tx: &tokio::sync::mpsc::UnboundedSender<AppMessage>
) {
    let state = app_state.borrow();
    
    let has_feeds = !state.feeds.is_empty();
    let is_loading = state.loading;
    let no_stories = state.stories.is_empty();
    
    if !has_feeds {
        ui.stack.set_visible_child_name("empty");
        ui.open_browser_btn.set_visible(false);
        ui.email_author_btn.set_visible(false);
        ui.title_widget.set_subtitle("");
    } else if is_loading {
        ui.stack.set_visible_child_name("loading");
        ui.loading_spinner.start();
        ui.open_browser_btn.set_visible(false);
        ui.email_author_btn.set_visible(false);
        ui.title_widget.set_subtitle("");
    } else if no_stories {
        ui.stack.set_visible_child_name("done");
        ui.loading_spinner.stop();
        ui.open_browser_btn.set_visible(false);
        ui.email_author_btn.set_visible(false);
        ui.title_widget.set_subtitle("");
    } else {
        ui.stack.set_visible_child_name("reader");
        ui.loading_spinner.stop();
        
        let story = state.stories[0].clone();
        
        ui.email_author_btn.set_visible(story.contact.is_some());
        ui.open_browser_btn.set_visible(!story.url.is_empty());
        
        let theme_class = match state.theme {
            Theme::Light => "theme-light",
            Theme::Dark => "theme-dark",
            Theme::Sepia => "theme-sepia",
            Theme::System => {
                if adw::StyleManager::default().is_dark() {
                    "theme-dark"
                } else {
                    "theme-light"
                }
            }
        };

        let author_html = if let Some(ref author) = story.author {
            format!("<div class=\"story-author\">By {}</div>", escape_html(author))
        } else {
            String::new()
        };

        let title_html = format!("<h1 class=\"story-title\">{}</h1>", escape_html(story.title.as_deref().unwrap_or("Untitled")));

        let html_content = format!(
            r#"<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<style>
:root {{
  --bg-color: #ffffff;
  --text-color: #242424;
  --border-color: rgba(0, 0, 0, 0.1);
  --details-bg: rgba(0, 0, 0, 0.02);
  --link-color: #3584e4;
  --blockquote-border-color: #3584e4;
  --blockquote-text-color: #5e5e5e;
  --code-bg: rgba(0, 0, 0, 0.05);
}}
html.theme-dark, body.theme-dark {{
  --bg-color: #1e1e1e;
  --text-color: #e0e0e0;
  --border-color: rgba(255, 255, 255, 0.1);
  --details-bg: rgba(255, 255, 255, 0.03);
  --link-color: #78aeed;
  --blockquote-border-color: #3584e4;
  --blockquote-text-color: #a0a0a0;
  --code-bg: rgba(255, 255, 255, 0.08);
}}
html.theme-sepia, body.theme-sepia {{
  --bg-color: #f4ecd8;
  --text-color: #433422;
  --border-color: rgba(67, 52, 34, 0.15);
  --details-bg: rgba(140, 98, 57, 0.04);
  --link-color: #8c6239;
  --blockquote-border-color: #8c6239;
  --blockquote-text-color: #6e5e4f;
  --code-bg: rgba(140, 98, 57, 0.08);
}}
html.theme-light, body.theme-light {{
  --bg-color: #ffffff;
  --text-color: #242424;
  --border-color: rgba(0, 0, 0, 0.1);
  --details-bg: rgba(0, 0, 0, 0.02);
  --link-color: #3584e4;
  --blockquote-border-color: #3584e4;
  --blockquote-text-color: #5e5e5e;
  --code-bg: rgba(0, 0, 0, 0.05);
}}
body {{
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
    line-height: 1.6;
    font-size: 16px;
    margin: 0;
    padding: 0;
    word-wrap: break-word;
    background-color: var(--bg-color);
    color: var(--text-color);
}}
.container {{
    max-width: 680px;
    margin: 0 auto;
    padding: 24px 16px;
}}
.story-title {{
    font-size: 2.2em;
    font-weight: 800;
    line-height: 1.2;
    margin-top: 0;
    margin-bottom: 8px;
}}
.story-author {{
    font-size: 1em;
    opacity: 0.7;
    margin-bottom: 24px;
}}
details {{
    border: 1px solid var(--border-color);
    border-radius: 6px;
    padding: 12px;
    margin-bottom: 16px;
    background-color: var(--details-bg);
}}
summary {{
    font-weight: bold;
    cursor: pointer;
    outline: none;
    padding: 4px;
}}
details[open] {{
    padding-bottom: 12px;
}}
details[open] summary {{
    border-bottom: 1px solid var(--border-color);
    margin-bottom: 8px;
}}
iframe {{
    max-width: 100%;
    border: none;
    border-radius: 6px;
    display: block;
    margin: 16px auto;
}}
a {{
    color: var(--link-color);
    text-decoration: none;
}}
a:hover {{
    text-decoration: underline;
}}
blockquote {{
    border-left: 4px solid var(--blockquote-border-color);
    padding-left: 12px;
    margin: 16px 0;
    font-style: italic;
    color: var(--blockquote-text-color);
}}
pre, code {{
    font-family: monospace;
    background-color: var(--code-bg);
    border-radius: 4px;
}}
pre {{
    padding: 12px;
    overflow-x: auto;
}}
code {{
    padding: 2px 4px;
}}
img {{
    max-width: 100%;
    height: auto;
    border-radius: 6px;
    display: block;
    margin: 16px auto;
}}
</style>
</head>
<body class="{}">
<div class="container">
  {}
  {}
  <div class="story-content">
    {}
  </div>
</div>
</body>
</html>"#,
            theme_class,
            title_html,
            author_html,
            story.html
        );
        
        ui.webview.load_html(&html_content, Some(&story.url));
        
        let progress = if state.org_story_count > 0 {
            1.0 - (state.stories.len() as f64 / state.org_story_count as f64)
        } else {
            1.0
        };
        ui.progress_bar.set_fraction(progress);

        let remaining = state.stories.len();
        let total = state.org_story_count;
        let subtitle = if total <= 1 {
            if remaining == 1 {
                "1 story left".to_string()
            } else {
                "No stories left".to_string()
            }
        } else {
            format!("{} of {} stories left", remaining, total)
        };
        ui.title_widget.set_subtitle(&subtitle);
    }
}
