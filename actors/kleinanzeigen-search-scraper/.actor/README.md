# Kleinanzeigen Search Scraper

Search Kleinanzeigen listings by keyword, location, category, page, and EUR price range through Scrappa. Use it to monitor local marketplace prices, collect listing URLs, track availability, or compare category supply across German cities.

This actor is intended for paid pay-per-result pricing with the `listing-result` event. Recommended launch pricing is **$0.20-$0.30 per 1,000 saved Kleinanzeigen listings** so users pay for successful dataset rows, not empty runs.

## Features

- Search public Kleinanzeigen listings by required query
- Filter by location, category, result page, minimum price, and maximum price
- Run up to 25 searches in one Apify run through `searches[]`
- Save each listing as one dataset item for usage-aligned charging
- Extract listing ID, title, URL, price, numeric price, location, image, description, shipping flag, and request metadata
- Save an aggregate `OUTPUT` record with request summaries, saved listing counts, and raw Scrappa responses
- Keeps scraping work on Scrappa infrastructure; the Actor is a thin Apify wrapper

## Input

Provide either a single top-level search or a `searches[]` batch. When `searches[]` is provided, top-level search fields are ignored.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `query` | string | Yes for single search | Search query, for example `iphone`, `fahrrad`, or `sofa` |
| `page` | integer | No | One-based search results page, 1-100. Default `1` |
| `location` | string | No | City, district, or location filter, for example `Berlin` |
| `category` | string | No | Category slug or keyword, for example `elektronik`, `auto`, or `moebel` |
| `price_min` | integer | No | Minimum listing price in EUR |
| `price_max` | integer | No | Maximum listing price in EUR. Must be greater than or equal to `price_min` |
| `searches` | array | No | Batch of up to 25 search objects using the same fields |

## Single Search Example

```json
{
  "query": "iphone",
  "location": "Berlin",
  "page": 1,
  "price_min": 100,
  "price_max": 900
}
```

## Batch Search Example

```json
{
  "searches": [
    {
      "query": "iphone",
      "location": "Berlin",
      "page": 1
    },
    {
      "query": "fahrrad",
      "location": "Hamburg",
      "price_max": 500
    }
  ]
}
```

## Output

Each listing is saved as one dataset item:

```json
{
  "id": "2987654321",
  "title": "Apple iPhone 15 Pro 256GB",
  "price": "850 EUR",
  "price_numeric": 850,
  "location": "10115 Mitte",
  "url": "https://www.kleinanzeigen.de/s-anzeige/example/2987654321",
  "image_url": "https://img.kleinanzeigen.de/example.jpg",
  "description": "Sehr guter Zustand",
  "has_shipping": true,
  "request_query": "iphone",
  "request_location": "Berlin",
  "request_page": 1
}
```

The `OUTPUT` key-value-store record includes `searches_requested`, `searches_completed`, `listings_extracted`, optional `status_message`, and the raw Scrappa responses for completed searches.

## Notes

Kleinanzeigen does not provide a public developer API. This actor returns public marketplace listing data through Scrappa's structured Kleinanzeigen endpoint. For higher-volume access, direct API usage, or additional Kleinanzeigen endpoints, use Scrappa at https://scrappa.co.
