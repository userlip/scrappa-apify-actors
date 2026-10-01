# Arbeitsagentur Jobs Scraper

Find German job listings from the Federal Employment Agency with titles, employers, locations and work details. Set a job title or occupation and a location, then narrow listings by work type or posting date.

## What data can you extract?

Job fields reflect listings published by Germany’s Federal Employment Agency.

| Field | Type | Description |
| --- | --- | --- |
| `title` | text | Title of the job listing, as shown by Germany’s Federal Employment Agency; null when no title is published. |
| `occupation` | text | Occupation shown for the job listing by Germany’s Federal Employment Agency; null when Germany’s Federal Employment Agency does not provide the value. |
| `company_name` | text | Employer name attached to the job listing, as shown by Germany’s Federal Employment Agency; null when the listing does not identify its employer. |
| `location_formatted` | text | Formatted location shown for the job listing by Germany’s Federal Employment Agency, in the format used by the source; null when it is omitted. |
| `location_city` | text | Location city shown for the job listing by Germany’s Federal Employment Agency, in the format used by the source; null when it is omitted. |
| `postal_code` | text | Postal code shown for the job listing by Germany’s Federal Employment Agency, in the format used by the source; null when it is omitted. |
| `region` | text | Region shown for the job listing by Germany’s Federal Employment Agency; null when Germany’s Federal Employment Agency does not provide the value. |
| `country` | text | Country shown for the job listing by Germany’s Federal Employment Agency; null when Germany’s Federal Employment Agency does not provide the value. |
| `latitude` | number | Latitude for this job listing on Germany’s Federal Employment Agency, in decimal degrees; null when the source provides no coordinates. |
| `longitude` | number | Longitude for this job listing on Germany’s Federal Employment Agency, in decimal degrees; null when the source provides no coordinates. |
| `published_date` | text | Date this review was published shown by Germany’s Federal Employment Agency, in YYYY-MM-DD when the source provides a calendar date; null if the source omits the date. |
| `start_date` | text | Start date for this job listing shown by Germany’s Federal Employment Agency, in YYYY-MM-DD when the source provides a calendar date; null if the source omits the date. |
| `job_url` | link | Job page url for this job listing on Germany’s Federal Employment Agency; null when the source does not provide a URL. |
| `reference_number` | text | source reference number for the job listing, assigned by Germany’s Federal Employment Agency; null when the source does not expose it. |
| `distance_km` | text | Distance for this job listing, in kilometers; null when no distance is available. |

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
  "was": "Software Entwickler",
  "wo": "Berlin",
  "arbeitszeit": "vz;ho",
  "page": 1
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

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

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "title": "Senior Product Analyst, Retail Insights",
  "company_name": "Northstar Market Labs",
  "published_date": "2026-09-25",
  "occupation": "Product analyst",
  "location_formatted": "Seattle, WA",
  "location_city": "Seattle",
  "postal_code": "98101",
  "region": "Washington"
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

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~arbeitsagentur-jobs-scraper/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### How can I find Federal Employment Agency jobs by occupation?

Enter a job title or occupation and a location. Use the listed work-type and posting-date filters to narrow the German job listings.

## Related Scrappa Actors

- [Google Jobs Scraper](https://apify.com/thescrappa/google-jobs-scraper)
- [Indeed Jobs Scraper](https://apify.com/thescrappa/indeed-jobs-scraper)
- [Kununu Jobs Scraper](https://apify.com/thescrappa/kununu-jobs-scraper)
- [LinkedIn Job Details Scraper](https://apify.com/thescrappa/linkedin-job-details-scraper)
- [LinkedIn Jobs Search Scraper](https://apify.com/thescrappa/linkedin-jobs-search-scraper)
