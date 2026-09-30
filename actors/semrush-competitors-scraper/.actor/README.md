# Semrush Competitors Scraper

Find domains Semrush identifies as organic search competitors, with traffic and Authority Score data.

## Data you get

- **domain**: Result website domain.
- **authority_score**: Semrush Authority Score.
- **visits**: Estimated monthly website visits.
- **semrush_rank**: Semrush domain rank.

## Use cases

- SEO competitor discovery
- Market mapping
- Domain benchmarking

## How to use

Add one or more entries to **domains**. Each entry maps its **domain** value to the Scrappa **domain** input. Shared endpoint options can be set at the top level.

```json
{
  "domains": [
    {
      "domain": "wikipedia.org"
    }
  ],
  "maxResults": 20
}
```

## Output example

This synthetic example shows the response fields and the input value attached to each result.

```json
{
  "domain": "example.org",
  "authority_score": 70,
  "visits": 25000,
  "semrush_rank": 54321,
  "input_domain": "wikipedia.org",
  "scraped_at": "2026-01-01T00:00:00Z"
}
```

## Pricing

$0.30 per 1,000 results. The Actor writes one dataset item for each result.

The Actor saves up to **maxResults** dataset items across the run.

## FAQ

### Is scraping this data legal?

Scraping rules depend on the source, the data, and how you use it. Review the applicable laws, source terms, and privacy requirements for your use case. You are responsible for your collection and use of the data.

### Are there request limits?

You can submit up to 100 batch entries per run. Set **maxResults** to cap saved rows. Scrappa API limits and source availability also apply.

### Can I use the output with integrations or the API?

Yes. Read results from the Apify dataset, use the Apify API or client libraries, or connect the dataset to your existing data workflow. Each row includes **input_domain** and **scraped_at** for traceability.

## Related Actors

- [Semrush Domain Overview Scraper](https://apify.com/thescrappa/semrush-domain-overview-scraper)
- [Similarweb Traffic Analytics Scraper](https://apify.com/thescrappa/similarweb-traffic-analytics-scraper)

## Search terms

`Semrush Competitors Scraper`, `domain`, `authority_score`, `visits`, `/semrush/domain/competitors API`
