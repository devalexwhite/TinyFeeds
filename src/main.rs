use std::{
    env::home_dir,
    fs::{self, File, create_dir_all},
    io::{BufRead, BufReader},
    path::Path,
};

use chrono::Datelike;
use rss::Channel;

#[tokio::main]
async fn main() {
    let mut app = App::new();
    app.fetch_stories().await;
}

#[derive(Debug)]
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

        App {
            feeds: Vec::new(),
            stories: Vec::new(),
        }
    }

    async fn fetch_stories(&mut self) {
        let today = chrono::Local::now();

        for feed in self.feeds.clone() {
            if let Ok(feed_content) = reqwest::get(feed).await {
                if let Ok(feed_bytes) = feed_content.bytes().await {
                    if let Ok(channel) = Channel::read_from(&feed_bytes[..]) {
                        for story in channel.items {
                            if let Some(pub_date) = story.pub_date {
                                let pub_date_c =
                                    chrono::DateTime::parse_from_rfc2822(pub_date.clone().as_str())
                                        .unwrap();

                                if pub_date_c.year() != today.year()
                                    || pub_date_c.month() != today.month()
                                    || pub_date_c.day() != today.day()
                                {
                                    continue;
                                }

                                self.stories.push(Story {
                                    author: channel.title.clone(),
                                    url: story.link.unwrap_or(String::from("")),
                                    html: story.description.unwrap_or(String::from("")),
                                });
                            }
                        }
                    }
                }
            }
        }
        println!("{:?}", self.stories);
    }
}

fn file_lines(filename: impl AsRef<Path>) -> Vec<String> {
    let file = File::open(filename).expect("File not found");
    let buf = BufReader::new(file);
    buf.lines()
        .map(|l| l.expect("Could not read line."))
        .collect()
}
