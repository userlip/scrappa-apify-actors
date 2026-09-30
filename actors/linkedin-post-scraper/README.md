# LinkedIn Post Scraper for Audience Research

The LinkedIn Post Scraper for Audience Research collects public records and structured source fields from LinkedIn. Provide one or more public URLs; the actor saves source fields such as `title`, `author_name`, `date_published`, and `reactions_total` to an Apify dataset.

## What data can you extract?

The dataset contains fields returned by LinkedIn. The field names below match the Actor output schema. A source may leave optional values empty or omit fields when they are not available for a result.

| Field | Type | Description |
| --- | --- | --- |
| `title` | text | Title returned for this result. |
| `author_name` | text | Author returned for this result. |
| `date_published` | text | Published returned for this result. |
| `reactions_total` | number | Reactions returned for this result. |
| `comments_count` | number | Comments returned for this result. |

## Use cases

- Collect public records and structured source fields for audience and content research.
- Review public profile, post, or engagement fields returned for each item.
- Export the dataset to a social reporting or creator workflow.

## How to use

1. Open the **Input** tab and use the example JSON below.
2. Change the query, URL, identifier, or other fields you need. Set the lookup fields to match the query or identifier you want to collect.
3. Start the Actor. Open the run's default dataset to inspect, download, or export the returned records.

```json
{
  "url": "https://www.linkedin.com/posts/microsoft_worktrendindex-activity-7457369463198437376-3F4k"
}
```

Apify stores the run output in a dataset. You can download the dataset in JSON, CSV, Excel, XML, or other available formats from the run page.

## Output example

This illustrative record uses synthetic values. It shows the real output field names; optional source values may be null or absent.

```json
{
  "title": "Example result",
  "author_name": "Sample Member",
  "date_published": "2026-09-30T10:00:00Z",
  "reactions_total": 42,
  "comments_count": 42
}
```

## Input fields

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `url` | string | Yes | URL of the LinkedIn post or article to scrape (e.g., https://linkedin.com/pulse/... or https://linkedin.com/posts/...) |
| `use_cache` | boolean | No | Use cached results if available to reduce costs. When disabled, the actor omits the cache flag because Scrappa does not accept use_cache=0. |
| `maximum_cache_age` | integer | No | Maximum age of cached results in seconds. Must be at least 1. Constraints: minimum 1. |

## Pricing

**Current live price:** $0.30 per 1,000 results.

The price is based on the latest live Apify pricing entry. Per-result charges depend on the number of billed results returned. If the price line lists subscription tiers, the rate shown for each tier applies to that Apify subscription level.

## FAQ

### Is it legal to scrape this data?

This Actor is intended for data that is publicly available from LinkedIn. You are responsible for following the source site's terms, privacy and copyright rules, and the laws that apply to your use of the data. Only collect information you have a lawful basis to use.

### What limits should I expect?

Use the field constraints and limits in the input table. The source controls which records are available, so a narrow query or unavailable page can return fewer results or none. Keep batch lists small when you need a quick first run.

### Can I run it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/actors/thescrappa~linkedin-post-scraper/runs`. The response includes `defaultDatasetId`; use it to fetch the run's dataset items. See the [Apify Run Actor API](https://docs.apify.com/api/v2/actors-runs-post) and [Actor runs guide](https://docs.apify.com/api/v2/actors-actor-runs). Apify integrations for [Make](https://docs.apify.com/integrations/make), [Zapier](https://docs.apify.com/integrations/zapier), and [n8n](https://docs.apify.com/integrations/n8n) can trigger runs and pass results to the next workflow step. Send dataset rows to Google Sheets through a Sheets step or a workflow integration.

### What happens if a request fails?

Apify reports input validation and source request errors in the run details. Review the error, correct the input, and retry after a temporary source problem. Depending on when a request stops, the dataset may be empty or contain results collected before the failure.

## Related Scrappa Actors

- [LinkedIn Company Scraper for Lead Research](https://apify.com/thescrappa/linkedin-company-scraper)
- [LinkedIn Job Details Scraper for Hiring Teams](https://apify.com/thescrappa/linkedin-job-details-scraper)
- [LinkedIn Jobs Search Scraper for Hiring Teams](https://apify.com/thescrappa/linkedin-jobs-search-scraper)
- [LinkedIn Profile Scraper for Lead Research](https://apify.com/thescrappa/linkedin-profile-scraper)
- [LinkedIn Search Scraper for Lead Research](https://apify.com/thescrappa/linkedin-search-scraper)
