# Google Finance Markets Scraper for Investors

The Google Finance Markets Scraper for Investors collects market movers and performance data from Google Finance. Provide a market trend view such as gainers or losers, or leave the default market overview; the actor saves source fields such as `item_type`, `section`, `trend`, and `trend_group` to an Apify dataset.

## What data can you extract?

The dataset contains fields returned by Google Finance. The field names below match the Actor output schema. A source may leave optional values empty or omit fields when they are not available for a result.

| Field | Type | Description |
| --- | --- | --- |
| `item_type` | text | Item Type returned for this result. |
| `section` | text | Section returned for this result. |
| `trend` | text | Trend returned for this result. |
| `trend_group` | text | Trend Group returned for this result. |
| `position` | number | Position returned for this result. |
| `stock` | text | Stock returned for this result. |
| `name` | text | Name returned for this result. |
| `symbol` | text | Symbol returned for this result. |
| `exchange` | text | Exchange returned for this result. |
| `price` | number | Price returned for this result. |
| `currency` | text | Currency returned for this result. |
| `price_movement_direction` | text | Direction returned for this result. |
| `price_movement_value` | number | Change returned for this result. |
| `price_movement_percentage` | number | % Change returned for this result. |
| `from_currency` | text | From returned for this result. |
| `to_currency` | text | To returned for this result. |
| `title` | text | News Title returned for this result. |
| `source` | text | News Source returned for this result. |
| `date` | text | News Date returned for this result. |
| `link` | link | Link returned for this result. |
| `snippet` | text | Snippet returned for this result. |
| `thumbnail` | image | Thumbnail returned for this result. |
| `request_trend` | text | Requested Trend returned for this result. |
| `request_index_market` | text | Requested Index Market returned for this result. |
| `request_hl` | text | Language returned for this result. |
| `request_gl` | text | Country returned for this result. |

## Use cases

- Collect market movers and performance data to support market research.
- Compare records across the input queries or entities you provide.
- Export structured results to research and reporting workflows.

## How to use

1. Open the **Input** tab and use the example JSON below.
2. Change the query, URL, identifier, or other fields you need. Set the lookup fields to match the query or identifier you want to collect.
3. Start the Actor. Open the run's default dataset to inspect, download, or export the returned records.

```json
{
  "trend": "gainers"
}
```

Apify stores the run output in a dataset. You can download the dataset in JSON, CSV, Excel, XML, or other available formats from the run page.

## Output example

This illustrative record uses synthetic values. It shows the real output field names; optional source values may be null or absent.

```json
{
  "item_type": "Example value",
  "section": "Example value",
  "trend": "Example value",
  "trend_group": "Example value",
  "position": 42,
  "stock": "Example value",
  "name": "Example value"
}
```

## Input fields

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `trend` | string | No | Optional Google Finance trend view. Leave as Market overview for the broad markets page. Constraints: allowed values: , gainers, losers, most-active, indexes, climate-leaders, cryptocurrencies, currencies. |
| `index_market` | string | No | Regional filter for the indexes trend. Leave as Default unless Trend is indexes. Stale values from other trends are ignored. Constraints: allowed values: , americas, europe-middle-east-africa, asia-pacific. |
| `hl` | string | No | Google Finance language code, such as en, de, es, or zh-cn. |
| `gl` | string | No | Two-letter country code, such as us, gb, de, or ca. |

## Pricing

**Current live price:** $0.20 per 1,000 results.

The price is based on the latest live Apify pricing entry. Per-result charges depend on the number of billed results returned. If the price line lists subscription tiers, the rate shown for each tier applies to that Apify subscription level.

## FAQ

### Is it legal to scrape this data?

This Actor is intended for data that is publicly available from Google Finance. You are responsible for following the source site's terms, privacy and copyright rules, and the laws that apply to your use of the data. Only collect information you have a lawful basis to use.

### What limits should I expect?

Use the field constraints and limits in the input table. The source controls which records are available, so a narrow query or unavailable page can return fewer results or none. Keep batch lists small when you need a quick first run.

### Can I run it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/actors/thescrappa~google-finance-markets-scraper/runs`. The response includes `defaultDatasetId`; use it to fetch the run's dataset items. See the [Apify Run Actor API](https://docs.apify.com/api/v2/actors-runs-post) and [Actor runs guide](https://docs.apify.com/api/v2/actors-actor-runs). Apify integrations for [Make](https://docs.apify.com/integrations/make), [Zapier](https://docs.apify.com/integrations/zapier), and [n8n](https://docs.apify.com/integrations/n8n) can trigger runs and pass results to the next workflow step. Send dataset rows to Google Sheets through a Sheets step or a workflow integration.

### What happens if a request fails?

Apify reports input validation and source request errors in the run details. Review the error, correct the input, and retry after a temporary source problem. Depending on when a request stops, the dataset may be empty or contain results collected before the failure.

## Related Scrappa Actors

- [Google Finance Historical Prices Scraper](https://apify.com/thescrappa/google-finance-historical-prices-scraper)
- [Google Finance Indices Scraper for Market Research](https://apify.com/thescrappa/google-finance-indices-scraper)
- [Google Finance Intraday Scraper for Research](https://apify.com/thescrappa/google-finance-intraday-scraper)
- [Google Finance Quote Scraper for Market Research](https://apify.com/thescrappa/google-finance-quote-scraper)
- [Google Finance Search Scraper for Market Research](https://apify.com/thescrappa/google-finance-search-scraper)
