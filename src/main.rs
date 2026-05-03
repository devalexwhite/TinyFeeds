use frostmark::{MarkState, MarkWidget};
use iced::{
    Element, Font,
    Length::Fill,
    Task, Theme,
    alignment::Horizontal,
    color,
    widget::{button, column, container, scrollable, text},
};
use rss::Channel;
use std::{
    env::home_dir,
    fs::{self, File, create_dir_all},
    io::{BufRead, BufReader},
    path::Path,
};

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
    out_of_stories: bool,
    loading: bool,
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
                        out_of_stories: false,
                        loading: true,
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
            },
            Task::done(Message::FetchStories),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::FetchStories => {
                self.loading = true;
                Task::perform(fetch_stories(self.feeds.clone()), Message::SetStories)
            }
            Message::SetStories(stories) => {
                self.stories = stories.clone();

                Task::done(Message::SetStory)
            }
            Message::ReadStory => {
                self.stories.remove(0);

                Task::done(Message::SetStory)
            }
            Message::SetStory => {
                if self.stories.len() > 0 {
                    self.out_of_stories = false;
                    self.mark_state = MarkState::with_html(self.stories[0].html.clone().as_str());
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
            container(scrollable(column![
                container(MarkWidget::new(&self.mark_state),)
                    .padding([20, 10])
                    .height(Fill),
                container(
                    button("Next Story")
                        .padding([10, 15])
                        .on_press(Message::ReadStory)
                )
                .style(|_| container::Style {
                    background: Some(color!(0x1e1e2e).into()),
                    ..Default::default()
                })
                .padding([20, 0])
                .width(Fill)
                .align_x(Horizontal::Center),
            ]))
            .width(Fill)
            .height(Fill)
            .style(container::rounded_box)
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

    for feed in feeds {
        if let Ok(feed_content) = reqwest::get(feed).await {
            if let Ok(feed_bytes) = feed_content.bytes().await {
                if let Ok(channel) = Channel::read_from(&feed_bytes[..]) {
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
                                html: content,
                            });
                        }
                    }
                }
            }
        }
    }

    stories
}
