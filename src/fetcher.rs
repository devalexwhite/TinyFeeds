use std::time::Duration;
use std::rc::Rc;
use std::cell::RefCell;
use reqwest::Client;
use feedparser_rs::parse;
use crate::models::{Story, AppMessage, AppState};

pub fn trigger_fetch(
    state: &Rc<RefCell<AppState>>,
    tx: tokio::sync::mpsc::UnboundedSender<AppMessage>,
) {
    let feeds = state.borrow().feeds.clone();
    let read_stories = state.borrow().read_stories.clone();
    let dev_mode = state.borrow().dev_mode;

    tokio::spawn(async move {
        if dev_mode {
            tokio::time::sleep(Duration::from_secs(1)).await;
            let dummy_html = r#"<p><strong>This is a sample story testing HTML features.</strong></p>
<h1>It tests the UI in dev mode</h1>
<h3>Enjoy being stuck in <em>sample land</em></h3>
<p>Here is a collapsible detail tag:</p>
<details>
  <summary>Click to see secret content</summary>
  <p>This content was hidden inside a details tag! Pretty cool, huh?</p>
  <ul>
    <li>Nested item 1</li>
    <li>Nested item 2</li>
  </ul>
</details>
<p>Here is an iframe embed (simulated):</p>
<iframe width="560" height="315" src="https://www.youtube.com/embed/dQw4w9WgXcQ" title="YouTube video player" allowfullscreen></iframe>
<p>And some blockquote:</p>
<blockquote>
  "This is a blockquote element that should stand out visually and look very premium."
</blockquote>
<p>Some code snippet:</p>
<pre><code>fn main() {
    println!("Hello, TinyFeeds!");
}</code></pre>
<p>And an external link: <a href="https://thatalexguy.dev">thatalexguy.dev</a></p>"#.to_string();

            let dummy = Story {
                title: Some("Just a test article".to_string()),
                author: Some("Alex White".to_string()),
                url: "https://thatalexguy.dev".to_string(),
                html: dummy_html,
                contact: Some("hi@thatalexguy.dev".to_string()),
            };
            tx.send(AppMessage::StoriesFetched(vec![dummy])).ok();
            return;
        }

        let mut stories = Vec::new();
        let mut tasks = Vec::new();
        for feed in feeds {
            let rs = read_stories.clone();
            tasks.push(tokio::spawn(fetch_feed_stories(feed, rs)));
        }

        for task in tasks {
            if let Ok(res) = task.await {
                stories.extend(res);
            }
        }

        tx.send(AppMessage::StoriesFetched(stories)).ok();
    });
}

pub async fn fetch_feed_stories(feed: String, read_stories: Vec<String>) -> Vec<Story> {
    let today = chrono::Local::now();
    let client = Client::new();
    let mut stories = Vec::new();

    let feed_content = match client
        .get(&feed)
        .timeout(Duration::from_secs(10))
        .send()
        .await
    {
        Ok(res) => res,
        Err(_) => return stories,
    };

    let feed_bytes = match feed_content.bytes().await {
        Ok(bytes) => bytes,
        Err(_) => return stories,
    };

    let channel = match parse(&feed_bytes[..]) {
        Ok(ch) => ch,
        Err(_) => return stories,
    };

    for story in channel.entries {
        // Resolve date either via updated or published field (RSS 2.0 uses published)
        if let Some(pub_date) = story.updated.or(story.published) {
            let story_url = story.link.clone().unwrap_or_default();
            if !story_url.is_empty() && read_stories.contains(&story_url) {
                continue;
            }

            // Check if story is from today. Use continue instead of break to allow for out-of-order/pinned posts.
            if pub_date.with_timezone(&chrono::Local).date_naive() != today.date_naive() {
                continue;
            }

            let mut content = story
                .content
                .iter()
                .map(|e| e.value.clone())
                .fold(String::new(), |a, e| format!("{}{}", a, e));

            if content.is_empty() {
                if let Some(ref summary) = story.summary {
                    content = summary.clone();
                }
            }

            let contact = find_story_email(&story, &channel.feed);
            stories.push(Story {
                author: story.author.map(|f| f.to_string()),
                title: story.title.clone(),
                url: story_url,
                contact,
                html: content,
            });
        }
    }
    stories
}

pub fn extract_email_from_string(s: &str) -> Option<String> {
    let delimiters = [' ', '(', ')', '<', '>', '[', ']', ',', ';'];
    for part in s.split(&delimiters[..]) {
        let part = part.trim();
        if part.contains('@') && part.contains('.') {
            if let Some(at_idx) = part.find('@') {
                let local_part = &part[..at_idx];
                let domain_part = &part[at_idx + 1..];
                if !local_part.is_empty() && !domain_part.is_empty() && domain_part.contains('.') {
                    let cleaned = part.trim_end_matches(|c: char| {
                        c.is_ascii_punctuation() && c != '.' && c != '-' && c != '_'
                    });
                    return Some(cleaned.to_string());
                }
            }
        }
    }
    None
}

pub fn find_story_email(
    story: &feedparser_rs::Entry,
    feed_meta: &feedparser_rs::FeedMeta,
) -> Option<String> {
    // 1. Try entry-level author details
    if let Some(ref ad) = story.author_detail {
        if let Some(ref email) = ad.email {
            if let Some(extracted) = extract_email_from_string(email) {
                return Some(extracted);
            }
        }
        if let Some(ref name) = ad.name {
            if let Some(extracted) = extract_email_from_string(name) {
                return Some(extracted);
            }
        }
    }

    // 2. Try entry-level authors list
    for author in &story.authors {
        if let Some(ref email) = author.email {
            if let Some(extracted) = extract_email_from_string(email) {
                return Some(extracted);
            }
        }
        if let Some(ref name) = author.name {
            if let Some(extracted) = extract_email_from_string(name) {
                return Some(extracted);
            }
        }
    }

    // 3. Try entry-level contributors list
    for contributor in &story.contributors {
        if let Some(ref email) = contributor.email {
            if let Some(extracted) = extract_email_from_string(email) {
                return Some(extracted);
            }
        }
        if let Some(ref name) = contributor.name {
            if let Some(extracted) = extract_email_from_string(name) {
                return Some(extracted);
            }
        }
    }

    // 4. Try parsing the raw entry author string
    if let Some(ref author_str) = story.author {
        if let Some(extracted) = extract_email_from_string(author_str) {
            return Some(extracted);
        }
    }

    // 5. Try feed-level author details
    if let Some(ref ad) = feed_meta.author_detail {
        if let Some(ref email) = ad.email {
            if let Some(extracted) = extract_email_from_string(email) {
                return Some(extracted);
            }
        }
        if let Some(ref name) = ad.name {
            if let Some(extracted) = extract_email_from_string(name) {
                return Some(extracted);
            }
        }
    }

    // 6. Try feed-level publisher details (managingEditor / webMaster in RSS)
    if let Some(ref pd) = feed_meta.publisher_detail {
        if let Some(ref email) = pd.email {
            if let Some(extracted) = extract_email_from_string(email) {
                return Some(extracted);
            }
        }
        if let Some(ref name) = pd.name {
            if let Some(extracted) = extract_email_from_string(name) {
                return Some(extracted);
            }
        }
    }

    // 7. Try feed-level authors list
    for author in &feed_meta.authors {
        if let Some(ref email) = author.email {
            if let Some(extracted) = extract_email_from_string(email) {
                return Some(extracted);
            }
        }
        if let Some(ref name) = author.name {
            if let Some(extracted) = extract_email_from_string(name) {
                return Some(extracted);
            }
        }
    }

    // 8. Try feed-level contributors list
    for contributor in &feed_meta.contributors {
        if let Some(ref email) = contributor.email {
            if let Some(extracted) = extract_email_from_string(email) {
                return Some(extracted);
            }
        }
        if let Some(ref name) = contributor.name {
            if let Some(extracted) = extract_email_from_string(name) {
                return Some(extracted);
            }
        }
    }

    // 9. Try parsing the raw feed author string
    if let Some(ref author_str) = feed_meta.author {
        if let Some(extracted) = extract_email_from_string(author_str) {
            return Some(extracted);
        }
    }

    // 10. Try parsing the raw feed publisher string
    if let Some(ref publisher_str) = feed_meta.publisher {
        if let Some(extracted) = extract_email_from_string(publisher_str) {
            return Some(extracted);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_email() {
        assert_eq!(
            extract_email_from_string("email@email.com (Author Name)"),
            Some("email@email.com".to_string())
        );
        assert_eq!(
            extract_email_from_string("Author Name <email@email.com>"),
            Some("email@email.com".to_string())
        );
        assert_eq!(
            extract_email_from_string("email@email.com"),
            Some("email@email.com".to_string())
        );
        assert_eq!(
            extract_email_from_string("Author Name (email@email.com)"),
            Some("email@email.com".to_string())
        );
        assert_eq!(extract_email_from_string("Author Name"), None);
    }

    #[test]
    fn test_email_parsing() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<rss version="2.0">
<channel>
 <title>Test RSS</title>
 <description>Test</description>
 <link>http://example.com</link>
  <managingEditor>editor@example.com (Editor Name)</managingEditor>
  <webMaster>webmaster@example.com (Webmaster Name)</webMaster>
 <item>
  <title>Test Item</title>
  <link>http://example.com/item</link>
  <author>email@email.com (Author Name)</author>
 </item>
</channel>
</rss>"#;
        let feed = feedparser_rs::parse(xml.as_bytes()).unwrap();
        let entry = &feed.entries[0];

        // Assert that we successfully resolve the entry-level author email
        assert_eq!(
            find_story_email(entry, &feed.feed),
            Some("email@email.com".to_string())
        );

        // Assert feed level fallback to managingEditor
        let mut entry_no_author = entry.clone();
        entry_no_author.author_detail = None;
        entry_no_author.author = None;
        assert_eq!(
            find_story_email(&entry_no_author, &feed.feed),
            Some("editor@example.com".to_string())
        );

        // Assert feed level fallback to webMaster
        let mut feed_meta_no_editor = feed.feed.clone();
        feed_meta_no_editor.author_detail = None;
        feed_meta_no_editor.author = None;
        assert_eq!(
            find_story_email(&entry_no_author, &feed_meta_no_editor),
            Some("webmaster@example.com".to_string())
        );
    }
}

