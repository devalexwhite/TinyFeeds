# TinyFeeds

- Barebones RSS feed reader
- Feeds stored in a plain text file
- A "read.txt" file is used to track post titles that have been read
- A "favorites.txt" file stores starred post titles
  - Maybe it's like "{feed_url}/{post_title}"
- 2 pane, feeds on left and reader on right


- Check for feeds.txt, add default if not exists
- Load feeds.txt into Vector<String> feed_urls
- Fetch each feed, add past X days of posts into Vector<Post> posts
- Send UI message that posts loaded
- When user clicks post title, message sent that fetches content in right pane viewer

## New plan

- This app only shows post for the current day
- A "read.txt" file stores the already seen posts, those posts are filtered out of the feed view
- App loads -> Shows you the first unread post -> Button at bottom to view next post -> Adds to "read.txt" and opens next post
