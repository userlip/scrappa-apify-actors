# LinkedIn Jobs Search Scraper

Search public LinkedIn job listing pages through the Scrappa LinkedIn Jobs Search API.

## Features

- Searches LinkedIn job listing pages with an automatic `site:linkedin.com/jobs/view/` constraint
- Supports keyword, company, seniority, and location terms in one query
- Country, language, safe-search, date, duplicate-filter, and pagination controls
- Saves each LinkedIn job search result to the dataset
- Saves the full raw Scrappa response to the key-value store

## Input

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `query` | string | No | Job search query. Defaults to `software engineer remote` when the actor is started with empty or placeholder input. |
| `num` | integer | No | Results per page. Accepted range is 1 to 20. |
| `page` | integer | No | Page number for pagination. Mutually exclusive with `start`. |
| `start` | integer | No | Starting result index for pagination. Mutually exclusive with `page`. |
| `hl` | string | No | Interface language, for example `en`, `de`, or `es`. |
| `gl` | string | No | Geolocation country code, for example `us`, `de`, or `uk`. |
| `lr` | string | No | Language restrict, for example `lang_en`. |
| `cr` | string | No | Country restrict, for example `countryUS`. |
| `safe` | string | No | Safe search setting: `off` or `active`. |
| `dateRestrict` | string | No | Date filter, for example `d7`, `w1`, `m1`, or `y1`. |
| `sort` | string | No | Google sort parameter, for example `date`. |
| `filter` | integer | No | Duplicate filtering, `0` or `1`. |
| `rights` | string | No | Usage-rights filter supported by Google Search. |

## Output

### Dataset

Each item from `organic_results` is saved as one dataset row.

```json
{
  "position": 1,
  "title": "Software Engineer - Example Corp",
  "link": "https://www.linkedin.com/jobs/view/1234567890",
  "displayed_link": "https://www.linkedin.com/jobs/view/1234567890",
  "snippet": "Example Corp is hiring a Software Engineer..."
}
```

### Key-Value Store

The complete Scrappa response is saved to the `OUTPUT` key, including search metadata and pagination.

```json
{
  "organic_results": [],
  "search_information": {
    "query_displayed": "site:linkedin.com/jobs/view/ software engineer remote",
    "total_results": 123
  },
  "pagination": {
    "current_page": 1,
    "pages": []
  }
}
```

## Example

```json
{
  "query": "software engineer remote",
  "num": 10,
  "hl": "en",
  "gl": "us",
  "safe": "off"
}
```

## Support

For issues or questions, contact us through Apify.
