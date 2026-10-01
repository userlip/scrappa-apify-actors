# Kununu Salary by Job Title Scraper

Retrieve Kununu salary statistics by job title and country, including pay ranges, averages, city comparisons, experience groups, and related roles. Batch titles in one run.

## What data can you extract?

| Field | Type | Description |
| --- | --- | --- |
| `success` | Boolean | True when Kununu returned salary statistics for the requested title. |
| `data` | Object | Resolved job-title statistics, salary ranges, city and experience comparisons, and related roles. |
| `meta` | Object | Resolved title, alternatives, fetch timing, and cache status. |
| `input_job_title` | String | Job title submitted for the Kununu salary lookup. |
| `scraped_at` | String | UTC date and time when this record was collected. |

## Use cases

- Job seekers can compare reported salary ranges for a target role.
- Compensation teams can review pay differences across cities and experience groups.
- Labor-market analysts can compare related jobs and salary distributions.

## How to use

1. Add a job title to each entry in `job_titles`.
2. Choose Germany, Austria, or Switzerland for the salary market.
3. Review the matched title and salary statistics in the returned object.

```json
{
  "job_titles": [
    {
      "job_title": "Softwareentwickler"
    }
  ],
  "country": "de",
  "maxResults": 10
}
```

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `job_titles` | Array\<object\> | Yes | Job titles whose Kununu salary statistics you want to retrieve. |
| `job_titles[].job_title` | string | Yes per entry | Job title to resolve to the closest available Kununu salary profile. |
| `job_titles[].country` | string | No | Kununu country market code, such as de, at, or ch. Accepted values: de, at, ch. |
| `country` | string | No | Kununu country market code, such as de, at, or ch. Accepted values: de, at, ch. |
| `maxResults` | integer | No | Maximum dataset items to save across this entire run. |

## Output example

```json
{
  "success": true,
  "data": {
    "jobTitle": {
      "currentSearchTerm": "Softwareentwickler",
      "hasSearched": true,
      "stats": {
        "jobTitle": "Softwareentwickler",
        "normalizedJobTitle": "Software Developer",
        "countryCode": "de",
        "currency": "EUR",
        "range": {
          "average": 64200,
          "minimum": 48000,
          "maximum": 91000,
          "numberOfEntries": 1240
        },
        "cities": [
          {
            "cityName": "Berlin",
            "average": 66800,
            "minBound": 51000,
            "maxBound": 92000,
            "numberOfEntries": 210
          }
        ],
        "experienceGraph": {
          "experienceGroups": [
            {
              "experienceGroup": "3 to 5 years",
              "totalAverage": 65500,
              "maleAverage": 66200,
              "femaleAverage": 64200
            }
          ]
        }
      }
    },
    "careerPaths": {
      "items": {
        "likelihood": [
          {
            "jobTitle": "Senior Software Developer",
            "likelihood": 0.38,
            "average": 74800,
            "numberOfJobs": 520
          }
        ]
      }
    },
    "educationLevels": {
      "items": [
        {
          "id": 6,
          "level": 6,
          "countryCode": "de",
          "label": "Bachelor degree"
        }
      ]
    },
    "caveat": "Salary values are estimates based on available reports."
  },
  "meta": {
    "resolution": {
      "term": "Softwareentwickler",
      "matchedTitle": "Software Developer",
      "slug": "software-developer",
      "id": 614,
      "alternatives": [
        {
          "id": 615,
          "title": "Software Engineer",
          "matchingScore": 0.91
        }
      ]
    },
    "duration_ms": 1420,
    "cached": true
  },
  "input_job_title": "Softwareentwickler",
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

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~kununu-salary-scraper/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### What if Kununu matches a different job title?

The response includes the resolved title and may include alternatives so you can confirm which statistics were selected.

## Related Scrappa Actors

- [Kununu Company Salaries Scraper](https://apify.com/thescrappa/kununu-company-salaries-scraper)
- [Kununu Company Profile Scraper](https://apify.com/thescrappa/kununu-company-details-scraper)
- [Kununu Jobs Scraper](https://apify.com/thescrappa/kununu-jobs-scraper)
