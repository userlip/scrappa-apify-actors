# LinkedIn Search Scraper

Find public LinkedIn profile, company, post, jobs, and hiring page URLs through the Scrappa LinkedIn Search API.

This actor is a thin Apify marketplace wrapper around Scrappa. It validates input, calls `https://scrappa.co/api/linkedin/search`, and saves one dataset item per organic result.

## Features

- Search Google-indexed LinkedIn pages with precise `site:linkedin.com/...` queries
- Discover profile URLs, company pages, posts, job pages, and hiring pages
- Supports country, language, safe-search, date, duplicate-filter, and pagination controls
- Returns up to 20 organic results per run to keep result costs amortized
- Saves a compact `OUTPUT` summary with search metadata and pagination

## Example Searches

```json
{
  "query": "site:linkedin.com/in founder AI Berlin",
  "num": 3,
  "gl": "de",
  "hl": "en"
}
```

```json
{
  "query": "site:linkedin.com/company fintech recruiting London",
  "num": 10,
  "gl": "gb",
  "hl": "en"
}
```

```json
{
  "query": "site:linkedin.com/posts \"product launch\" \"cybersecurity\"",
  "num": 10,
  "dateRestrict": "m1"
}
```

## Input

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `query` | string | Yes | Google-style LinkedIn search query. Use `site:linkedin.com/in`, `site:linkedin.com/company`, `site:linkedin.com/posts`, or `site:linkedin.com/jobs/view` to target result types. |
| `num` | integer | No | Organic results per run. Accepted range is 1 to 20. Defaults to 10. |
| `page` | integer | No | Page number for pagination. Mutually exclusive with `start`. |
| `start` | integer | No | Starting result index for pagination. Mutually exclusive with `page`. |
| `hl` | string | No | Interface language, for example `en`, `de`, or `es`. |
| `gl` | string | No | Geolocation country code, for example `us`, `de`, or `gb`. |
| `lr` | string | No | Language restrict, for example `lang_en`. |
| `cr` | string | No | Country restrict, for example `countryUS`. |
| `safe` | string | No | Safe search setting: `off` or `active`. |
| `dateRestrict` | string | No | Date filter, for example `d7`, `w1`, `m1`, or `y1`. |
| `sort` | string | No | Google sort parameter, for example `date`. |
| `filter` | integer | No | Duplicate filtering, `0` or `1`. |
| `rights` | string | No | Usage-rights filter supported by Google Search. |

## Output

Each item from `organic_results` is saved as one dataset row.

```json
{
  "position": 1,
  "title": "Jane Founder - AI Startup",
  "link": "https://www.linkedin.com/in/example-founder",
  "displayed_link": "https://www.linkedin.com/in/example-founder",
  "snippet": "Founder working on AI products in Berlin..."
}
```

The key-value store `OUTPUT` key contains a compact run summary with result counts, search metadata, and pagination.

```json
{
  "results": 3,
  "total_results": 5750000,
  "current_page": 1,
  "pages": 10,
  "search_information": {
    "query_displayed": "site:linkedin.com/in founder AI Berlin"
  },
  "pagination": {
    "current_page": 1,
    "pages": [
      { "page": 1, "start": 0 },
      { "page": 2, "start": 3 },
      { "page": 3, "start": 6 },
      { "page": 4, "start": 9 },
      { "page": 5, "start": 12 },
      { "page": 6, "start": 15 },
      { "page": 7, "start": 18 },
      { "page": 8, "start": 21 },
      { "page": 9, "start": 24 },
      { "page": 10, "start": 27 }
    ]
  }
}
```

LinkedIn search results depend on pages indexed by Google and may vary by query, language, country, and date filters.

## Support

For issues or questions, contact us through Apify.
