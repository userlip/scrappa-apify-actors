# Google Finance Quote Scraper

Look up Google Finance share prices with ticker, exchange and percentage change. Enter the ticker and exchange, then request period statistics only when they are useful for your quote.

## What data can you extract?

Prices and percentage changes follow the currency and units Google Finance displays for each instrument.

| Field | Type | Description |
| --- | --- | --- |
| `symbol` | text | Ticker symbol shown for the stock quote by Google Finance, in the format used by the source; null when it is omitted. |
| `exchange` | text | Exchange code shown for the stock quote by Google Finance; null when Google Finance does not provide the value. |
| `name` | text | Name of the stock quote, as shown by Google Finance; null when no name is published. |
| `current_price` | number | Current quote for this stock quote, as a numeric quote in the record currency; null when Google Finance provides no price. |
| `currency` | text | Currency code for this stock quote, formatted as Google Finance displays it, including the currency when shown; null when unavailable. |
| `price_change` | number | Price change for this stock quote, as a numeric change in the record currency; null when Google Finance provides no price. |
| `percent_change` | number | Percentage price change reported by Google Finance, as a percentage or share in the source format; null when no estimate is available. |
| `market_status` | text | Status reported for the stock quote by Google Finance; null when Google Finance does not provide the value. |
| `key_stats` | object | Financial statistics with source labels such as market capitalization, volume and currency from Google Finance; null when the source provides no details. |
| `about` | object | Company statistics with market capitalization, average volume, exchange and related financial labels from Google Finance; null when the source provides no details. |
| `financials` | array of objects | Company financial reports with report title, period, value and currency from Google Finance; an empty list when no entries are available. |
| `news` | array of objects | Related finance articles with headline, publisher link and publication date from Google Finance; an empty list when no entries are available. |
| `discover_more` | array of objects | Related Google Finance stories with headline, source and publication time; an empty list when no entries are available. |
| `related_tickers` | array of objects | Related financial instruments with ticker symbol and display name from Google Finance; an empty list when no entries are available. |
| `pagination` | object | Pagination details with next-page token and whether another page is available from Google Finance; null when the source provides no details. |
| `request_symbol` | text | Ticker symbol passed to Google Finance. This input value is copied into the output row; null when it was not supplied. |
| `request_exchange` | text | Stock exchange code passed to Google Finance. This input value is copied into the output row; null when it was not supplied. |
| `request_period_type` | text | Financial reporting period passed to Google Finance. This input value is copied into the output row; null when it was not supplied. |
| `request_hl` | text | Interface language code passed to Google Finance; Use a language code such as en or de. This input value is copied into the output row; null when it was not supplied. |
| `request_gl` | text | Two-letter country or region code passed to Google Finance; Use a two-letter country code such as us or de. This input value is copied into the output row; null when it was not supplied. |
| `upstream_fallback` | object | Fallback details with the reason, omitted request parameters and primary error message; null when the first quote request succeeds. |

## Use cases

- Investors can compare price movement and market values for tickers they follow.
- Analysts can chart quotes or historical points beside the symbol and exchange.
- Finance teams can refresh market data in recurring spreadsheet reports.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Set `symbol` to the search term or source identifier you want to look up, then use the optional filters listed below.
3. Start the run and open its default dataset to inspect or download the rows.

```json
{
  "symbol": "AAPL",
  "exchange": "NASDAQ"
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `symbol` | string | Yes | Ticker symbol to fetch, such as AAPL, MSFT, TSLA, or VOO. |
| `exchange` | string | No | Google Finance exchange code, such as NASDAQ, NYSE, NSE, LON, or TSE. Recommended to avoid symbol ambiguity. |
| `period_type` | string | No | Financial statement period filter. Constraints: allowed values: quarterly, annual. |
| `hl` | string | No | Google Finance language code, such as en, de, es, or zh-cn. |
| `gl` | string | No | Two-letter country code, such as us, gb, de, or ca. |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "symbol": "NML",
  "name": "Northstar Market Labs",
  "current_price": 198.53,
  "currency": "USD",
  "price_change": 2.91,
  "percent_change": 1.49,
  "key_stats": {
    "Market cap": "$2.96B",
    "Average volume": "1.24M",
    "Currency": "USD"
  },
  "about": {
    "description": "Northstar Market Labs builds inventory planning software for independent retailers.",
    "founded": "2016",
    "headquarters": "Seattle, WA"
  }
}
```

## Pricing

**Current live price:** $0.20 per 1,000 results.

Each saved dataset record counts as one result.

## FAQ

### Is it legal to collect public information?

This Actor is for information visible on the public source pages it reads. You are responsible for checking the source terms, privacy and copyright rules, and the laws that apply to your use.

### How many records will a run return?

The result count depends on the input limits, pagination settings and what the source makes available. A small query or unavailable page can return fewer records, including none.

### Can I call it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~google-finance-quote-scraper/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### Why does a Google Finance quote omit period statistics?

Provide the supported `symbol` and `exchange`, then select `period_type` only when you need financial-period data. A fallback quote can omit that optional period request.

## Related Scrappa Actors

- [Google Finance Historical Prices Scraper](https://apify.com/thescrappa/google-finance-historical-prices-scraper)
- [Google Finance Indices Scraper](https://apify.com/thescrappa/google-finance-indices-scraper)
- [Google Finance Intraday Scraper](https://apify.com/thescrappa/google-finance-intraday-scraper)
- [Google Finance Markets Scraper](https://apify.com/thescrappa/google-finance-markets-scraper)
- [Google Finance Search Scraper](https://apify.com/thescrappa/google-finance-search-scraper)
