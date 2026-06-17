# TrustedShops Reviews Scraper

Extract TrustedShops review data by shop TSID or TrustedShops profile URL. This Actor is a thin Apify wrapper around Scrappa's TrustedShops Reviews API and writes one dataset item per review.

## What You Get

- TrustedShops reviews for one or many shops in a single Apify run
- One charged dataset item per saved review
- Normalized fields for dashboards and monitoring: `tsid`, `shop_name`, `rating`, `review_title`, `review_text`, `created_at`, `verified`, `criteria`, `review_id`, `page`, and `source_url`
- Raw TrustedShops review fields preserved on each dataset item when Scrappa returns additional fields
- Batch input to avoid one-run-per-shop workflows

## Example Input

```json
{
  "tsids": [
    "XFB15FFBDE1DEE7A55D292A7D48598A6A"
  ],
  "urls": [
    "https://www.trustedshops.de/bewertung/info_XFB15FFBDE1DEE7A55D292A7D48598A6A.html"
  ],
  "page": 1,
  "max_pages": 2,
  "size": 20
}
```

`tsids` and `urls` can be mixed. Duplicate TSIDs are fetched once.

## Output

Each review is saved as a dataset item:

```json
{
  "tsid": "XFB15FFBDE1DEE7A55D292A7D48598A6A",
  "shop_name": "Example Shop",
  "rating": 5,
  "review_title": "Great service",
  "review_text": "Fast delivery and helpful support.",
  "created_at": "2026-05-20T10:15:00Z",
  "verified": true,
  "criteria": {},
  "review_id": "review-123",
  "page": 1,
  "source_url": "https://www.trustedshops.de/bewertung/info_XFB15FFBDE1DEE7A55D292A7D48598A6A.html"
}
```

## Pricing

This Actor uses pay-per-event pricing. The `review-result` event is charged once per saved review.

Proposed event price: `$0.00025` per review.

## Direct API

For higher-volume workloads, scheduled reputation monitoring, or direct backend integration, use Scrappa's API directly:

```http
GET https://scrappa.co/api/trustedshops/reviews/{tsid}?page=1&size=20
```

Scrappa keeps the scraping workload on Scrappa infrastructure while Apify handles runs, datasets, and marketplace billing.
