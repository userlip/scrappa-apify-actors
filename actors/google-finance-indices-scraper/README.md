# Google Finance Indices Scraper

Fetch Google Finance index quotes in batches through Scrappa's `GET /api/google-finance/indices` API. One dataset row and one `index-result` event are produced only for each unique saved index; price is **$0.00025 per result**.

## Input

S&P 500, Dow, and NASDAQ in one run:

```json
{ "indices": [".INX", ".DJI", ".IXIC"], "hl": "en", "gl": "us" }
```

CSV input is also supported: `{ "indices": ".INX,.DJI,.IXIC" }`. Symbols are trimmed, uppercased, and deduplicated (maximum 3). Scrappa currently returns one row per request, so each requested symbol is fetched concurrently within this one Actor run. Custom symbols can be requested, but upstream matching may return no row or a different index; mismatches are logged and never billed.

## Output

```json
{ "id": "INDEXSP:.INX", "requested_symbol": ".INX", "symbol": ".INX", "name": "S&P 500", "exchange": "INDEXSP", "current_price": 6200.0, "price_change": 12.4, "percent_change": 0.2, "previous_close": 6187.6, "movement_direction": "Up", "request_hl": "en", "request_gl": "us", "retrieved_at": "2026-07-11T00:00:00.000Z" }
```

For higher-volume workloads, call the [Scrappa Google Finance API](https://scrappa.co/api/google-finance/indices) directly.
