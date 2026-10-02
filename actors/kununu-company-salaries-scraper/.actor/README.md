# Kununu Company Salaries Scraper

Collect company-level Kununu salary ranges with job titles, minimums, maximums, medians, averages, and report counts. Batch employer slugs for pay comparisons by country.

## What data can you extract?

| Field | Type | Description |
| --- | --- | --- |
| `jobRoleTitleId` | Integer | JobRoleTitleId value associated with this kununu company salaries record. |
| `jobRoleTitle` | String | JobRoleTitle value associated with this kununu company salaries record. |
| `jobRoleTitleSlug` | String | JobRoleTitleSlug value associated with this kununu company salaries record. |
| `profileRangeMin` | Integer | ProfileRangeMin value associated with this kununu company salaries record. |
| `profileRangeMax` | Integer | ProfileRangeMax value associated with this kununu company salaries record. |
| `profileMedian` | Integer | ProfileMedian value associated with this kununu company salaries record. |
| `profileAverage` | Integer | ProfileAverage value associated with this kununu company salaries record. |
| `profileNumberOfEntries` | Integer | ProfileNumberOfEntries value associated with this kununu company salaries record. |
| `isClaimed` | Boolean | IsClaimed value associated with this kununu company salaries record. |
| `isHighlighted` | Boolean | IsHighlighted value associated with this kununu company salaries record. |
| `input_company_slug` | String | Kununu employer profile slug used for this lookup. |
| `scraped_at` | String | UTC date and time when this record was collected. |

## Use cases

- Compensation teams can benchmark reported pay by job title.
- Labor-market analysts can compare salary ranges across employers.
- Job seekers can research reported pay bands before applying.

## How to use

1. Add a Kununu company slug to each entry in `companies`.
2. Choose the country for the employer salary data.
3. Use `maxResults` to cap saved job-title salary ranges.

```json
{
  "companies": [
    {
      "company_slug": "sap"
    }
  ],
  "country": "de",
  "maxResults": 10
}
```

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `companies` | Array\<object\> | Yes | Kununu company slugs whose reported salary ranges you want to collect. |
| `companies[].company_slug` | string | Yes per entry | Company slug from the public Kununu employer profile URL. |
| `companies[].country` | string | No | Kununu country market code, such as de, at, or ch. Accepted values: de, at, ch. |
| `country` | string | Yes | Kununu country market code, such as de, at, or ch. Accepted values: de, at, ch. |
| `maxResults` | integer | No | Maximum dataset items to save across this entire run. |

## Output example

```json
{
  "jobRoleTitleId": 5521,
  "jobRoleTitle": "Software Engineer",
  "jobRoleTitleSlug": "software-engineer",
  "profileRangeMin": 56000,
  "profileRangeMax": 94000,
  "profileMedian": 73500,
  "profileAverage": 74800,
  "profileNumberOfEntries": 82,
  "isClaimed": false,
  "isHighlighted": true,
  "input_company_slug": "sap",
  "scraped_at": "2026-10-01T12:00:00Z"
}
```

## Pricing

$0.30 per 1,000 results. Apify saves one dataset item per result and applies the Actor’s per-result price to saved items.

Set **maxResults** to cap the number of dataset items saved in one run.

## FAQ

### Is scraping this information legal?

Rules depend on the source, location, data type, and intended use. Check applicable laws and source terms, and make sure your workflow follows privacy and data protection requirements.

### What limits apply?

Submit up to 100 batch entries per run. Use **maxResults** to cap saved items. Results also depend on source availability and your Scrappa API plan.

### Can I start runs through the Apify API?

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~kununu-company-salaries-scraper/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### Are salary values official employer pay bands?

No. Kununu salary figures reflect the information available in its salary pages and may be based on employee-submitted reports.

## Related Scrappa Actors

- [Kununu Company Profile Scraper](https://apify.com/thescrappa/kununu-company-details-scraper)
- [Kununu Salary by Job Title Scraper](https://apify.com/thescrappa/kununu-salary-scraper)
- [Kununu Company Search Scraper](https://apify.com/thescrappa/kununu-company-search-scraper)
