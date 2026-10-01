# Semrush Competitors Scraper

Submit domains to retrieve sites Semrush associates with organic search competition. Compare each domain’s authority, estimated visits, and rank.

## What data can you extract?

| Field | Type | Description |
| --- | --- | --- |
| `domain` | String | Competing website domain identified for the searched site. |
| `authority_score` | Integer | Semrush Authority Score reported for the competing domain. |
| `visits` | Integer | Estimated monthly visits for the competing website. |
| `semrush_rank` | Integer | Semrush global rank assigned to the competing domain. |
| `input_domain` | String | Website domain used to find this competitor. |
| `scraped_at` | String | UTC date and time when this competitor record was collected. |

## Use cases

- SEO specialists can identify competing domains for a client website.
- Agencies can rank possible competitors by estimated traffic and authority.
- Growth teams can expand site-analysis lists with relevant competing domains.

## How to use

1. Add one root domain or subdomain to `domains` for each comparison.
2. Set `maxResults` to cap competitor rows across the batch.
3. Sort the dataset by authority, visits, or Semrush rank.

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

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `domains` | Array\<object\> | Yes | Domains for which to find organic search competitors. |
| `domains[].domain` | string | Yes per entry | Root domain or subdomain to analyze |
| `maxResults` | integer | No | Maximum dataset items to save across this entire run. |

## Output example

```json
{
  "domain": "britannica.com",
  "authority_score": 70,
  "visits": 25000,
  "semrush_rank": 54321,
  "input_domain": "wikipedia.org",
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

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~semrush-competitors-scraper/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### Are visits measured values?

Visits are Semrush estimates and should be treated as directional competitor metrics.

## Related Scrappa Actors

- [Semrush Domain Overview Scraper](https://apify.com/thescrappa/semrush-domain-overview-scraper)
- [Similarweb Traffic Analytics Scraper](https://apify.com/thescrappa/similarweb-traffic-analytics-scraper)
- [Bing Search Scraper](https://apify.com/thescrappa/bing-search-scraper)
