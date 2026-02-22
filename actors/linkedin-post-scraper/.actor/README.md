# LinkedIn Post Scraper

Scrape public LinkedIn posts and articles without login. Extract content, author information, reactions, comments, and related articles from any public LinkedIn post or Pulse article.

## Features

- **Post Content** - Full article text and images
- **Author Info** - Name, headline, profile URL, and image
- **Reactions** - Total reactions, likes, and comment counts
- **Topics** - Article topics and tags
- **Related Articles** - Recommended reading from LinkedIn
- **Comments** - Post comments and engagement
- **No Login Required** - Scrape public posts without authentication
- **Cache Support** - Reduce costs with intelligent caching

## Input

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `url` | string | Yes | LinkedIn post or article URL |
| `use_cache` | boolean | No | Use cached results if available (default: true) |
| `maximum_cache_age` | integer | No | Max cache age in seconds (default: 3600) |

### Supported URL Formats

- LinkedIn Pulse articles: `https://www.linkedin.com/pulse/article-title-author-id/`
- LinkedIn posts: `https://www.linkedin.com/posts/...`

## Output

### Dataset (Post Data)

Each post is saved to the dataset with the following structure:

```json
{
  "title": "The Future of Artificial Intelligence",
  "url": "https://www.linkedin.com/pulse/...",
  "date_published": "2024-01-15",
  "date_modified": "2024-01-16",
  "image": "https://media.licdn.com/...",
  "body": "Full article text...",
  "author_name": "John Doe",
  "author_url": "https://www.linkedin.com/in/johndoe",
  "author_image": "https://media.licdn.com/...",
  "author_headline": "AI Researcher | Tech Entrepreneur",
  "reactions_total": 1250,
  "reactions_likes": 1100,
  "comments_count": 150,
  "topics": ["Artificial Intelligence", "Technology", "Future"],
  "more_articles": [...],
  "comments": [...],
  "success": true
}
```

### Key-Value Store (Full Response)

The complete response is saved to the `OUTPUT` key, including nested author object and reactions breakdown:

```json
{
  "success": true,
  "title": "Article Title",
  "url": "https://www.linkedin.com/pulse/...",
  "date_published": "2024-01-15",
  "date_modified": "2024-01-16",
  "image": "https://media.licdn.com/...",
  "body": "Full article content...",
  "author": {
    "name": "John Doe",
    "url": "https://www.linkedin.com/in/johndoe",
    "image": "https://media.licdn.com/...",
    "headline": "AI Researcher | Tech Entrepreneur"
  },
  "reactions": {
    "total": 1250,
    "likes": 1100,
    "comments": 150
  },
  "topics": ["Artificial Intelligence", "Technology"],
  "more_articles": [
    {
      "title": "Related Article",
      "url": "https://...",
      "author": "Jane Smith"
    }
  ],
  "comments": [
    {
      "author": "User Name",
      "text": "Great insights!",
      "date": "2024-01-15"
    }
  ]
}
```

## Example

```json
{
  "url": "https://www.linkedin.com/pulse/future-artificial-intelligence-how-ai-reshaping-our-world-doe-1e/",
  "use_cache": true,
  "maximum_cache_age": 3600
}
```

## Pricing

$0.30 per 1,000 results. No additional API keys or LinkedIn login required.

## Support

For issues or questions, contact us through Apify.
