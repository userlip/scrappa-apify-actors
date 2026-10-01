# LinkedIn Search Scraper

Find LinkedIn pages through search, with titles, displayed links and result snippets. Add a LinkedIn page type such as profile, company, post or job to the search query.

## What data can you extract?

Profile, company and post details reflect public LinkedIn pages; the source may omit optional fields.

| Field | Type | Description |
| --- | --- | --- |
| `position` | number | Result position in the LinkedIn LinkedIn search result list, as a whole number; null when the source does not supply one. |
| `title` | text | Title of the LinkedIn search result, as shown by LinkedIn; null when no title is published. |
| `link` | link | Result link for this LinkedIn search result on LinkedIn; null when the source does not provide a URL. |
| `displayed_link` | text | Displayed domain shown for the LinkedIn search result by LinkedIn, in the format used by the source; null when it is omitted. |
| `snippet` | text | Search snippet from LinkedIn for this LinkedIn search result; null when the source has no text to show. |

## Use cases

- SEO teams can check which pages and domains appear for a query.
- Communications teams can monitor snippets and links for a brand or topic.
- Researchers can compare titles and domains across locales or repeat searches.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Set `query` to the search term or source identifier you want to look up, then use the optional filters listed below.
3. Start the run and open its default dataset to inspect or download the rows.

```json
{
  "query": "site:linkedin.com/in founder AI Berlin"
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `query` | string | Yes | Google-style LinkedIn search query. Use site filters such as site:linkedin.com/in, site:linkedin.com/company, site:linkedin.com/posts, or site:linkedin.com/jobs/view to target result types. |
| `num` | integer | No | Number of organic results to return. Scrappa accepts 1 to 20 results per page. Constraints: minimum 1; maximum 20. |
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
  "title": "A practical guide to independent neighborhood shops",
  "link": "https://search.example.com/results/market-guide",
  "displayed_link": "northstar.example/market-guide",
  "snippet": "A guide to choosing containers, light and watering schedules for a compact balcony herb garden."
}
```

## Pricing

**Current live price:** $0.30 per 1,000 searches.

Each processed search or query is counted according to the rate shown above.

## FAQ

### Is it legal to collect public information?

This Actor is for information visible on the public source pages it reads. You are responsible for checking the source terms, privacy and copyright rules, and the laws that apply to your use.

### How many records will a run return?

The result count depends on the input limits, pagination settings and what the source makes available. A small query or unavailable page can return fewer records, including none.

### Can I call it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~linkedin-search-scraper/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### Can LinkedIn Search target profiles, companies, posts or jobs?

Yes. Add the appropriate `site:linkedin.com` path to `query`, such as `/in`, `/company`, `/posts` or `/jobs/view`, to focus the result type.

## Related Scrappa Actors

- [LinkedIn Company Scraper](https://apify.com/thescrappa/linkedin-company-scraper)
- [LinkedIn Job Details Scraper](https://apify.com/thescrappa/linkedin-job-details-scraper)
- [LinkedIn Jobs Search Scraper](https://apify.com/thescrappa/linkedin-jobs-search-scraper)
- [LinkedIn Post Scraper](https://apify.com/thescrappa/linkedin-post-scraper)
- [LinkedIn Profile Scraper](https://apify.com/thescrappa/linkedin-profile-scraper)
