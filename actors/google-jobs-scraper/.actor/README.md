# Google Jobs Scraper

Search job listings indexed by Google Jobs through the Scrappa Google Jobs API.

## Features

- Job listings with title, company, location, source, description, and metadata
- Country, language, Google domain, and encoded-location targeting
- Google Jobs filter token support
- Next-page token support for pagination
- Full raw API response saved to the key-value store

## Input

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `q` | string | Conditionally | Job search query. Required unless `next_page_token` is provided. Defaults to `software engineer` when the actor is started with an empty input. |
| `next_page_token` | string | Conditionally | Token from a previous response to fetch the next page. |
| `gl` | string | No | Two-letter country code, for example `us`, `uk`, or `de`. |
| `hl` | string | No | Two-letter language code, for example `en`, `de`, or `es`. |
| `google_domain` | string | No | Google domain to query, for example `google.com` or `google.de`. |
| `uule` | string | No | Google-encoded location parameter for precise geolocation. |
| `lrad` | integer | No | Search radius in miles. Requires `uule`. |
| `uds` | string | No | Dynamic filter string returned in a previous response. |

## Output

### Dataset

Each job in the `jobs` array is saved to the dataset.

```json
{
  "title": "Software Engineer",
  "company": "Example Corp",
  "location": "New York, NY",
  "via": "LinkedIn",
  "description": "Build and maintain production systems...",
  "job_id": "example-job-id"
}
```

### Key-Value Store

The complete Scrappa response is saved to the `OUTPUT` key, including filters and pagination tokens when returned.

```json
{
  "jobs": [],
  "filters": [],
  "next_page_token": "..."
}
```

## Example

```json
{
  "q": "software engineer",
  "gl": "us",
  "hl": "en",
  "google_domain": "google.com"
}
```

## Support

For issues or questions, contact us through Apify.
