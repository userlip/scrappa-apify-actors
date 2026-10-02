# Semrush Sensor Winners & Losers Scraper

Track the domains with the largest Semrush Sensor movements by database and category. Review ranking change totals alongside tracked keyword counts and recent movement history.

## What data can you extract?

| Field | Type | Description |
| --- | --- | --- |
| `domain` | String | Website hostname with a notable Sensor movement. |
| `diff` | Integer | Net keyword ranking movement reported for this domain. |
| `keywords` | Integer | Number of tracked keywords in the Sensor report. |
| `new_keywords` | Integer | Tracked keywords newly appearing for the domain. |
| `lost_keywords` | Integer | Tracked keywords no longer appearing for the domain. |
| `date` | String | Report date returned for the selected Sensor market. |
| `category` | Integer | Sensor category identifier associated with the movement list. |
| `diffs` | Array\<Object\> | Recent numeric movement history with date markers and change values. |
| `input_category` | Integer | Semrush Sensor category selected for this movement list. |
| `scraped_at` | String | UTC date and time when this record was collected. |

## Use cases

- SEO teams can spot domains with sharp visibility changes.
- Site owners can compare new and lost keyword counts by category.
- Analysts can monitor daily movement across desktop and mobile databases.

## How to use

1. Choose a Semrush database such as `US` and add category IDs to `categories`.
2. Leave `date` empty to use the current report date or provide a specific date.
3. Set `maxResults` to cap the number of domain movers saved.

```json
{
  "categories": [
    {
      "category": 1
    }
  ],
  "db": "US",
  "maxResults": 20
}
```

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `categories` | Array\<object\> | Yes | Semrush Sensor category IDs to check for ranking movers. |
| `categories[].category` | integer | Yes per entry | Category sent to the source search. |
| `categories[].db` | string | No | Database sent to the source search. |
| `categories[].date` | string | No | Date sent to the source search. When omitted, the Sensor report date defaults to today. |
| `db` | string | Yes | Database sent to the source search. |
| `date` | string | No | Date sent to the source search. When omitted, the Sensor report date defaults to today. |
| `maxResults` | integer | No | Maximum dataset items to save across this entire run. |

## Output example

```json
{
  "domain": "papertrail.io",
  "diff": 37,
  "keywords": 18420,
  "new_keywords": 146,
  "lost_keywords": 109,
  "date": "2025-09-01",
  "category": 1,
  "diffs": [
    {
      "date": 1756684800,
      "diff": 12
    },
    {
      "date": 1756771200,
      "diff": 18
    },
    {
      "date": 1756857600,
      "diff": 24
    }
  ],
  "input_category": 1,
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

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~semrush-sensor-movers-scraper/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### Which date is used by default?

When you leave `date` empty, the Actor uses today’s date for the Sensor report.

## Related Scrappa Actors

- [Semrush Domain Overview Scraper](https://apify.com/thescrappa/semrush-domain-overview-scraper)
- [Semrush Competitors Scraper](https://apify.com/thescrappa/semrush-competitors-scraper)
- [Google Trends Interest Scraper](https://apify.com/thescrappa/google-trends-interest-scraper)
