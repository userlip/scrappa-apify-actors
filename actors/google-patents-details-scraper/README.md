# Google Patents Details Scraper for IP Research

The Google Patents Details Scraper for IP Research collects patent records, inventors, and filing details from Google Patents. Provide one or more public URLs; the actor saves source fields such as `success`, `input_patent_id`, `normalized_patent_id`, and `patent_id` to an Apify dataset.

## What data can you extract?

The dataset contains fields returned by Google Patents. The field names below match the Actor output schema. A source may leave optional values empty or omit fields when they are not available for a result.

| Field | Type | Description |
| --- | --- | --- |
| `success` | boolean | Success returned for this result. |
| `input_patent_id` | text | Input returned for this result. |
| `normalized_patent_id` | text | Normalized ID returned for this result. |
| `patent_id` | text | Patent ID returned for this result. |
| `publication_number` | text | Publication # returned for this result. |
| `patent_page` | link | Patent Page returned for this result. |
| `title` | text | Title returned for this result. |
| `abstract` | text | Abstract returned for this result. |
| `inventors` | text | inventors returned for this result. |
| `assignees` | text | assignees returned for this result. |
| `dates` | text | dates returned for this result. |
| `country` | text | Country returned for this result. |
| `language` | text | Language returned for this result. |
| `application_number` | text | Application # returned for this result. |
| `prior_art_keywords` | text | prior art keywords returned for this result. |
| `links` | text | links returned for this result. |
| `citations` | text | citations returned for this result. |
| `inventor_count` | number | Inventors returned for this result. |
| `assignee_count` | number | Assignees returned for this result. |
| `citation_count` | number | Citations returned for this result. |
| `cached` | boolean | Cached returned for this result. |
| `response_time_ms` | number | Response Time returned for this result. |
| `error` | text | Error returned for this result. |
| `status_code` | number | Status returned for this result. |

## Use cases

- Collect patent records, inventors, and filing details to support market research.
- Compare records across the input queries or entities you provide.
- Export structured results to research and reporting workflows.

## How to use

1. Open the **Input** tab and use the example JSON below.
2. Change the query, URL, identifier, or other fields you need. For multi-target work, use `patent_ids` and start with a short list.
3. Start the Actor. Open the run's default dataset to inspect, download, or export the returned records.

```json
{
  "patent_ids": [
    "US9789384B1"
  ]
}
```

Apify stores the run output in a dataset. You can download the dataset in JSON, CSV, Excel, XML, or other available formats from the run page.

## Output example

This illustrative record uses synthetic values. It shows the real output field names; optional source values may be null or absent.

```json
{
  "success": true,
  "input_patent_id": "example-123",
  "normalized_patent_id": "example-123",
  "patent_id": "example-123",
  "publication_number": "Example value",
  "patent_page": "Example value",
  "title": "Example result"
}
```

## Input fields

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `patent_id` | string | No | Single patent publication ID or full Google Patents ID, such as US9789384B1 or patent/US9789384B1/en. |
| `patent_ids` | array of string | No | Batch of patent publication IDs. Each item can be a short publication ID or a full Google Patents ID. |
| `url` | string | No | Single Google Patents URL, such as https://patents.google.com/patent/US9789384B1. |
| `urls` | array of string | No | Batch of Google Patents URLs to enrich in the same Apify run. |

## Pricing

**Current live price:** $0.20 per 1,000 results.

The price is based on the latest live Apify pricing entry. Per-result charges depend on the number of billed results returned. If the price line lists subscription tiers, the rate shown for each tier applies to that Apify subscription level.

## FAQ

### Is it legal to scrape this data?

This Actor is intended for data that is publicly available from Google Patents. You are responsible for following the source site's terms, privacy and copyright rules, and the laws that apply to your use of the data. Only collect information you have a lawful basis to use.

### What limits should I expect?

Use the field constraints and limits in the input table. The source controls which records are available, so a narrow query or unavailable page can return fewer results or none. Keep batch lists small when you need a quick first run.

### Can I run it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/actors/thescrappa~google-patents-details-scraper/runs`. The response includes `defaultDatasetId`; use it to fetch the run's dataset items. See the [Apify Run Actor API](https://docs.apify.com/api/v2/actors-runs-post) and [Actor runs guide](https://docs.apify.com/api/v2/actors-actor-runs). Apify integrations for [Make](https://docs.apify.com/integrations/make), [Zapier](https://docs.apify.com/integrations/zapier), and [n8n](https://docs.apify.com/integrations/n8n) can trigger runs and pass results to the next workflow step. Send dataset rows to Google Sheets through a Sheets step or a workflow integration.

### What happens if a request fails?

Apify reports input validation and source request errors in the run details. Review the error, correct the input, and retry after a temporary source problem. Depending on when a request stops, the dataset may be empty or contain results collected before the failure.

## Related Scrappa Actors

- [Google Patents Search Scraper for IP Research](https://apify.com/thescrappa/google-patents-search-scraper)
- [Google Search Results Scraper for SEO Research](https://apify.com/thescrappa/google-search-scraper)
- [Google Trends Related Queries Scraper for SEO](https://apify.com/thescrappa/google-trends-related-queries-scraper)
