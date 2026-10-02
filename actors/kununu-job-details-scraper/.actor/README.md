# Kununu Job Details Scraper

Retrieve public Kununu job postings with title, employer, location, employment type, salary information, application link, and employer rating context. Batch job URLs in one run.

## What data can you extract?

| Field | Type | Description |
| --- | --- | --- |
| `success` | Boolean | True when Kununu returned a parsed job detail response. |
| `data` | Object | Job posting, employer, location, compensation, application, and company context. |
| `meta` | Object | Source label and fetch timing for the job detail lookup. |
| `input_url` | String | Hotel or job page URL supplied for this lookup. |
| `scraped_at` | String | UTC date and time when this record was collected. |

## Use cases

- Recruiters can review public posting details and application links.
- Job seekers can compare role pay ranges and employer ratings.
- Labor-market researchers can track details across public listings.

## How to use

1. Add a public Kununu job-posting URL to each entry in `jobs`.
2. Set `maxResults` to cap the number of job records saved.
3. Read the posting, employer, salary, and application fields from each result.

```json
{
  "jobs": [
    {
      "url": "https://www.kununu.com/de/job/4b83063a-0875-4d72-aed0-5dd03c738ec9"
    }
  ],
  "maxResults": 10
}
```

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `jobs` | Array\<object\> | Yes | Public Kununu job-posting URLs whose details you want to retrieve. |
| `jobs[].url` | string | Yes per entry | Public source page URL used to retrieve this record. |
| `maxResults` | integer | No | Maximum dataset items to save across this entire run. |

## Output example

```json
{
  "success": true,
  "data": {
    "id": "job-531902",
    "url": "https://www.kununu.com/de/job/531902",
    "title": "Senior Data Analyst",
    "company": {
      "name": "Northstar Analytics GmbH",
      "uuid": "e2b0e11a-b612-4d09-b3ee-8c4e76535fc1",
      "slug": "northstar-analytics",
      "score": 4.4,
      "isTopCompany": true,
      "industryId": 12,
      "website": "https://northstar-analytics.com",
      "countryCode": "de"
    },
    "description_html": "<p>Build data products and reporting workflows with a collaborative analytics team.</p>",
    "postedAt": "2026-09-15T08:00:00Z",
    "activeUntil": "2026-11-15T23:59:00Z",
    "city": "Berlin",
    "addressRegion": "Berlin",
    "employmentTypes": [
      "Full time"
    ],
    "salary": {
      "currency": "EUR",
      "minimum": 65000,
      "maximum": 82000
    },
    "industrySalary": {
      "countryCode": "de",
      "currency": "EUR",
      "jobTitle": "Data Analyst",
      "range": {
        "average": 64200,
        "lowerBound": 48000,
        "upperBound": 91000,
        "numberOfDataPoints": 1240
      },
      "aliases": [
        "Data Analyst",
        "Business Analyst"
      ]
    },
    "kununuJobTitle": {
      "id": 614,
      "groupId": 77,
      "title": "Data Analyst"
    },
    "application": {
      "type": "external",
      "url": "https://northstar-analytics.com/careers/data-analyst"
    },
    "satisfaction": {
      "recommendationRate": {
        "percentage": 88,
        "totalReviews": 810,
        "recommendedTotalReviews": 713,
        "notRecommendedTotalReviews": 97
      },
      "roundedScore": 4,
      "score": 4.4,
      "totalReviews": 810
    },
    "source": "company-career-page",
    "paidType": "organic",
    "industryId": 12,
    "status": "active",
    "similarJobs": [
      {
        "id": "job-531901",
        "title": "Analytics Engineer",
        "url": "https://www.kununu.com/de/job/531901",
        "postedAt": "2026-09-10T08:00:00Z",
        "city": "Berlin",
        "employmentTypes": [
          "Full time"
        ],
        "source": "company-career-page",
        "company": {
          "uuid": "e2b0e11a-b612-4d09-b3ee-8c4e76535fc1",
          "name": "Northstar Analytics GmbH",
          "slug": "northstar-analytics",
          "score": 4.4,
          "isTopCompany": true,
          "industryId": 12,
          "countryCode": "de",
          "website": "https://northstar-analytics.com"
        },
        "kununuJobTitle": {
          "id": 615,
          "title": "Analytics Engineer",
          "salaryAverage": 72000,
          "salaryLowerBound": 56000,
          "salaryUpperBound": 93000,
          "salaryDataPoints": 120
        }
      }
    ],
    "nextJobId": "job-531901"
  },
  "meta": {
    "source": "kununu",
    "duration_ms": 1320
  },
  "input_url": "https://www.kununu.com/de/job/4b83063a-0875-4d72-aed0-5dd03c738ec9",
  "scraped_at": "2026-10-01T12:00:00Z"
}
```

## Pricing

$0.50 per 1,000 results. Apify saves one dataset item per result and applies the Actor’s per-result price to saved items.

Set **maxResults** to cap the number of dataset items saved in one run.

## FAQ

### Is scraping this information legal?

Rules depend on the source, location, data type, and intended use. Check applicable laws and source terms, and make sure your workflow follows privacy and data protection requirements.

### What limits apply?

Submit up to 100 batch entries per run. Use **maxResults** to cap saved items. Results also depend on source availability and your Scrappa API plan.

### Can I start runs through the Apify API?

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~kununu-job-details-scraper/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### Can I use an expired job URL?

The source may no longer return full details for expired or removed postings. Use an active public Kununu job URL for the best chance of a complete record.

## Related Scrappa Actors

- [Kununu Jobs Scraper](https://apify.com/thescrappa/kununu-jobs-scraper)
- [Kununu Company Profile Scraper](https://apify.com/thescrappa/kununu-company-details-scraper)
- [Kununu Company Salaries Scraper](https://apify.com/thescrappa/kununu-company-salaries-scraper)
