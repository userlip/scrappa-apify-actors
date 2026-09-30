# Google News Scraper for Media Monitoring

The Google News Scraper for Media Monitoring collects public records and structured source fields from Google News. Provide a search phrase or a short list of phrases; the actor saves source fields such as `position`, `title`, `link`, and `source_name` to an Apify dataset.

## What data can you extract?

The dataset contains fields returned by Google News. The field names below match the Actor output schema. A source may leave optional values empty or omit fields when they are not available for a result.

| Field | Type | Description |
| --- | --- | --- |
| `position` | number | # returned for this result. |
| `title` | text | Title returned for this result. |
| `link` | link | URL returned for this result. |
| `source_name` | text | Source returned for this result. |
| `date` | text | Date returned for this result. |
| `iso_date` | date | ISO Date returned for this result. |
| `snippet` | text | Snippet returned for this result. |
| `thumbnail` | image | Thumbnail returned for this result. |
| `story_token` | text | Story Token returned for this result. |
| `request_q` | text | Query returned for this result. |
| `request_gl` | text | Country returned for this result. |
| `request_hl` | text | Language returned for this result. |

## Use cases

- Collect public records and structured source fields from Google News for news monitoring.
- Review the structured fields returned for each result.
- Export the dataset or schedule recurring runs in Apify.

## How to use

1. Open the **Input** tab and use the example JSON below.
2. Change the query, URL, identifier, or other fields you need. For multi-target work, use `queries` and start with a short list.
3. Start the Actor. Open the run's default dataset to inspect, download, or export the returned records.

```json
{
  "queries": [
    "artificial intelligence"
  ],
  "q": "artificial intelligence",
  "page": 1
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
  "source_name": "Example value",
  "date": "2026-09-30T10:00:00Z",
  "iso_date": "2026-09-30T10:00:00Z",
  "snippet": "Example public text."
}
```

## Input fields

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `queries` | array of string | No | Recommended. Process many Google News keyword searches in one Apify run so run startup and storage overhead are shared across results. Leave empty when using token parameters. Constraints: maximum 10 items. |
| `q` | string | No | Backward-compatible single keyword or phrase to search in Google News. Prefer Search Queries for normal usage, especially when running more than one keyword. Leave empty when using a token parameter. |
| `gl` | string | No | Two-letter Google News country code, such as us, gb, de, or fr. |
| `hl` | string | No | Two-letter Google News interface language, such as en, de, es, or fr. |
| `page` | integer | No | Page number for pagination. Do not use together with Start Offset. Constraints: minimum 1. |
| `start` | integer | No | Zero-based result offset for pagination. Do not use together with Page. Constraints: minimum 0. |
| `so` | integer | No | Sort order: 0 for relevance, 1 for date. Constraints: minimum 0; maximum 1. |
| `topic_token` | string | No | Google News topic token. Cannot be used with Search Query. |
| `kgmid` | string | No | Google Knowledge Graph entity ID starting with /m/ or /g/. Must be used alone. |
| `publication_token` | string | No | Google News publication token. Cannot be used with Search Query. |
| `section_token` | string | No | Google News section token. Cannot be used with Search Query. |
| `story_token` | string | No | Google News story cluster token. Cannot be used with Search Query. |

## Pricing

**Current live price:** Apify Free tier: $0.30 per 1,000 results; Bronze: $0.25 per 1,000 results; Silver: $0.22 per 1,000 results; Gold, Platinum, and Diamond: $0.20 per 1,000 results.

The price is based on the latest live Apify pricing entry. Per-result charges depend on the number of billed results returned. If the price line lists subscription tiers, the rate shown for each tier applies to that Apify subscription level.

## FAQ

### Is it legal to scrape this data?

This Actor is intended for data that is publicly available from Google News. You are responsible for following the source site's terms, privacy and copyright rules, and the laws that apply to your use of the data. Only collect information you have a lawful basis to use.

### What limits should I expect?

Use the field constraints and limits in the input table. The source controls which records are available, so a narrow query or unavailable page can return fewer results or none. Keep batch lists small when you need a quick first run.

### Can I run it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/actors/thescrappa~google-news-scraper/runs`. The response includes `defaultDatasetId`; use it to fetch the run's dataset items. See the [Apify Run Actor API](https://docs.apify.com/api/v2/actors-runs-post) and [Actor runs guide](https://docs.apify.com/api/v2/actors-actor-runs). Apify integrations for [Make](https://docs.apify.com/integrations/make), [Zapier](https://docs.apify.com/integrations/zapier), and [n8n](https://docs.apify.com/integrations/n8n) can trigger runs and pass results to the next workflow step. Send dataset rows to Google Sheets through a Sheets step or a workflow integration.

### What happens if a request fails?

Apify reports input validation and source request errors in the run details. Review the error, correct the input, and retry after a temporary source problem. Depending on when a request stops, the dataset may be empty or contain results collected before the failure.

## Related Scrappa Actors

- [Google Search Results Scraper for SEO Research](https://apify.com/thescrappa/google-search-scraper)
- [Google Videos Scraper for Creator Research](https://apify.com/thescrappa/google-videos-scraper)
- [YouTube Search Results Scraper for Video Analysis](https://apify.com/thescrappa/youtube-api-search-data)
