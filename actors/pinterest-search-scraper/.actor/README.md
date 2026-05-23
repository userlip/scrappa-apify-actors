# Pinterest Search Scraper

Search Pinterest pins by keyword and export one dataset item per pin. This actor is a thin Scrappa-powered wrapper around Scrappa's `GET /api/pinterest/search` endpoint: Apify validates input, batches keyword searches in one run, calls Scrappa, and writes Pinterest pin results to the dataset.

## Use Cases

- Find Pinterest pins for visual discovery, ecommerce research, and content trend monitoring.
- Collect pin titles, descriptions, images, links, domains, pinner and board context, and social metrics by keyword.
- Page through Pinterest keyword search using the returned `nextBookmark`.
- Batch related Pinterest keyword searches in one Apify run instead of paying run overhead for each query.

## Input

Use `queries` for batch searches. `query` is kept for compatibility with single-query integrations.

```json
{
  "queries": ["home decor", "kitchen ideas"],
  "limit": 25
}
```

Pagination example:

```json
{
  "query": "home decor",
  "limit": 25,
  "bookmark": "PASTE_NEXT_BOOKMARK_HERE"
}
```

| Field | Type | Required | Description |
|---|---:|---:|---|
| `queries` | array | No | Preferred batch input. Up to 100 Pinterest keyword searches per run. |
| `query` | string | No | Single-query compatibility input. |
| `limit` | integer | No | Pins requested per query. Defaults to `50`, capped at `250`. Pinterest may return fewer results. |
| `bookmark` | string | No | Optional pagination bookmark from `nextBookmark`. Applied to each query in the run. |

Provide at least one value in `queries` or `query`.

## Output

The actor writes one dataset item per Pinterest pin. Each item includes the original pin data plus normalized fields:

```json
{
  "id": "123456789",
  "title": "Small apartment storage ideas",
  "description": "Storage ideas for small spaces",
  "image_url": "https://i.pinimg.com/originals/example.jpg",
  "link": "https://example.com/storage",
  "domain": "example.com",
  "pinner_username": "homeideas",
  "board_name": "Home Decor",
  "has_video": false,
  "repin_count": 42,
  "comment_count": 3,
  "request_query": "home decor",
  "request_limit": 25,
  "request_bookmark": null,
  "count": 25,
  "results_count": 25,
  "nextBookmark": "NEXT_PAGE_BOOKMARK"
}
```

`count`, `results_count`, and `nextBookmark` are copied from Scrappa's Pinterest search response when available, so short pages and pagination state are visible in the dataset.

## Scale

This actor is designed for high-intent Pinterest keyword search from Apify. For higher-volume visual discovery, ecommerce research, or direct API integration, use Scrappa's Pinterest Search API directly at `https://scrappa.co/api/pinterest/search`.
