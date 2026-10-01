# Google Finance Indices Scraper

Compare Google Finance index quotes with tickers, index levels and daily changes. Supply supported index codes to compare their current levels and daily movement.

## What data can you extract?

Prices and percentage changes follow the currency and units Google Finance displays for each instrument.

| Field | Type | Description |
| --- | --- | --- |
| `id` | text | source ID for the market-index quote, assigned by Google Finance; null when the source does not expose it. |
| `requested_symbol` | text | Requested symbol shown for the market-index quote by Google Finance, in the format used by the source; null when it is omitted. |
| `symbol` | text | Ticker symbol shown for the market-index quote by Google Finance, in the format used by the source; null when it is omitted. |
| `name` | text | Name of the market-index quote, as shown by Google Finance; null when no name is published. |
| `exchange` | text | Exchange code shown for the market-index quote by Google Finance; null when Google Finance does not provide the value. |
| `current_price` | number | Current quote for this market-index quote, as a numeric quote in the record currency; null when Google Finance provides no price. |
| `price_change` | number | Price change for this market-index quote, as a numeric change in the record currency; null when Google Finance provides no price. |
| `percent_change` | number | Percentage price change reported by Google Finance, as a percentage or share in the source format; null when no estimate is available. |
| `previous_close` | number | Previous close for this market-index quote, as a numeric previous closing amount in the record currency; null when Google Finance provides no price. |
| `movement_direction` | text | Movement direction shown for the market-index quote by Google Finance; null when Google Finance does not provide the value. |
| `request_hl` | text | Interface language code passed to Google Finance; Use a language code such as en or de. This input value is copied into the output row; null when it was not supplied. |
| `request_gl` | text | Two-letter country or region code passed to Google Finance; Use a two-letter country code such as us or de. This input value is copied into the output row; null when it was not supplied. |
| `retrieved_at` | date | Time the page was retrieved shown by Google Finance, in ISO 8601 date and time; null if the source omits the date. |

## Use cases

- Investors can compare price movement and market values for tickers they follow.
- Analysts can chart quotes or historical points beside the symbol and exchange.
- Finance teams can refresh market data in recurring spreadsheet reports.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Set `indices` to the search term or source identifier you want to look up, then use the optional filters listed below.
3. Start the run and open its default dataset to inspect or download the rows.

```json
{
  "indices": [
    ".INX"
  ]
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `indices` | string/array | No | A comma-separated list or JSON array of up to 3 symbols, for example .INX, .DJI, .IXIC. |
| `hl` | string | No | Google Finance language code, such as en or de. |
| `gl` | string | No | Two-letter Google Finance country code, such as us or de. |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "name": "NASDAQ-100",
  "current_price": 20184.23,
  "id": "NASDAQ:NDX",
  "requested_symbol": "NDX",
  "symbol": "NDX",
  "exchange": "NASDAQ",
  "price_change": 132.67,
  "percent_change": 0.66
}
```

## Pricing

**Current live price:** $0.25 per 1,000 results.

Each saved dataset record counts as one result.

## FAQ

### Is it legal to collect public information?

This Actor is for information visible on the public source pages it reads. You are responsible for checking the source terms, privacy and copyright rules, and the laws that apply to your use.

### How many records will a run return?

The result count depends on the input limits, pagination settings and what the source makes available. A small query or unavailable page can return fewer records, including none.

### Can I call it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~google-finance-indices-scraper/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### How do I choose an index for Google Finance Indices?

Submit supported ticker symbols or index codes in `indices`. Google Finance may not resolve every regional symbol, so use the code shown for the index.

## Related Scrappa Actors

- [Google Finance Historical Prices Scraper](https://apify.com/thescrappa/google-finance-historical-prices-scraper)
- [Google Finance Intraday Scraper](https://apify.com/thescrappa/google-finance-intraday-scraper)
- [Google Finance Markets Scraper](https://apify.com/thescrappa/google-finance-markets-scraper)
- [Google Finance Quote Scraper](https://apify.com/thescrappa/google-finance-quote-scraper)
- [Google Finance Search Scraper](https://apify.com/thescrappa/google-finance-search-scraper)
