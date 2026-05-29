use clap::Parser;
use feedparser_rs::parse;
use readabilityrs::{Readability, ReadabilityOptions};
use reqwest::Client;
use std::{
    collections::{HashMap, HashSet},
    env::home_dir,
    fs::{File, OpenOptions, create_dir_all},
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    rc::Rc,
    cell::RefCell,
    time::Duration,
};

use gtk4::prelude::*;
use libadwaita::prelude::*;
use gtk4::{self as gtk, gdk, glib, gdk_pixbuf};
use libadwaita::{self as adw};

use crate::image_loader::download_image;

mod image_loader;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[arg(long, short, action)]
    dev_mode: bool,
}

#[derive(Debug, Clone)]
struct Story {
    author: Option<String>,
    title: Option<String>,
    url: String,
    html: String,
    contact: Option<String>,
}

#[allow(dead_code)]
enum AppMessage {
    StoriesFetched(Vec<Story>),
    FetchFailed(String),
    ImageDownloaded { url: String, bytes: Vec<u8> },
    ImageDownloadFailed { url: String, error: String },
}

struct ImageWidgetRef {
    stack: gtk::Stack,
    picture: gtk::Picture,
    error_label: gtk::Label,
}

struct AppState {
    feeds: Vec<String>,
    stories: Vec<Story>,
    read_stories: Vec<String>,
    org_story_count: usize,
    dev_mode: bool,
    loading: bool,
    image_widgets: HashMap<String, Vec<ImageWidgetRef>>,
    images_in_progress: HashSet<String>,
    theme: Theme,
}

#[allow(dead_code)]
#[derive(Clone)]
struct AppUi {
    window: adw::ApplicationWindow,
    stack: gtk::Stack,
    open_browser_btn: gtk::Button,
    email_author_btn: gtk::Button,
    empty_edit_btn: gtk::Button,
    loading_spinner: gtk::Spinner,
    progress_bar: gtk::ProgressBar,
    title_label: gtk::Label,
    author_label: gtk::Label,
    body_box: gtk::Box,
    scrolled_window: gtk::ScrolledWindow,
    close_btn: gtk::Button,
    title_widget: adw::WindowTitle,
}

fn main() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to build Tokio runtime");
    let _guard = rt.enter();

    let args = Args::parse();
    
    let app = adw::Application::builder()
        .application_id("dev.thatalexguy.tinyfeeds")
        .build();

    app.connect_activate(move |app| {
        build_ui(app, args.dev_mode);
    });

    app.run_with_args::<&str>(&[]);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Theme {
    System,
    Light,
    Sepia,
    Dark,
}

impl Theme {
    fn to_str(self) -> &'static str {
        match self {
            Theme::System => "system",
            Theme::Light => "light",
            Theme::Sepia => "sepia",
            Theme::Dark => "dark",
        }
    }

    fn from_str(s: &str) -> Self {
        match s {
            "light" => Theme::Light,
            "sepia" => Theme::Sepia,
            "dark" => Theme::Dark,
            _ => Theme::System,
        }
    }
}

fn save_theme(theme: Theme) {
    if let Some(config) = get_config_file_path("theme.txt") {
        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(config)
            .unwrap();
        let _ = writeln!(file, "{}", theme.to_str());
    }
}

fn apply_theme(theme: Theme, window: &adw::ApplicationWindow) {
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

fn create_theme_button(
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

fn update_theme_buttons_ui(selected_theme: Theme, buttons: &[(Theme, gtk::Button, gtk::Image)]) {
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

fn build_ui(app: &adw::Application, dev_mode: bool) {
    // Register custom CSS stylesheet for themes and custom controls
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
        &gdk::Display::default().unwrap(),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    let read_stories = if let Some(config) = get_config_file_path("read.txt") {
        if !config.exists() {
            if let Some(parent) = config.parent() {
                create_dir_all(parent).expect("Failed to create config directory.");
            }
            File::create(&config).expect("Failed to create read.txt");
        }
        file_lines(config.as_path())
    } else {
        Vec::new()
    };

    let feeds = if let Some(config) = get_config_file_path("feeds.txt") {
        if !config.exists() {
            if let Some(parent) = config.parent() {
                create_dir_all(parent).expect("Failed to create config directory.");
            }
            File::create(&config).expect("Failed to create feeds.txt");
        }
        file_lines(config.as_path())
    } else {
        let mut feeds_base_path = home_dir().unwrap_or_default();
        feeds_base_path.push(".config/tinyfeeds/");
        create_dir_all(feeds_base_path.clone()).expect("Failed to create config directory.");
        let mut feeds_file_path = feeds_base_path;
        feeds_file_path.push("feeds.txt");
        File::create(feeds_file_path).expect("Failed to create config file.");
        Vec::new()
    };

    let theme = if let Some(config) = get_config_file_path("theme.txt") {
        if config.exists() {
            let lines = file_lines(&config);
            if let Some(line) = lines.first() {
                Theme::from_str(line.trim())
            } else {
                Theme::System
            }
        } else {
            Theme::System
        }
    } else {
        Theme::System
    };

    let state = Rc::new(RefCell::new(AppState {
        feeds,
        stories: Vec::new(),
        read_stories,
        org_story_count: 0,
        dev_mode,
        loading: true,
        image_widgets: HashMap::new(),
        images_in_progress: HashSet::new(),
        theme,
    }));

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("TinyFeeds")
        .default_width(800)
        .default_height(700)
        .build();

    apply_theme(theme, &window);

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

    update_theme_buttons_ui(theme, &theme_buttons);

    let theme_buttons_rc = Rc::new(theme_buttons);

    let state_c = state.clone();
    let window_c = window.clone();
    let theme_buttons_c = theme_buttons_rc.clone();
    system_btn.connect_clicked(move |_| {
        let mut s = state_c.borrow_mut();
        s.theme = Theme::System;
        drop(s);
        apply_theme(Theme::System, &window_c);
        save_theme(Theme::System);
        update_theme_buttons_ui(Theme::System, &theme_buttons_c);
    });

    let state_c = state.clone();
    let window_c = window.clone();
    let theme_buttons_c = theme_buttons_rc.clone();
    light_btn.connect_clicked(move |_| {
        let mut s = state_c.borrow_mut();
        s.theme = Theme::Light;
        drop(s);
        apply_theme(Theme::Light, &window_c);
        save_theme(Theme::Light);
        update_theme_buttons_ui(Theme::Light, &theme_buttons_c);
    });

    let state_c = state.clone();
    let window_c = window.clone();
    let theme_buttons_c = theme_buttons_rc.clone();
    sepia_btn.connect_clicked(move |_| {
        let mut s = state_c.borrow_mut();
        s.theme = Theme::Sepia;
        drop(s);
        apply_theme(Theme::Sepia, &window_c);
        save_theme(Theme::Sepia);
        update_theme_buttons_ui(Theme::Sepia, &theme_buttons_c);
    });

    let state_c = state.clone();
    let window_c = window.clone();
    let theme_buttons_c = theme_buttons_rc.clone();
    dark_btn.connect_clicked(move |_| {
        let mut s = state_c.borrow_mut();
        s.theme = Theme::Dark;
        drop(s);
        apply_theme(Theme::Dark, &window_c);
        save_theme(Theme::Dark);
        update_theme_buttons_ui(Theme::Dark, &theme_buttons_c);
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
        let about = gtk::AboutDialog::builder()
            .program_name("TinyFeeds")
            .version("0.1.0")
            .website("https://thatalexguy.dev")
            .authors(vec!["Alex White".to_string()])
            .license_type(gtk::License::Gpl30Only)
            .transient_for(&window_c)
            .build();
        about.present();
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

    let scrolled_window = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .vexpand(true)
        .build();
        
    let clamp = adw::Clamp::builder()
        .maximum_size(680)
        .tightening_threshold(560)
        .build();
        
    let content_box = gtk::Box::new(gtk::Orientation::Vertical, 16);
    content_box.set_margin_top(24);
    content_box.set_margin_bottom(24);
    content_box.set_margin_start(16);
    content_box.set_margin_end(16);

    let title_label = gtk::Label::builder()
        .wrap(true)
        .halign(gtk::Align::Start)
        .selectable(true)
        .build();
    title_label.add_css_class("title-1");
    
    let author_label = gtk::Label::builder()
        .wrap(true)
        .halign(gtk::Align::Start)
        .selectable(true)
        .build();
    author_label.add_css_class("dim-label");
    author_label.add_css_class("body");
    author_label.set_margin_bottom(24);

    let body_box = gtk::Box::new(gtk::Orientation::Vertical, 0);

    content_box.append(&title_label);
    content_box.append(&author_label);
    content_box.append(&body_box);
    
    clamp.set_child(Some(&content_box));
    scrolled_window.set_child(Some(&clamp));
    reader_box.append(&scrolled_window);

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
        title_label,
        author_label,
        body_box,
        scrolled_window,
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
                AppMessage::ImageDownloaded { url, bytes } => {
                    handle_downloaded_image(&state_clone, url, bytes);
                }
                AppMessage::ImageDownloadFailed { url, error } => {
                    handle_download_failed(&state_clone, url, error);
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
            add_story_read(story);
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

fn open_feeds_file() {
    if let Some(path) = get_config_file_path("feeds.txt") {
        let editor = std::env::var("VISUAL")
            .or_else(|_| std::env::var("EDITOR"))
            .unwrap_or_else(|_| "xdg-open".to_string());
        let _ = std::process::Command::new(editor).arg(path).spawn();
    }
}

fn trigger_fetch(state: &Rc<RefCell<AppState>>, tx: tokio::sync::mpsc::UnboundedSender<AppMessage>) {
    let feeds = state.borrow().feeds.clone();
    let read_stories = state.borrow().read_stories.clone();
    let dev_mode = state.borrow().dev_mode;

    tokio::spawn(async move {
        if dev_mode {
            tokio::time::sleep(Duration::from_secs(1)).await;
            let dummy = Story {
                title: Some("Just a test article".to_string()),
                author: Some("Alex White".to_string()),
                url: "https://thatalexguy.dev".to_string(),
                html: "<b>This is a sample story</b><h1>It tests the UI in dev mode</h1><h3>Enjoy being stuck in <i>sample land</i></h3><details><summary>Test</summary>Hi there</details><p>ajskdkajdlkajd sajd ksajdlkaj dlkajsdkjsakdjaslkdja lkdjsalkd jaslkd jaklsjd kajsd lkjsa dlja d</p><p>kjsad lkjsadlsa jdalksd jlksajdak jdlkjsad lasjd lksajdlsajd lkajsd lkjsa dlkjsa dkja lkdjasd </p>".to_string(),
                contact: Some("hi@thatalexguy.dev".to_string()),
            };
            tx.send(AppMessage::StoriesFetched(vec![dummy])).ok();
            return;
        }

        let mut stories = Vec::new();
        let mut tasks = Vec::new();
        for feed in feeds {
            let rs = read_stories.clone();
            tasks.push(tokio::spawn(fetch_feed_stories(feed, rs)));
        }

        for task in tasks {
            if let Ok(res) = task.await {
                stories.extend(res);
            }
        }

        tx.send(AppMessage::StoriesFetched(stories)).ok();
    });
}

fn clear_container(container: &gtk::Box) {
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
     .replace('<', "&lt;")
     .replace('>', "&gt;")
     .replace('"', "&quot;")
     .replace('\'', "&apos;")
}

fn handle_downloaded_image(app_state: &Rc<RefCell<AppState>>, url: String, bytes: Vec<u8>) {
    let state = app_state.borrow();
    
    let loader = gdk_pixbuf::PixbufLoader::new();
    if loader.write(&bytes).is_ok() && loader.close().is_ok() {
        if let Some(pixbuf) = loader.pixbuf() {
            let texture = gdk::Texture::for_pixbuf(&pixbuf);
            
            if let Some(refs) = state.image_widgets.get(&url) {
                for r in refs {
                    r.picture.set_paintable(Some(&texture));
                    r.stack.set_visible_child(&r.picture);
                }
            }
        }
    } else {
        if let Some(refs) = state.image_widgets.get(&url) {
            for r in refs {
                r.stack.set_visible_child(&r.error_label);
            }
        }
    }
}

fn handle_download_failed(app_state: &Rc<RefCell<AppState>>, url: String, error: String) {
    eprintln!("Failed to download image {}: {}", url, error);
    let state = app_state.borrow();
    if let Some(refs) = state.image_widgets.get(&url) {
        for r in refs {
            r.stack.set_visible_child(&r.error_label);
        }
    }
}

fn render_markdown(
    markdown: &str, 
    container: &gtk::Box, 
    app_state: &Rc<RefCell<AppState>>,
    tx: &tokio::sync::mpsc::UnboundedSender<AppMessage>
) {
    let parser = pulldown_cmark::Parser::new(markdown);
    
    let mut current_markup = String::new();
    let mut is_ordered = false;
    let mut list_index = 0;
    
    let mut in_blockquote = false;
    let mut blockquote_markup = String::new();
    
    let mut in_code_block = false;
    let mut code_block_text = String::new();
    
    for event in parser {
        match event {
            pulldown_cmark::Event::Start(tag) => {
                match tag {
                    pulldown_cmark::Tag::Heading { .. } => {
                        current_markup.clear();
                    }
                    pulldown_cmark::Tag::Paragraph => {
                        if !in_blockquote {
                            current_markup.clear();
                        }
                    }
                    pulldown_cmark::Tag::BlockQuote(_) => {
                        in_blockquote = true;
                        blockquote_markup.clear();
                    }
                    pulldown_cmark::Tag::CodeBlock(..) => {
                        in_code_block = true;
                        code_block_text.clear();
                    }
                    pulldown_cmark::Tag::List(ordered_start) => {
                        is_ordered = ordered_start.is_some();
                        list_index = ordered_start.unwrap_or(1) as usize;
                    }
                    pulldown_cmark::Tag::Item => {
                        current_markup.clear();
                        if is_ordered {
                            current_markup.push_str(&format!("{}. ", list_index));
                            list_index += 1;
                        } else {
                            current_markup.push_str("• ");
                        }
                    }
                    pulldown_cmark::Tag::Emphasis => {
                        current_markup.push_str("<i>");
                    }
                    pulldown_cmark::Tag::Strong => {
                        current_markup.push_str("<b>");
                    }
                    pulldown_cmark::Tag::Strikethrough => {
                        current_markup.push_str("<s>");
                    }
                    pulldown_cmark::Tag::Link { dest_url, .. } => {
                        current_markup.push_str(&format!("<a href=\"{}\">", escape_html(&dest_url)));
                    }
                    pulldown_cmark::Tag::Image { dest_url, .. } => {
                        let stack = gtk::Stack::new();
                        stack.set_transition_type(gtk::StackTransitionType::Crossfade);
                        
                        let spinner = gtk::Spinner::new();
                        spinner.start();
                        spinner.set_halign(gtk::Align::Center);
                        spinner.set_valign(gtk::Align::Center);
                        
                        let picture = gtk::Picture::new();
                        picture.set_keep_aspect_ratio(true);
                        picture.set_halign(gtk::Align::Center);
                        picture.set_margin_top(12);
                        picture.set_margin_bottom(12);
                        picture.set_height_request(200);
                        
                        let error_label = gtk::Label::new(Some("Failed to load image"));
                        error_label.add_css_class("dim-label");
                        
                        stack.add_child(&spinner);
                        stack.add_child(&picture);
                        stack.add_child(&error_label);
                        
                        stack.set_visible_child(&spinner);
                        
                        container.append(&stack);
                        
                        let mut state = app_state.borrow_mut();
                        state.image_widgets.entry(dest_url.to_string()).or_default().push(ImageWidgetRef {
                            stack: stack.clone(),
                            picture: picture.clone(),
                            error_label: error_label.clone(),
                        });
                        
                        if state.images_in_progress.insert(dest_url.to_string()) {
                            let url = dest_url.to_string();
                            let base_url = state.stories.first().map(|s| s.url.clone());
                            let tx_c = tx.clone();
                            
                            tokio::spawn(async move {
                                match download_image(url.clone(), base_url).await {
                                    Ok(img) => {
                                        tx_c.send(AppMessage::ImageDownloaded { url, bytes: img.bytes }).ok();
                                    }
                                    Err(err) => {
                                        tx_c.send(AppMessage::ImageDownloadFailed { url, error: err }).ok();
                                    }
                                }
                            });
                        }
                    }
                    _ => {}
                }
            }
            pulldown_cmark::Event::End(tag) => {
                match tag {
                    pulldown_cmark::TagEnd::Heading(level) => {
                        let label = gtk::Label::new(None);
                        let markup = match level {
                            pulldown_cmark::HeadingLevel::H1 => format!("<span size=\"xx-large\" weight=\"bold\">{}</span>", current_markup),
                            pulldown_cmark::HeadingLevel::H2 => format!("<span size=\"x-large\" weight=\"bold\">{}</span>", current_markup),
                            pulldown_cmark::HeadingLevel::H3 => format!("<span size=\"large\" weight=\"bold\">{}</span>", current_markup),
                            _ => format!("<span size=\"medium\" weight=\"bold\">{}</span>", current_markup),
                        };
                        label.set_markup(&markup);
                        label.set_wrap(true);
                        label.set_halign(gtk::Align::Start);
                        label.set_selectable(true);
                        label.set_margin_top(16);
                        label.set_margin_bottom(8);
                        container.append(&label);
                    }
                    pulldown_cmark::TagEnd::Paragraph => {
                        if !in_blockquote {
                            let label = gtk::Label::new(None);
                            label.set_markup(&current_markup);
                            label.set_wrap(true);
                            label.set_halign(gtk::Align::Start);
                            label.set_selectable(true);
                            label.set_margin_bottom(12);
                            label.connect_activate_link(|_, uri| {
                                webbrowser::open(uri).ok();
                                glib::Propagation::Stop
                            });
                            container.append(&label);
                        }
                    }
                    pulldown_cmark::TagEnd::BlockQuote(_) => {
                        in_blockquote = false;
                        let label = gtk::Label::new(None);
                        label.set_markup(&blockquote_markup);
                        label.set_wrap(true);
                        label.set_halign(gtk::Align::Start);
                        label.set_selectable(true);
                        label.add_css_class("blockquote");
                        label.set_margin_bottom(12);
                        label.connect_activate_link(|_, uri| {
                            webbrowser::open(uri).ok();
                            glib::Propagation::Stop
                        });
                        container.append(&label);
                    }
                    pulldown_cmark::TagEnd::CodeBlock => {
                        in_code_block = false;
                        let label = gtk::Label::new(Some(&code_block_text));
                        label.set_wrap(true);
                        label.set_halign(gtk::Align::Start);
                        label.set_selectable(true);
                        label.add_css_class("code-block");
                        label.set_margin_bottom(12);
                        container.append(&label);
                    }
                    pulldown_cmark::TagEnd::List(..) => {}
                    pulldown_cmark::TagEnd::Item => {
                        let label = gtk::Label::new(None);
                        label.set_markup(&current_markup);
                        label.set_wrap(true);
                        label.set_halign(gtk::Align::Start);
                        label.set_selectable(true);
                        label.set_margin_start(20);
                        label.set_margin_bottom(6);
                        label.connect_activate_link(|_, uri| {
                            webbrowser::open(uri).ok();
                            glib::Propagation::Stop
                        });
                        container.append(&label);
                    }
                    pulldown_cmark::TagEnd::Emphasis => {
                        current_markup.push_str("</i>");
                    }
                    pulldown_cmark::TagEnd::Strong => {
                        current_markup.push_str("</b>");
                    }
                    pulldown_cmark::TagEnd::Strikethrough => {
                        current_markup.push_str("</s>");
                    }
                    pulldown_cmark::TagEnd::Link => {
                        current_markup.push_str("</a>");
                    }
                    _ => {}
                }
            }
            pulldown_cmark::Event::Text(text) => {
                let escaped = escape_html(&text);
                if in_code_block {
                    code_block_text.push_str(&text);
                } else if in_blockquote {
                    blockquote_markup.push_str(&escaped);
                } else {
                    current_markup.push_str(&escaped);
                }
            }
            pulldown_cmark::Event::Code(text) => {
                let escaped = escape_html(&text);
                let formatted = format!("<tt>{}</tt>", escaped);
                if in_blockquote {
                    blockquote_markup.push_str(&formatted);
                } else {
                    current_markup.push_str(&formatted);
                }
            }
            pulldown_cmark::Event::SoftBreak | pulldown_cmark::Event::HardBreak => {
                if in_blockquote {
                    blockquote_markup.push('\n');
                } else {
                    current_markup.push('\n');
                }
            }
            pulldown_cmark::Event::Rule => {
                let separator = gtk::Separator::new(gtk::Orientation::Horizontal);
                separator.set_margin_top(12);
                separator.set_margin_bottom(12);
                container.append(&separator);
            }
            _ => {}
        }
    }
}

fn update_ui(
    app_state: &Rc<RefCell<AppState>>, 
    ui: &AppUi, 
    tx: &tokio::sync::mpsc::UnboundedSender<AppMessage>
) {
    let mut state = app_state.borrow_mut();
    
    state.image_widgets.clear();
    state.images_in_progress.clear();
    
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
        
        ui.title_label.set_text(story.title.as_deref().unwrap_or("Untitled"));
        
        if let Some(ref author) = story.author {
            ui.author_label.set_text(&format!("By {}", author));
            ui.author_label.set_visible(true);
        } else {
            ui.author_label.set_visible(false);
        }
        
        ui.email_author_btn.set_visible(story.contact.is_some());
        ui.open_browser_btn.set_visible(!story.url.is_empty());
        
        let vadjustment = ui.scrolled_window.vadjustment();
        vadjustment.set_value(vadjustment.lower());
        
        clear_container(&ui.body_box);
        
        let readability_options = ReadabilityOptions {
            output_markdown: true,
            ..Default::default()
        };
        
        let markdown = if let Ok(rd) = Readability::new(
            story.html.as_str(),
            Some(story.url.as_str()),
            Some(readability_options),
        ) && let Some(article) = rd.parse() {
            article.markdown_content.unwrap_or_else(|| story.html.clone())
        } else {
            story.html.clone()
        };
        
        drop(state); 
        render_markdown(&markdown, &ui.body_box, app_state, tx);
        
        let state = app_state.borrow();
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



fn file_lines(filename: impl AsRef<Path>) -> Vec<String> {
    let file = File::open(filename).expect("File not found");
    let buf = BufReader::new(file);
    buf.lines()
        .map(|l| l.expect("Could not read line."))
        .collect()
}

async fn fetch_feed_stories(feed: String, read_stories: Vec<String>) -> Vec<Story> {
    let today = chrono::Local::now();
    let client = Client::new();
    let mut stories = Vec::new();

    if let Ok(feed_content) = client
        .get(feed)
        .timeout(Duration::from_secs(10))
        .send()
        .await
        && let Ok(feed_bytes) = feed_content.bytes().await
        && let Ok(channel) = parse(&feed_bytes[..])
    {
        for story in channel.entries {
            if let Some(pub_date) = story.updated {
                if let Some(url) = story.link.clone() {
                    if read_stories.contains(&url) {
                        continue;
                    }
                }

                if pub_date.date_naive() != today.date_naive() {
                    break;
                }

                let mut content = story
                    .content
                    .iter()
                    .map(|e| e.value.clone())
                    .fold(String::from(""), |a, e| format!("{}{}", a, e));

                if content.is_empty() {
                    if let Some(ref summary) = story.summary {
                        content = summary.clone();
                    }
                }

                let contact = find_story_email(&story, &channel.feed);
                stories.push(Story {
                    author: story.author.map(|f| f.to_string()),
                    title: story.title,
                    url: story.link.unwrap_or(String::from("")),
                    contact,
                    html: content,
                });
            }
        }
    }
    stories
}

fn add_story_read(story: Story) {
    if let Some(config) = get_config_file_path("read.txt") {
        let mut file = OpenOptions::new()
            .write(true)
            .append(true)
            .create(true)
            .open(config)
            .unwrap();

        if let Err(e) = writeln!(file, "{}", story.url) {
            eprintln!("Couldn't write to file: {}", e);
        }
    }
}

fn get_config_file_path(filename: &str) -> Option<PathBuf> {
    if let Some(mut home) = home_dir() {
        home.push(format!(".config/tinyfeeds/{}", filename));
        return Some(home);
    }
    None
}

fn extract_email_from_string(s: &str) -> Option<String> {
    let delimiters = [' ', '(', ')', '<', '>', '[', ']', ',', ';'];
    for part in s.split(&delimiters[..]) {
        let part = part.trim();
        if part.contains('@') && part.contains('.') {
            if let Some(at_idx) = part.find('@') {
                let local_part = &part[..at_idx];
                let domain_part = &part[at_idx + 1..];
                if !local_part.is_empty() && !domain_part.is_empty() && domain_part.contains('.') {
                    let cleaned = part.trim_end_matches(|c: char| c.is_ascii_punctuation() && c != '.' && c != '-' && c != '_');
                    return Some(cleaned.to_string());
                }
            }
        }
    }
    None
}

fn find_story_email(story: &feedparser_rs::Entry, feed_meta: &feedparser_rs::FeedMeta) -> Option<String> {
    // 1. Try entry-level author details
    if let Some(ref ad) = story.author_detail {
        if let Some(ref email) = ad.email {
            if let Some(extracted) = extract_email_from_string(email) {
                return Some(extracted);
            }
        }
        if let Some(ref name) = ad.name {
            if let Some(extracted) = extract_email_from_string(name) {
                return Some(extracted);
            }
        }
    }

    // 2. Try entry-level authors list
    for author in &story.authors {
        if let Some(ref email) = author.email {
            if let Some(extracted) = extract_email_from_string(email) {
                return Some(extracted);
            }
        }
        if let Some(ref name) = author.name {
            if let Some(extracted) = extract_email_from_string(name) {
                return Some(extracted);
            }
        }
    }

    // 3. Try entry-level contributors list
    for contributor in &story.contributors {
        if let Some(ref email) = contributor.email {
            if let Some(extracted) = extract_email_from_string(email) {
                return Some(extracted);
            }
        }
        if let Some(ref name) = contributor.name {
            if let Some(extracted) = extract_email_from_string(name) {
                return Some(extracted);
            }
        }
    }

    // 4. Try parsing the raw entry author string
    if let Some(ref author_str) = story.author {
        if let Some(extracted) = extract_email_from_string(author_str) {
            return Some(extracted);
        }
    }

    // 5. Try feed-level author details
    if let Some(ref ad) = feed_meta.author_detail {
        if let Some(ref email) = ad.email {
            if let Some(extracted) = extract_email_from_string(email) {
                return Some(extracted);
            }
        }
        if let Some(ref name) = ad.name {
            if let Some(extracted) = extract_email_from_string(name) {
                return Some(extracted);
            }
        }
    }

    // 6. Try feed-level publisher details (managingEditor / webMaster in RSS)
    if let Some(ref pd) = feed_meta.publisher_detail {
        if let Some(ref email) = pd.email {
            if let Some(extracted) = extract_email_from_string(email) {
                return Some(extracted);
            }
        }
        if let Some(ref name) = pd.name {
            if let Some(extracted) = extract_email_from_string(name) {
                return Some(extracted);
            }
        }
    }

    // 7. Try feed-level authors list
    for author in &feed_meta.authors {
        if let Some(ref email) = author.email {
            if let Some(extracted) = extract_email_from_string(email) {
                return Some(extracted);
            }
        }
        if let Some(ref name) = author.name {
            if let Some(extracted) = extract_email_from_string(name) {
                return Some(extracted);
            }
        }
    }

    // 8. Try feed-level contributors list
    for contributor in &feed_meta.contributors {
        if let Some(ref email) = contributor.email {
            if let Some(extracted) = extract_email_from_string(email) {
                return Some(extracted);
            }
        }
        if let Some(ref name) = contributor.name {
            if let Some(extracted) = extract_email_from_string(name) {
                return Some(extracted);
            }
        }
    }

    // 9. Try parsing the raw feed author string
    if let Some(ref author_str) = feed_meta.author {
        if let Some(extracted) = extract_email_from_string(author_str) {
            return Some(extracted);
        }
    }

    // 10. Try parsing the raw feed publisher string
    if let Some(ref publisher_str) = feed_meta.publisher {
        if let Some(extracted) = extract_email_from_string(publisher_str) {
            return Some(extracted);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_email() {
        assert_eq!(extract_email_from_string("email@email.com (Author Name)"), Some("email@email.com".to_string()));
        assert_eq!(extract_email_from_string("Author Name <email@email.com>"), Some("email@email.com".to_string()));
        assert_eq!(extract_email_from_string("email@email.com"), Some("email@email.com".to_string()));
        assert_eq!(extract_email_from_string("Author Name (email@email.com)"), Some("email@email.com".to_string()));
        assert_eq!(extract_email_from_string("Author Name"), None);
    }

    #[test]
    fn test_email_parsing() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<rss version="2.0">
<channel>
 <title>Test RSS</title>
 <description>Test</description>
 <link>http://example.com</link>
  <managingEditor>editor@example.com (Editor Name)</managingEditor>
  <webMaster>webmaster@example.com (Webmaster Name)</webMaster>
 <item>
  <title>Test Item</title>
  <link>http://example.com/item</link>
  <author>email@email.com (Author Name)</author>
 </item>
</channel>
</rss>"#;
        let feed = feedparser_rs::parse(xml.as_bytes()).unwrap();
        let entry = &feed.entries[0];
        
        // Assert that we successfully resolve the entry-level author email
        assert_eq!(find_story_email(entry, &feed.feed), Some("email@email.com".to_string()));

        // Assert feed level fallback to managingEditor
        let mut entry_no_author = entry.clone();
        entry_no_author.author_detail = None;
        entry_no_author.author = None;
        assert_eq!(find_story_email(&entry_no_author, &feed.feed), Some("editor@example.com".to_string()));

        // Assert feed level fallback to webMaster
        let mut feed_meta_no_editor = feed.feed.clone();
        feed_meta_no_editor.author_detail = None;
        feed_meta_no_editor.author = None;
        assert_eq!(find_story_email(&entry_no_author, &feed_meta_no_editor), Some("webmaster@example.com".to_string()));
    }
}
