# Indeed Job Listings Scraper for Hiring Teams

The Indeed Job Listings Scraper for Hiring Teams collects job listings and job details from Indeed. Provide a search phrase or a short list of phrases; the actor saves source fields such as `title`, `company_name`, `location_formatted`, and `salary` to an Apify dataset.

## What data can you extract?

The dataset contains fields returned by Indeed. The field names below match the Actor output schema. A source may leave optional values empty or omit fields when they are not available for a result.

| Field | Type | Description |
| --- | --- | --- |
| `title` | text | Title returned for this result. |
| `company_name` | text | Company returned for this result. |
| `location_formatted` | text | Location returned for this result. |
| `salary` | object | Salary returned for this result. |
| `attributes` | array | Attributes returned for this result. |
| `date_published` | text | Date Published returned for this result. |
| `apply_url` | link | Apply URL returned for this result. |

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
  "query": "software engineer",
  "location": "New York",
  "limit": 5
}
```

Apify stores the run output in a dataset. You can download the dataset in JSON, CSV, Excel, XML, or other available formats from the run page.

## Output example

This illustrative record uses synthetic values. It shows the real output field names; optional source values may be null or absent.

```json
{
  "title": "Example result",
  "company_name": "Example Company",
  "location_formatted": "Example location",
  "salary": {},
  "attributes": [],
  "date_published": "2026-09-30T10:00:00Z",
  "apply_url": "https://example.com/result/1"
}
```

## Input fields

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `query` | string | No | Search keywords, for example software engineer, nurse, or marketing manager. Empty input defaults to a narrow US example query. |
| `location` | string | No | City, state, region, or remote location, for example New York, San Francisco, CA, Berlin, or remote. |
| `country` | string | No | Two-letter country code supported by Indeed, for example US, GB, CA, AU, DE, or FR. |
| `radius` | integer | No | Search radius from the provided location. Scrappa accepts 0 to 100. Constraints: minimum 0; maximum 100. |
| `radius_unit` | string | No | Unit used for the search radius. Constraints: allowed values: MILES, KILOMETERS. |
| `job_type` | string | No | Filter by job type. Constraints: allowed values: full_time, part_time, contract, internship, remote. |
| `sort` | string | No | Sort order for Indeed results. Constraints: allowed values: relevance, date. |
| `limit` | integer | No | Number of jobs to return in one request. Scrappa accepts 1 to 100. Constraints: minimum 1; maximum 100. |
| `cursor` | string | No | Cursor returned as data.pagination.next_cursor by a previous response. |
| `hl` | string | No | Two-letter language code for localization, for example en, de, fr, or es. |
| `gl` | string | No | Two-letter country code for geolocation/localization, for example US, DE, GB, or FR. |

## Pricing

**Current live price:** $0.30 per 1,000 results.

The price is based on the latest live Apify pricing entry. Per-result charges depend on the number of billed results returned. If the price line lists subscription tiers, the rate shown for each tier applies to that Apify subscription level.

## FAQ

### Is it legal to scrape this data?

This Actor is intended for data that is publicly available from Indeed. You are responsible for following the source site's terms, privacy and copyright rules, and the laws that apply to your use of the data. Only collect information you have a lawful basis to use.

### What limits should I expect?

Use the field constraints and limits in the input table. The source controls which records are available, so a narrow query or unavailable page can return fewer results or none. Keep batch lists small when you need a quick first run.

### Can I run it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/actors/thescrappa~indeed-jobs-scraper/runs`. The response includes `defaultDatasetId`; use it to fetch the run's dataset items. See the [Apify Run Actor API](https://docs.apify.com/api/v2/actors-runs-post) and [Actor runs guide](https://docs.apify.com/api/v2/actors-actor-runs). Apify integrations for [Make](https://docs.apify.com/integrations/make), [Zapier](https://docs.apify.com/integrations/zapier), and [n8n](https://docs.apify.com/integrations/n8n) can trigger runs and pass results to the next workflow step. Send dataset rows to Google Sheets through a Sheets step or a workflow integration.

### What happens if a request fails?

Apify reports input validation and source request errors in the run details. Review the error, correct the input, and retry after a temporary source problem. Depending on when a request stops, the dataset may be empty or contain results collected before the failure.

## Related Scrappa Actors

- [Arbeitsagentur Jobs Scraper for Candidate Research](https://apify.com/thescrappa/arbeitsagentur-jobs-scraper)
- [Google Jobs Data Scraper for Hiring Teams](https://apify.com/thescrappa/google-jobs-scraper)
- [Kununu Jobs Scraper for Employer Research](https://apify.com/thescrappa/kununu-jobs-scraper)
- [LinkedIn Job Details Scraper for Hiring Teams](https://apify.com/thescrappa/linkedin-job-details-scraper)
- [LinkedIn Jobs Search Scraper for Hiring Teams](https://apify.com/thescrappa/linkedin-jobs-search-scraper)
