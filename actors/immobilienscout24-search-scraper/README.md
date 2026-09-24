# ImmobilienScout24 Search Scraper

Rust Apify actor wrapper for Scrappa's `GET /api/immobilienscout24/search`
endpoint. Scraping stays on Scrappa infrastructure. The actor writes one Apify
dataset item per returned property listing and charges the `property-result`
event for saved listings in pay-per-event runs.

## Input

```json
{
  "location": "1276003001",
  "type": "apartment-rent",
  "price_max": 1500,
  "rooms_min": 2,
  "page": 1,
  "per_page": 20
}
```

Supported `type` values:

- `apartment-rent`
- `apartment-buy`
- `house-rent`
- `house-buy`

Optional filters are `price_min`, `price_max`, `rooms_min`, `rooms_max`,
`size_min`, and `size_max`.

## Output

Each dataset item is a property listing from ImmobilienScout24 plus request
context fields such as `request_location`, `request_type`, `request_page`, and
the optional filter values used for the run.

The actor also writes the trimmed Scrappa response to key-value store key
`OUTPUT` for compatibility with users who expect a single response object.

The actor retries Scrappa timeouts and HTTP 408, 429, 500, 502, 503, and 504
responses up to three attempts. Each request has a 90-second deadline.
