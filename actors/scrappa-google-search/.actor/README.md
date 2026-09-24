# Scrappa Google Search

Scrape Google Search results at scale. Extract organic results, knowledge panels, related searches, People Also Ask, local results, and more. Supports geo-targeting, language settings, and pagination.

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
| `query` | string | Yes | Search query |
| `location` | string | No | Location for geo-targeted results (e.g., "New York, NY") |
| `gl` | string | No | Country code (e.g., "us", "uk", "de") |
| `hl` | string | No | Language code (e.g., "en", "de", "es") |
| `google_domain` | string | No | Google domain to query (e.g., "google.com", "google.de") |
| `start` | integer | No | Pagination offset (0, 10, 20...) |
| `amount` | integer | No | Number of results (1-100, default: 10) |
| `safe` | string | No | Safe Search setting (`off` or `active`) |
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

The complete response is saved to the `OUTPUT` key, including:

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

The actor saves one dataset item per organic result, in Scrappa's result order. Dataset writes are limited to the remaining Apify `PAY_PER_EVENT` budget. If the spending limit covers only part of the result set, the affordable prefix is saved and the complete Scrappa response remains available in `OUTPUT`.

## Runtime

The production actor is a Rust binary. It reads `INPUT` from the default key-value store, calls Scrappa with `SCRAPPA_API_KEY`, writes organic results to the default dataset, and stores the full response as `OUTPUT`. It uses `ACTOR_DEFAULT_KEY_VALUE_STORE_ID`, `ACTOR_DEFAULT_DATASET_ID`, `ACTOR_RUN_ID`, `ACTOR_INPUT_KEY` (defaults to `INPUT`), and `APIFY_TOKEN` for Apify storage and run-pricing access. Scrappa requests have a 60-second timeout and retry up to three times for transient API or connection failures.

## Example

```json
{
  "query": "best pizza in chicago",
  "location": "Chicago, IL, USA",
  "gl": "us",
  "hl": "en",
  "amount": 20
}
```

## Pricing

$0.30 per 1,000 results. No additional API keys required.

## Support

For issues or questions, contact us through Apify.
