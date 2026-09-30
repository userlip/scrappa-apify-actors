# Google Shopping Scraper

Find product listings from Google Shopping with seller, price, rating, and product details.

## Data you get

- **title**: Result title or listing name.
- **source**: Publisher, seller, or source name.
- **price**: Listed product or property price.
- **rating**: Average product or app rating.
- **reviews**: Review count when provided.

## Use cases

- Product price monitoring
- Catalog research
- Retail competitor analysis

## How to use

Add one or more entries to **queries**. Each entry maps its **q** value to the Scrappa **q** input. Shared endpoint options can be set at the top level.

```json
{
  "queries": [
    {
      "q": "running shoes"
    }
  ],
  "maxResults": 20,
  "maxPages": 2
}
```

## Output example

This synthetic example shows the response fields and the input value attached to each result.

```json
{
  "position": 1,
  "category": "Electronics",
  "title": "Example Wireless Headphones",
  "source": "Example Store",
  "thumbnail": "https://example.com/product.jpg",
  "price": "$49.00",
  "rating": 4.6,
  "reviews": 120,
  "input_q": "running shoes",
  "scraped_at": "2026-01-01T00:00:00Z"
}
```

## Pricing

$0.30 per 1,000 results. The Actor writes one dataset item for each result.

This Actor supports pagination and stops at the configured **maxPages** or **maxResults** limit.

## FAQ

### Is scraping this data legal?

Scraping rules depend on the source, the data, and how you use it. Review the applicable laws, source terms, and privacy requirements for your use case. You are responsible for your collection and use of the data.

### Are there request limits?

You can submit up to 100 batch entries per run. Set **maxResults** to cap saved rows and **maxPages** to bound pagination for each entry. Scrappa API limits and source availability also apply.

### Can I use the output with integrations or the API?

Yes. Read results from the Apify dataset, use the Apify API or client libraries, or connect the dataset to your existing data workflow. Each row includes **input_q** and **scraped_at** for traceability.

## Related Actors

- [Apple App Store Search Scraper](https://apify.com/thescrappa/apple-app-store-search-scraper)
- [Google Search Scraper](https://apify.com/thescrappa/google-search-scraper)

## Search terms

`Google Shopping Scraper`, `title`, `source`, `price`, `/google/shopping API`
