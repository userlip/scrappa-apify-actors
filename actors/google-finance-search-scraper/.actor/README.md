# Google Finance Search Scraper for Market Research

The Google Finance Search Scraper for Market Research collects matching ticker and company records from Google Finance. Provide a search phrase or a short list of phrases; the actor saves source fields such as `query`, `position`, `name`, and `symbol` to an Apify dataset.

## What data can you extract?

The dataset contains fields returned by Google Finance. The field names below match the Actor output schema. A source may leave optional values empty or omit fields when they are not available for a result.

| Field | Type | Description |
| --- | --- | --- |
| `query` | text | Query returned for this result. |
| `position` | number | Position returned for this result. |
| `name` | text | Name returned for this result. |
| `symbol` | text | Symbol returned for this result. |
| `exchange` | text | Exchange returned for this result. |
| `stock` | text | Stock returned for this result. |
| `type` | text | Type returned for this result. |
| `currency` | text | Currency returned for this result. |
| `price` | number | Price returned for this result. |
| `price_change` | number | Change returned for this result. |
| `percent_change` | number | % Change returned for this result. |
| `link` | link | Link returned for this result. |
| `google_finance_url` | link | Google Finance URL returned for this result. |
| `market` | text | Market returned for this result. |
| `request_hl` | text | Language returned for this result. |
| `request_gl` | text | Country returned for this result. |
| `raw_result` | object | Raw Result returned for this result. |

## Use cases

- Collect matching ticker and company records to support market research.
- Compare records across the input queries or entities you provide.
- Export structured results to research and reporting workflows.

## How to use

1. Open the **Input** tab and use the example JSON below.
2. Change the query, URL, identifier, or other fields you need. For multi-target work, use `queries` and start with a short list.
3. Start the Actor. Open the run's default dataset to inspect, download, or export the returned records.

```json
{
  "q": "AAPL",
  "queries": [
    "AAPL"
  ]
}
```

Apify stores the run output in a dataset. You can download the dataset in JSON, CSV, Excel, XML, or other available formats from the run page.

## Output example

This illustrative record uses synthetic values. It shows the real output field names; optional source values may be null or absent.

```json
{
  "query": "Example result",
  "position": 42,
  "name": "Example value",
  "symbol": "AAPL",
  "exchange": "Example value",
  "stock": "Example value",
  "type": "Example value"
}
```

## Input fields

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `q` | string | No | Single ticker, company, ETF, index, fund, or finance instrument search query. Ignored when queries is provided. |
| `queries` | array of string | No | Batch of up to 25 Google Finance search queries processed in one Actor run. One dataset item is written per matched result. Constraints: maximum 25 items. |
| `hl` | string | No | Google Finance language code, such as en, de, es, or zh-cn. |
| `gl` | string | No | Country code, such as us, gb, de, or ca. |

## Pricing

**Current live price:** $0.20 per 1,000 searches.

The price is based on the latest live Apify pricing entry. Per-result charges depend on the number of billed results returned. If the price line lists subscription tiers, the rate shown for each tier applies to that Apify subscription level.

## FAQ

### Is it legal to scrape this data?

This Actor is intended for data that is publicly available from Google Finance. You are responsible for following the source site's terms, privacy and copyright rules, and the laws that apply to your use of the data. Only collect information you have a lawful basis to use.

### What limits should I expect?

Use the field constraints and limits in the input table. The source controls which records are available, so a narrow query or unavailable page can return fewer results or none. Keep batch lists small when you need a quick first run.

### Can I run it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/actors/thescrappa~google-finance-search-scraper/runs`. The response includes `defaultDatasetId`; use it to fetch the run's dataset items. See the [Apify Run Actor API](https://docs.apify.com/api/v2/actors-runs-post) and [Actor runs guide](https://docs.apify.com/api/v2/actors-actor-runs). Apify integrations for [Make](https://docs.apify.com/integrations/make), [Zapier](https://docs.apify.com/integrations/zapier), and [n8n](https://docs.apify.com/integrations/n8n) can trigger runs and pass results to the next workflow step. Send dataset rows to Google Sheets through a Sheets step or a workflow integration.

### What happens if a request fails?

Apify reports input validation and source request errors in the run details. Review the error, correct the input, and retry after a temporary source problem. Depending on when a request stops, the dataset may be empty or contain results collected before the failure.

## Related Scrappa Actors

- [Google Finance Historical Prices Scraper](https://apify.com/thescrappa/google-finance-historical-prices-scraper)
- [Google Finance Indices Scraper for Market Research](https://apify.com/thescrappa/google-finance-indices-scraper)
- [Google Finance Intraday Scraper for Research](https://apify.com/thescrappa/google-finance-intraday-scraper)
- [Google Finance Markets Scraper for Investors](https://apify.com/thescrappa/google-finance-markets-scraper)
- [Google Finance Quote Scraper for Market Research](https://apify.com/thescrappa/google-finance-quote-scraper)
