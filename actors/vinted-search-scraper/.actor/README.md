# Vinted Search Scraper

Search Vinted marketplace listings by keyword and filters through Scrappa. Use it to monitor resale prices, find underpriced secondhand fashion, track brand/category supply, or collect item IDs for later detail enrichment.

This actor is intended for paid pay-per-result pricing with the `item-result` event. Recommended launch pricing is **$0.20 per 1,000 saved Vinted listings** so users pay for successful dataset rows, not empty runs.

## Features

- Search Vinted listings by keyword or browse by filters
- Target 19 Vinted markets: France, Germany, Spain, Italy, Netherlands, Belgium, Austria, Poland, Czech Republic, Lithuania, Luxembourg, Slovakia, Hungary, Romania, Portugal, Sweden, Denmark, Finland, or United States
- Filter by category, brand, size, color, material, condition/status, and price range
- Sort by relevance, newest listings, lowest price, or highest price
- Fetch one or more result pages per run
- Extract listing title, URL, image, price, currency, brand, category, size, condition, seller, favorite count, view count, and request metadata
- Full Scrappa page responses saved to the `OUTPUT` key-value-store record

## Input

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `query` | string | No | Search query, for example `nike shoes` or `zara dress` |
| `country` | string | No | Vinted country code. Default `FR` |
| `page` | integer | No | First one-based result page, 1-999. Default `1` |
| `per_page` | integer | No | Listings per page, 1-100. Default `24` |
| `max_pages` | integer | No | Number of pages to fetch, 1-20. Default `1` |
| `order` | string | No | `relevance`, `newest_first`, `price_low_to_high`, or `price_high_to_low` |
| `catalog_ids` | string | No | Comma-separated Vinted category IDs |
| `brand_ids` | string | No | Comma-separated Vinted brand IDs |
| `size_ids` | string | No | Comma-separated Vinted size IDs |
| `color_ids` | string | No | Comma-separated Vinted color IDs |
| `material_ids` | string | No | Comma-separated Vinted material IDs |
| `status_ids` | string | No | Comma-separated Vinted condition/status IDs |
| `price_from` | number | No | Minimum listing price |
| `price_to` | number | No | Maximum listing price |

## Example Input

```json
{
  "query": "nike shoes",
  "country": "DE",
  "order": "newest_first",
  "per_page": 50,
  "max_pages": 2,
  "price_to": 80
}
```

## Output

Each listing is saved as one dataset item:

```json
{
  "id": "1234567890",
  "title": "Nike Air Max 90",
  "price_amount": "45.00",
  "price_currency": "EUR",
  "brand_name": "Nike",
  "category_name": "Shoes",
  "size_name": "EU 42",
  "condition": "Very good",
  "url": "https://www.vinted.de/items/1234567890-nike-air-max-90",
  "image_url": "https://images1.vinted.net/t/01_example/image.jpg",
  "seller_login": "seller123",
  "favourite_count": 15,
  "view_count": 234,
  "request_query": "nike shoes",
  "request_country": "DE",
  "request_page": 1
}
```

The `OUTPUT` record includes the request summary, pages fetched, saved listing count, reported pagination totals, and raw Scrappa responses for fetched pages.

## Notes

Vinted does not provide a public developer API. This actor returns public marketplace listing data through Scrappa's structured Vinted endpoint. For item details, seller profiles, shipping data, similar items, higher-volume access, or direct API usage, use Scrappa at https://scrappa.co.
