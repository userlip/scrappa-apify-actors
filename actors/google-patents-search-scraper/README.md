# Google Patents Search Scraper for IP Research

The Google Patents Search Scraper for IP Research collects patent records, inventors, and filing details from Google Patents. Provide a search phrase or a short list of phrases; the actor saves source fields such as `rank`, `title`, `patent_id`, and `publication_number` to an Apify dataset.

## What data can you extract?

The dataset contains fields returned by Google Patents. The field names below match the Actor output schema. A source may leave optional values empty or omit fields when they are not available for a result.

| Field | Type | Description |
| --- | --- | --- |
| `rank` | number | Rank returned for this result. |
| `title` | text | Title returned for this result. |
| `patent_id` | text | Patent ID returned for this result. |
| `publication_number` | text | Publication # returned for this result. |
| `patent_page` | link | Patent Page returned for this result. |
| `assignee` | text | Assignee returned for this result. |
| `inventor` | text | Inventor returned for this result. |
| `language` | text | Language returned for this result. |
| `priority_date` | date | Priority Date returned for this result. |
| `filing_date` | date | Filing Date returned for this result. |
| `grant_date` | date | Grant Date returned for this result. |
| `publication_date` | date | Publication Date returned for this result. |
| `pdf` | link | PDF returned for this result. |
| `family_countries` | text | Family Countries returned for this result. |
| `family_status_count` | number | Family Statuses returned for this result. |
| `request_q` | text | Query returned for this result. |
| `request_country` | text | Country Filter returned for this result. |
| `request_status` | text | Status Filter returned for this result. |
| `request_type` | text | Type Filter returned for this result. |
| `request_before` | text | Before Filter returned for this result. |
| `request_after` | text | After Filter returned for this result. |

## Use cases

- Collect patent records, inventors, and filing details to support market research.
- Compare records across the input queries or entities you provide.
- Export structured results to research and reporting workflows.

## How to use

1. Open the **Input** tab and use the example JSON below.
2. Change the query, URL, identifier, or other fields you need. Set the lookup fields to match the query or identifier you want to collect.
3. Start the Actor. Open the run's default dataset to inspect, download, or export the returned records.

```json
{
  "q": "wireless charging vehicle battery",
  "page": 1
}
```

Apify stores the run output in a dataset. You can download the dataset in JSON, CSV, Excel, XML, or other available formats from the run page.

## Output example

This illustrative record uses synthetic values. It shows the real output field names; optional source values may be null or absent.

```json
{
  "rank": 42,
  "title": "Example result",
  "patent_id": "example-123",
  "publication_number": "Example value",
  "patent_page": "Example value",
  "assignee": "Example value",
  "inventor": "Example value"
}
```

## Input fields

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `q` | string | Yes | Google Patents search query. Supports patent keywords and Boolean-style patent search text. |
| `page` | integer | No | One-based Google Patents result page. Constraints: minimum 1; maximum 100. |
| `num` | integer | No | Number of patent results to request on the page. Constraints: minimum 1; maximum 100. |
| `sort` | string | No | Leave empty for relevance, or sort by newest or oldest results. Constraints: allowed values: new, old. |
| `country` | string | No | Comma-separated patent country codes, such as US,EP,WO. |
| `language` | string | No | Google Patents language filter, such as ENGLISH, GERMAN, or FRENCH. |
| `status` | string | No | Filter by granted patents or applications. Constraints: allowed values: GRANT, APPLICATION. |
| `type` | string | No | Filter by utility patents or design patents. Constraints: allowed values: PATENT, DESIGN. |
| `before` | string | No | Filter patents before a filing or publication date. Format: filing:YYYYMMDD or publication:YYYYMMDD. |
| `after` | string | No | Filter patents after a filing or publication date. Format: filing:YYYYMMDD or publication:YYYYMMDD. |
| `inventor` | string | No | Comma-separated inventor names for people-focused patent searches. |
| `assignee` | string | No | Comma-separated assignee or company names for IP monitoring and competitor patent tracking. |

## Pricing

**Current live price:** $0.20 per 1,000 results.

The price is based on the latest live Apify pricing entry. Per-result charges depend on the number of billed results returned. If the price line lists subscription tiers, the rate shown for each tier applies to that Apify subscription level.

## FAQ

### Is it legal to scrape this data?

This Actor is intended for data that is publicly available from Google Patents. You are responsible for following the source site's terms, privacy and copyright rules, and the laws that apply to your use of the data. Only collect information you have a lawful basis to use.

### What limits should I expect?

Use the field constraints and limits in the input table. The source controls which records are available, so a narrow query or unavailable page can return fewer results or none. Keep batch lists small when you need a quick first run.

### Can I run it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/actors/thescrappa~google-patents-search-scraper/runs`. The response includes `defaultDatasetId`; use it to fetch the run's dataset items. See the [Apify Run Actor API](https://docs.apify.com/api/v2/actors-runs-post) and [Actor runs guide](https://docs.apify.com/api/v2/actors-actor-runs). Apify integrations for [Make](https://docs.apify.com/integrations/make), [Zapier](https://docs.apify.com/integrations/zapier), and [n8n](https://docs.apify.com/integrations/n8n) can trigger runs and pass results to the next workflow step. Send dataset rows to Google Sheets through a Sheets step or a workflow integration.

### What happens if a request fails?

Apify reports input validation and source request errors in the run details. Review the error, correct the input, and retry after a temporary source problem. Depending on when a request stops, the dataset may be empty or contain results collected before the failure.

## Related Scrappa Actors

- [Google Patents Details Scraper for IP Research](https://apify.com/thescrappa/google-patents-details-scraper)
- [Google Search Results Scraper for SEO Research](https://apify.com/thescrappa/google-search-scraper)
- [Google Trends Related Queries Scraper for SEO](https://apify.com/thescrappa/google-trends-related-queries-scraper)
