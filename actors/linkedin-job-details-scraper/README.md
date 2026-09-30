# LinkedIn Job Details Scraper for Hiring Teams

The LinkedIn Job Details Scraper for Hiring Teams collects job listings and job details from LinkedIn. Provide one or more public URLs; the actor saves source fields such as `success`, `title`, `company`, and `location` to an Apify dataset.

## What data can you extract?

The dataset contains fields returned by LinkedIn. The field names below match the Actor output schema. A source may leave optional values empty or omit fields when they are not available for a result.

| Field | Type | Description |
| --- | --- | --- |
| `success` | boolean | Success returned for this result. |
| `title` | text | Title returned for this result. |
| `company` | text | Company returned for this result. |
| `location` | text | Location returned for this result. |
| `employment_type` | text | Employment Type returned for this result. |
| `seniority_level` | text | Seniority returned for this result. |
| `posted_date` | date | Posted Date returned for this result. |
| `applicants` | text | Applicants returned for this result. |
| `apply_url` | link | Apply URL returned for this result. |
| `url` | link | LinkedIn Job URL returned for this result. |
| `input_url` | link | Input URL returned for this result. |
| `normalized_url` | link | Normalized URL returned for this result. |
| `status_code` | number | Status returned for this result. |
| `message` | text | Message returned for this result. |
| `error_type` | text | Error Type returned for this result. |
| `error` | text | Error returned for this result. |

## Use cases

- Build a focused list of job listings and job details for a role, employer, or location.
- Compare job titles, employers, locations, and other returned listing fields.
- Send structured listings to a recruiting report or hiring workflow.

## How to use

1. Open the **Input** tab and use the example JSON below.
2. Change the query, URL, identifier, or other fields you need. For multi-target work, use `urls` and start with a short list.
3. Start the Actor. Open the run's default dataset to inspect, download, or export the returned records.

```json
{
  "urls": [
    "https://www.linkedin.com/jobs/view/senior-software-engineer-remote-at-givedirectly-4360951742"
  ]
}
```

Apify stores the run output in a dataset. You can download the dataset in JSON, CSV, Excel, XML, or other available formats from the run page.

## Output example

This illustrative record uses synthetic values. It shows the real output field names; optional source values may be null or absent.

```json
{
  "success": true,
  "title": "Example result",
  "company": "Example Company",
  "location": "Example location",
  "employment_type": "Example value",
  "seniority_level": "Example value",
  "posted_date": "2026-09-30T10:00:00Z"
}
```

## Input fields

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `urls` | array of string | No | Recommended. Process many LinkedIn job URLs in one Apify run so run startup and storage overhead are shared across results. Constraints: minimum 1 items. |
| `url` | string | No | Backward-compatible single job URL. Prefer URLs for normal usage, especially when processing more than one job. |
| `use_cache` | boolean | No | Use cached data if available to reduce costs and speed up requests. |
| `maximum_cache_age` | integer | No | Maximum age of cached data in seconds (default: 2592000 = 30 days). Must be at least 1 second. Only used if use_cache is enabled. Constraints: minimum 1. |

## Pricing

**Current live price:** $0.30 per 1,000 results.

The price is based on the latest live Apify pricing entry. Per-result charges depend on the number of billed results returned. If the price line lists subscription tiers, the rate shown for each tier applies to that Apify subscription level.

## FAQ

### Is it legal to scrape this data?

This Actor is intended for data that is publicly available from LinkedIn. You are responsible for following the source site's terms, privacy and copyright rules, and the laws that apply to your use of the data. Only collect information you have a lawful basis to use.

### What limits should I expect?

Use the field constraints and limits in the input table. The source controls which records are available, so a narrow query or unavailable page can return fewer results or none. Keep batch lists small when you need a quick first run.

### Can I run it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/actors/thescrappa~linkedin-job-details-scraper/runs`. The response includes `defaultDatasetId`; use it to fetch the run's dataset items. See the [Apify Run Actor API](https://docs.apify.com/api/v2/actors-runs-post) and [Actor runs guide](https://docs.apify.com/api/v2/actors-actor-runs). Apify integrations for [Make](https://docs.apify.com/integrations/make), [Zapier](https://docs.apify.com/integrations/zapier), and [n8n](https://docs.apify.com/integrations/n8n) can trigger runs and pass results to the next workflow step. Send dataset rows to Google Sheets through a Sheets step or a workflow integration.

### What happens if a request fails?

Apify reports input validation and source request errors in the run details. Review the error, correct the input, and retry after a temporary source problem. Depending on when a request stops, the dataset may be empty or contain results collected before the failure.

## Related Scrappa Actors

- [LinkedIn Company Scraper for Lead Research](https://apify.com/thescrappa/linkedin-company-scraper)
- [LinkedIn Jobs Search Scraper for Hiring Teams](https://apify.com/thescrappa/linkedin-jobs-search-scraper)
- [LinkedIn Post Scraper for Audience Research](https://apify.com/thescrappa/linkedin-post-scraper)
- [LinkedIn Profile Scraper for Lead Research](https://apify.com/thescrappa/linkedin-profile-scraper)
- [LinkedIn Search Scraper for Lead Research](https://apify.com/thescrappa/linkedin-search-scraper)
