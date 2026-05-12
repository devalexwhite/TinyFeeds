use clap::Parser;
use feedparser_rs::parse;
use frostmark::{MarkState, MarkWidget};
use iced::{
    Element, Font,
    Length::Fill,
    Padding, Task, Theme,
    alignment::Horizontal,
    widget::{Row, button, column, container, rule, scrollable, space, text},
};
use reqwest::Client;
use std::{
    env::home_dir,
    fs::{self, File, OpenOptions, create_dir_all},
    io::{BufRead, BufReader, Write},
    path::Path,
    time::Duration,
};

use crate::ui::button_outline;

mod ui;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[arg(long, short, action)]
    dev_mode: bool,
}

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .title("TinyFeeds")
        .theme(App::theme)
        .run()
}

#[derive(Debug, Clone)]
struct Story {
    author: Option<String>,
    title: Option<String>,
    url: String,
    html: String,
    contact: Option<String>,
}

#[derive(Debug, Clone)]
enum Message {
    FetchStories,
    SetStories(Vec<Story>),
    ReadStory,
    SetStory,
    OpenLink(String),
    OpenInBrowser,
    EmailAuthor,
}

struct App {
    feeds: Vec<String>,
    stories: Vec<Story>,
    mark_state: MarkState,
    out_of_stories: bool,
    loading: bool,
    dev_mode: bool,
}

impl App {
    fn new() -> (Self, Task<Message>) {
        let args = Args::parse();

        if let Some(home) = home_dir() {
            let mut feeds_file_path = home.clone();
            feeds_file_path.push(".config/tinyfeeds/feeds.txt");

            if let Ok(true) = fs::exists(feeds_file_path.as_path()) {
                return (
                    App {
                        feeds: file_lines(feeds_file_path.as_path()),
                        stories: Vec::new(),
                        mark_state: MarkState::with_html(""),
                        out_of_stories: false,
                        loading: true,
                        dev_mode: args.dev_mode,
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
                out_of_stories: false,
                loading: true,
                dev_mode: args.dev_mode,
            },
            Task::done(Message::FetchStories),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::FetchStories => {
                if self.dev_mode == true {
                    self.stories.push(Story {
                        title: Some(String::from("Just a test article")),
                        author: Some(String::from("Alex White")),
                        url: String::from("https://thatalexguy.dev"),
                        html: String::from("<b>This is a sample story</b><h1>It tests the UI in dev mode</h1><h3>Enjoy being stuck in <i>sample land</i></h3>
                            <p>ajskdkajdlkajd sajd ksajdlkaj dlkajsdkjsakdjaslkdja lkdjsalkd jaslkd jaklsjd kajsd lkjsa dlja d</p><p>kjsad lkjsadlsa jdalksd jlksajdak jdlkjsad lasjd
                            lksajdlsajd lkajsd lkjsa dlkjsa dkja lkdjasd </p><p>kasjd lkajsd akjdlksa jdlksajd lksajd jsadlkjsad jsadlksajd lkjsadlk jsadlkajsd ljas dlkjsad lkjsad lkjaslkdj sad</p><p>kasjd lkajsd akjdlksa jdlksajd lksajd jsadlkjsad jsadlksajd lkjsadlk jsadlkajsd ljas dlkjsad lkjsad lkjaslkdj sad</p><p>kasjd lkajsd akjdlksa jdlksajd lksajd jsadlkjsad jsadlksajd lkjsadlk jsadlkajsd ljas dlkjsad lkjsad lkjaslkdj sad</p><p>kasjd lkajsd akjdlksa jdlksajd lksajd jsadlkjsad jsadlksajd lkjsadlk jsadlkajsd ljas dlkjsad lkjsad lkjaslkdj sad</p><p>kasjd lkajsd akjdlksa jdlksajd lksajd jsadlkjsad jsadlksajd lkjsadlk jsadlkajsd ljas dlkjsad lkjsad lkjaslkdj sad</p><p>kasjd lkajsd akjdlksa jdlksajd lksajd jsadlkjsad jsadlksajd lkjsadlk jsadlkajsd ljas dlkjsad lkjsad lkjaslkdj sad</p>"),
                        contact: Some(String::from("hi@thatalexguy.dev")),
                    });

                    return Task::done(Message::SetStory);
                }
                self.loading = true;

                let tasks = self
                    .feeds
                    .iter()
                    .map(|f| Task::perform(fetch_feed_stories(f.clone()), Message::SetStories));

                Task::batch(tasks).chain(Task::done(Message::SetStory))
            }
            Message::OpenLink(link) => {
                webbrowser::open(&link).unwrap_or_else(|_| println!("Failed to open browser"));

                Task::none()
            }
            Message::OpenInBrowser => {
                if self.stories.len() > 0 {
                    let url = self.stories[0].url.clone();
                    if !url.is_empty() {
                        webbrowser::open(&url)
                            .unwrap_or_else(|_| println!("Failed to open browser"));
                    }
                }

                Task::none()
            }
            Message::EmailAuthor => {
                if self.stories.len() > 0 {
                    if let Some(email) = self.stories[0].contact.clone() {
                        webbrowser::open(&format!("mailto:{}", email))
                            .unwrap_or_else(|_| println!("Failed to open browser"));
                    }
                }

                Task::none()
            }
            Message::SetStories(stories) => {
                for story in stories {
                    self.stories.push(story.clone());
                }

                Task::none()
            }
            Message::ReadStory => {
                let story = self.stories.remove(0);

                self.out_of_stories = self.stories.len() == 0;
                add_story_read(story);

                Task::done(Message::SetStory)
            }
            Message::SetStory => {
                if self.stories.len() > 0 {
                    self.out_of_stories = false;
                    self.mark_state =
                        MarkState::with_html_and_markdown(self.stories[0].html.clone().as_str());
                } else {
                    self.out_of_stories = true;
                }
                self.loading = false;

                Task::none()
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        if self.loading || self.out_of_stories {
            let message = if self.out_of_stories == true {
                "That's it, check back later."
            } else {
                "Checking for stories..."
            };
            text(message)
                .font(Font::MONOSPACE)
                .size(20)
                .width(Fill)
                .height(Fill)
                .center()
                .into()
        } else {
            let mut actions_row = Row::new();
            actions_row = actions_row.push(
                button("Open in Browser")
                    .style(button_outline)
                    .on_press(Message::OpenInBrowser),
            );

            if !self.stories[0].contact.is_some() {
                actions_row = actions_row.push(
                    button("Email Author")
                        .style(button_outline)
                        .on_press(Message::EmailAuthor),
                );
            }

            let centered_actions_row = container(actions_row.spacing(20).padding([10, 0]))
                .width(Fill)
                .align_x(Horizontal::Center);
            container(column![
                container(column![centered_actions_row, rule::horizontal(2)])
                    .width(Fill)
                    .padding(10),
                column![
                    scrollable(
                        container(
                            container(column![
                                container(column![
                                    if let Some(title) = self.stories[0].title.clone() {
                                        container(text(title).size(40))
                                    } else {
                                        container(space())
                                    },
                                    if let Some(author) = self.stories[0].author.clone() {
                                        container(text(format!("By {}", author)).size(16)).padding(
                                            Padding {
                                                top: 5.0,
                                                left: 0.0,
                                                right: 0.0,
                                                bottom: 40.0,
                                            },
                                        )
                                    } else {
                                        container(space()).padding(Padding {
                                            top: 5.0,
                                            left: 0.0,
                                            right: 0.0,
                                            bottom: 40.0,
                                        })
                                    }
                                ]),
                                MarkWidget::new(&self.mark_state)
                                    .paragraph_spacing(20.0)
                                    .on_clicking_link(Message::OpenLink)
                            ])
                            .max_width(800)
                        )
                        .width(Fill)
                        .align_x(Horizontal::Center)
                        .padding([20, 10])
                    )
                    .height(Fill),
                    container(
                        button(
                            container(text("Next Story"))
                                .width(Fill)
                                .align_x(Horizontal::Center)
                        )
                        .padding([20, 15])
                        .width(Fill)
                        .on_press(Message::ReadStory)
                    )
                    .width(Fill)
                    .align_x(Horizontal::Center)
                ]
                .height(Fill)
                .width(Fill),
            ])
            .width(Fill)
            .height(Fill)
            .into()
        }
    }

    fn theme(&self) -> Option<Theme> {
        Some(iced::Theme::CatppuccinLatte)
    }
}

fn file_lines(filename: impl AsRef<Path>) -> Vec<String> {
    let file = File::open(filename).expect("File not found");
    let buf = BufReader::new(file);
    buf.lines()
        .map(|l| l.expect("Could not read line."))
        .collect()
}

async fn fetch_feed_stories(feed: String) -> Vec<Story> {
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
                if pub_date.date_naive() != today.date_naive() {
                    break;
                }

                let mut content = story
                    .content
                    .iter()
                    .map(|e| e.value.clone())
                    .fold(String::from(""), |a, e| format!("{}{}", a, e));

                if content.is_empty() && story.summary.clone().is_some() {
                    content = story.summary.unwrap();
                }

                stories.push(Story {
                    author: if story.author.is_some() {
                        Some(story.author.unwrap().to_string())
                    } else {
                        None
                    },
                    title: if let Some(title) = story.title {
                        Some(title)
                    } else {
                        None
                    },
                    url: story.link.unwrap_or(String::from("")),
                    contact: if let Some(ad) = story.author_detail.clone()
                        && let Some(email) = ad.email
                    {
                        Some(email.to_string())
                    } else {
                        None
                    },
                    html: content,
                });
            }
        }
    }
    stories
}

fn add_story_read(story: Story) {
    if let Some(home) = home_dir() {
        let mut read_file_path = home.clone();
        read_file_path.push(".config/tinyfeeds/read.txt");

        let mut file = OpenOptions::new()
            .write(true)
            .append(true)
            .create(true)
            .open(read_file_path)
            .unwrap();

        if let Err(e) = writeln!(file, "{}", story.url) {
            eprintln!("Couldn't write to file: {}", e);
        }
    }
}
