use clap::Parser;
use frostmark::{MarkState, MarkWidget};
use iced::{
    Element, Font,
    Length::Fill,
    Task, Theme,
    alignment::Horizontal,
    widget::{Row, button, column, container, rule, scrollable, text},
};
use reqwest::Client;
use rss::Channel;
use std::{
    env::home_dir,
    fs::{self, File, create_dir_all},
    io::{BufRead, BufReader},
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
    author: String,
    url: String,
    html: String,
    contact: String,
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
                        author: String::from("Alex White"),
                        url: String::from("https://thatalexguy.dev"),
                        html: String::from("<b>This is a sample story</b><h1>It tests the UI in dev mode</h1><h3>Enjoy being stuck in <i>sample land</i></h3>
                            <p>ajskdkajdlkajd sajd ksajdlkaj dlkajsdkjsakdjaslkdja lkdjsalkd jaslkd jaklsjd kajsd lkjsa dlja d</p><p>kjsad lkjsadlsa jdalksd jlksajdak jdlkjsad lasjd
                            lksajdlsajd lkajsd lkjsa dlkjsa dkja lkdjasd </p><p>kasjd lkajsd akjdlksa jdlksajd lksajd jsadlkjsad jsadlksajd lkjsadlk jsadlkajsd ljas dlkjsad lkjsad lkjaslkdj sad</p><p>kasjd lkajsd akjdlksa jdlksajd lksajd jsadlkjsad jsadlksajd lkjsadlk jsadlkajsd ljas dlkjsad lkjsad lkjaslkdj sad</p><p>kasjd lkajsd akjdlksa jdlksajd lksajd jsadlkjsad jsadlksajd lkjsadlk jsadlkajsd ljas dlkjsad lkjsad lkjaslkdj sad</p><p>kasjd lkajsd akjdlksa jdlksajd lksajd jsadlkjsad jsadlksajd lkjsadlk jsadlkajsd ljas dlkjsad lkjsad lkjaslkdj sad</p><p>kasjd lkajsd akjdlksa jdlksajd lksajd jsadlkjsad jsadlksajd lkjsadlk jsadlkajsd ljas dlkjsad lkjsad lkjaslkdj sad</p><p>kasjd lkajsd akjdlksa jdlksajd lksajd jsadlkjsad jsadlksajd lkjsadlk jsadlkajsd ljas dlkjsad lkjsad lkjaslkdj sad</p>"),
                        contact: String::from("hi@thatalexguy.dev"),
                    });
                    return Task::done(Message::SetStory);
                }
                self.loading = true;

                Task::perform(fetch_stories(self.feeds.clone()), Message::SetStories)
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
                    let email = self.stories[0].contact.clone();
                    if !email.is_empty() {
                        webbrowser::open(&format!("mailto:{}", email))
                            .unwrap_or_else(|_| println!("Failed to open browser"));
                    }
                }

                Task::none()
            }
            Message::SetStories(stories) => {
                self.stories = stories.clone();

                Task::done(Message::SetStory)
            }
            Message::ReadStory => {
                self.stories.remove(0);

                self.out_of_stories = self.stories.len() == 0;

                Task::done(Message::SetStory)
            }
            Message::SetStory => {
                if self.stories.len() > 0 {
                    self.out_of_stories = false;
                    self.mark_state =
                        MarkState::with_html_and_markdown(self.stories[0].html.clone().as_str());
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

            if !self.stories[0].contact.is_empty() {
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
                            container(
                                MarkWidget::new(&self.mark_state)
                                    .paragraph_spacing(20.0)
                                    .on_clicking_link(Message::OpenLink)
                            )
                            .max_width(600)
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

async fn fetch_stories(feeds: Vec<String>) -> Vec<Story> {
    let today = chrono::Local::now();
    let mut stories = Vec::new();

    let client = Client::new();

    for feed in feeds {
        if let Ok(feed_content) = client
            .get(feed)
            .timeout(Duration::from_secs(2))
            .send()
            .await
            && let Ok(feed_bytes) = feed_content.bytes().await
            && let Ok(channel) = Channel::read_from(&feed_bytes[..])
        {
            for story in channel.items {
                if let Some(pub_date) = story.pub_date {
                    let pub_date_c =
                        chrono::DateTime::parse_from_rfc2822(pub_date.clone().as_str())
                            .unwrap()
                            .with_timezone(&chrono::Local);

                    if pub_date_c.date_naive() != today.date_naive() {
                        continue;
                    }

                    let content = if story.content.is_some() {
                        story.content.unwrap()
                    } else {
                        story.description.unwrap_or(String::from(""))
                    };

                    stories.push(Story {
                        author: channel.title.clone(),
                        url: story.link.unwrap_or(String::from("")),
                        contact: story
                            .author
                            .unwrap_or(channel.managing_editor.clone().unwrap_or(String::from(""))),
                        html: content,
                    });
                }
            }
        }
    }

    stories
}

async fn add_story_read(story: Story) {}
