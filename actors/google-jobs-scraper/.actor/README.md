# Google Jobs Scraper

Find Google Jobs postings with job titles, employers, locations and available salary details. Use a role or employer phrase and the supported market settings to find matching openings.

## What data can you extract?

Job titles, employers and locations reflect the postings Google Jobs displays for the selected search.

| Field | Type | Description |
| --- | --- | --- |
| `title` | text | Title of the job listing, as shown by Google Jobs; null when no title is published. |
| `company` | text | Company shown for the job listing by Google Jobs, in the format used by the source; null when it is omitted. |
| `company_name` | text | Employer name attached to the job listing, as shown by Google Jobs; null when the listing does not identify its employer. |
| `location` | text | Location shown for the job listing by Google Jobs, in the format used by the source; null when it is omitted. |
| `via` | text | Road or route waypoint shown for the job listing by Google Jobs, in the format used by the source; null when it is omitted. |
| `description` | text | Description text from Google Jobs for this job listing; null when the source has no text to show. |
| `job_id` | text | job ID for the job listing, assigned by Google Jobs; null when the source does not expose it. |

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
  "q": "nurse jobs in Austin"
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `q` | string | No | Job search query. Leave empty only when using a next page token. A missing or blank `q` uses the Actor’s default US job query when no `next_page_token` is supplied. |
| `next_page_token` | string | No | Pagination token returned by a previous Google Jobs response. When provided, the query can be omitted. |
| `gl` | string | No | Two-letter country code for search results (e.g., 'us', 'uk', 'de', 'fr') |
| `hl` | string | No | Two-letter language code for the interface (e.g., 'en', 'de', 'es', 'fr') |
| `google_domain` | string | No | Google domain to query (e.g., 'google.com', 'google.de', 'google.co.uk') |
| `uule` | string | No | Google-encoded location parameter for precise geolocation. |
| `lrad` | integer | No | Search radius around the encoded location in miles. Supported values: 5, 10, 25, 50, or 100. Requires uule. Constraints: minimum 5; maximum 100. |
| `uds` | string | No | Dynamic Google Jobs filter string returned in the filters section of a previous response. |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "title": "Senior Product Analyst, Retail Insights",
  "company_name": "Northstar Market Labs",
  "description": "Plan product experiments, review retail trends and share findings with the analytics team.",
  "location": "Seattle, WA",
  "company": "Northstar Market Labs",
  "via": "Company website",
  "job_id": "job_7814"
}
```

## Pricing

**Current live price:** Apify Free tier: $0.30 per 1,000 results; Bronze: $0.25 per 1,000 results; Silver: $0.22 per 1,000 results; Gold, Platinum, and Diamond: $0.20 per 1,000 results.

Each saved job listing counts as one result.

## FAQ

### Is it legal to collect public information?

This Actor is for information visible on the public source pages it reads. You are responsible for checking the source terms, privacy and copyright rules, and the laws that apply to your use.

### How many records will a run return?

The result count depends on the input limits, pagination settings and what the source makes available. A small query or unavailable page can return fewer records, including none.

### Can I call it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~google-jobs-scraper/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### How do I narrow Google Jobs results to a location?

Search for a role or phrase using the required query and use the supported location or locale inputs. Salary details appear only when Google Jobs provides them.

## Related Scrappa Actors

- [Arbeitsagentur Jobs Scraper](https://apify.com/thescrappa/arbeitsagentur-jobs-scraper)
- [Indeed Jobs Scraper](https://apify.com/thescrappa/indeed-jobs-scraper)
- [Kununu Jobs Scraper](https://apify.com/thescrappa/kununu-jobs-scraper)
- [LinkedIn Job Details Scraper](https://apify.com/thescrappa/linkedin-job-details-scraper)
- [LinkedIn Jobs Search Scraper](https://apify.com/thescrappa/linkedin-jobs-search-scraper)
