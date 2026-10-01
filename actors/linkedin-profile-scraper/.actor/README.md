# LinkedIn Profile Scraper

Review a public LinkedIn profile with name, headline, location and follower count. Submit one or more public LinkedIn profile URLs to build a consistent profile dataset.

## What data can you extract?

Profile, company and post details reflect public LinkedIn pages; the source may omit optional fields.

| Field | Type | Description |
| --- | --- | --- |
| `name` | text | Name of the professional profile, as shown by LinkedIn; null when no name is published. |
| `location` | text | Location shown for the professional profile by LinkedIn, in the format used by the source; null when it is omitted. |
| `followers` | number | Number of followers shown by LinkedIn, as a whole number; zero is possible, and null means no count was reported. |
| `connections` | number | Number of connections shown by LinkedIn, as a whole number; zero is possible, and null means no count was reported. |
| `about` | text | Company statistics with market capitalization, average volume, exchange and related financial labels from LinkedIn; null when the source provides no details. |

## Use cases

- Teams can review source records before a follow-up decision.
- Researchers can compare available records across targets or runs.
- Analysts can use source links to maintain a focused dataset.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Put the supported targets in `urls` and use the identifier or URL format required by LinkedIn.
3. Start the run and open its default dataset to inspect or download the rows.

```json
{
  "urls": [
    "https://www.linkedin.com/in/williamhgates"
  ]
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `urls` | array of string | No | Recommended. Process many LinkedIn profile URLs in one Apify run so run startup and storage overhead are shared across results. Constraints: minimum 1 items. |
| `url` | string | No | Backward-compatible single profile URL. Prefer URLs for normal usage, especially when processing more than one profile. |
| `use_cache` | boolean | No | Use cached data if available to reduce costs and speed up requests |
| `maximum_cache_age` | integer | No | Maximum age of cached data in seconds (default: 2592000 = 30 days). Must be at least 1 second. Only used if use_cache is enabled. Constraints: minimum 1. |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "name": "Morgan Lee",
  "followers": 18600,
  "location": "Seattle, WA",
  "connections": 742,
  "about": "Product analytics leader focused on marketplace growth and customer research."
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

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~linkedin-profile-scraper/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### Can I submit several LinkedIn profile URLs together?

Yes. Add public profile URLs to `urls`, or use `url` for one profile. Only details visible to the source request can be returned.

## Related Scrappa Actors

- [LinkedIn Company Scraper](https://apify.com/thescrappa/linkedin-company-scraper)
- [LinkedIn Job Details Scraper](https://apify.com/thescrappa/linkedin-job-details-scraper)
- [LinkedIn Jobs Search Scraper](https://apify.com/thescrappa/linkedin-jobs-search-scraper)
- [LinkedIn Post Scraper](https://apify.com/thescrappa/linkedin-post-scraper)
- [LinkedIn Search Scraper](https://apify.com/thescrappa/linkedin-search-scraper)
