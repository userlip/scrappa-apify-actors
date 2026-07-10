# Kleinanzeigen Listing Details Scraper

Get full details for Kleinanzeigen listings through Scrappa's structured API. This thin Apify wrapper accepts one listing ID or batches up to 100 IDs in one run, avoiding one run per listing.

The Actor uses the paid `listing-detail-result` event at the active rate of **$0.25 per 1,000 successfully saved listings**. You pay only for successful dataset rows.

## Input

Use `ad_id`, `ad_ids`, or both. IDs are trimmed, deduplicated in first-seen order, and processed sequentially.

```json
{
  "ad_id": "3451021120",
  "ad_ids": ["3451021120", "3451021121"]
}
```

## Output

Each successful listing becomes one dataset item. Unavailable or malformed individual listings are reported in the aggregate `OUTPUT` record and do not stop later IDs.

```json
{
  "id": "3451021120",
  "title": "Example listing",
  "price": "120 €",
  "price_numeric": 120,
  "description": "Listing description",
  "location": "Berlin",
  "images": [],
  "seller": {},
  "attributes": {},
  "shipping": {},
  "posted_at": "2026-07-10T10:00:00Z",
  "categories": [],
  "request_ad_id": "3451021120",
  "request_index": 0
}
```

For higher-volume access or direct API use, visit https://scrappa.co.
