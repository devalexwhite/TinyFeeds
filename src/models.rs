
#[derive(Debug, Clone)]
pub struct Story {
    pub author: Option<String>,
    pub title: Option<String>,
    pub url: String,
    pub html: String,
    pub contact: Option<String>,
}

#[derive(Debug, Clone)]
pub enum AppMessage {
    StoriesFetched(Vec<Story>),
    #[allow(dead_code)]
    FetchFailed(String),
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
    pub theme: Theme,
}
