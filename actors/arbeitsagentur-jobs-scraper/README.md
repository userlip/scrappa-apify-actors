# Arbeitsagentur Jobs Scraper for Candidate Research

The Arbeitsagentur Jobs Scraper for Candidate Research collects job listings and job details from Germany Federal Employment Agency. Provide the fields listed below; the actor saves source fields such as `title`, `occupation`, `company_name`, and `location_formatted` to an Apify dataset.

## What data can you extract?

The dataset contains fields returned by Germany Federal Employment Agency. The field names below match the Actor output schema. A source may leave optional values empty or omit fields when they are not available for a result.

| Field | Type | Description |
| --- | --- | --- |
| `title` | text | Title returned for this result. |
| `occupation` | text | Occupation returned for this result. |
| `company_name` | text | Employer returned for this result. |
| `location_formatted` | text | Location returned for this result. |
| `location_city` | text | City returned for this result. |
| `postal_code` | text | Postal Code returned for this result. |
| `region` | text | Region returned for this result. |
| `country` | text | Country returned for this result. |
| `latitude` | number | Latitude returned for this result. |
| `longitude` | number | Longitude returned for this result. |
| `published_date` | text | Published returned for this result. |
| `start_date` | text | Start Date returned for this result. |
| `job_url` | link | Job URL returned for this result. |
| `reference_number` | text | Reference returned for this result. |
| `distance_km` | text | Distance returned for this result. |

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
  "was": "Software Entwickler",
  "wo": "Berlin",
  "arbeitszeit": "vz;ho",
  "page": 1
}
```

Apify stores the run output in a dataset. You can download the dataset in JSON, CSV, Excel, XML, or other available formats from the run page.

## Output example

This illustrative record uses synthetic values. It shows the real output field names; optional source values may be null or absent.

```json
{
  "title": "Example result",
  "occupation": "Example value",
  "company_name": "Example Company",
  "location_formatted": "Example location",
  "location_city": "New York",
  "postal_code": "Example value",
  "region": "Example value"
}
```

## Input fields

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `was` | string | No | Job title, keyword, or occupation, for example Software Entwickler, Pflegefachkraft, Ausbildung, or Elektriker. |
| `wo` | string | No | City, postal code, or region, for example Berlin, Hamburg, Munich, or 10115. |
| `umkreis` | integer | No | Search radius in kilometers around the selected location. Constraints: minimum 10; maximum 200. |
| `angebotsart` | integer | No | Arbeitsagentur offer type. Constraints: minimum 1; maximum 34. |
| `arbeitszeit` | string | No | Working-hours filters. Use one or semicolon-separated values: vz full-time, tz part-time, snw shift/night/weekend, ho home office, mj mini-job. |
| `veroeffentlichtseit` | integer | No | Only return jobs published in the selected number of days. Constraints: minimum 0; maximum 100. |
| `berufsfeld` | string | No | Optional Arbeitsagentur occupational field code. |
| `arbeitgeber` | string | No | Filter by employer or company name. |
| `befristung` | integer | No | Contract duration filter. Constraints: minimum 1; maximum 2. |
| `zeitarbeit` | boolean | No | Include temporary agency jobs. |
| `pav` | boolean | No | Include listings from private employment agencies. |
| `page` | integer | No | Arbeitsagentur result page to fetch. Pagination starts at 1. Constraints: minimum 1; maximum 500. |
| `size` | integer | No | Number of jobs to return in one request. Scrappa accepts 1 to 100. Constraints: minimum 1; maximum 100. |

## Pricing

**Current live price:** $0.30 per 1,000 results.

The price is based on the latest live Apify pricing entry. Per-result charges depend on the number of billed results returned. If the price line lists subscription tiers, the rate shown for each tier applies to that Apify subscription level.

## FAQ

### Is it legal to scrape this data?

This Actor is intended for data that is publicly available from Germany Federal Employment Agency. You are responsible for following the source site's terms, privacy and copyright rules, and the laws that apply to your use of the data. Only collect information you have a lawful basis to use.

### What limits should I expect?

Use the field constraints and limits in the input table. The source controls which records are available, so a narrow query or unavailable page can return fewer results or none. Keep batch lists small when you need a quick first run.

### Can I run it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/actors/thescrappa~arbeitsagentur-jobs-scraper/runs`. The response includes `defaultDatasetId`; use it to fetch the run's dataset items. See the [Apify Run Actor API](https://docs.apify.com/api/v2/actors-runs-post) and [Actor runs guide](https://docs.apify.com/api/v2/actors-actor-runs). Apify integrations for [Make](https://docs.apify.com/integrations/make), [Zapier](https://docs.apify.com/integrations/zapier), and [n8n](https://docs.apify.com/integrations/n8n) can trigger runs and pass results to the next workflow step. Send dataset rows to Google Sheets through a Sheets step or a workflow integration.

### What happens if a request fails?

Apify reports input validation and source request errors in the run details. Review the error, correct the input, and retry after a temporary source problem. Depending on when a request stops, the dataset may be empty or contain results collected before the failure.

## Related Scrappa Actors

- [Google Jobs Data Scraper for Hiring Teams](https://apify.com/thescrappa/google-jobs-scraper)
- [Indeed Job Listings Scraper for Hiring Teams](https://apify.com/thescrappa/indeed-jobs-scraper)
- [Kununu Jobs Scraper for Employer Research](https://apify.com/thescrappa/kununu-jobs-scraper)
- [LinkedIn Job Details Scraper for Hiring Teams](https://apify.com/thescrappa/linkedin-job-details-scraper)
- [LinkedIn Jobs Search Scraper for Hiring Teams](https://apify.com/thescrappa/linkedin-jobs-search-scraper)
