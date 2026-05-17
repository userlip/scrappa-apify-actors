# Google Videos Scraper

Search Google Videos and export structured video SERP results through the Scrappa Google Videos API.

## Features

- Video results with title, link, displayed link, thumbnail, snippet, duration, date, and key moments
- Country, language, Google domain, location, UULE, safe search, and language restriction controls
- Page or start-offset pagination
- Full raw Scrappa response saved to the key-value store

## Input

```json
{
  "q": "coffee brewing tutorial",
  "page": 1,
  "hl": "en",
  "gl": "us",
  "google_domain": "google.com",
  "safe": "off"
}
```

## Output

Each item in `video_results` is saved to the default dataset with request metadata:

```json
{
  "position": 1,
  "title": "Coffee Brewing Tutorial",
  "video_url": "https://www.youtube.com/watch?v=example",
  "thumbnail_url": "https://i.ytimg.com/vi/example/hqdefault.jpg",
  "snippet": "Learn how to brew coffee step by step.",
  "duration": "12:34",
  "date": "3 days ago",
  "key_moments_count": 2,
  "request_q": "coffee brewing tutorial",
  "request_gl": "us",
  "request_hl": "en"
}
```

The complete Scrappa response is saved to key-value store record `OUTPUT`, including `found_in_videos`, `short_videos`, `related_searches`, `pagination`, and `scrappa_pagination` when returned.
