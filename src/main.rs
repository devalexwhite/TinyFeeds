use clap::Parser;
use feedparser_rs::parse;
use frostmark::{MarkState, MarkWidget, UpdateMsg};
use iced::{
    Element, Font,
    Length::Fill,
    Padding, Task, Theme,
    alignment::{Horizontal, Vertical},
    exit, gradient,
    widget::{Row, button, column, container, image, rule, scrollable, space, svg, text},
};
use readabilityrs::{Readability, ReadabilityOptions};
use reqwest::Client;
use std::{
    collections::{HashMap, HashSet},
    env::home_dir,
    fs::{self, File, OpenOptions, create_dir_all},
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    time::Duration,
};

use crate::image_loader::Image;

mod image_loader;
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
    StoriesLoaded,
    CloseApp,
    ImageDownloaded(Result<Image, String>),
    UpdateState(UpdateMsg),
}

struct App {
    feeds: Vec<String>,
    stories: Vec<Story>,
    read_stories: Vec<String>,
    mark_state: MarkState,
    out_of_stories: bool,
    loading: bool,
    dev_mode: bool,
    org_story_count: usize,
    images_normal: HashMap<String, image::Handle>,
    images_svg: HashMap<String, svg::Handle>,
    images_in_progress: HashSet<String>,
}

impl App {
    fn new() -> (Self, Task<Message>) {
        let args = Args::parse();

        let read_stories = if let Some(config) = get_config_file_path("read.txt") {
            file_lines(config.as_path())
        } else {
            Vec::new()
        };

        let feeds = if let Some(config) = get_config_file_path("feeds.txt") {
            file_lines(config.as_path())
        } else {
            let mut feeds_base_path = home_dir().unwrap_or_default();
            feeds_base_path.push(".config/tinyfeeds/");
            create_dir_all(feeds_base_path.clone()).expect("Failed to create config directory.");
            let mut feeds_file_path = feeds_base_path;
            feeds_file_path.push("feeds.txt");
            File::create(feeds_file_path).expect("Failed to create config file.");
            Vec::new()
        };

        (
            App {
                feeds: feeds,
                read_stories: read_stories,
                stories: Vec::new(),
                mark_state: MarkState::with_html(""),
                out_of_stories: false,
                loading: true,
                dev_mode: args.dev_mode,
                org_story_count: 0,
                images_normal: HashMap::new(),
                images_svg: HashMap::new(),
                images_in_progress: HashSet::new(),
            },
            Task::done(Message::FetchStories),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::UpdateState(n) => {
                self.mark_state.update(n);
                Task::none()
            }
            Message::FetchStories => {
                if self.dev_mode {
                    self.stories.push(Story {
                        title: Some(String::from("Just a test article")),
                        author: Some(String::from("Alex White")),
                        url: String::from("https://thatalexguy.dev"),
                        html: String::from(r"
                            <b>This is a sample story</b><h1>It tests the UI in dev mode</h1><h3>Enjoy being stuck in <i>sample land</i></h3>
                            <p>ajskdkajdlkajd sajd ksajdlkaj dlkajsdkjsakdjaslkdja lkdjsalkd jaslkd jaklsjd kajsd lkjsa dlja d</p><p>kjsad lkjsadlsa jdalksd jlksajdak jdlkjsad lasjd
                            lksajdlsajd lkajsd lkjsa dlkjsa dkja lkdjasd </p><p>kasjd lkajsd akjdlksa jdlksajd lksajd jsadlkjsad jsadlksajd lkjsadlk jsadlkajsd ljas dlkjsad lkjsad lkjaslkdj sad</p>
                            <p>kasjd lkajsd akjdlksa jdlksajd lksajd jsadlkjsad jsadlksajd lkjsadlk jsadlkajsd ljas dlkjsad lkjsad lkjaslkdj sad</p><p>kasjd lkajsd akjdlksa jdlksajd lksajd jsadlkjsad jsadlksajd lkjsadlk jsadlkajsd ljas dlkjsad lkjsad lkjaslkdj sad</p>
                            <35;281;57M<p>kasjd lkajsd akjdlksa jdlksajd lksajd jsadlkjsad jsadlksajd lkjsadlk jsadlkajsd ljas dlkjsad lkjsad lkjaslkdj sad</p><p>kasjd lkajsd akjdlksa jdlksajd lksajd jsadlkjsad jsadlksajd lkjsadlk jsadlkajsd ljas dlkjsad lkjsad lkjaslkdj sad</p>
                            <p>kasjd lkajsd akjdlksa jdlksajd lksajd jsadlkjsad jsadlksajd lkjsadlk jsadlkajsd ljas dlkjsad lkjsad lkjaslkdj sad</p>
                            "),
                        contact: Some(String::from("hi@thatalexguy.dev")),
                    });

                    return Task::done(Message::SetStory);
                }
                self.loading = true;

                let tasks = self.feeds.iter().map(|f| {
                    Task::perform(
                        fetch_feed_stories(f.clone(), self.read_stories.clone()),
                        Message::SetStories,
                    )
                });

                Task::batch(tasks).chain(Task::done(Message::StoriesLoaded))
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
            Message::ImageDownloaded(res) => match res {
                Ok(image) => {
                    if image.is_svg {
                        self.images_svg
                            .insert(image.url, svg::Handle::from_memory(image.bytes));
                    } else {
                        self.images_normal
                            .insert(image.url, image::Handle::from_bytes(image.bytes));
                    }
                    Task::none()
                }
                Err(err) => {
                    eprintln!("Couldn't download image: {err}");
                    Task::none()
                }
            },
            Message::SetStories(stories) => {
                self.stories.extend(stories);

                Task::none()
            }
            Message::ReadStory => {
                let story = self.stories.remove(0);

                self.out_of_stories = self.stories.is_empty();
                add_story_read(story);

                Task::done(Message::SetStory)
            }
            Message::StoriesLoaded => {
                self.org_story_count = self.stories.len();

                Task::done(Message::SetStory)
            }
            Message::CloseApp => exit(),
            Message::SetStory => {
                if self.stories.len() > 0 {
                    self.out_of_stories = false;

                    let readability_options = ReadabilityOptions {
                        output_markdown: true,
                        ..Default::default()
                    };

                    if let Ok(rd) = Readability::new(
                        self.stories[0].html.clone().as_str(),
                        Some(self.stories[0].url.as_str()),
                        Some(readability_options),
                    ) && let Some(article) = rd.parse()
                    {
                        if let Some(md) = article.markdown_content {
                            self.mark_state = MarkState::with_markdown_only(md.as_str());
                        } else {
                            self.mark_state = MarkState::with_html(
                                article.content.unwrap_or(String::from("")).as_str(),
                            );
                        }
                    } else {
                        self.mark_state = MarkState::with_html_and_markdown(
                            self.stories[0].html.clone().as_str(),
                        );
                    }
                } else {
                    self.out_of_stories = true;
                }
                self.loading = false;
                self.download_images()
            }
        }
    }

    fn draw_image(&self, info: frostmark::ImageInfo) -> Element<'static, Message> {
        if let Some(image) = self.images_normal.get(info.url).cloned() {
            let mut img = iced::widget::image(image);
            if let Some(w) = info.width {
                img = img.width(w);
            }
            img.into()
        } else if let Some(image) = self.images_svg.get(info.url).cloned() {
            let mut img = iced::widget::svg(image);
            if let Some(w) = info.width {
                img = img.width(w);
            }
            img.into()
        } else {
            "...".into()
        }
    }

    fn download_images(&mut self) -> Task<Message> {
        Task::batch(self.mark_state.find_image_links().into_iter().map(|url| {
            if self.images_in_progress.insert(url.clone()) {
                Task::perform(image_loader::download_image(url), Message::ImageDownloaded)
            } else {
                Task::none()
            }
        }))
    }

    fn view(&self) -> Element<'_, Message> {
        if self.loading || self.out_of_stories {
            let message = if self.out_of_stories {
                "That's it, check back later."
            } else {
                "Checking for stories..."
            };
            container(column![
                text(message).font(Font::MONOSPACE).size(20),
                if self.out_of_stories {
                    container(button("See ya!").on_press(Message::CloseApp)).padding([20, 0])
                } else {
                    container(space())
                }
            ])
            .width(Fill)
            .align_x(Horizontal::Center)
            .align_y(Vertical::Center)
            .height(Fill)
            .into()
        } else {
            let mut actions_row = Row::new();
            actions_row = actions_row.push(
                button("Open in Browser")
                    .style(button::secondary)
                    .on_press(Message::OpenInBrowser),
            );

            if self.stories[0].contact.is_some() {
                actions_row = actions_row.push(
                    button("Email Author")
                        .style(button::secondary)
                        .on_press(Message::EmailAuthor),
                );
            }

            let read_progress = 1.0
                - self.stories.len() as f32
                    / if self.org_story_count > 0 {
                        self.org_story_count as f32
                    } else {
                        1.0
                    };

            let centered_actions_row = container(actions_row.spacing(20).padding([10, 0]))
                .width(Fill)
                .align_x(Horizontal::Right);
            container(column![
                container(column![centered_actions_row, rule::horizontal(2)])
                    .width(Fill)
                    .padding(10),
                column![
                    scrollable(
                        container(container(column![
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
                                .on_drawing_image(|info| container(self.draw_image(info))
                                    .padding([20, 0])
                                    .into())
                                .on_updating_state(|n| Message::UpdateState(n))
                                .on_clicking_link(Message::OpenLink)
                        ]))
                        .width(Fill)
                        .align_x(Horizontal::Center)
                        .padding([20, 15])
                    )
                    .height(Fill),
                    container(column![
                        button(
                            container(text("Next Story"))
                                .width(Fill)
                                .align_x(Horizontal::Center)
                        )
                        .style(move |theme: &Theme, _| {
                            let palette = theme.extended_palette();
                            let progress_bg = iced::Background::Gradient(
                                gradient::Linear::new(1.57)
                                    .add_stop(read_progress as f32, palette.primary.strong.color)
                                    .add_stop(
                                        read_progress + 0.2 as f32,
                                        palette.secondary.weak.color,
                                    )
                                    .add_stop(1.0, palette.secondary.weak.color)
                                    .into(),
                            );

                            button::Style {
                                background: Some(progress_bg.clone()),
                                ..Default::default()
                            }
                        })
                        .padding([20, 15])
                        .width(Fill)
                        .on_press(Message::ReadStory)
                    ])
                    .width(Fill)
                    .padding(20)
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
        Some(iced::Theme::TokyoNight)
    }
}

fn file_lines(filename: impl AsRef<Path>) -> Vec<String> {
    let file = File::open(filename).expect("File not found");
    let buf = BufReader::new(file);
    buf.lines()
        .map(|l| l.expect("Could not read line."))
        .collect()
}

async fn fetch_feed_stories(feed: String, read_stories: Vec<String>) -> Vec<Story> {
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
                if let Some(url) = story.link.clone() {
                    if read_stories.contains(&url) {
                        continue;
                    }
                }

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
                    author: story.author.map(|f| f.to_string()),
                    title: story.title,
                    url: story.link.unwrap_or(String::from("")),
                    contact: if let Some(ad) = story.author_detail.clone() {
                        ad.email.map(|f| f.to_string())
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
    if let Some(config) = get_config_file_path("read.txt") {
        let mut file = OpenOptions::new()
            .write(true)
            .append(true)
            .create(true)
            .open(config)
            .unwrap();

        if let Err(e) = writeln!(file, "{}", story.url) {
            eprintln!("Couldn't write to file: {}", e);
        }
    }
}

fn get_config_file_path(filename: &str) -> Option<PathBuf> {
    if let Some(mut home) = home_dir() {
        home.push(format!(".config/tinyfeeds/{}", filename));
        return Some(home);
    }
    None
}
