# Google Finance Markets Scraper

Review Google Finance market movers by section, trend and price movement. Choose the market trend and index view that match the movers you want to review.

## What data can you extract?

Prices and percentage changes follow the currency and units Google Finance displays for each instrument.

| Field | Type | Description |
| --- | --- | --- |
| `item_type` | text | Category assigned to the market mover by Google Finance; null when Google Finance does not provide the value. |
| `section` | text | Market section shown for the market mover by Google Finance; null when Google Finance does not provide the value. |
| `trend` | text | Market trend shown for the market mover by Google Finance; null when Google Finance does not provide the value. |
| `trend_group` | text | Market trend group shown for the market mover by Google Finance; null when Google Finance does not provide the value. |
| `position` | number | Result position in the Google Finance market mover list, as a whole number; null when the source does not supply one. |
| `stock` | text | Ticker symbol shown for the market mover by Google Finance, in the format used by the source; null when it is omitted. |
| `name` | text | Name of the market mover, as shown by Google Finance; null when no name is published. |
| `symbol` | text | Ticker symbol shown for the market mover by Google Finance, in the format used by the source; null when it is omitted. |
| `exchange` | text | Exchange code shown for the market mover by Google Finance; null when Google Finance does not provide the value. |
| `price` | number | Listed price for this market mover, as a numeric amount in the listing currency; null when Google Finance provides no price. |
| `currency` | text | Currency code for this market mover, formatted as Google Finance displays it, including the currency when shown; null when unavailable. |
| `price_movement_direction` | text | Price movement direction shown for the market mover by Google Finance; null when Google Finance does not provide the value. |
| `price_movement_value` | number | Price movement value shown for the market mover by Google Finance, in the format used by the source; null when it is omitted. |
| `price_movement_percentage` | number | Price movement percentage reported by Google Finance, as a percentage or share in the source format; null when no estimate is available. |
| `from_currency` | text | Three-letter currency code, such as USD or EUR; null when Google Finance does not provide the value. |
| `to_currency` | text | Three-letter currency code, such as USD or EUR; null when Google Finance does not provide the value. |
| `title` | text | Title of the market mover, as shown by Google Finance; null when no title is published. |
| `source` | text | Source or language label shown for the market mover by Google Finance, in the format used by the source; null when it is omitted. |
| `date` | text | Date shown for the market mover shown by Google Finance, in YYYY-MM-DD when the source provides a calendar date; null if the source omits the date. |
| `link` | link | Result link for this market mover on Google Finance; null when the source does not provide a URL. |
| `snippet` | text | Search snippet from Google Finance for this market mover; null when the source has no text to show. |
| `thumbnail` | image | Thumbnail url for this market mover on Google Finance; null when the source does not provide a URL. |
| `request_trend` | text | Market trend passed to Google Finance. This input value is copied into the output row; null when it was not supplied. |
| `request_index_market` | text | Market index code passed to Google Finance. This input value is copied into the output row; null when it was not supplied. |
| `request_hl` | text | Interface language code passed to Google Finance; Use a language code such as en or de. This input value is copied into the output row; null when it was not supplied. |
| `request_gl` | text | Two-letter country or region code passed to Google Finance; Use a two-letter country code such as us or de. This input value is copied into the output row; null when it was not supplied. |

## Use cases

- Investors can compare price movement and market values for tickers they follow.
- Analysts can chart quotes or historical points beside the symbol and exchange.
- Finance teams can refresh market data in recurring spreadsheet reports.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Set `trend` to the search term or source identifier you want to look up, then use the optional filters listed below.
3. Start the run and open its default dataset to inspect or download the rows.

```json
{
  "trend": "gainers"
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `trend` | string | No | Optional Google Finance trend view. Leave as Market overview for the broad markets page. Constraints: allowed values: , gainers, losers, most-active, indexes, climate-leaders, cryptocurrencies, currencies. |
| `index_market` | string | No | Regional filter for the indexes trend. Leave as Default unless Trend is indexes. Stale values from other trends are ignored. Constraints: allowed values: , americas, europe-middle-east-africa, asia-pacific. |
| `hl` | string | No | Google Finance language code, such as en, de, es, or zh-cn. |
| `gl` | string | No | Two-letter country code, such as us, gb, de, or ca. |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "title": "Northstar Market Labs quarterly results",
  "name": "Northstar Market Labs",
  "price": 198.53,
  "date": "1790294400000",
  "link": "https://search.example.com/results/market-guide",
  "item_type": "Market mover",
  "section": "most_active",
  "trend": "top_gainers"
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

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~google-finance-markets-scraper/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### How do I select a Google Finance market view?

Choose a supported `trend` and, when relevant, an `index_market`. The returned movers reflect that view and the locale supplied through the input.

## Related Scrappa Actors

- [Google Finance Historical Prices Scraper](https://apify.com/thescrappa/google-finance-historical-prices-scraper)
- [Google Finance Indices Scraper](https://apify.com/thescrappa/google-finance-indices-scraper)
- [Google Finance Intraday Scraper](https://apify.com/thescrappa/google-finance-intraday-scraper)
- [Google Finance Quote Scraper](https://apify.com/thescrappa/google-finance-quote-scraper)
- [Google Finance Search Scraper](https://apify.com/thescrappa/google-finance-search-scraper)
