use std::collections::{HashMap, HashSet};
use gtk4::{self as gtk};

#[derive(Debug, Clone)]
pub struct Story {
    pub author: Option<String>,
    pub title: Option<String>,
    pub url: String,
    pub markdown: String, // Preprocessed readability markdown
    pub contact: Option<String>,
}

#[derive(Debug, Clone)]
pub enum AppMessage {
    StoriesFetched(Vec<Story>),
    #[allow(dead_code)]
    FetchFailed(String),
    ImageDownloaded { url: String, bytes: Vec<u8> },
    ImageDownloadFailed { url: String, error: String },
}

#[derive(Debug, Clone)]
pub struct ImageWidgetRef {
    pub stack: gtk::Stack,
    pub picture: gtk::Picture,
    pub error_label: gtk::Label,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    System,
    Light,
    Sepia,
    Dark,
}

impl Theme {
    pub fn to_str(self) -> &'static str {
        match self {
            Theme::System => "system",
            Theme::Light => "light",
            Theme::Sepia => "sepia",
            Theme::Dark => "dark",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "light" => Theme::Light,
            "sepia" => Theme::Sepia,
            "dark" => Theme::Dark,
            _ => Theme::System,
        }
    }
}

pub struct AppState {
    pub feeds: Vec<String>,
    pub stories: Vec<Story>,
    pub read_stories: Vec<String>,
    pub org_story_count: usize,
    pub dev_mode: bool,
    pub loading: bool,
    pub image_widgets: HashMap<String, Vec<ImageWidgetRef>>,
    pub images_in_progress: HashSet<String>,
    pub theme: Theme,
}
