# Semrush Domain Overview Scraper

Retrieve a domain overview with organic search metrics, backlink counts, traffic estimates, and available engagement history. Submit multiple domains in one run.

## What data can you extract?

| Field | Type | Description |
| --- | --- | --- |
| `search_parameters` | Object | Domain and market database selected for this Semrush overview. |
| `domain` | String | Website domain summarized by the overview. |
| `covered` | Boolean | True when Semrush returned coverage for this domain and market. |
| `coverage` | Object | Availability indicators for domain, backlink, and traffic metrics. |
| `semrush_rank` | Integer | Semrush global rank reported for the domain. |
| `organic_keywords` | Integer | Estimated number of keywords bringing organic search visits. |
| `organic_traffic` | Integer | Estimated monthly visits from organic search. |
| `organic_traffic_cost` | Integer | Estimated monthly value of the organic search traffic. |
| `authority_score` | Integer | Semrush Authority Score for the analyzed domain. |
| `backlinks_count` | Integer | Total backlinks reported for the domain. |
| `referring_domains_count` | Integer | Number of unique domains linking to the website. |
| `website` | Object | Website details such as country, visits, engagement, and traffic history. |
| `input_domain` | String | Website domain submitted for the overview lookup. |
| `scraped_at` | String | UTC date and time when this domain overview was collected. |

## Use cases

- SEO consultants can benchmark traffic, keywords, authority, and backlinks.
- Marketing teams can compare traffic and engagement trends for target websites.
- Sales teams can enrich company research with organic visibility metrics.

## How to use

1. Add one root domain or subdomain to `domains` for each overview.
2. Set `maxResults` to cap saved domain records.
3. Read overview metrics and website engagement fields.

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
| `domains` | Array\<object\> | Yes | Domains for which to retrieve an overview. |
| `domains[].domain` | string | Yes per entry | Root domain or subdomain to analyze |
| `maxResults` | integer | No | Maximum dataset items to save across this entire run. |

## Output example

```json
{
  "search_parameters": {
    "engine": "semrush_domain_overview",
    "domain": "notion.so"
  },
  "domain": "notion.so",
  "covered": true,
  "coverage": {
    "domain_metrics": true,
    "backlink_counts": true,
    "website_traffic": true
  },
  "semrush_rank": 12345,
  "organic_keywords": 1200,
  "organic_traffic": 25000,
  "organic_traffic_cost": 18000,
  "authority_score": 72,
  "backlinks_count": 5000,
  "referring_domains_count": 650,
  "website": {
    "domain": "notion.so",
    "country": "us",
    "visits": 48000,
    "engagement": {
      "pages_per_visit": 3.1,
      "avg_visit_duration_seconds": 180,
      "bounce_rate": 0.44
    },
    "visits_history": [
      {
        "month": "2025-01",
        "visits": 118000
      },
      {
        "month": "2025-02",
        "visits": 124000
      }
    ],
    "device_history": [
      {
        "device": "desktop",
        "share": 0.67
      },
      {
        "device": "mobile",
        "share": 0.33
      }
    ]
  },
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

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~semrush-domain-overview-scraper/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### Can the overview contain missing metrics?

Yes. Coverage varies by domain and region. Check `covered` and `coverage` before comparing metrics.

## Related Scrappa Actors

- [Semrush Competitors Scraper](https://apify.com/thescrappa/semrush-competitors-scraper)
- [Similarweb Traffic Analytics Scraper](https://apify.com/thescrappa/similarweb-traffic-analytics-scraper)
- [Bing Search Scraper](https://apify.com/thescrappa/bing-search-scraper)
