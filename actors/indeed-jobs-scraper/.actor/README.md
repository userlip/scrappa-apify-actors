# Indeed Jobs Scraper

Search Indeed job listings with titles, employers, locations and salary ranges when provided. Enter the role and location you want to hire for, then review salary details when Indeed publishes them.

## What data can you extract?

Job fields reflect the listings and details available on Indeed for the selected search.

| Field | Type | Description |
| --- | --- | --- |
| `title` | text | Title of the job listing, as shown by Indeed; null when no title is published. |
| `company_name` | text | Employer name attached to the job listing, as shown by Indeed; null when the listing does not identify its employer. |
| `location_formatted` | text | Formatted location shown for the job listing by Indeed, in the format used by the source; null when it is omitted. |
| `salary` | object | Salary range with minimum, maximum and currency from Indeed; null when the source provides no details. |
| `attributes` | array | Job attributes grouped by label and value from Indeed; an empty list when no entries are available. |
| `date_published` | text | Date the article was published shown by Indeed, in YYYY-MM-DD when the source provides a calendar date; null if the source omits the date. |
| `apply_url` | link | Application url for this job listing on Indeed; null when the source does not provide a URL. |

## Use cases

- Recruiters can compare titles, locations and employers while mapping an open role market.
- Hiring teams can track posting requirements and employment terms across vacancies.
- Job boards can refresh vacancy records from source pages their users follow.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Set a role or keyword and location, then apply only the job type, date and workplace filters shown in the input table.
3. Start the run and open its default dataset to inspect or download the rows.

```json
{
  "query": "software engineer",
  "location": "New York",
  "limit": 5
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

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

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "title": "Senior Product Analyst, Retail Insights",
  "company_name": "Northstar Market Labs",
  "location_formatted": "Seattle, WA",
  "salary": {
    "min": 68000,
    "max": 92000,
    "currency": "USD"
  },
  "attributes": [
    {
      "label": "Work model",
      "value": "Hybrid"
    },
    {
      "label": "Schedule",
      "value": "Full time"
    }
  ],
  "date_published": "2026-09-25",
  "apply_url": "https://jobs.example.com/product-analyst-7814/apply"
}
```

## Pricing

**Current live price:** $0.30 per 1,000 results.

Each saved job listing counts as one result.

## FAQ

### Is it legal to collect public information?

This Actor is for information visible on the public source pages it reads. You are responsible for checking the source terms, privacy and copyright rules, and the laws that apply to your use.

### How many records will a run return?

The result count depends on the input limits, pagination settings and what the source makes available. A small query or unavailable page can return fewer records, including none.

### Can I call it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~indeed-jobs-scraper/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### Can Indeed Jobs return salary ranges for every listing?

Salary fields appear only when the Indeed listing includes pay information. Search by role and location, and review the output for available salary details.

## Related Scrappa Actors

- [Arbeitsagentur Jobs Scraper](https://apify.com/thescrappa/arbeitsagentur-jobs-scraper)
- [Google Jobs Scraper](https://apify.com/thescrappa/google-jobs-scraper)
- [Kununu Jobs Scraper](https://apify.com/thescrappa/kununu-jobs-scraper)
- [LinkedIn Job Details Scraper](https://apify.com/thescrappa/linkedin-job-details-scraper)
- [LinkedIn Jobs Search Scraper](https://apify.com/thescrappa/linkedin-jobs-search-scraper)
