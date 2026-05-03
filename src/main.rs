use std::{
    env::home_dir,
    fs::{self, File, create_dir_all},
    io::{BufRead, BufReader},
    path::Path,
};

use chrono::Datelike;
use frostmark::{MarkState, MarkWidget};
use iced::{
    Element, Task,
    widget::{button, column, container, scrollable, text},
};
use rss::Channel;

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view).run()
}

#[derive(Debug, Clone)]
struct Story {
    author: String,
    url: String,
    html: String,
}

#[derive(Debug, Clone)]
enum Message {
    FetchStories,
    SetStories(Vec<Story>),
    ReadStory,
    SetStory,
}

struct App {
    feeds: Vec<String>,
    stories: Vec<Story>,
    mark_state: MarkState,
}

impl App {
    fn new() -> (Self, Task<Message>) {
        if let Some(home) = home_dir() {
            let mut feeds_file_path = home.clone();
            feeds_file_path.push(".config/tinyfeeds/feeds.txt");

            if let Ok(true) = fs::exists(feeds_file_path.as_path()) {
                return (
                    App {
                        feeds: file_lines(feeds_file_path.as_path()),
                        stories: Vec::new(),
                        mark_state: MarkState::with_html(""),
                    },
                    Task::done(Message::FetchStories),
                );
            }

            let mut feeds_base_path = home.clone();

            feeds_base_path.push(".config/tinyfeeds/");

            create_dir_all(feeds_base_path).expect("Failed to create config directory.");
            File::create(feeds_file_path).expect("Failed to create config file.");
        }

        (
            App {
                feeds: Vec::new(),
                stories: Vec::new(),
                mark_state: MarkState::with_html(""),
            },
            Task::done(Message::FetchStories),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::FetchStories => {
                Task::perform(fetch_stories(self.feeds.clone()), Message::SetStories)
            }
            Message::SetStories(stories) => {
                self.stories = stories.clone();

                Task::done(Message::SetStory)
            }
            Message::ReadStory => {
                self.stories.pop();

                Task::done(Message::SetStory)
            }
            Message::SetStory => {
                if self.stories.len() > 0 {
                    self.mark_state = MarkState::with_html(self.stories[0].html.clone().as_str());
                } else {
                    self.mark_state = MarkState::with_markdown_only("# No More Stories Today!");
                }
                Task::none()
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        if self.stories.len() == 0 {
            container(text("Loading Feeds...").size(30))
                .padding(10)
                .center(800)
                .style(container::rounded_box)
                .into()
        } else {
            container(scrollable(column![
                MarkWidget::new(&self.mark_state),
                button("Next Story").on_press(Message::ReadStory),
            ]))
            .padding(10)
            .center(800)
            .style(container::rounded_box)
            .into()
        }
    }
}

fn file_lines(filename: impl AsRef<Path>) -> Vec<String> {
    let file = File::open(filename).expect("File not found");
    let buf = BufReader::new(file);
    buf.lines()
        .map(|l| l.expect("Could not read line."))
        .collect()
}

async fn fetch_stories(feeds: Vec<String>) -> Vec<Story> {
    let today = chrono::Local::now();
    let mut stories = Vec::new();

    for feed in feeds {
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

                            stories.push(Story {
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

    stories
}
