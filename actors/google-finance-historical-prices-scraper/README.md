# Google Finance Historical Prices Scraper

Track historical Google Finance prices with dates, closing values and daily changes. Choose a ticker, exchange and supported date range to review past daily or interval prices.

## What data can you extract?

Prices and percentage changes follow the currency and units Google Finance displays for each instrument.

| Field | Type | Description |
| --- | --- | --- |
| `position` | number | Result position in the Google Finance historical market-price point list, as a whole number; null when the source does not supply one. |
| `date` | number | Trading date for this Google Finance price point, in YYYY-MM-DD format; null when no date is available. |
| `date_iso` | text | Trading calendar date converted to YYYY-MM-DD from the Google Finance timestamp; null when the timestamp cannot be converted. |
| `close` | number | Closing price for this historical market-price point, as a numeric closing amount in the record currency; null when Google Finance provides no price. |
| `change` | number | Price change for this historical market-price point, as a numeric change in the record currency; null when Google Finance provides no price. |
| `percent_change` | number | Percentage price change reported by Google Finance, as a percentage or share in the source format; null when no estimate is available. |
| `volume` | number | Trading or search volume shown for the historical market-price point by Google Finance, in the format used by the source; null when it is omitted. |
| `symbol` | text | Ticker symbol shown for the historical market-price point by Google Finance, in the format used by the source; null when it is omitted. |
| `exchange` | text | Exchange code shown for the historical market-price point by Google Finance; null when Google Finance does not provide the value. |
| `currency` | text | Currency code for this historical market-price point, formatted as Google Finance displays it, including the currency when shown; null when unavailable. |
| `previous_close` | number | Previous close for this historical market-price point, as a numeric previous closing amount in the record currency; null when Google Finance provides no price. |
| `request_symbol` | text | Ticker symbol passed to Google Finance. This input value is copied into the output row; null when it was not supplied. |
| `request_exchange` | text | Stock exchange code passed to Google Finance. This input value is copied into the output row; null when it was not supplied. |
| `request_range` | number | Historical date range passed to Google Finance. This input value is copied into the output row; null when it was not supplied. |
| `request_start_date` | text | Range start date passed to Google Finance; Use YYYY-MM-DD when a calendar date is required. This input value is copied into the output row; null when it was not supplied. |
| `request_end_date` | text | Range end date passed to Google Finance; Use YYYY-MM-DD when a calendar date is required. This input value is copied into the output row; null when it was not supplied. |
| `request_interval` | text | Price interval passed to Google Finance. This input value is copied into the output row; null when it was not supplied. |
| `request_hl` | text | Interface language code passed to Google Finance; Use a language code such as en or de. This input value is copied into the output row; null when it was not supplied. |
| `request_gl` | text | Two-letter country or region code passed to Google Finance; Use a two-letter country code such as us or de. This input value is copied into the output row; null when it was not supplied. |

## Use cases

- Investors can compare price movement and market values for tickers they follow.
- Analysts can chart quotes or historical points beside the symbol and exchange.
- Finance teams can refresh market data in recurring spreadsheet reports.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Choose a ticker, exchange and supported date range or interval to control the price history.
3. Start the run and open its default dataset to inspect or download the rows.

```json
{
  "symbol": "AAPL",
  "exchange": "NASDAQ",
  "range": 6
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `symbol` | string | Yes | Ticker symbol to fetch, such as AAPL, MSFT, TSLA, or VOO. |
| `exchange` | string | No | Google Finance exchange code, such as NASDAQ, NYSE, NSE, LON, or TSE. Recommended to avoid symbol ambiguity. |
| `range` | integer | No | Preset time range. Recommended and most stable. Use either this or both start_date and end_date. Constraints: minimum 1; maximum 8. |
| `start_date` | string | No | Custom range start date in YYYY-MM-DD format. Must be used with end_date and without range. Scrappa may return no data for custom ranges; preset range is more stable. |
| `end_date` | string | No | Custom range end date in YYYY-MM-DD format. Must be used with start_date and without range. Scrappa may return no data for custom ranges; preset range is more stable. |
| `interval` | string | No | Historical data interval. Constraints: allowed values: daily, weekly, monthly. |
| `hl` | string | No | Google Finance language code, such as en, de, es, or zh-cn. |
| `gl` | string | No | Country code, such as us, gb, de, or ca. |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "date": 1790294400000,
  "date_iso": "2026-09-25",
  "close": 195.62,
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

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~google-finance-historical-prices-scraper/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### Which dates define the Google Finance price history?

Set `symbol` and `exchange`, then provide the supported `range` or `start_date` and `end_date`. Use `interval` to choose the price-point spacing listed in Input.

## Related Scrappa Actors

- [Google Finance Indices Scraper](https://apify.com/thescrappa/google-finance-indices-scraper)
- [Google Finance Intraday Scraper](https://apify.com/thescrappa/google-finance-intraday-scraper)
- [Google Finance Markets Scraper](https://apify.com/thescrappa/google-finance-markets-scraper)
- [Google Finance Quote Scraper](https://apify.com/thescrappa/google-finance-quote-scraper)
- [Google Finance Search Scraper](https://apify.com/thescrappa/google-finance-search-scraper)
