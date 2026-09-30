# Google Finance Indices Scraper for Market Research

The Google Finance Indices Scraper for Market Research collects market index data and price changes from Google Finance. Provide one or more Google Finance index symbols; the actor saves source fields such as `id`, `requested_symbol`, `symbol`, and `name` to an Apify dataset.

## What data can you extract?

The dataset contains fields returned by Google Finance. The field names below match the Actor output schema. A source may leave optional values empty or omit fields when they are not available for a result.

| Field | Type | Description |
| --- | --- | --- |
| `id` | text | id returned for this result. |
| `requested_symbol` | text | requested symbol returned for this result. |
| `symbol` | text | symbol returned for this result. |
| `name` | text | name returned for this result. |
| `exchange` | text | exchange returned for this result. |
| `current_price` | number | current price returned for this result. |
| `price_change` | number | price change returned for this result. |
| `percent_change` | number | percent change returned for this result. |
| `previous_close` | number | previous close returned for this result. |
| `movement_direction` | text | movement direction returned for this result. |
| `request_hl` | text | request hl returned for this result. |
| `request_gl` | text | request gl returned for this result. |
| `retrieved_at` | date | retrieved at returned for this result. |

## Use cases

- Collect market index data and price changes to support market research.
- Compare records across the input queries or entities you provide.
- Export structured results to research and reporting workflows.

## How to use

1. Open the **Input** tab and use the example JSON below.
2. Change the query, URL, identifier, or other fields you need. For multi-target work, use `indices` and start with a short list.
3. Start the Actor. Open the run's default dataset to inspect, download, or export the returned records.

```json
{
  "indices": [
    ".INX"
  ]
}
```

Apify stores the run output in a dataset. You can download the dataset in JSON, CSV, Excel, XML, or other available formats from the run page.

## Output example

This illustrative record uses synthetic values. It shows the real output field names; optional source values may be null or absent.

```json
{
  "id": "INDEXSP:.INX",
  "requested_symbol": ".INX",
  "symbol": ".INX",
  "name": "S&P 500",
  "exchange": "indexsp",
  "current_price": 6200.5,
  "price_change": -12.25,
  "percent_change": -0.2,
  "previous_close": 6212.75,
  "movement_direction": "DOWN",
  "request_hl": "en",
  "request_gl": "us",
  "retrieved_at": "2026-09-30T10:00:00Z"
}
```

## Input fields

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `indices` | string/array | No | A comma-separated list or JSON array of up to 3 symbols, for example .INX, .DJI, .IXIC. |
| `hl` | string | No | Google Finance language code, such as en or de. |
| `gl` | string | No | Two-letter Google Finance country code, such as us or de. |

## Pricing

**Current live price:** $0.25 per 1,000 results.

The price is based on the latest live Apify pricing entry. Per-result charges depend on the number of billed results returned. If the price line lists subscription tiers, the rate shown for each tier applies to that Apify subscription level.

## FAQ

### Is it legal to scrape this data?

This Actor is intended for data that is publicly available from Google Finance. You are responsible for following the source site's terms, privacy and copyright rules, and the laws that apply to your use of the data. Only collect information you have a lawful basis to use.

### What limits should I expect?

Use the field constraints and limits in the input table. The source controls which records are available, so a narrow query or unavailable page can return fewer results or none. Keep batch lists small when you need a quick first run.

### Can I run it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/actors/thescrappa~google-finance-indices-scraper/runs`. The response includes `defaultDatasetId`; use it to fetch the run's dataset items. See the [Apify Run Actor API](https://docs.apify.com/api/v2/actors-runs-post) and [Actor runs guide](https://docs.apify.com/api/v2/actors-actor-runs). Apify integrations for [Make](https://docs.apify.com/integrations/make), [Zapier](https://docs.apify.com/integrations/zapier), and [n8n](https://docs.apify.com/integrations/n8n) can trigger runs and pass results to the next workflow step. Send dataset rows to Google Sheets through a Sheets step or a workflow integration.

### What happens if a request fails?

Apify reports input validation and source request errors in the run details. Review the error, correct the input, and retry after a temporary source problem. Depending on when a request stops, the dataset may be empty or contain results collected before the failure.

## Related Scrappa Actors

- [Google Finance Historical Prices Scraper](https://apify.com/thescrappa/google-finance-historical-prices-scraper)
- [Google Finance Intraday Scraper for Research](https://apify.com/thescrappa/google-finance-intraday-scraper)
- [Google Finance Markets Scraper for Investors](https://apify.com/thescrappa/google-finance-markets-scraper)
- [Google Finance Quote Scraper for Market Research](https://apify.com/thescrappa/google-finance-quote-scraper)
- [Google Finance Search Scraper for Market Research](https://apify.com/thescrappa/google-finance-search-scraper)
