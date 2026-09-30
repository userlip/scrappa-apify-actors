# LinkedIn Jobs Search Scraper for Hiring Teams

The LinkedIn Jobs Search Scraper for Hiring Teams collects job listings and job details from LinkedIn. Provide a search phrase or a short list of phrases; the actor saves source fields such as `position`, `title`, `link`, and `displayed_link` to an Apify dataset.

## What data can you extract?

The dataset contains fields returned by LinkedIn. The field names below match the Actor output schema. A source may leave optional values empty or omit fields when they are not available for a result.

| Field | Type | Description |
| --- | --- | --- |
| `position` | number | Position returned for this result. |
| `title` | text | Title returned for this result. |
| `link` | link | LinkedIn URL returned for this result. |
| `displayed_link` | text | Displayed Link returned for this result. |
| `snippet` | text | Snippet returned for this result. |

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
  "query": "software engineer remote"
}
```

Apify stores the run output in a dataset. You can download the dataset in JSON, CSV, Excel, XML, or other available formats from the run page.

## Output example

This illustrative record uses synthetic values. It shows the real output field names; optional source values may be null or absent.

```json
{
  "position": 42,
  "title": "Example result",
  "link": "https://example.com/result/1",
  "displayed_link": "https://example.com/result/1",
  "snippet": "Example public text."
}
```

## Input fields

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

## Pricing

**Current live price:** $0.30 per 1,000 results.

The price is based on the latest live Apify pricing entry. Per-result charges depend on the number of billed results returned. If the price line lists subscription tiers, the rate shown for each tier applies to that Apify subscription level.

## FAQ

### Is it legal to scrape this data?

This Actor is intended for data that is publicly available from LinkedIn. You are responsible for following the source site's terms, privacy and copyright rules, and the laws that apply to your use of the data. Only collect information you have a lawful basis to use.

### What limits should I expect?

Use the field constraints and limits in the input table. The source controls which records are available, so a narrow query or unavailable page can return fewer results or none. Keep batch lists small when you need a quick first run.

### Can I run it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/actors/thescrappa~linkedin-jobs-search-scraper/runs`. The response includes `defaultDatasetId`; use it to fetch the run's dataset items. See the [Apify Run Actor API](https://docs.apify.com/api/v2/actors-runs-post) and [Actor runs guide](https://docs.apify.com/api/v2/actors-actor-runs). Apify integrations for [Make](https://docs.apify.com/integrations/make), [Zapier](https://docs.apify.com/integrations/zapier), and [n8n](https://docs.apify.com/integrations/n8n) can trigger runs and pass results to the next workflow step. Send dataset rows to Google Sheets through a Sheets step or a workflow integration.

### What happens if a request fails?

Apify reports input validation and source request errors in the run details. Review the error, correct the input, and retry after a temporary source problem. Depending on when a request stops, the dataset may be empty or contain results collected before the failure.

## Related Scrappa Actors

- [LinkedIn Company Scraper for Lead Research](https://apify.com/thescrappa/linkedin-company-scraper)
- [LinkedIn Job Details Scraper for Hiring Teams](https://apify.com/thescrappa/linkedin-job-details-scraper)
- [LinkedIn Post Scraper for Audience Research](https://apify.com/thescrappa/linkedin-post-scraper)
- [LinkedIn Profile Scraper for Lead Research](https://apify.com/thescrappa/linkedin-profile-scraper)
- [LinkedIn Search Scraper for Lead Research](https://apify.com/thescrappa/linkedin-search-scraper)
