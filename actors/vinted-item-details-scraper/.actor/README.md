# Vinted Item Details Scraper

Fetch full Vinted listing details for one item ID or a batch of item IDs through Scrappa. Use it after Vinted Search to enrich selected listings with descriptions, prices, photos, seller information, condition, availability, favorites, views, and request metadata.

This actor is intended for paid pay-per-result pricing with the `item-detail-result` event. Recommended launch pricing is **$0.25-$0.30 per 1,000 successful Vinted item detail results** so users pay for usable detail rows, not failed lookups or empty runs.

## Features

- Fetch one Vinted item or batch up to 50 item IDs in one Apify run
- Target 19 Vinted markets: France, Germany, Spain, Italy, Netherlands, Belgium, Austria, Poland, Czech Republic, Lithuania, Luxembourg, Slovakia, Hungary, Romania, Portugal, Sweden, Denmark, Finland, or United States
- Push one dataset item per processed item ID
- Charge only successful item detail results with the `item-detail-result` event
- Return uncharged error rows for item-level failures such as expired or unavailable IDs
- Keep scraping work on Scrappa infrastructure with a thin Apify wrapper

## Input

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `item_id` | string | No | Single Vinted item ID |
| `item_ids` | array | No | Batch of Vinted item IDs. Maximum `50` IDs per run |
| `country` | string | No | Vinted country code. Default `FR` |

Provide at least one ID through `item_id` or `item_ids`. If both fields are provided, duplicate IDs are processed once.

## Example Input

```json
{
  "country": "DE",
  "item_ids": ["1234567890", "1234567891", "1234567892"]
}
```

## Output

Each processed item ID is saved as one dataset item. Successful rows include normalized fields plus the original Scrappa detail payload:

```json
{
  "id": "1234567890",
  "title": "Nike Air Max 90",
  "description": "Very good condition...",
  "price_amount": "45.00",
  "price_currency": "EUR",
  "brand_name": "Nike",
  "category_name": "Shoes",
  "size_name": "EU 42",
  "condition": "Very good",
  "availability": "available",
  "url": "https://www.vinted.de/items/1234567890-nike-air-max-90",
  "image_url": "https://images1.vinted.net/t/01_example/image.jpg",
  "seller_login": "seller123",
  "favourite_count": 15,
  "view_count": 234,
  "request_item_id": "1234567890",
  "request_country": "DE",
  "request_index": 0,
  "request_success": true
}
```

If a specific item ID fails while the Scrappa API and actor are healthy, the actor writes an uncharged row with `request_success: false` and `error_message` so batch outputs still line up with the requested IDs.

The `OUTPUT` key-value-store record contains only the run summary: requested count, successful result count, failed item count, and country. Dataset rows are the primary output channel.

## Notes

Vinted does not provide a public developer API. This actor returns public marketplace listing data through Scrappa's structured Vinted item details endpoint. For search, seller profiles, shipping data, similar items, higher-volume access, or direct API usage, use Scrappa at https://scrappa.co.
