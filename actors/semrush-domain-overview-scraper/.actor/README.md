# Semrush Domain Overview Scraper

Retrieve Semrush domain overview metrics for organic search, backlinks, and traffic.

## Data you get

- **domain**: Result website domain.
- **semrush_rank**: Semrush domain rank.
- **organic_keywords**: Estimated organic keyword count.
- **organic_traffic**: Estimated monthly organic visits.
- **authority_score**: Semrush Authority Score.

## Use cases

- SEO competitor analysis
- Domain research
- Organic visibility tracking

## How to use

Add one or more entries to **domains**. Each entry maps its **domain** value to the Scrappa **domain** input. Shared endpoint options can be set at the top level.

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

## Output example

This synthetic example shows the response fields and the input value attached to each result.

```json
{
  "search_parameters": {
    "engine": "semrush_domain_overview",
    "domain": "example.com"
  },
  "domain": "example.com",
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
    "domain": "example.com",
    "country": "us",
    "visits": 48000,
    "engagement": {
      "pages_per_visit": 3.1,
      "avg_visit_duration_seconds": 180,
      "bounce_rate": 0.44
    },
    "visits_history": [],
    "device_history": []
  },
  "input_domain": "notion.so",
  "scraped_at": "2026-01-01T00:00:00Z"
}
```

## Pricing

$0.50 per 1,000 results. The Actor writes one dataset item for each result.

The Actor saves up to **maxResults** dataset items across the run.

## FAQ

### Is scraping this data legal?

Scraping rules depend on the source, the data, and how you use it. Review the applicable laws, source terms, and privacy requirements for your use case. You are responsible for your collection and use of the data.

### Are there request limits?

You can submit up to 100 batch entries per run. Set **maxResults** to cap saved rows. Scrappa API limits and source availability also apply.

### Can I use the output with integrations or the API?

Yes. Read results from the Apify dataset, use the Apify API or client libraries, or connect the dataset to your existing data workflow. Each row includes **input_domain** and **scraped_at** for traceability.

## Related Actors

- [Semrush Competitors Scraper](https://apify.com/thescrappa/semrush-competitors-scraper)
- [Similarweb Traffic Analytics Scraper](https://apify.com/thescrappa/similarweb-traffic-analytics-scraper)

## Search terms

`Semrush Domain Overview Scraper`, `domain`, `semrush_rank`, `organic_keywords`, `/semrush/domain/overview API`
