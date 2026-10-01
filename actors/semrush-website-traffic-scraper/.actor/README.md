# Semrush Website Traffic Scraper

Check Semrush-style website traffic estimates for a list of domains. Each domain record includes visits, rankings, engagement estimates, device history, and country-level traffic shares.

## What data can you extract?

| Field | Type | Description |
| --- | --- | --- |
| `search_parameters` | Object | Source engine and domain used for the traffic estimate. |
| `covered` | Boolean | True when the domain has traffic coverage in this report. |
| `domain` | String | Website hostname Semrush reports for the overview. |
| `country` | String | Country database used for the current estimate. |
| `categories` | Array\<Object\> | Industry categories associated with the domain. |
| `actual_date` | String | Date of the traffic snapshot reported by Semrush. |
| `ranks` | Object | Global, country, and category rank information. |
| `authority_score` | Integer | Semrush authority score reported for the domain. |
| `visits` | Integer | Estimated website visits for the reported period. |
| `engagement` | Object | Engagement estimates such as pages per visit, visit duration in seconds, and bounce rate. |
| `visits_history` | Array\<Object\> | Monthly estimated visit totals across the available history. |
| `device_history` | Array\<Object\> | Monthly estimated traffic split between desktop and mobile devices. |
| `traffic_by_country` | Array\<Object\> | Estimated visits and traffic shares for the leading country markets. |
| `input_domain` | String | Website domain submitted for Semrush traffic estimates. |
| `scraped_at` | String | UTC date and time when this record was collected. |

## Use cases

- SEO consultants can compare estimated visits and authority across competitor domains.
- Marketing analysts can track traffic mix by device and country.
- Lead research teams can add traffic estimates to company qualification lists.

## How to use

1. Add each website hostname to `domains` without a URL scheme.
2. Use `maxResults` to cap the number of domain records saved.
3. Review the country database and snapshot date alongside traffic estimates.

```json
{
  "domains": [
    {
      "domain": "notion.so"
    }
  ],
  "maxResults": 20
}
```

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `domains` | Array\<object\> | Yes | Website domains to check for Semrush traffic estimates. |
| `domains[].domain` | string | Yes per entry | Domain sent to the source search. |
| `maxResults` | integer | No | Maximum dataset items to save across this entire run. |

## Output example

```json
{
  "search_parameters": {
    "engine": "semrush",
    "domain": "notion.so"
  },
  "covered": true,
  "domain": "notion.so",
  "country": "us",
  "categories": [
    {
      "slug": "business-and-industry",
      "name": "Business and Industry"
    }
  ],
  "actual_date": "2025-09-01",
  "ranks": {
    "global": 1250,
    "country": {
      "value": 88,
      "name": "United States",
      "database": "us",
      "slug": "us"
    },
    "category": {
      "database": "us"
    }
  },
  "authority_score": 71,
  "visits": 8120000,
  "engagement": {
    "pages_per_visit": 3.8,
    "avg_visit_duration_seconds": 185,
    "bounce_rate": 0.42
  },
  "visits_history": [
    {
      "date": "2025-07-01",
      "visits": 7900000
    },
    {
      "date": "2025-08-01",
      "visits": 8030000
    },
    {
      "date": "2025-09-01",
      "visits": 8120000
    }
  ],
  "device_history": [
    {
      "date": "2025-09-01",
      "visits": 8120000,
      "desktop_visits": 4872000,
      "mobile_visits": 3248000
    },
    {
      "date": "2025-08-01",
      "visits": 8030000,
      "desktop_visits": 4818000,
      "mobile_visits": 3212000
    }
  ],
  "traffic_by_country": [
    {
      "country": "US",
      "country_name": "United States",
      "traffic": 3400000,
      "traffic_share": 0.42,
      "desktop_share": 0.52,
      "mobile_share": 0.48
    }
  ],
  "input_domain": "notion.so",
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

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~semrush-website-traffic-scraper/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### Does one domain create one result?

Yes. The Actor saves one full traffic overview record for each domain submitted.

## Related Scrappa Actors

- [Semrush Domain Overview Scraper](https://apify.com/thescrappa/semrush-domain-overview-scraper)
- [Semrush Competitors Scraper](https://apify.com/thescrappa/semrush-competitors-scraper)
- [Similarweb Traffic Analytics Scraper](https://apify.com/thescrappa/similarweb-traffic-analytics-scraper)
