use std::env::home_dir;
use std::fs::{File, OpenOptions, create_dir_all};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use crate::models::Theme;

pub fn get_config_file_path(filename: &str) -> Option<PathBuf> {
    if let Some(mut home) = home_dir() {
        home.push(format!(".config/tinyfeeds/{}", filename));
        return Some(home);
    }
    None
}

pub fn file_lines(filename: impl AsRef<Path>) -> Vec<String> {
    let file = match File::open(filename) {
        Ok(f) => f,
        Err(_) => return Vec::new(),
    };
    let buf = BufReader::new(file);
    buf.lines()
        .map(|l| l.expect("Could not read line."))
        .collect()
}

pub fn save_theme(theme: Theme) {
    if let Some(config) = get_config_file_path("theme.txt") {
        if let Some(parent) = config.parent() {
            let _ = create_dir_all(parent);
        }
        if let Ok(mut file) = OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(config)
        {
            let _ = writeln!(file, "{}", theme.to_str());
        }
    }
}

pub fn add_story_read(url: &str) {
    if let Some(config) = get_config_file_path("read.txt") {
        if let Some(parent) = config.parent() {
            let _ = create_dir_all(parent);
        }
        if let Ok(mut file) = OpenOptions::new()
            .write(true)
            .append(true)
            .create(true)
            .open(config)
        {
            if let Err(e) = writeln!(file, "{}", url) {
                eprintln!("Couldn't write to read.txt: {}", e);
            }
        }
    }
}

pub fn open_feeds_file() {
    if let Some(path) = get_config_file_path("feeds.txt") {
        let editor = std::env::var("VISUAL")
            .or_else(|_| std::env::var("EDITOR"))
            .unwrap_or_else(|_| "xdg-open".to_string());
        let _ = std::process::Command::new(editor).arg(path).spawn();
    }
}

pub fn ensure_config_exists() -> (Vec<String>, Vec<String>, Theme) {
    let read_stories = if let Some(config) = get_config_file_path("read.txt") {
        if !config.exists() {
            if let Some(parent) = config.parent() {
                let _ = create_dir_all(parent);
            }
            let _ = File::create(&config);
        }
        file_lines(config.as_path())
    } else {
        Vec::new()
    };

    let feeds = if let Some(config) = get_config_file_path("feeds.txt") {
        if !config.exists() {
            if let Some(parent) = config.parent() {
                let _ = create_dir_all(parent);
            }
            let _ = File::create(&config);
        }
        file_lines(config.as_path())
    } else {
        let mut feeds_base_path = home_dir().unwrap_or_default();
        feeds_base_path.push(".config/tinyfeeds/");
        let _ = create_dir_all(feeds_base_path.clone());
        let mut feeds_file_path = feeds_base_path;
        feeds_file_path.push("feeds.txt");
        let _ = File::create(feeds_file_path);
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

    (feeds, read_stories, theme)
}
