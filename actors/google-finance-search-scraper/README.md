# Google Finance Search Scraper

Find matching financial instruments on Google Finance by company name or ticker. Look up companies by name or ticker and provide several queries when building a watchlist.

## What data can you extract?

Company names and tickers reflect matching financial instruments listed by Google Finance.

| Field | Type | Description |
| --- | --- | --- |
| `query` | text | Query shown for the matching financial instrument by Google Finance, in the format used by the source; null when it is omitted. |
| `position` | number | Result position in the Google Finance matching financial instrument list, as a whole number; null when the source does not supply one. |
| `name` | text | Name of the matching financial instrument, as shown by Google Finance; null when no name is published. |
| `symbol` | text | Ticker symbol shown for the matching financial instrument by Google Finance, in the format used by the source; null when it is omitted. |
| `exchange` | text | Exchange code shown for the matching financial instrument by Google Finance; null when Google Finance does not provide the value. |
| `stock` | text | Stock identifier returned by Google Finance for the matching instrument; it may include an exchange, such as `AAPL:NASDAQ`, and is null when the source omits it. |
| `type` | text | Category assigned to the matching financial instrument by Google Finance; null when Google Finance does not provide the value. |
| `currency` | text | Currency code for this matching financial instrument, formatted as Google Finance displays it, including the currency when shown; null when unavailable. |
| `price` | number | Listed price for this matching financial instrument, as a numeric amount in the listing currency; null when Google Finance provides no price. |
| `price_change` | number | Price change for this matching financial instrument, as a numeric change in the record currency; null when Google Finance provides no price. |
| `percent_change` | number | Percentage price change reported by Google Finance, as a percentage or share in the source format; null when no estimate is available. |
| `link` | link | Result link for this matching financial instrument on Google Finance; null when the source does not provide a URL. |
| `google_finance_url` | link | Google finance url for this matching financial instrument on Google Finance; null when the source does not provide a URL. |
| `market` | text | Market shown for the matching financial instrument by Google Finance; null when Google Finance does not provide the value. |
| `request_hl` | text | Interface language code passed to Google Finance; Use a language code such as en or de. This input value is copied into the output row; null when it was not supplied. |
| `request_gl` | text | Two-letter country or region code passed to Google Finance; Use a two-letter country code such as us or de. This input value is copied into the output row; null when it was not supplied. |
| `raw_result` | object | Financial instrument fields supplied by Google Finance, such as ticker, exchange and quote details; null when the source provides no details. |

## Use cases

- Investors can compare price movement and market values for tickers they follow.
- Analysts can chart quotes or historical points beside the symbol and exchange.
- Finance teams can refresh market data in recurring spreadsheet reports.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Put the supported targets in `queries` and use the identifier or URL format required by Google Finance.
3. Start the run and open its default dataset to inspect or download the rows.

```json
{
  "q": "AAPL",
  "queries": [
    "AAPL"
  ]
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `q` | string | Conditional | Single ticker, company, ETF, index, fund, or finance instrument query. Required when `queries` is omitted; ignored when `queries` is provided. |
| `queries` | array of string | Conditional | Batch of up to 25 Google Finance search queries processed in one Actor run. Required unless `q` is supplied. One dataset item is written per matched result. Constraints: maximum 25 items. |
| `hl` | string | No | Google Finance language code, such as en, de, es, or zh-cn. |
| `gl` | string | No | Country code, such as us, gb, de, or ca. |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "name": "Northstar Market Labs",
  "symbol": "NML",
  "exchange": "NASDAQ",
  "type": "Video",
  "raw_result": {
    "title": "Northstar Market Labs",
    "link": "https://www.google.com/finance/quote/NML:NASDAQ",
    "type": "stock",
    "price": "$198.53"
  },
  "price": 198.53,
  "link": "https://search.example.com/results/market-guide",
  "query": "weekend markets in Seattle"
}
```

## Pricing

**Current live price:** $0.20 per 1,000 results.

Each saved search-result row counts as one result. A search that saves no rows has no result charge.

## FAQ

### Is it legal to collect public information?

This Actor is for information visible on the public source pages it reads. You are responsible for checking the source terms, privacy and copyright rules, and the laws that apply to your use.

### How many records will a run return?

The result count depends on the input limits, pagination settings and what the source makes available. A small query or unavailable page can return fewer records, including none.

### Can I call it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~google-finance-search-scraper/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### Can Google Finance Search match a company name as well as a ticker?

Yes. Enter a company name or ticker in `q`, or submit several queries through `queries`. The results show the matching instruments Google Finance returns.

## Related Scrappa Actors

- [Google Finance Historical Prices Scraper](https://apify.com/thescrappa/google-finance-historical-prices-scraper)
- [Google Finance Indices Scraper](https://apify.com/thescrappa/google-finance-indices-scraper)
- [Google Finance Intraday Scraper](https://apify.com/thescrappa/google-finance-intraday-scraper)
- [Google Finance Markets Scraper](https://apify.com/thescrappa/google-finance-markets-scraper)
- [Google Finance Quote Scraper](https://apify.com/thescrappa/google-finance-quote-scraper)
