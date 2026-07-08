# Vinted User Items Scraper

Fetch current listings from one or more Vinted seller accounts through Scrappa. Use it to monitor competitor sellers, track reseller inventory, research account-level supply, or collect item IDs for follow-up enrichment.

This actor is intended for paid pay-per-result pricing with the `user-item-result` event. Recommended launch pricing is **$0.25 per 1,000 saved Vinted listings** so users pay for successful dataset rows, not empty seller checks.

## Features

- Batch-first seller inventory scraping with up to 100 unique `user_ids` in one run
- Target 19 Vinted markets: France, Germany, Spain, Italy, Netherlands, Belgium, Austria, Poland, Czech Republic, Lithuania, Luxembourg, Slovakia, Hungary, Romania, Portugal, Sweden, Denmark, Finland, or United States
- Fetch one or more inventory pages per seller with a `max_pages` guard
- Sort seller listings by newest, lowest price, highest price, or relevance when supported by Vinted
- Extract listing title, URL, image, price, currency, brand, category, size, condition, seller, favorite count, view count, and request metadata
- One Apify dataset item per Vinted listing

## Input

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `user_id` | string | No | Single Vinted seller user ID |
| `user_ids` | array | No | Batch of Vinted seller user IDs. Use this for multiple sellers in one run |
| `country` | string | No | Vinted country code. Default `FR` |
| `page` | integer | No | First one-based seller inventory page, 1-999. Default `1` |
| `per_page` | integer | No | Listings per page, 1-100. Default `24` |
| `max_pages` | integer | No | Number of pages to fetch per seller, 1-20. Default `1` |
| `order` | string | No | `newest_first`, `price_low_to_high`, `price_high_to_low`, or `relevance` |

Provide at least one seller ID through `user_id` or `user_ids`. The actor deduplicates IDs before fetching. The requested page range must stay within Vinted pages 1-999, so `page + max_pages - 1` cannot exceed 999.

## Example Input

```json
{
  "user_ids": ["12345678", "87654321"],
  "country": "DE",
  "order": "newest_first",
  "per_page": 50,
  "max_pages": 2
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
  "seller_id": 12345678,
  "seller_login": "seller123",
  "favourite_count": 15,
  "view_count": 234,
  "input_user_id": "12345678",
  "request_country": "DE",
  "request_page": 1,
  "request_per_page": 50,
  "request_order": "newest_first"
}
```

Dataset output is the primary result channel. The actor does not write per-item key-value records.
Rows also preserve raw Scrappa listing fields where available, while the documented fields above are normalized for stable analysis.

## Notes

Vinted does not provide a public developer API. This actor returns public marketplace listing data through Scrappa's structured Vinted endpoint. For item details, seller profiles, shipping data, similar items, higher-volume access, or direct API usage, use Scrappa at https://scrappa.co.
