# Google Finance Intraday Scraper

Check Google Finance intraday prices with timestamps, price changes and market volume. Submit one or more tickers to inspect intraday price points available from Google Finance.

## What data can you extract?

Prices and percentage changes follow the currency and units Google Finance displays for each instrument.

| Field | Type | Description |
| --- | --- | --- |
| `position` | number | Result position in the Google Finance intraday market-price point list, as a whole number; null when the source does not supply one. |
| `date` | text | Date for this Google Finance intraday quote, in the source date format; null when no date is returned. |
| `date_iso` | text | Parsed UTC timestamp for the point, formatted as ISO 8601 with milliseconds, for example `2025-06-16T13:30:00.000Z`; null when the source date cannot be parsed. |
| `price` | number | Listed price for this intraday market-price point, as a numeric amount in the listing currency; null when Google Finance provides no price. |
| `change` | number | Price change for this intraday market-price point, as a numeric change in the record currency; null when Google Finance provides no price. |
| `percent_change` | number | Percentage price change reported by Google Finance, as a percentage or share in the source format; null when no estimate is available. |
| `volume` | number | Trading or search volume shown for the intraday market-price point by Google Finance, in the format used by the source; null when it is omitted. |
| `symbol` | text | Ticker symbol shown for the intraday market-price point by Google Finance, in the format used by the source; null when it is omitted. |
| `exchange` | text | Exchange code shown for the intraday market-price point by Google Finance; null when Google Finance does not provide the value. |
| `currency` | text | Currency code for this intraday market-price point, formatted as Google Finance displays it, including the currency when shown; null when unavailable. |
| `request_symbol` | text | Ticker symbol passed to Google Finance. This input value is copied into the output row; null when it was not supplied. |
| `request_exchange` | text | Stock exchange code passed to Google Finance. This input value is copied into the output row; null when it was not supplied. |
| `request_hl` | text | Interface language code passed to Google Finance; Use a language code such as en or de. This input value is copied into the output row; null when it was not supplied. |
| `request_gl` | text | Two-letter country or region code passed to Google Finance; Use a two-letter country code such as us or de. This input value is copied into the output row; null when it was not supplied. |

## Use cases

- Investors can compare price movement and market values for tickers they follow.
- Analysts can chart quotes or historical points beside the symbol and exchange.
- Finance teams can refresh market data in recurring spreadsheet reports.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Enter one or more ticker symbols, then choose the language and country for the quote page.
3. Start the run and open its default dataset to inspect or download the rows.

```json
{
  "symbols": [
    {
      "symbol": "AAPL",
      "exchange": "NASDAQ"
    }
  ]
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `symbols` | array of object | Yes | Ticker symbols to process in one Apify run. Batching avoids one run per ticker and keeps Apify overhead low. Constraints: minimum 1 items. |
| `hl` | string | No | Google Finance language code, such as en, de, es, or zh-cn. |
| `gl` | string | No | Two-letter country code, such as us, gb, de, or ca. |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "price": 198.53,
  "date": "Jun 16 2025, 09:30 AM UTC-04:00",
  "date_iso": "2025-06-16T13:30:00.000Z",
  "change": 1.74,
  "percent_change": 1.49,
  "volume": 184500,
  "symbol": "NML",
  "exchange": "NASDAQ"
}
```

## Pricing

**Current live price:** $0.20 per 1,000 price points.

Each saved price point counts as one result.

## FAQ

### Is it legal to collect public information?

This Actor is for information visible on the public source pages it reads. You are responsible for checking the source terms, privacy and copyright rules, and the laws that apply to your use.

### How many records will a run return?

The result count depends on the input limits, pagination settings and what the source makes available. A small query or unavailable page can return fewer records, including none.

### Can I call it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~google-finance-intraday-scraper/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### Can Google Finance Intraday look up more than one ticker?

Yes. Provide ticker symbols in `symbols`. Intraday points depend on the selected market and the data Google Finance makes available for each symbol.

## Related Scrappa Actors

- [Google Finance Historical Prices Scraper](https://apify.com/thescrappa/google-finance-historical-prices-scraper)
- [Google Finance Indices Scraper](https://apify.com/thescrappa/google-finance-indices-scraper)
- [Google Finance Markets Scraper](https://apify.com/thescrappa/google-finance-markets-scraper)
- [Google Finance Quote Scraper](https://apify.com/thescrappa/google-finance-quote-scraper)
- [Google Finance Search Scraper](https://apify.com/thescrappa/google-finance-search-scraper)
