# Google Videos Scraper

Search Google Videos and export dataset-ready video SERP results for content research, publisher monitoring, YouTube competitive research, video SEO, and market intelligence. The actor wraps Scrappa's `/api/google/videos` endpoint.

## What you get

- Video title, result URL, displayed link, thumbnail, snippet, duration, and date
- Key moments count plus the raw key moments payload when Google exposes it
- Google SERP controls for country, language, Google domain, safe search, time filters, pagination, and language restriction
- Full Scrappa response saved to key-value store record `OUTPUT`

## Input

```json
{
  "q": "coffee brewing tutorial",
  "page": 1,
  "hl": "en",
  "gl": "us",
  "google_domain": "google.com",
  "tbs": "qdr:w",
  "safe": "off"
}
```

## Output

Each dataset item is one Google Videos result:

```json
{
  "position": 1,
  "title": "Coffee Brewing Tutorial",
  "video_url": "https://www.youtube.com/watch?v=example",
  "displayed_link": "www.youtube.com > watch",
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

## Notes

For higher-volume Google Videos collection or direct API access, use Scrappa's Google Videos API at `https://scrappa.co/api/google/videos`.
