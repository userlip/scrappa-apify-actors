# Google Play Reviews Scraper

Collect public Google Play reviews for app and media products with pagination.

## Data you get

- **author**: Public reviewer label returned by the source.
- **rating**: Average product or app rating.
- **text**: Review or result text.
- **date**: Review or publication date.
- **review_id**: Review identifier when returned.

## Use cases

- App review analysis
- Release feedback monitoring
- Product sentiment research

## How to use

Add one or more entries to **product_ids**. Each entry maps its **id** value to the Scrappa **id** input. Shared endpoint options can be set at the top level.

```json
{
  "product_ids": [
    {
      "id": "com.spotify.music"
    }
  ],
  "all_reviews": true,
  "num": 20,
  "sort_by": "newest",
  "hl": "en",
  "gl": "US",
  "store": "apps",
  "maxResults": 20,
  "maxPages": 2
}
```

## Output example

This synthetic example shows the response fields and the input value attached to each result.

```json
{
  "review_id": "synthetic-review-1",
  "author": "Example Reviewer",
  "rating": 5,
  "title": "Useful app",
  "text": "A synthetic review for this fixture.",
  "date": "2026-01-02",
  "helpful_votes": 2,
  "version": "1.0",
  "input_id": "com.spotify.music",
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

Yes. Read results from the Apify dataset, use the Apify API or client libraries, or connect the dataset to your existing data workflow. Each row includes **input_id** and **scraped_at** for traceability.

## Related Actors

- [Google Search Scraper](https://apify.com/thescrappa/google-search-scraper)
- [Apple App Store App Details Scraper](https://apify.com/thescrappa/apple-app-store-app-details-scraper)

## Search terms

`Google Play Reviews Scraper`, `author`, `rating`, `text`, `/google/play/product/reviews API`
