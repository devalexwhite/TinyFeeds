use std::rc::Rc;
use std::cell::RefCell;
use libadwaita as adw;
use gtk4::{self as gtk, gdk, gdk_pixbuf, glib};
use libadwaita::prelude::*;
use crate::models::{AppState, AppMessage, Theme};
use crate::config::{save_theme, add_story_read, open_feeds_file};
use crate::fetcher::trigger_fetch;
use crate::ui::theme::{load_custom_css, apply_theme, create_theme_button, update_theme_buttons_ui};
use crate::ui::markdown::{clear_container, render_markdown};

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
    pub title_label: gtk::Label,
    pub author_label: gtk::Label,
    pub body_box: gtk::Box,
    pub scrolled_window: gtk::ScrolledWindow,
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
        
        let markdown = story.markdown.clone();
        
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

pub fn handle_downloaded_image(app_state: &Rc<RefCell<AppState>>, url: String, bytes: Vec<u8>) {
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

pub fn handle_download_failed(app_state: &Rc<RefCell<AppState>>, url: String, error: String) {
    eprintln!("Failed to download image {}: {}", url, error);
    let state = app_state.borrow();
    if let Some(refs) = state.image_widgets.get(&url) {
        for r in refs {
            r.stack.set_visible_child(&r.error_label);
        }
    }
}
