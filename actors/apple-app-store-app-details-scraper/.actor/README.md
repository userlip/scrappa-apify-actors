# Apple App Store App Details Scraper

Look up Apple App Store apps and supported product types by product ID. Review listing facts, ratings, privacy labels, available reviews, and related products in one record.

## What data can you extract?

| Field | Type | Description |
| --- | --- | --- |
| `search_parameters` | Object | Apple product ID, selected storefront country, and storefront type used for the app lookup. |
| `product` | Object | App name, publisher, category, price, current version, privacy labels, and other product-page details. |
| `ratings` | Object | Average star score, total rating count, and rating distribution reported for the app. |
| `similar` | Array\<object\> | Other App Store products Apple surfaces as similar to this app. |
| `more_by_developer` | Array\<object\> | Other App Store products listed by the same developer account. |
| `reviews` | Array\<object\> | Review records returned for this app, with written feedback and star ratings when available. |
| `reviews_pagination` | Object | Review offsets, page size, and flags showing whether another review page is available. |
| `input_product_id` | String | Apple product ID submitted for this app details lookup. |
| `scraped_at` | String | UTC date and time when this app record was collected. |

## Use cases

- App catalog teams can enrich product records with publisher, version, ratings, and privacy labels.
- Market analysts can compare app pricing and popularity across storefronts.
- App publishers can review how their listings appear beside similar products.

## How to use

1. Add one Apple product ID to `product_ids` for each supported product.
2. Choose store vertical and country for the target storefront.
3. Run the Actor and read one detailed item per product ID.

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

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `product_ids` | Array\<object\> | Yes | Apple product IDs to look up. Add one object per supported product. |
| `product_ids[].product_id` | string | Yes per entry | Apple product id \(numeric adam id; UMC ids also accepted for movies/TV\) |
| `product_ids[].id` | string | No | Legacy alias for the Apple product ID. |
| `product_ids[].store` | string | No | Store vertical: apps, mac, books, audiobooks, movies, tv, music, or podcasts |
| `product_ids[].country` | string | No | Two-letter Apple storefront country code \(default: us\) |
| `product_ids[].season_id` | string | No | TV season id; returns the season product instead of the show |
| `product_ids[].num` | integer | No | Requested review result count per page |
| `product_ids[].offset` | integer | No | Review result offset for the selected App Store item. |
| `product_ids[].next_page_token` | string | No | Review pagination token from a previous response |
| `id` | string | No | Legacy alias for the Apple product ID. |
| `store` | string | No | Store vertical: apps, mac, books, audiobooks, movies, tv, music, or podcasts |
| `country` | string | No | Two-letter Apple storefront country code \(default: us\) |
| `season_id` | string | No | TV season id; returns the season product instead of the show |
| `num` | integer | No | Requested review result count per page |
| `offset` | integer | No | Review result offset for the selected App Store item. |
| `next_page_token` | string | No | Review pagination token from a previous response |
| `maxResults` | integer | No | Maximum dataset items to save across this entire run. |

## Output example

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
    "title": "Pocket Notes: Daily Planner",
    "artist": "Northstar Mobile Studio",
    "url": "https://apps.apple.com/us/app/pocket-notes-daily-planner/id1234567890",
    "rating": 4.8,
    "rating_count": 250,
    "privacy": {
      "privacyTypes": [
        {
          "dataCategories": [
            "Contact Info"
          ],
          "description": "Product usage data is linked to the account for analytics.",
          "identifier": "product_usage",
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
  "similar": [
    {
      "id": "1562071743",
      "title": "Fieldnote: Simple Journal",
      "artist": "Fieldnote Apps",
      "rating": 4.7,
      "url": "https://apps.apple.com/us/app/fieldnote-simple-journal/id1000000001"
    }
  ],
  "more_by_developer": [
    {
      "id": "1494974266",
      "title": "Northstar Habit Tracker",
      "artist": "Northstar Mobile Studio",
      "url": "https://apps.apple.com/us/app/northstar-habit-tracker/id1000000002"
    }
  ],
  "reviews": [
    {
      "id": "review-2026-0814-01",
      "author": "Jordan Lee",
      "rating": 5,
      "title": "Clear and useful",
      "text": "The reminders are useful and the weekly view is easy to scan.",
      "date": "2026-08-14",
      "review": "A clear, lightweight way to organize daily plans."
    }
  ],
  "reviews_pagination": {
    "offset": 0,
    "limit": 20,
    "next_offset": 20,
    "end_of_history": true
  },
  "input_product_id": "1232780281",
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

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~apple-app-store-app-details-scraper/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### Which Apple product types can I look up?

Set `store` to apps, Mac apps, books, audiobooks, movies, TV, music, or podcasts when supported.

## Related Scrappa Actors

- [Apple App Store Search Scraper](https://apify.com/thescrappa/apple-app-store-search-scraper)
- [Google Shopping Scraper](https://apify.com/thescrappa/google-shopping-scraper)
- [Google Lens Visual Search Scraper](https://apify.com/thescrappa/google-lens-scraper)
