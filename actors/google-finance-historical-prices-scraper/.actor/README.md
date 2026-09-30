# Google Finance Historical Prices Scraper

The Google Finance Historical Prices Scraper collects historical price points and trading dates from Google Finance. Provide one or more ticker symbols; the actor saves source fields such as `position`, `date`, `date_iso`, and `close` to an Apify dataset.

## What data can you extract?

The dataset contains fields returned by Google Finance. The field names below match the Actor output schema. A source may leave optional values empty or omit fields when they are not available for a result.

| Field | Type | Description |
| --- | --- | --- |
| `position` | number | # returned for this result. |
| `date` | number | Timestamp returned for this result. |
| `date_iso` | text | Date returned for this result. |
| `close` | number | Close returned for this result. |
| `change` | number | Change returned for this result. |
| `percent_change` | number | % Change returned for this result. |
| `volume` | number | Volume returned for this result. |
| `symbol` | text | Symbol returned for this result. |
| `exchange` | text | Exchange returned for this result. |
| `currency` | text | Currency returned for this result. |
| `previous_close` | number | Previous Close returned for this result. |
| `request_symbol` | text | Requested Symbol returned for this result. |
| `request_exchange` | text | Requested Exchange returned for this result. |
| `request_range` | number | Range returned for this result. |
| `request_start_date` | text | Start Date returned for this result. |
| `request_end_date` | text | End Date returned for this result. |
| `request_interval` | text | Interval returned for this result. |
| `request_hl` | text | Language returned for this result. |
| `request_gl` | text | Country returned for this result. |

## Use cases

- Collect historical price points and trading dates to support market research.
- Compare records across the input queries or entities you provide.
- Export structured results to research and reporting workflows.

## How to use

1. Open the **Input** tab and use the example JSON below.
2. Change the query, URL, identifier, or other fields you need. Set the lookup fields to match the query or identifier you want to collect.
3. Start the Actor. Open the run's default dataset to inspect, download, or export the returned records.

```json
{
  "symbol": "AAPL",
  "exchange": "NASDAQ",
  "range": 6
}
```

Apify stores the run output in a dataset. You can download the dataset in JSON, CSV, Excel, XML, or other available formats from the run page.

## Output example

This illustrative record uses synthetic values. It shows the real output field names; optional source values may be null or absent.

```json
{
  "position": 42,
  "date": "2026-09-30T10:00:00Z",
  "date_iso": "2026-09-30T10:00:00Z",
  "close": 42,
  "change": 42,
  "percent_change": 42,
  "volume": 42
}
```

## Input fields

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

## Pricing

**Current live price:** $0.20 per 1,000 price points.

The price is based on the latest live Apify pricing entry. Per-result charges depend on the number of billed results returned. If the price line lists subscription tiers, the rate shown for each tier applies to that Apify subscription level.

## FAQ

### Is it legal to scrape this data?

This Actor is intended for data that is publicly available from Google Finance. You are responsible for following the source site's terms, privacy and copyright rules, and the laws that apply to your use of the data. Only collect information you have a lawful basis to use.

### What limits should I expect?

Use the field constraints and limits in the input table. The source controls which records are available, so a narrow query or unavailable page can return fewer results or none. Keep batch lists small when you need a quick first run.

### Can I run it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/actors/thescrappa~google-finance-historical-prices-scraper/runs`. The response includes `defaultDatasetId`; use it to fetch the run's dataset items. See the [Apify Run Actor API](https://docs.apify.com/api/v2/actors-runs-post) and [Actor runs guide](https://docs.apify.com/api/v2/actors-actor-runs). Apify integrations for [Make](https://docs.apify.com/integrations/make), [Zapier](https://docs.apify.com/integrations/zapier), and [n8n](https://docs.apify.com/integrations/n8n) can trigger runs and pass results to the next workflow step. Send dataset rows to Google Sheets through a Sheets step or a workflow integration.

### What happens if a request fails?

Apify reports input validation and source request errors in the run details. Review the error, correct the input, and retry after a temporary source problem. Depending on when a request stops, the dataset may be empty or contain results collected before the failure.

## Related Scrappa Actors

- [Google Finance Indices Scraper for Market Research](https://apify.com/thescrappa/google-finance-indices-scraper)
- [Google Finance Intraday Scraper for Research](https://apify.com/thescrappa/google-finance-intraday-scraper)
- [Google Finance Markets Scraper for Investors](https://apify.com/thescrappa/google-finance-markets-scraper)
- [Google Finance Quote Scraper for Market Research](https://apify.com/thescrappa/google-finance-quote-scraper)
- [Google Finance Search Scraper for Market Research](https://apify.com/thescrappa/google-finance-search-scraper)
