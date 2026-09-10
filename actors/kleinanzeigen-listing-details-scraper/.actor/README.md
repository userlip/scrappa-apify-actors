# Kleinanzeigen Listing Details Scraper

Get full details for Kleinanzeigen listings through Scrappa's structured API. This thin Apify wrapper accepts one listing ID or batches up to 100 IDs in one run, avoiding one run per listing.

The Actor uses the paid `listing-detail-result` event at the proposed rate of **$0.25 per 1,000 successfully saved listings**. You pay only for successful dataset rows.

## Input

The prefilled `query: "fahrrad"` searches current listings and saves one successful detail, trying up to three distinct IDs. This avoids an example ad expiring. Discovery makes at most four Scrappa requests, each with a 60-second timeout and no retries (240 seconds of HTTP work plus Actor overhead). Search rows themselves are never saved or charged.

Use `ad_id`, `ad_ids`, or both to fetch specific listings; these take priority over `query`. IDs are trimmed, deduplicated in first-seen order, and processed sequentially.

```json
{
  "ad_id": "3451021120",
  "ad_ids": ["3451021120", "3451021121"]
}
```

## Output

Each successful listing becomes one dataset item. Unavailable or malformed individual listings are reported in the aggregate `OUTPUT` record and do not stop later IDs. A run where every attempted detail fails ends as FAILED after writing OUTPUT.

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
