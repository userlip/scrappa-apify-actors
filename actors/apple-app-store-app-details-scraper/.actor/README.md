# Apple App Store App Details Scraper

Retrieve Apple App Store product details, ratings, privacy labels, reviews, and related apps.

## Data you get

- **title**: Result title or listing name.
- **artist**: App developer or listed artist.
- **rating**: Average product or app rating.
- **rating_count**: Number of ratings.
- **privacy**: Apple privacy label information.

## Use cases

- App intelligence
- Privacy label review
- Product catalog enrichment

## How to use

Add one or more entries to **product_ids**. Each entry maps its **product_id** value to the Scrappa **product_id** input. Shared endpoint options can be set at the top level.

```json
{
  "product_ids": [
    {
      "product_id": "1232780281"
    }
  ],
  "country": "us",
  "store": "apps",
  "maxResults": 20
}
```

## Output example

This synthetic example shows the response fields and the input value attached to each result.

```json
{
  "search_parameters": {
    "product_id": "1234567890",
    "country": "us",
    "store": "apps",
    "engine": "apple_app_store"
  },
  "product": {
    "id": "1234567890",
    "store": "apps",
    "title": "Example Notes",
    "artist": "Example Studio",
    "url": "https://apps.apple.com/app/example-notes/id1234567890",
    "rating": 4.8,
    "rating_count": 250,
    "privacy": {
      "privacyTypes": [
        {
          "dataCategories": [
            "Contact Info"
          ],
          "description": "Synthetic privacy label.",
          "identifier": "example",
          "privacyType": "data_used_to_track_you",
          "purposes": [
            "Analytics"
          ]
        }
      ]
    }
  },
  "ratings": {
    "rating_count": 250,
    "rating_average": 4.8,
    "histogram": [
      1,
      2,
      3,
      10,
      234
    ],
    "total_reviews": 120
  },
  "similar": [],
  "more_by_developer": [],
  "reviews": [],
  "reviews_pagination": {
    "offset": 0,
    "limit": 20,
    "next_offset": 20,
    "end_of_history": true
  },
  "input_product_id": "1232780281",
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

Yes. Read results from the Apify dataset, use the Apify API or client libraries, or connect the dataset to your existing data workflow. Each row includes **input_product_id** and **scraped_at** for traceability.

## Related Actors

- [Apple App Store Search Scraper](https://apify.com/thescrappa/apple-app-store-search-scraper)
- [Google Play Reviews Scraper](https://apify.com/thescrappa/google-play-reviews-scraper)

## Search terms

`Apple App Store App Details Scraper`, `title`, `artist`, `rating`, `/apple/app-store/details API`
