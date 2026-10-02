# Kununu Top Companies Scraper

Browse Kununu top-company rankings with employer scores, review totals, recommendation rates, locations, benefits, and award information. Batch country markets and page results.

## What data can you extract?

| Field | Type | Description |
| --- | --- | --- |
| `industryId` | Integer | IndustryId value associated with this kununu top companies record. |
| `location` | Object | Location information reported for this source record. |
| `logo` | String | Logo value associated with this kununu top companies record. |
| `name` | String | Name displayed by the source for this record. |
| `uuid` | String | Uuid value associated with this kununu top companies record. |
| `score` | Object | Numeric rating or score reported by the source. |
| `slug` | String | Short profile or listing identifier used in the source URL. |
| `isTopCompany` | Boolean | True when the source marks this employer as a top company. |
| `totalJobs` | Integer | Number of current job listings reported for this employer. |
| `totalReviews` | Integer | Total review count reported for this company. |
| `isVerified` | Boolean | IsVerified value associated with this kununu top companies record. |
| `recommendationRate` | Integer | RecommendationRate value associated with this kununu top companies record. |
| `salarySatisfaction` | Number | SalarySatisfaction value associated with this kununu top companies record. |
| `benefits` | Array\<String\> | Public benefit labels associated with this employer. |
| `type` | String | Type value associated with this kununu top companies record. |
| `awards` | Array\<Object\> | Awards value associated with this kununu top companies record. |
| `scoreBreakdown` | Array\<Object\> | ScoreBreakdown value associated with this kununu top companies record. |
| `snippets` | Array\<Object\> | Snippets value associated with this kununu top companies record. |
| `url` | String | Public source or listing URL associated with this record. |
| `input_country` | String | Kununu country market code used for this ranking request. |
| `scraped_at` | String | UTC date and time when this record was collected. |

## Use cases

- Job seekers can discover highly rated employers in a market.
- Recruiters can compare public employer scores and review totals.
- Workplace analysts can study award, benefits, and recommendation data.

## How to use

1. Add a country code to each entry in `countries`.
2. Choose an award year, page, and sort order when needed.
3. Set `maxPages` and `maxResults` to bound the ranking collection.

```json
{
  "countries": [
    {
      "country": "de"
    }
  ],
  "page": 1,
  "sort": "number-reviews-desc",
  "maxResults": 10,
  "maxPages": 1
}
```

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `countries` | Array\<object\> | Yes | Country markets whose ranked Kununu top-company lists you want to retrieve. |
| `countries[].country` | string | Yes per entry | Kununu country market code, such as de, at, or ch. Accepted values: de, at, ch. |
| `countries[].year` | integer | No | Kununu top-company award year to request. |
| `countries[].page` | integer | No | One-based result page to request. |
| `countries[].sort` | string | No | Sort order for returned search results. Accepted values: number-reviews-desc. |
| `year` | integer | No | Kununu top-company award year to request. |
| `page` | integer | No | One-based result page to request. |
| `sort` | string | No | Sort order for returned search results. Accepted values: number-reviews-desc. |
| `maxResults` | integer | No | Maximum dataset items to save across this entire run. |
| `maxPages` | integer | No | Maximum pages to request for each batch entry. |

## Output example

```json
{
  "industryId": 12,
  "location": {
    "city": "Walldorf",
    "countryCode": "de"
  },
  "logo": "https://assets.kununu.com/logos/northstar.svg",
  "name": "Northstar Analytics GmbH",
  "uuid": "e2b0e11a-b612-4d09-b3ee-8c4e76535fc1",
  "score": {
    "value": 4.4,
    "rounded": 4
  },
  "slug": "northstar-analytics",
  "isTopCompany": true,
  "totalJobs": 34,
  "totalReviews": 810,
  "isVerified": true,
  "recommendationRate": 88,
  "salarySatisfaction": 4.1,
  "benefits": [
    "Flexible hours",
    "Remote work"
  ],
  "type": "company",
  "awards": [
    {
      "name": "Top Company",
      "detail": "Kununu Top Company Award",
      "badges": [
        {
          "image": "https://assets.kununu.com/badges/top-company.svg",
          "year": 2026,
          "name": "Top Company"
        }
      ]
    }
  ],
  "scoreBreakdown": [
    {
      "id": "work-life-balance",
      "score": 4.5,
      "rounded_score": 5
    }
  ],
  "snippets": [
    {
      "uuid": "snippet-241",
      "text": "Supportive team and clear goals."
    }
  ],
  "url": "https://www.kununu.com/de/northstar-analytics",
  "input_country": "de",
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

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~kununu-top-companies-scraper/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### Which country codes are supported?

Kununu top-company data supports Germany, Austria, and Switzerland with `de`, `at`, and `ch`.

## Related Scrappa Actors

- [Kununu Company Search Scraper](https://apify.com/thescrappa/kununu-company-search-scraper)
- [Kununu Company Profile Scraper](https://apify.com/thescrappa/kununu-company-details-scraper)
- [Kununu Jobs Scraper](https://apify.com/thescrappa/kununu-jobs-scraper)
