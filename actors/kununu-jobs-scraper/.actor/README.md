# Kununu Jobs Scraper for Employer Research

The Kununu Jobs Scraper for Employer Research collects job listings and job details from kununu. Provide a search phrase or a short list of phrases; the actor saves source fields such as `title`, `company_name`, `company_score`, and `company_is_top_company` to an Apify dataset.

## What data can you extract?

The dataset contains fields returned by kununu. The field names below match the Actor output schema. A source may leave optional values empty or omit fields when they are not available for a result.

| Field | Type | Description |
| --- | --- | --- |
| `title` | text | Title returned for this result. |
| `company_name` | text | Company returned for this result. |
| `company_score` | number | Company Score returned for this result. |
| `company_is_top_company` | boolean | Top Company returned for this result. |
| `location_formatted` | text | Location returned for this result. |
| `workplace` | text | Workplace returned for this result. |
| `workplace_model` | text | Workplace Model returned for this result. |
| `employment_type` | text | Employment Type returned for this result. |
| `employment_types` | array | Employment Types returned for this result. |
| `career_level` | text | Career Level returned for this result. |
| `benefits` | array | Benefits returned for this result. |
| `salary` | object | Salary returned for this result. |
| `date_posted` | text | Date Posted returned for this result. |
| `posted_at` | text | Posted At returned for this result. |
| `job_url` | link | Job URL returned for this result. |

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
  "query": "Software Engineer",
  "location": "Berlin",
  "page": 1
}
```

Apify stores the run output in a dataset. You can download the dataset in JSON, CSV, Excel, XML, or other available formats from the run page.

## Output example

This illustrative record uses synthetic values. It shows the real output field names; optional source values may be null or absent.

```json
{
  "title": "Example result",
  "company_name": "Example Company",
  "company_score": 4.7,
  "company_is_top_company": true,
  "location_formatted": "Example location",
  "workplace": "Example value",
  "workplace_model": "Example value"
}
```

## Input fields

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `query` | string | No | Job title or keyword, for example Software Engineer, Product Manager, Pflegefachkraft, or Sales. |
| `location` | string | No | City or location name. Pass a plain city name such as Berlin, Munich, Vienna, Zurich, or Hamburg. |
| `country` | string | No | Kununu country market. Constraints: allowed values: de, at, ch. |
| `page` | integer | No | First Kununu result page to fetch. Kununu returns about 30 results per page. Constraints: minimum 1; maximum 100. |
| `max_pages` | integer | No | Number of consecutive Kununu result pages to fetch in one run. Constraints: minimum 1; maximum 10. |
| `radius` | integer | No | Search radius in kilometers around the selected location. Allowed values: 10, 20, 30, 50, 100, or 200. Constraints: minimum 10; maximum 200. |
| `sort` | string | No | Sort by newest listings or by Kununu company score. Leave empty for Kununu relevance order. Constraints: allowed values: , newest, kununuScore. |
| `workplace` | array of string | No | Filter jobs by remote, hybrid, or on-site workplace model. |
| `employment_types` | array of string | No | Filter by employment type. |
| `career_level` | array of string | No | Filter by career level. Values follow Kununu's career-level IDs. |
| `kununu_score` | array of string | No | Filter by company score range. |
| `industry` | array of integer | No | Kununu industry IDs from 1 to 44. Constraints: maximum 10 items. |
| `discipline` | array of integer | No | Kununu discipline or field-of-activity IDs from 1001 to 1022. Constraints: maximum 10 items. |
| `benefits` | array of string | No | Filter jobs by employer benefits. |
| `is_top_company` | boolean | No | Only return employers with Kununu Top Company status. |
| `include_raw_job` | boolean | No | Move unmapped Scrappa job fields into raw_job instead of spreading them at the top level. Mapped fields remain at the top level. |

## Pricing

**Current live price:** $0.30 per 1,000 results.

The price is based on the latest live Apify pricing entry. Per-result charges depend on the number of billed results returned. If the price line lists subscription tiers, the rate shown for each tier applies to that Apify subscription level.

## FAQ

### Is it legal to scrape this data?

This Actor is intended for data that is publicly available from kununu. You are responsible for following the source site's terms, privacy and copyright rules, and the laws that apply to your use of the data. Only collect information you have a lawful basis to use.

### What limits should I expect?

Use the field constraints and limits in the input table. The source controls which records are available, so a narrow query or unavailable page can return fewer results or none. Keep batch lists small when you need a quick first run.

### Can I run it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/actors/thescrappa~kununu-jobs-scraper/runs`. The response includes `defaultDatasetId`; use it to fetch the run's dataset items. See the [Apify Run Actor API](https://docs.apify.com/api/v2/actors-runs-post) and [Actor runs guide](https://docs.apify.com/api/v2/actors-actor-runs). Apify integrations for [Make](https://docs.apify.com/integrations/make), [Zapier](https://docs.apify.com/integrations/zapier), and [n8n](https://docs.apify.com/integrations/n8n) can trigger runs and pass results to the next workflow step. Send dataset rows to Google Sheets through a Sheets step or a workflow integration.

### What happens if a request fails?

Apify reports input validation and source request errors in the run details. Review the error, correct the input, and retry after a temporary source problem. Depending on when a request stops, the dataset may be empty or contain results collected before the failure.

## Related Scrappa Actors

- [Arbeitsagentur Jobs Scraper for Candidate Research](https://apify.com/thescrappa/arbeitsagentur-jobs-scraper)
- [Google Jobs Data Scraper for Hiring Teams](https://apify.com/thescrappa/google-jobs-scraper)
- [Indeed Job Listings Scraper for Hiring Teams](https://apify.com/thescrappa/indeed-jobs-scraper)
- [LinkedIn Job Details Scraper for Hiring Teams](https://apify.com/thescrappa/linkedin-job-details-scraper)
- [LinkedIn Jobs Search Scraper for Hiring Teams](https://apify.com/thescrappa/linkedin-jobs-search-scraper)
