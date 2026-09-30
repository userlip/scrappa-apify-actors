# Google Trends Autocomplete Scraper for SEO

The Google Trends Autocomplete Scraper for SEO collects search suggestions and related terms from Google Trends. Provide a search phrase or a short list of phrases; the actor saves source fields such as `position`, `suggestion`, `type`, and `source_keyword` to an Apify dataset.

## What data can you extract?

The dataset contains fields returned by Google Trends. The field names below match the Actor output schema. A source may leave optional values empty or omit fields when they are not available for a result.

| Field | Type | Description |
| --- | --- | --- |
| `position` | number | # returned for this result. |
| `suggestion` | text | Suggestion returned for this result. |
| `type` | text | Type returned for this result. |
| `source_keyword` | text | Source Keyword returned for this result. |
| `request_geo` | text | Location returned for this result. |
| `request_hl` | text | Language returned for this result. |
| `response_time_ms` | number | Response Time returned for this result. |

## Use cases

- Collect search suggestions and related terms to support SEO research.
- Compare results across search terms, websites, or markets.
- Export the dataset to a content, keyword, or reporting workflow.

## How to use

1. Open the **Input** tab and use the example JSON below.
2. Change the query, URL, identifier, or other fields you need. Set the lookup fields to match the query or identifier you want to collect.
3. Start the Actor. Open the run's default dataset to inspect, download, or export the returned records.

```json
{
  "query": "tesla"
}
```

Apify stores the run output in a dataset. You can download the dataset in JSON, CSV, Excel, XML, or other available formats from the run page.

## Output example

This illustrative record uses synthetic values. It shows the real output field names; optional source values may be null or absent.

```json
{
  "position": 42,
  "suggestion": "Example value",
  "type": "Example value",
  "source_keyword": "Example result",
  "request_geo": "Example value",
  "request_hl": "Example value",
  "response_time_ms": 42
}
```

## Input fields

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `query` | string | No | Partial keyword or phrase to expand with Google Trends autocomplete suggestions. |
| `q` | string | No | Alias for query when reusing direct Scrappa API inputs. Either Search Query or this alias is required. |
| `geo` | string | No | Geographic location code, such as US, GB, DE, or Worldwide. |
| `hl` | string | No | Two-letter language code, such as en, de, es, or fr. |

## Pricing

**Current live price:** $0.20 per 1,000 results.

The price is based on the latest live Apify pricing entry. Per-result charges depend on the number of billed results returned. If the price line lists subscription tiers, the rate shown for each tier applies to that Apify subscription level.

## FAQ

### Is it legal to scrape this data?

This Actor is intended for data that is publicly available from Google Trends. You are responsible for following the source site's terms, privacy and copyright rules, and the laws that apply to your use of the data. Only collect information you have a lawful basis to use.

### What limits should I expect?

Use the field constraints and limits in the input table. The source controls which records are available, so a narrow query or unavailable page can return fewer results or none. Keep batch lists small when you need a quick first run.

### Can I run it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/actors/thescrappa~google-trends-autocomplete-scraper/runs`. The response includes `defaultDatasetId`; use it to fetch the run's dataset items. See the [Apify Run Actor API](https://docs.apify.com/api/v2/actors-runs-post) and [Actor runs guide](https://docs.apify.com/api/v2/actors-actor-runs). Apify integrations for [Make](https://docs.apify.com/integrations/make), [Zapier](https://docs.apify.com/integrations/zapier), and [n8n](https://docs.apify.com/integrations/n8n) can trigger runs and pass results to the next workflow step. Send dataset rows to Google Sheets through a Sheets step or a workflow integration.

### What happens if a request fails?

Apify reports input validation and source request errors in the run details. Review the error, correct the input, and retry after a temporary source problem. Depending on when a request stops, the dataset may be empty or contain results collected before the failure.

## Related Scrappa Actors

- [Google Trends Interest Scraper for SEO Research](https://apify.com/thescrappa/google-trends-interest-scraper)
- [Google Trends Related Queries Scraper for SEO](https://apify.com/thescrappa/google-trends-related-queries-scraper)
- [Google Images Scraper for Creator Research](https://apify.com/thescrappa/google-images-scraper)
- [Google Maps Reviews Scraper for Local Reputation](https://apify.com/thescrappa/google-maps-reviews-scraper)
