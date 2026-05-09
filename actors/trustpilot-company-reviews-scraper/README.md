# Trustpilot Company Reviews Scraper

Scrape Trustpilot company reviews by domain using Scrappa's `/api/trustpilot/company-reviews` endpoint. The actor is built for review monitoring, reputation analysis, and competitor research.

## What you get

- Review text, titles, IDs, ratings, dates, language, verification status, reviewer data, and company replies
- Filters for rating, keyword, verified status, replies, date range, locale, and sort order
- Multi-page collection up to Trustpilot's public page limit
- Full Scrappa responses saved to key-value store record `OUTPUT`

## Example Input

```json
{
  "company_domain": "amazon.com",
  "locale": "en-US",
  "page": 1,
  "max_pages": 1,
  "per_page": 20,
  "sort": "recency"
}
```

## Example Dataset Item

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
  "request_sort": "recency"
}
```

For higher-volume Trustpilot review monitoring or direct API access, use Scrappa at `https://scrappa.co/api/trustpilot/company-reviews`.
