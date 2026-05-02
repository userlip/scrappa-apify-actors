# Google News Scraper

Scrape Google News results for media monitoring, market research, brand tracking, competitor intelligence, and editorial research. The actor wraps Scrappa's `/api/google/news` endpoint and returns structured news articles in an Apify dataset.

## What you get

- News article title, link, source, snippet, dates, thumbnails, and story tokens
- Keyword search with country, language, sort, and pagination controls
- Topic, publication, section, story, and Knowledge Graph token support
- Full Scrappa response saved to key-value store record `OUTPUT`

## Input

Use `q` for normal Google News keyword search:

```json
{
  "q": "artificial intelligence",
  "gl": "us",
  "hl": "en",
  "page": 1,
  "so": 1
}
```

Use one of the token inputs when browsing a Google News topic, publication, section, or story cluster. Do not combine `q` with token inputs. `kgmid` must be used alone.

## Output

Each dataset item is one Google News result:

```json
{
  "position": 1,
  "title": "Example News Article",
  "link": "https://example.com/news/article",
  "source_name": "Example Source",
  "date": "2 hours ago",
  "iso_date": "2026-05-02T10:30:00+00:00",
  "snippet": "Article summary text...",
  "thumbnail": "https://news.google.com/api/attachments/example-thumbnail.jpg",
  "story_token": "CAAq...",
  "request_q": "artificial intelligence",
  "request_gl": "us",
  "request_hl": "en"
}
```

## Notes

For higher-volume Google News monitoring or direct API access, use Scrappa's Google News API at `https://scrappa.co/api/google/news`.
