# Indeed Jobs Scraper

Search Indeed job listings through the Scrappa Indeed Jobs API.

## Features

- Indeed job listings with title, company, location, salary, description, attributes, date, and apply URL
- Country, language, geolocation, radius, job type, and sort targeting
- Cursor pagination through `data.pagination.next_cursor`
- Full raw API response saved to the key-value store
- Paid usage-aligned actor surface for high-intent Indeed job-search workflows

## Input

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `query` | string | No | Job search keywords. Defaults to `software engineer` when the actor is started with empty or placeholder input. |
| `location` | string | No | City, state, region, or remote location. Defaults to `New York` for empty input. |
| `country` | string | No | Two-letter country code such as `US`, `GB`, `CA`, `AU`, `DE`, or `FR`. |
| `radius` | integer | No | Search radius from the location, 0 to 100. |
| `radius_unit` | string | No | `MILES` or `KILOMETERS`. |
| `job_type` | string | No | `full_time`, `part_time`, `contract`, `internship`, or `remote`. |
| `sort` | string | No | `relevance` or `date`. |
| `limit` | integer | No | Results per request, 1 to 100. |
| `cursor` | string | No | Pagination cursor from a previous response. |
| `hl` | string | No | Two-letter interface language code. |
| `gl` | string | No | Two-letter geolocation country code. |

## Output

### Dataset

Each job in `data.jobs` is saved to the dataset.

```json
{
  "id": "example-job-id",
  "title": "Software Engineer",
  "company_name": "Example Corp",
  "company": {
    "name": "Example Corp",
    "website": "https://example.com"
  },
  "location_formatted": "New York, NY",
  "location": {
    "city": "New York",
    "state": "NY",
    "country": "US",
    "formatted": "New York, NY",
    "is_remote": false
  },
  "salary": {
    "min": 120000,
    "max": 150000,
    "currency": "USD",
    "unit": "year"
  },
  "attributes": ["Full-time"],
  "date_published": "2026-05-01 12:00:00",
  "apply_url": "https://www.indeed.com/viewjob?jk=example"
}
```

### Key-Value Store

The complete Scrappa response is saved to the `OUTPUT` key, including pagination and metadata.

```json
{
  "success": true,
  "data": {
    "jobs": [],
    "pagination": {
      "next_cursor": "...",
      "has_more": true
    },
    "metadata": {
      "total_results": 20
    }
  }
}
```

## Example

```json
{
  "query": "software engineer",
  "location": "New York",
  "country": "US",
  "job_type": "full_time",
  "limit": 20
}
```

## Support

For issues or questions, contact us through Apify.
