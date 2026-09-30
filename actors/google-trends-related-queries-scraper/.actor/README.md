# Google Trends Related Queries Scraper for SEO

The Google Trends Related Queries Scraper for SEO collects related queries and topic interest data from Google Trends. Provide a search phrase or a short list of phrases; the actor saves source fields such as `position`, `result_kind`, `type`, and `query` to an Apify dataset.

## What data can you extract?

The dataset contains fields returned by Google Trends. The field names below match the Actor output schema. A source may leave optional values empty or omit fields when they are not available for a result.

| Field | Type | Description |
| --- | --- | --- |
| `position` | number | # returned for this result. |
| `result_kind` | text | Kind returned for this result. |
| `type` | text | Type returned for this result. |
| `query` | text | Query returned for this result. |
| `topic` | text | Topic returned for this result. |
| `topic_type` | text | Topic Type returned for this result. |
| `value` | number | Value returned for this result. |
| `formatted_value` | text | Formatted Value returned for this result. |
| `link` | link | Link returned for this result. |
| `source_keyword` | text | Source Keyword returned for this result. |
| `request_geo` | text | Location returned for this result. |
| `request_time_range` | text | Time Range returned for this result. |
| `request_hl` | text | Language returned for this result. |
| `request_search_type` | text | Search Type returned for this result. |
| `response_time_ms` | number | Response Time returned for this result. |

## Use cases

- Collect related queries and topic interest data to support SEO research.
- Compare results across search terms, websites, or markets.
- Export the dataset to a content, keyword, or reporting workflow.

## How to use

1. Open the **Input** tab and use the example JSON below.
2. Change the query, URL, identifier, or other fields you need. Set the lookup fields to match the query or identifier you want to collect.
3. Start the Actor. Open the run's default dataset to inspect, download, or export the returned records.

```json
{
  "query": "coffee"
}
```

Apify stores the run output in a dataset. You can download the dataset in JSON, CSV, Excel, XML, or other available formats from the run page.

## Output example

This illustrative record uses synthetic values. It shows the real output field names; optional source values may be null or absent.

```json
{
  "position": 42,
  "result_kind": "Example value",
  "type": "Example value",
  "query": "Example result",
  "topic": "Example value",
  "topic_type": "Example value",
  "value": 42
}
```

## Input fields

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `query` | string | Yes | Keyword or phrase to expand with Google Trends related queries. |
| `q` | string | No | Alias for query when reusing direct Scrappa API inputs. Apify users should fill Search Query. |
| `geo` | string | No | Geographic location code, such as US, GB, DE, or Worldwide. |
| `time_range` | string | No | Time period for related query discovery. Constraints: allowed values: 1h, 4h, 1d, 7d, 30d, 90d, 1y, 5y, all. |
| `hl` | string | No | Two-letter language code, such as en, de, es, or fr. |
| `search_type` | string | No | Google Trends vertical to analyze. Constraints: allowed values: web, images, news, youtube, shopping. |
| `include_autocomplete` | boolean | No | Also call the Google Trends autocomplete endpoint and include suggestions in OUTPUT. Dataset rows remain focused on related queries and topics. |

## Pricing

**Current live price:** $0.20 per 1,000 queries.

The price is based on the latest live Apify pricing entry. Per-result charges depend on the number of billed results returned. If the price line lists subscription tiers, the rate shown for each tier applies to that Apify subscription level.

## FAQ

### Is it legal to scrape this data?

This Actor is intended for data that is publicly available from Google Trends. You are responsible for following the source site's terms, privacy and copyright rules, and the laws that apply to your use of the data. Only collect information you have a lawful basis to use.

### What limits should I expect?

Use the field constraints and limits in the input table. The source controls which records are available, so a narrow query or unavailable page can return fewer results or none. Keep batch lists small when you need a quick first run.

### Can I run it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/actors/thescrappa~google-trends-related-queries-scraper/runs`. The response includes `defaultDatasetId`; use it to fetch the run's dataset items. See the [Apify Run Actor API](https://docs.apify.com/api/v2/actors-runs-post) and [Actor runs guide](https://docs.apify.com/api/v2/actors-actor-runs). Apify integrations for [Make](https://docs.apify.com/integrations/make), [Zapier](https://docs.apify.com/integrations/zapier), and [n8n](https://docs.apify.com/integrations/n8n) can trigger runs and pass results to the next workflow step. Send dataset rows to Google Sheets through a Sheets step or a workflow integration.

### What happens if a request fails?

Apify reports input validation and source request errors in the run details. Review the error, correct the input, and retry after a temporary source problem. Depending on when a request stops, the dataset may be empty or contain results collected before the failure.

## Related Scrappa Actors

- [Google Trends Autocomplete Scraper for SEO](https://apify.com/thescrappa/google-trends-autocomplete-scraper)
- [Google Trends Interest Scraper for SEO Research](https://apify.com/thescrappa/google-trends-interest-scraper)
- [Google Images Scraper for Creator Research](https://apify.com/thescrappa/google-images-scraper)
- [Google Maps Reviews Scraper for Local Reputation](https://apify.com/thescrappa/google-maps-reviews-scraper)
