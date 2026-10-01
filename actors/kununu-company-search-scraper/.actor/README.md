# Kununu Company Search Scraper

Search public Kununu company profiles by name or keyword and collect employer slugs, locations, industry IDs, ratings, review totals, and profile links. Batch several searches in one run.

## What data can you extract?

| Field | Type | Description |
| --- | --- | --- |
| `id` | String | Stable source identifier for this record. |
| `uuid` | String | Uuid value associated with this kununu company search record. |
| `name` | String | Name displayed by the source for this record. |
| `slug` | String | Short profile or listing identifier used in the source URL. |
| `url` | String | Public source or listing URL associated with this record. |
| `industry` | Integer | Industry value associated with this kununu company search record. |
| `location` | Object | Location information reported for this source record. |
| `ratings` | Object | Rating values or rating counts reported by the source. |
| `reviewCount` | Integer | Number of reviews reported for this company or property. |
| `totalJobs` | Integer | Number of current job listings reported for this employer. |
| `isTopCompany` | Boolean | True when the source marks this employer as a top company. |
| `benefits` | Array\<String\> | Public benefit labels associated with this employer. |
| `input_query` | String | Company name or keyword submitted for the Kununu search. |
| `scraped_at` | String | UTC date and time when this record was collected. |

## Use cases

- Recruiters can find employer profiles and review totals by company name.
- Labor-market analysts can compare public ratings across employers.
- B2B researchers can build company lists with location and industry context.

## How to use

1. Add a company name or keyword to each entry in `queries`.
2. Choose country filters and set `limit` and `maxPages` for each search.
3. Use `maxResults` to cap saved company matches across the run.

```json
{
  "queries": [
    {
      "query": "sap"
    }
  ],
  "limit": 10,
  "maxResults": 10,
  "maxPages": 1,
  "country_filter": "de"
}
```

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `queries` | Array\<object\> | Yes | Company names or keywords to search across public Kununu employer profiles. |
| `queries[].query` | string | Yes per entry | Company name or keyword used to search Kununu employer profiles. |
| `queries[].limit` | integer | No | Maximum number of company matches requested for this search page. |
| `queries[].offset` | integer | No | Number of company matches to skip before returning the next page. |
| `queries[].country_filter` | string | No | One Kununu country code used to filter search matches. The code is sent as one countries\[\] value. |
| `queries[].industry` | integer | No | Numeric Kununu industry identifier used to narrow a company search. |
| `limit` | integer | No | Maximum number of company matches requested for this search page. |
| `offset` | integer | No | Number of company matches to skip before returning the next page. |
| `country_filter` | string | No | One Kununu country code used to filter search matches. The code is sent as one countries\[\] value. |
| `industry` | integer | No | Numeric Kununu industry identifier used to narrow a company search. |
| `maxResults` | integer | No | Maximum dataset items to save across this entire run. |
| `maxPages` | integer | No | Maximum pages to request for each batch entry. |

## Output example

```json
{
  "id": "company-2108",
  "uuid": "0b4f44e3-65a8-4f3a-b435-7c66e5d555c4",
  "name": "SAP SE",
  "slug": "sap-se",
  "url": "https://www.kununu.com/de/sap-se",
  "industry": 12,
  "location": {
    "city": "Walldorf",
    "countryCode": "de"
  },
  "ratings": {
    "overall": 4.1,
    "rounded": 4
  },
  "reviewCount": 23840,
  "totalJobs": 126,
  "isTopCompany": true,
  "benefits": [
    "Flexible working hours"
  ],
  "input_query": "sap",
  "scraped_at": "2026-10-01T12:00:00Z"
}
```

## Pricing

$0.30 per 1,000 results. Apify saves one dataset item per result and applies the Actor’s per-result price to saved items.

Pagination follows the source response. Set **maxPages** per batch entry and **maxResults** across the run.

## FAQ

### Is scraping this information legal?

Rules depend on the source, location, data type, and intended use. Check applicable laws and source terms, and make sure your workflow follows privacy and data protection requirements.

### What limits apply?

Submit up to 100 batch entries per run. Use **maxResults** to cap saved items and **maxPages** to limit pages for each entry. Results also depend on source availability and your Scrappa API plan.

### Can I start runs through the Apify API?

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~kununu-company-search-scraper/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### Can I search for a partial company name?

Yes. Use a company name fragment or keyword and review the matched profile names and slugs.

## Related Scrappa Actors

- [Kununu Company Profile Scraper](https://apify.com/thescrappa/kununu-company-details-scraper)
- [Kununu Company Salaries Scraper](https://apify.com/thescrappa/kununu-company-salaries-scraper)
- [Kununu Jobs Scraper](https://apify.com/thescrappa/kununu-jobs-scraper)
