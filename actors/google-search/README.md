# Google Search Results Scraper for SEO Research

The Google Search Results Scraper for SEO Research collects search result titles, links, and snippets from google search scraper. Provide a search phrase or a short list of phrases; the actor saves source fields such as `position`, `title`, `link`, and `displayed_link` to an Apify dataset.

## What data can you extract?

The dataset contains fields returned by google search scraper. The field names below match the Actor output schema. A source may leave optional values empty or omit fields when they are not available for a result.

| Field | Type | Description |
| --- | --- | --- |
| `position` | number | # returned for this result. |
| `title` | text | Title returned for this result. |
| `link` | link | URL returned for this result. |
| `displayed_link` | text | Display URL returned for this result. |
| `snippet` | text | Snippet returned for this result. |
| `source` | text | Source returned for this result. |

## Use cases

- Collect search result titles, links, and snippets to support SEO research.
- Compare results across search terms, websites, or markets.
- Export the dataset to a content, keyword, or reporting workflow.

## How to use

1. Open the **Input** tab and use the example JSON below.
2. Change the query, URL, identifier, or other fields you need. Set the lookup fields to match the query or identifier you want to collect.
3. Start the Actor. Open the run's default dataset to inspect, download, or export the returned records.

```json
{
  "query": "best restaurants in new york"
}
```

Apify stores the run output in a dataset. You can download the dataset in JSON, CSV, Excel, XML, or other available formats from the run page.

## Output example

This illustrative record uses synthetic values. It shows the real output field names; optional source values may be null or absent.

```json
{
  "position": 42,
  "title": "Example result",
  "link": "https://example.com/result/1",
  "displayed_link": "https://example.com/result/1",
  "snippet": "Example public text.",
  "source": "Example value"
}
```

## Input fields

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `query` | string | Yes | The search term or phrase to look up on Google |
| `location` | string | No | Geographic location for localized results (e.g., 'New York, NY, USA', 'London, UK') |
| `gl` | string | No | Two-letter country code for Google's country service (e.g., 'us', 'uk', 'de', 'fr') |
| `hl` | string | No | Two-letter language code for the interface (e.g., 'en', 'de', 'es', 'fr') |
| `google_domain` | string | No | Google domain to query (e.g., 'google.com', 'google.de', 'google.co.uk') |
| `start` | integer | No | Result offset for pagination. Use 0 for first page, 10 for second, 20 for third, etc. Constraints: minimum 0. |
| `amount` | integer | No | How many results to return per request (1-100) Constraints: minimum 1; maximum 100. |
| `safe` | string | No | Filter explicit content from results Constraints: allowed values: off, active. |
| `tbs` | string | No | Filter results by time. Examples: 'qdr:h' (past hour), 'qdr:d' (past day), 'qdr:w' (past week), 'qdr:m' (past month), 'qdr:y' (past year) |
| `tbm` | string | No | Type of Google search to perform Constraints: allowed values: , nws, vid, isch, shop. |

## Pricing

**Current live price:** Apify Free tier: $0.30 per 1,000 results; Bronze: $0.25 per 1,000 results; Silver: $0.22 per 1,000 results; Gold, Platinum, and Diamond: $0.20 per 1,000 results.

The price is based on the latest live Apify pricing entry. Per-result charges depend on the number of billed results returned. If the price line lists subscription tiers, the rate shown for each tier applies to that Apify subscription level.

## FAQ

### Is it legal to scrape this data?

This Actor is intended for data that is publicly available from google search scraper. You are responsible for following the source site's terms, privacy and copyright rules, and the laws that apply to your use of the data. Only collect information you have a lawful basis to use.

### What limits should I expect?

Use the field constraints and limits in the input table. The source controls which records are available, so a narrow query or unavailable page can return fewer results or none. Keep batch lists small when you need a quick first run.

### Can I run it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/actors/thescrappa~google-search-scraper/runs`. The response includes `defaultDatasetId`; use it to fetch the run's dataset items. See the [Apify Run Actor API](https://docs.apify.com/api/v2/actors-runs-post) and [Actor runs guide](https://docs.apify.com/api/v2/actors-actor-runs). Apify integrations for [Make](https://docs.apify.com/integrations/make), [Zapier](https://docs.apify.com/integrations/zapier), and [n8n](https://docs.apify.com/integrations/n8n) can trigger runs and pass results to the next workflow step. Send dataset rows to Google Sheets through a Sheets step or a workflow integration.

### What happens if a request fails?

Apify reports input validation and source request errors in the run details. Review the error, correct the input, and retry after a temporary source problem. Depending on when a request stops, the dataset may be empty or contain results collected before the failure.

## Related Scrappa Actors

- [Google Trends Autocomplete Scraper for SEO](https://apify.com/thescrappa/google-trends-autocomplete-scraper)
- [Google Trends Interest Scraper for SEO Research](https://apify.com/thescrappa/google-trends-interest-scraper)
- [Google Trends Related Queries Scraper for SEO](https://apify.com/thescrappa/google-trends-related-queries-scraper)
- [Google Search SERP Scraper for SEO Research](https://apify.com/thescrappa/scrappa-google-search)
- [Startpage Search Scraper for SEO Research](https://apify.com/thescrappa/startpage-search-scraper)
