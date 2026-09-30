# Apple App Store Search Scraper

Search Apple App Store listings by keyword and return app metadata, ratings, pricing, and links.

## Data you get

- **title**: Result title or listing name.
- **artist**: App developer or listed artist.
- **rating**: Average product or app rating.
- **rating_count**: Number of ratings.
- **url**: Canonical result URL.

## Use cases

- App market research
- App discovery
- Ratings and pricing analysis

## How to use

Add one or more entries to **queries**. Each entry maps its **query** value to the Scrappa **query** input. Shared endpoint options can be set at the top level.

```json
{
  "queries": [
    {
      "query": "notion"
    }
  ],
  "country": "us",
  "store": "apps",
  "limit": 10,
  "maxResults": 20
}
```

## Output example

This synthetic example shows the response fields and the input value attached to each result.

```json
{
  "id": "1234567890",
  "type": "apps",
  "title": "Example Notes",
  "artist": "Example Studio",
  "artist_id": "12345",
  "description": "A synthetic product description.",
  "url": "https://apps.apple.com/app/example-notes/id1234567890",
  "artwork": "https://example.com/app.png",
  "genres": [
    "Productivity"
  ],
  "price": 0,
  "formatted_price": "Free",
  "currency": "USD",
  "rating": 4.8,
  "rating_count": 250,
  "release_date": "2025-01-01",
  "version": "1.0",
  "content_rating": "4+",
  "seller": "Example Studio",
  "input_query": "notion",
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

Yes. Read results from the Apify dataset, use the Apify API or client libraries, or connect the dataset to your existing data workflow. Each row includes **input_query** and **scraped_at** for traceability.

## Related Actors

- [Apple App Store App Details Scraper](https://apify.com/thescrappa/apple-app-store-app-details-scraper)
- [Google Search Scraper](https://apify.com/thescrappa/google-search-scraper)

## Search terms

`Apple App Store Search Scraper`, `title`, `artist`, `rating`, `/apple/app-store/search API`
