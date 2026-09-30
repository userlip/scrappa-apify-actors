# Google Jobs Data Scraper for Hiring Teams

The Google Jobs Data Scraper for Hiring Teams collects job listings and job details from Google Jobs. Provide a search phrase or a short list of phrases; the actor saves source fields such as `title`, `company`, `company_name`, and `location` to an Apify dataset.

## What data can you extract?

The dataset contains fields returned by Google Jobs. The field names below match the Actor output schema. A source may leave optional values empty or omit fields when they are not available for a result.

| Field | Type | Description |
| --- | --- | --- |
| `title` | text | Title returned for this result. |
| `company` | text | Company returned for this result. |
| `company_name` | text | Company Name returned for this result. |
| `location` | text | Location returned for this result. |
| `via` | text | Via returned for this result. |
| `description` | text | Description returned for this result. |
| `job_id` | text | Job ID returned for this result. |

## Use cases

- Build a focused list of job listings and job details for a role, employer, or location.
- Compare job titles, employers, locations, and other returned listing fields.
- Send structured listings to a recruiting report or hiring workflow.

## How to use

1. Open the **Input** tab and use the example JSON below.
2. Change the query, URL, identifier, or other fields you need. Set the lookup fields to match the query or identifier you want to collect.
3. Start the Actor. Open the run's default dataset to inspect, download, or export the returned records.

```json
{
  "q": "nurse jobs in Austin"
}
```

Apify stores the run output in a dataset. You can download the dataset in JSON, CSV, Excel, XML, or other available formats from the run page.

## Output example

This illustrative record uses synthetic values. It shows the real output field names; optional source values may be null or absent.

```json
{
  "title": "Example result",
  "company": "Example Company",
  "company_name": "Example Company",
  "location": "Example location",
  "via": "Example value",
  "description": "Example public text.",
  "job_id": "example-123"
}
```

## Input fields

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `q` | string | No | Job search query. Leave empty only when using a next page token. Empty or placeholder input defaults to a narrow US example query. |
| `next_page_token` | string | No | Pagination token returned by a previous Google Jobs response. When provided, the query can be omitted. |
| `gl` | string | No | Two-letter country code for search results (e.g., 'us', 'uk', 'de', 'fr') |
| `hl` | string | No | Two-letter language code for the interface (e.g., 'en', 'de', 'es', 'fr') |
| `google_domain` | string | No | Google domain to query (e.g., 'google.com', 'google.de', 'google.co.uk') |
| `uule` | string | No | Google-encoded location parameter for precise geolocation. |
| `lrad` | integer | No | Search radius around the encoded location in miles. Supported values: 5, 10, 25, 50, or 100. Requires uule. Constraints: minimum 5; maximum 100. |
| `uds` | string | No | Dynamic Google Jobs filter string returned in the filters section of a previous response. |

## Pricing

**Current live price:** Apify Free tier: $0.30 per 1,000 results; Bronze: $0.25 per 1,000 results; Silver: $0.22 per 1,000 results; Gold, Platinum, and Diamond: $0.20 per 1,000 results.

The price is based on the latest live Apify pricing entry. Per-result charges depend on the number of billed results returned. If the price line lists subscription tiers, the rate shown for each tier applies to that Apify subscription level.

## FAQ

### Is it legal to scrape this data?

This Actor is intended for data that is publicly available from Google Jobs. You are responsible for following the source site's terms, privacy and copyright rules, and the laws that apply to your use of the data. Only collect information you have a lawful basis to use.

### What limits should I expect?

Use the field constraints and limits in the input table. The source controls which records are available, so a narrow query or unavailable page can return fewer results or none. Keep batch lists small when you need a quick first run.

### Can I run it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/actors/thescrappa~google-jobs-scraper/runs`. The response includes `defaultDatasetId`; use it to fetch the run's dataset items. See the [Apify Run Actor API](https://docs.apify.com/api/v2/actors-runs-post) and [Actor runs guide](https://docs.apify.com/api/v2/actors-actor-runs). Apify integrations for [Make](https://docs.apify.com/integrations/make), [Zapier](https://docs.apify.com/integrations/zapier), and [n8n](https://docs.apify.com/integrations/n8n) can trigger runs and pass results to the next workflow step. Send dataset rows to Google Sheets through a Sheets step or a workflow integration.

### What happens if a request fails?

Apify reports input validation and source request errors in the run details. Review the error, correct the input, and retry after a temporary source problem. Depending on when a request stops, the dataset may be empty or contain results collected before the failure.

## Related Scrappa Actors

- [Arbeitsagentur Jobs Scraper for Candidate Research](https://apify.com/thescrappa/arbeitsagentur-jobs-scraper)
- [Indeed Job Listings Scraper for Hiring Teams](https://apify.com/thescrappa/indeed-jobs-scraper)
- [Kununu Jobs Scraper for Employer Research](https://apify.com/thescrappa/kununu-jobs-scraper)
- [LinkedIn Job Details Scraper for Hiring Teams](https://apify.com/thescrappa/linkedin-job-details-scraper)
- [LinkedIn Jobs Search Scraper for Hiring Teams](https://apify.com/thescrappa/linkedin-jobs-search-scraper)
