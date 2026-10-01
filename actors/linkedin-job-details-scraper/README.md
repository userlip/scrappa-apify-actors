# LinkedIn Job Details Scraper

Review a LinkedIn job post with title, employer, location and employment details. Pass a public LinkedIn job URL or a list of URLs to inspect several vacancies in one run.

## What data can you extract?

Profile, company and post details reflect public LinkedIn pages; the source may omit optional fields.

| Field | Type | Description |
| --- | --- | --- |
| `success` | boolean | Whether the lookup completed successfully; false is a reported value, while null means LinkedIn provided no flag. |
| `title` | text | Title of the job listing, as shown by LinkedIn; null when no title is published. |
| `company` | text | Company shown for the job listing by LinkedIn, in the format used by the source; null when it is omitted. |
| `location` | text | Location shown for the job listing by LinkedIn, in the format used by the source; null when it is omitted. |
| `employment_type` | text | Employment type shown for the job listing by LinkedIn; null when LinkedIn does not provide the value. |
| `seniority_level` | text | Seniority level shown for the job listing by LinkedIn, in the format used by the source; null when it is omitted. |
| `posted_date` | date | Job posting date for this job listing shown by LinkedIn, in YYYY-MM-DD when the source provides a calendar date; null if the source omits the date. |
| `applicants` | text | Applicants shown for the job listing by LinkedIn, in the format used by the source; null when it is omitted. |
| `apply_url` | link | Application url for this job listing on LinkedIn; null when the source does not provide a URL. |
| `url` | link | Source page url for this job listing on LinkedIn; null when the source does not provide a URL. |
| `input_url` | link | Source page url passed to LinkedIn. This input value is copied into the output row; null when it was not supplied. |
| `normalized_url` | link | Normalized url for this job listing on LinkedIn; null when the source does not provide a URL. |
| `status_code` | number | Http status code shown for the job listing by LinkedIn, in the format used by the source; null when it is omitted. |
| `message` | text | Diagnostic text for the LinkedIn lookup; null when the request completes without an error. |
| `error_type` | text | Diagnostic text for the LinkedIn lookup; null when the request completes without an error. |
| `error` | text | Diagnostic text for the LinkedIn lookup; null when the request completes without an error. |

## Use cases

- Recruiters can compare titles, locations and employers while mapping an open role market.
- Hiring teams can track posting requirements and employment terms across vacancies.
- Job boards can refresh vacancy records from source pages their users follow.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Put the supported targets in `urls` and use the identifier or URL format required by LinkedIn.
3. Start the run and open its default dataset to inspect or download the rows.

```json
{
  "urls": [
    "https://www.linkedin.com/jobs/view/senior-software-engineer-remote-at-givedirectly-4360951742"
  ]
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `urls` | array of string | No | Recommended. Process many LinkedIn job URLs in one Apify run so run startup and storage overhead are shared across results. Constraints: minimum 1 items. |
| `url` | string | No | Backward-compatible single job URL. Prefer URLs for normal usage, especially when processing more than one job. |
| `use_cache` | boolean | No | Use cached data if available to reduce costs and speed up requests. |
| `maximum_cache_age` | integer | No | Maximum age of cached data in seconds (default: 2592000 = 30 days). Must be at least 1 second. Only used if use_cache is enabled. Constraints: minimum 1. |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "title": "Senior Product Analyst, Retail Insights",
  "location": "Seattle, WA",
  "url": "https://listings.example.com/record/731-alder-way",
  "company": "Northstar Market Labs",
  "employment_type": "Full time",
  "seniority_level": "Mid-Senior level",
  "posted_date": "2026-09-25",
  "applicants": "Applicants for the job listing on LinkedIn"
}
```

## Pricing

**Current live price:** $0.30 per 1,000 results.

Each completed profile or detail lookup counts as one result.

## FAQ

### Is it legal to collect public information?

This Actor is for information visible on the public source pages it reads. You are responsible for checking the source terms, privacy and copyright rules, and the laws that apply to your use.

### How many records will a run return?

The result count depends on the input limits, pagination settings and what the source makes available. A small query or unavailable page can return fewer records, including none.

### Can I call it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~linkedin-job-details-scraper/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### Which LinkedIn URL does Job Details accept?

Submit the public LinkedIn job-post URL in `url`, or provide several through `urls`. Expired or restricted job pages may return an error or incomplete details.

## Related Scrappa Actors

- [LinkedIn Company Scraper](https://apify.com/thescrappa/linkedin-company-scraper)
- [LinkedIn Jobs Search Scraper](https://apify.com/thescrappa/linkedin-jobs-search-scraper)
- [LinkedIn Post Scraper](https://apify.com/thescrappa/linkedin-post-scraper)
- [LinkedIn Profile Scraper](https://apify.com/thescrappa/linkedin-profile-scraper)
- [LinkedIn Search Scraper](https://apify.com/thescrappa/linkedin-search-scraper)
