# Kununu Jobs Scraper

Find jobs listed on kununu with titles, employers, company ratings and work arrangements. Search the role or company you have in mind and review work arrangement and employer ratings when shown.

## What data can you extract?

Employer ratings and review details follow public kununu pages; a review may not include every field.

| Field | Type | Description |
| --- | --- | --- |
| `title` | text | Title of the job listing, as shown by kununu; null when no title is published. |
| `company_name` | text | Employer name attached to the job listing, as shown by kununu; null when the listing does not identify its employer. |
| `company_score` | number | Employer score for this job listing, on kununu’s employer rating scale; null when no score is shown. |
| `company_is_top_company` | boolean | Whether kununu marks the employer as a top company; false is a reported value, while null means kununu provided no flag. |
| `location_formatted` | text | Formatted location shown for the job listing by kununu, in the format used by the source; null when it is omitted. |
| `workplace` | text | Workplace shown for the job listing by kununu, in the format used by the source; null when it is omitted. |
| `workplace_model` | text | Work arrangement shown for the job listing by kununu; null when kununu does not provide the value. |
| `employment_type` | text | Employment type shown for the job listing by kununu; null when kununu does not provide the value. |
| `employment_types` | array | Employment types offered for the role, such as full-time or part-time from kununu; an empty list when no entries are available. |
| `career_level` | text | Career level shown for the job listing by kununu, in the format used by the source; null when it is omitted. |
| `benefits` | array | Benefits reported for this role from kununu; an empty list when no entries are available. |
| `salary` | object | Salary range with minimum, maximum and currency from kununu; null when the source provides no details. |
| `date_posted` | text | Date the role was posted shown by kununu, in YYYY-MM-DD when the source provides a calendar date; null if the source omits the date. |
| `posted_at` | text | Time the post was published shown by kununu, in ISO 8601 date and time; null if the source omits the date. |
| `job_url` | link | Job page url for this job listing on kununu; null when the source does not provide a URL. |

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
  "query": "Software Engineer",
  "location": "Berlin",
  "page": 1
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

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

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "employment_types": [
    "Full time",
    "Part time"
  ],
  "title": "Senior Product Analyst, Retail Insights",
  "company_name": "Northstar Market Labs",
  "company_score": 4.3,
  "company_is_top_company": true,
  "location_formatted": "Seattle, WA",
  "workplace": "Seattle, WA",
  "workplace_model": "Hybrid"
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

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~kununu-jobs-scraper/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### Can I search kununu jobs by company and role?

Use the supported job query and location fields from Input. The Actor returns public vacancies and employer details available on kununu.

## Related Scrappa Actors

- [Arbeitsagentur Jobs Scraper](https://apify.com/thescrappa/arbeitsagentur-jobs-scraper)
- [Google Jobs Scraper](https://apify.com/thescrappa/google-jobs-scraper)
- [Indeed Jobs Scraper](https://apify.com/thescrappa/indeed-jobs-scraper)
- [LinkedIn Job Details Scraper](https://apify.com/thescrappa/linkedin-job-details-scraper)
- [LinkedIn Jobs Search Scraper](https://apify.com/thescrappa/linkedin-jobs-search-scraper)
