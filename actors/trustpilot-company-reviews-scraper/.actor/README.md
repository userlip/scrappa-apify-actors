# Trustpilot Company Reviews Scraper

Extract Trustpilot reviews for a company domain through Scrappa. Use it for review monitoring, reputation analysis, customer-support QA, brand tracking, and competitor research.

## Features

- Review titles, text, IDs, ratings, languages, dates, verification status, and company replies
- Reviewer details from Trustpilot consumer profiles
- Filters for rating, verified reviews, company replies, keyword search, date posted, locale, and sort order
- Multi-page runs up to Trustpilot's public page limit
- Dataset rows optimized for Apify table views
- Full Scrappa responses saved to the `OUTPUT` key-value-store record

## Input

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `company_domain` | string | Yes | Company domain, for example `amazon.com` |
| `locale` | string | No | Trustpilot locale, default `en-US` |
| `page` | integer | No | First page to fetch, 1-10 |
| `max_pages` | integer | No | Number of pages to fetch, 1-10 |
| `per_page` | integer | No | Reviews per page after filtering, 1-100 |
| `sort` | string | No | `recency` or `relevance` |
| `rating` | string | No | Comma-separated ratings, for example `1,2` |
| `verified` | boolean | No | Return only verified reviews |
| `with_replies` | boolean | No | Return only reviews with company replies |
| `query` | string | No | Search within review title or text |
| `date_posted` | string | No | `any`, `last_30_days`, `last_3_months`, `last_6_months`, or `last_12_months` |

## Example Input

```json
{
  "company_domain": "amazon.com",
  "locale": "en-US",
  "page": 1,
  "max_pages": 2,
  "per_page": 20,
  "sort": "recency",
  "rating": "1,2",
  "date_posted": "last_30_days"
}
```

## Output

Each Trustpilot review is saved as one dataset item:

```json
{
  "id": "example-review-id",
  "title": "Delivery was late",
  "text": "The order arrived later than expected.",
  "rating": 2,
  "language": "en",
  "isVerified": true,
  "consumer_name": "Example Customer",
  "published_date": "2026-05-01T12:30:00.000Z",
  "company_domain": "amazon.com",
  "request_page": 1,
  "request_sort": "recency",
  "request_rating": "1,2"
}
```

The full combined response is saved to `OUTPUT`, including per-page Scrappa responses and pagination metadata.

## Notes

Trustpilot limits public unauthenticated access to approximately the first 10 review pages. For higher-volume review monitoring or direct API access, use Scrappa at https://scrappa.co.
