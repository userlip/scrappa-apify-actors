# Startpage Search Scraper for SEO Research

The Startpage Search Scraper for SEO Research collects search result titles, links, and snippets from Startpage. Provide a search phrase or a short list of phrases; the actor saves source fields such as `position`, `title`, `description`, and `url` to an Apify dataset.

## What data can you extract?

The dataset contains fields returned by Startpage. The field names below match the Actor output schema. A source may leave optional values empty or omit fields when they are not available for a result.

| Field | Type | Description |
| --- | --- | --- |
| `position` | number | # returned for this result. |
| `title` | text | Title returned for this result. |
| `description` | text | Description returned for this result. |
| `url` | link | URL returned for this result. |
| `domain` | text | Domain returned for this result. |
| `source` | text | Source returned for this result. |
| `query` | text | Query returned for this result. |
| `request_language` | text | Language returned for this result. |
| `request_page` | number | Page returned for this result. |
| `request_safe_search` | number | Safe Search returned for this result. |
| `total_results` | number | Total Results returned for this result. |

## Use cases

- Collect search result titles, links, and snippets to support SEO research.
- Compare results across search terms, websites, or markets.
- Export the dataset to a content, keyword, or reporting workflow.

## How to use

1. Open the **Input** tab and use the example JSON below.
2. Change the query, URL, identifier, or other fields you need. For multi-target work, use `queries` and start with a short list.
3. Start the Actor. Open the run's default dataset to inspect, download, or export the returned records.

```json
{
  "queries": [
    {
      "query": "privacy tools",
      "language": "english",
      "page": 0,
      "safe_search": true
    }
  ],
  "max_results_per_query": 5
}
```

Apify stores the run output in a dataset. You can download the dataset in JSON, CSV, Excel, XML, or other available formats from the run page.

## Output example

This illustrative record uses synthetic values. It shows the real output field names; optional source values may be null or absent.

```json
{
  "position": 42,
  "title": "Example result",
  "description": "Example public text.",
  "url": "https://example.com/result/1",
  "domain": "Example value",
  "source": "Example value",
  "query": "Example result"
}
```

## Input fields

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `queries` | array of object | Yes | Search requests to run in one Actor run. Constraints: minimum 1 items; maximum 100 items. |
| `max_results_per_query` | integer | No | Maximum organic results to save for each query. Constraints: minimum 1; maximum 100. |

## Pricing

**Current live price:** Apify Free tier: $0.30 per 1,000 results; Bronze: $0.25 per 1,000 results; Silver: $0.22 per 1,000 results; Gold, Platinum, and Diamond: $0.20 per 1,000 results.

The price is based on the latest live Apify pricing entry. Per-result charges depend on the number of billed results returned. If the price line lists subscription tiers, the rate shown for each tier applies to that Apify subscription level.

## FAQ

### Is it legal to scrape this data?

This Actor is intended for data that is publicly available from Startpage. You are responsible for following the source site's terms, privacy and copyright rules, and the laws that apply to your use of the data. Only collect information you have a lawful basis to use.

### What limits should I expect?

Use the field constraints and limits in the input table. The source controls which records are available, so a narrow query or unavailable page can return fewer results or none. Keep batch lists small when you need a quick first run.

### Can I run it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/actors/thescrappa~startpage-search-scraper/runs`. The response includes `defaultDatasetId`; use it to fetch the run's dataset items. See the [Apify Run Actor API](https://docs.apify.com/api/v2/actors-runs-post) and [Actor runs guide](https://docs.apify.com/api/v2/actors-actor-runs). Apify integrations for [Make](https://docs.apify.com/integrations/make), [Zapier](https://docs.apify.com/integrations/zapier), and [n8n](https://docs.apify.com/integrations/n8n) can trigger runs and pass results to the next workflow step. Send dataset rows to Google Sheets through a Sheets step or a workflow integration.

### What happens if a request fails?

Apify reports input validation and source request errors in the run details. Review the error, correct the input, and retry after a temporary source problem. Depending on when a request stops, the dataset may be empty or contain results collected before the failure.

## Related Scrappa Actors

- [Google Search Results Scraper for SEO Research](https://apify.com/thescrappa/google-search-scraper)
- [Google Trends Autocomplete Scraper for SEO](https://apify.com/thescrappa/google-trends-autocomplete-scraper)
- [Google Trends Interest Scraper for SEO Research](https://apify.com/thescrappa/google-trends-interest-scraper)
- [Google Trends Related Queries Scraper for SEO](https://apify.com/thescrappa/google-trends-related-queries-scraper)
- [Google Search SERP Scraper for SEO Research](https://apify.com/thescrappa/scrappa-google-search)
