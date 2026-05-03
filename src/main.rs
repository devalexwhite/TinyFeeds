use std::{
    env::home_dir,
    fs::{self, File, create_dir_all},
    io::{BufRead, BufReader},
    path::Path,
};

fn main() {
    let app = App::new();
}

struct Story {
    author: String,
    url: String,
    html: String,
}

struct App {
    feeds: Vec<String>,
    stories: Vec<Story>,
}

impl App {
    fn new() -> Self {
        if let Some(home) = home_dir() {
            let mut feeds_file_path = home.clone();
            feeds_file_path.push(".config/tinyfeeds/feeds.txt");

            if let Ok(true) = fs::exists(feeds_file_path.as_path()) {
                return App {
                    feeds: file_lines(feeds_file_path.as_path()),
                    stories: Vec::new(),
                };
            }

            let mut feeds_base_path = home.clone();

            feeds_base_path.push(".config/tinyfeeds/");

            create_dir_all(feeds_base_path).expect("Failed to create config directory.");
            File::create(feeds_file_path).expect("Failed to create config file.");
        }

        App { feeds: Vec::new() }
    }
}

fn file_lines(filename: impl AsRef<Path>) -> Vec<String> {
    let file = File::open(filename).expect("File not found");
    let buf = BufReader::new(file);
    buf.lines()
        .map(|l| l.expect("Could not read line."))
        .collect()
}
