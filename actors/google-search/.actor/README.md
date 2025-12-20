# Google Search Scraper

Extract Google Search results at scale. Get organic results, knowledge panels, People Also Ask, related searches, local results, and more.

## Features

- **Organic Results** - Title, URL, snippet, and position for each result
- **Knowledge Graph** - Rich information panels for entities
- **People Also Ask** - Related questions and answers
- **Related Searches** - Suggested search queries
- **Local Results** - Business listings with maps data
- **Inline Content** - Videos, images, and other embedded results
- **Geo-targeting** - Search from any location worldwide
- **Language Support** - Interface and results in any language
- **Pagination** - Retrieve multiple pages of results

## Input

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `apiKey` | string | Yes | Your Scrappa API key from https://scrappa.co |
| `query` | string | Yes | Search query |
| `location` | string | No | Location for geo-targeted results (e.g., "New York, NY") |
| `gl` | string | No | Country code (e.g., "us", "uk", "de") |
| `hl` | string | No | Language code (e.g., "en", "de", "es") |
| `start` | integer | No | Pagination offset (0, 10, 20...) |
| `amount` | integer | No | Number of results (1-100, default: 10) |
| `tbs` | string | No | Time filter (qdr:h, qdr:d, qdr:w, qdr:m, qdr:y) |
| `tbm` | string | No | Search type (nws, vid, isch, shop) |

## Output

### Dataset (Organic Results)

Each organic result is saved to the dataset:

```json
{
  "position": 1,
  "title": "Example Title",
  "link": "https://example.com/page",
  "displayed_link": "example.com › page",
  "snippet": "Description text from the search result...",
  "source": "example.com"
}
```

### Key-Value Store (Full Response)

The complete API response is saved to the `OUTPUT` key, including:

```json
{
  "search_information": {
    "query_displayed": "your search query",
    "total_results": 1000000
  },
  "organic_results": [...],
  "related_searches": [
    { "query": "related query", "link": "https://..." }
  ],
  "related_questions": [...],
  "knowledge_graph": {...},
  "local_results": {...},
  "inline_videos": [...],
  "inline_images": [...]
}
```

## Example

```json
{
  "apiKey": "your-api-key",
  "query": "best pizza in chicago",
  "location": "Chicago, IL, USA",
  "gl": "us",
  "hl": "en",
  "amount": 20
}
```

## Pricing

This actor uses the [Scrappa API](https://scrappa.co). You need a Scrappa API key to use it. Pricing is based on your Scrappa subscription plan.

## Support

- **API Documentation**: https://scrappa.co/documentation
- **Get API Key**: https://scrappa.co
