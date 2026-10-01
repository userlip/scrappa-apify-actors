# LinkedIn Company Scraper

Look up a LinkedIn company profile with industry, employee size, website and follower count. Use public company profile URLs, individually or as a batch, to compare company details.

## What data can you extract?

Company profile details reflect the public LinkedIn company page; some optional fields may be absent.

| Field | Type | Description |
| --- | --- | --- |
| `name` | text | Name of the company profile, as shown by LinkedIn; null when no name is published. |
| `industry` | text | Industry category shown for the company profile by LinkedIn; null when LinkedIn does not provide the value. |
| `size` | text | Size shown for the company profile by LinkedIn; null when LinkedIn does not provide the value. |
| `website` | link | Website url for this company profile on LinkedIn; null when the source does not provide a URL. |
| `followers` | number | Number of followers shown by LinkedIn, as a whole number; zero is possible, and null means no count was reported. |
| `employee_count` | number | Number of employees shown by LinkedIn, as a whole number; zero is possible, and null means no count was reported. |
| `type` | text | Category assigned to the company profile by LinkedIn; null when LinkedIn does not provide the value. |

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
    "https://www.linkedin.com/company/microsoft"
  ]
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `urls` | array of string | Conditional | Required unless `url` is supplied. Process multiple LinkedIn company page URLs in one run. Constraints: minimum 1 items. |
| `url` | string | Conditional | Required unless `urls` is supplied. Accepts one LinkedIn company page URL. |
| `use_cache` | boolean | No | Whether to use cached results if available |
| `maximum_cache_age` | integer | No | Maximum age of cached results in seconds. Only used when 'Use Cache' is enabled. Constraints: minimum 1. |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "name": "Northstar Market Labs",
  "followers": 18600,
  "industry": "Retail technology",
  "size": "51-200 employees",
  "website": "https://northstar.example",
  "employee_count": 840,
  "type": "Video"
}
```

## Pricing

**Current live price:** $0.30 per 1,000 results.

Each saved dataset record counts as one result.

## FAQ

### Is it legal to collect public information?

This Actor is for information visible on the public source pages it reads. You are responsible for checking the source terms, privacy and copyright rules, and the laws that apply to your use.

### How many records will a run return?

The Actor processes submitted URLs in order and can save one profile row per company. The run's remaining charge budget can limit saved rows; after it is exhausted, later URLs may still be processed without adding rows to the dataset.

### Can I call it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~linkedin-company-scraper/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### Can I look up several LinkedIn companies in one run?

Yes. Use `urls` for several public company profile URLs or `url` for one. The Actor reads publicly available company profile details.

## Related Scrappa Actors

- [LinkedIn Job Details Scraper](https://apify.com/thescrappa/linkedin-job-details-scraper)
- [LinkedIn Jobs Search Scraper](https://apify.com/thescrappa/linkedin-jobs-search-scraper)
- [LinkedIn Post Scraper](https://apify.com/thescrappa/linkedin-post-scraper)
- [LinkedIn Profile Scraper](https://apify.com/thescrappa/linkedin-profile-scraper)
- [LinkedIn Search Scraper](https://apify.com/thescrappa/linkedin-search-scraper)
