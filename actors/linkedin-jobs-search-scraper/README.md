# LinkedIn Jobs Search Scraper

Find LinkedIn job listings by title and location, with employer names and result links. Describe the role, seniority or location in the query and page through the supported Google results.

## What data can you extract?

Profile, company and post details reflect public LinkedIn pages; the source may omit optional fields.

| Field | Type | Description |
| --- | --- | --- |
| `position` | number | Result position in the LinkedIn job listing list, as a whole number; null when the source does not supply one. |
| `title` | text | Title of the job listing, as shown by LinkedIn; null when no title is published. |
| `link` | link | Result link for this job listing on LinkedIn; null when the source does not provide a URL. |
| `displayed_link` | text | Displayed domain shown for the job listing by LinkedIn, in the format used by the source; null when it is omitted. |
| `snippet` | text | Search snippet from LinkedIn for this job listing; null when the source has no text to show. |

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
  "query": "software engineer remote"
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `query` | string | No | Job search query. Include role, company, seniority, location, or hiring keywords. Empty input defaults to a narrow US example query. |
| `num` | integer | No | Number of results to return. Scrappa accepts 1 to 20 results per page. Constraints: minimum 1; maximum 20. |
| `page` | integer | No | Page number for pagination. Use either page or start, not both. Constraints: minimum 1; maximum 10. |
| `start` | integer | No | Starting result index for pagination. Use either start or page, not both. Constraints: minimum 0; maximum 170. |
| `hl` | string | No | Two-letter language code for Google's interface, for example en, de, es, or fr. |
| `gl` | string | No | Two-letter country code for geolocation, for example us, de, uk, or fr. |
| `lr` | string | No | Restrict results to a language, for example lang_en or lang_de. |
| `cr` | string | No | Restrict results to a country, for example countryUS or countryDE. |
| `safe` | string | No | Google safe search setting. Constraints: allowed values: off, active. |
| `dateRestrict` | string | No | Filter by date range, for example d7, w1, m1, or y1. |
| `sort` | string | No | Sort parameter supported by Google Search, for example date. |
| `filter` | integer | No | Enable or disable duplicate filtering. Constraints: minimum 0; maximum 1. |
| `rights` | string | No | Usage-rights filter supported by Google Search. |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "title": "Senior Product Analyst, Retail Insights",
  "link": "https://search.example.com/results/market-guide",
  "displayed_link": "northstar.example/market-guide",
  "snippet": "Plan product experiments, review retail trends and share findings with the analytics team."
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

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~linkedin-jobs-search-scraper/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### What fields can narrow a LinkedIn Jobs Search?

Use `query` for a role, company, seniority, location or hiring phrase. `gl`, `hl`, `dateRestrict` and the other listed options can further constrain Google-powered results.

## Related Scrappa Actors

- [LinkedIn Company Scraper](https://apify.com/thescrappa/linkedin-company-scraper)
- [LinkedIn Job Details Scraper](https://apify.com/thescrappa/linkedin-job-details-scraper)
- [LinkedIn Post Scraper](https://apify.com/thescrappa/linkedin-post-scraper)
- [LinkedIn Profile Scraper](https://apify.com/thescrappa/linkedin-profile-scraper)
- [LinkedIn Search Scraper](https://apify.com/thescrappa/linkedin-search-scraper)
