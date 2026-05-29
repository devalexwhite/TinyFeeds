use clap::Parser;
use std::rc::Rc;
use std::cell::RefCell;
use libadwaita as adw;
use libadwaita::prelude::*;

mod args;
mod config;
mod models;
mod fetcher;
mod ui;

use args::Args;
use models::AppState;
use config::ensure_config_exists;
use ui::build_ui;

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
        let (feeds, read_stories, theme) = ensure_config_exists();
        
        let state = Rc::new(RefCell::new(AppState {
            feeds,
            stories: Vec::new(),
            read_stories,
            org_story_count: 0,
            dev_mode: args.dev_mode,
            loading: true,
            theme,
        }));
        
        build_ui(app, state);
    });

    app.run_with_args::<&str>(&[]);
}
