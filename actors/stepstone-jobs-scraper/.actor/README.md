# Stepstone Jobs Scraper

Find Stepstone job listings with titles, employers, locations and salary details when available. Set a job title and location to compare current vacancies, with pay details where Stepstone provides them.

## What data can you extract?

Job fields reflect listings published on Stepstone for the selected search.

| Field | Type | Description |
| --- | --- | --- |
| `title` | text | Title of the job listing, as shown by Stepstone; null when no title is published. |
| `company_name` | text | Employer name attached to the job listing, as shown by Stepstone; null when the listing does not identify its employer. |
| `company_url` | link | Company url for this job listing on Stepstone; null when the source does not provide a URL. |
| `location_formatted` | text | Formatted location shown for the job listing by Stepstone, in the format used by the source; null when it is omitted. |
| `location_city` | text | Location city shown for the job listing by Stepstone, in the format used by the source; null when it is omitted. |
| `location_region` | text | Location region shown for the job listing by Stepstone, in the format used by the source; null when it is omitted. |
| `location_country` | text | Location country shown for the job listing by Stepstone, in the format used by the source; null when it is omitted. |
| `salary` | object | Salary range with minimum, maximum and currency from Stepstone; null when the source provides no details. |
| `date_posted` | text | Date the role was posted shown by Stepstone, in YYYY-MM-DD when the source provides a calendar date; null if the source omits the date. |
| `url` | link | Source page url for this job listing on Stepstone; null when the source does not provide a URL. |
| `skills` | array | Skills reported in this profile or job listing from Stepstone; an empty list when no entries are available. |
| `labels` | array | Labels assigned to the record by the source from Stepstone; an empty list when no entries are available. |
| `work_from_home` | boolean | Whether the job supports remote work; false is a reported value, while null means Stepstone provided no flag. |

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
  "location": "Berlin",
  "page": 1,
  "limit": 5
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `query` | string | No | Search keywords, for example software engineer, nurse, data analyst, or sales manager. Empty input defaults to a Berlin software example query. |
| `location` | string | No | City, region, or remote location, for example Berlin, Munich, Vienna, Amsterdam, Brussels, or remote. |
| `country` | string | No | Stepstone country market. Constraints: allowed values: de, at, nl, be. |
| `radius` | integer | No | Search radius from the provided location. Scrappa accepts 0 to 200. Constraints: minimum 0; maximum 200. |
| `sort` | string | No | Sort order for Stepstone results. Constraints: allowed values: relevance, date. |
| `job_type` | string | No | Filter by job type. Constraints: allowed values: full_time, part_time, internship, freelance. |
| `work_from_home` | boolean | No | Only return listings marked as work-from-home or home-office friendly. |
| `date_posted` | integer | No | Only return jobs posted in the selected number of days, for example 1, 3, 7, or 30. Constraints: minimum 1; maximum 30. |
| `page` | integer | No | Stepstone result page to fetch. Use data.pagination.next_page from a previous run to continue. Constraints: minimum 1; maximum 500. |
| `limit` | integer | No | Number of jobs to return in one request. Scrappa accepts 1 to 100. Constraints: minimum 1; maximum 100. |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "salary": {
    "min": 58000,
    "max": 76000,
    "currency": "EUR"
  },
  "title": "Senior Product Analyst, Retail Insights",
  "company_name": "Northstar Market Labs",
  "url": "https://listings.example.com/record/731-alder-way",
  "company_url": "https://source.example.com/record/market-guide",
  "location_formatted": "Seattle, WA",
  "location_city": "Seattle",
  "location_region": "Washington"
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

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~stepstone-jobs-scraper/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### How do I focus Stepstone Jobs on a role and city?

Enter the job title and location fields shown in Input. Salary and employment details are included when Stepstone publishes them.

## Related Scrappa Actors

- [Arbeitsagentur Jobs Scraper](https://apify.com/thescrappa/arbeitsagentur-jobs-scraper)
- [Google Jobs Scraper](https://apify.com/thescrappa/google-jobs-scraper)
- [Indeed Jobs Scraper](https://apify.com/thescrappa/indeed-jobs-scraper)
- [Kununu Jobs Scraper](https://apify.com/thescrappa/kununu-jobs-scraper)
- [LinkedIn Job Details Scraper](https://apify.com/thescrappa/linkedin-job-details-scraper)
