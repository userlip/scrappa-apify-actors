# Apple App Store Search Scraper

Search App Store listings with one or more phrases. Collect app identity, developer, pricing, rating, release, and storefront fields for catalog and market analysis.

## What data can you extract?

| Field | Type | Description |
| --- | --- | --- |
| `id` | String | Apple product ID assigned to the app in the selected storefront. |
| `type` | String | Apple storefront record type for the app listing. |
| `title` | String | App name displayed on the Apple App Store product page. |
| `artist` | String | Developer or publisher name shown on the app listing. |
| `artist_id` | String | Apple developer account ID associated with the listing. |
| `description` | String | App description displayed in the selected App Store storefront. |
| `url` | String | Public Apple App Store product page URL for the app. |
| `artwork` | String | Image URL for the app icon or storefront artwork. |
| `genres` | Array\<string\> | App Store categories assigned to the product. |
| `price` | Integer | Current app price in the selected storefront currency. |
| `formatted_price` | String | Localized price label displayed in the storefront. |
| `currency` | String | ISO currency code used for the listed price. |
| `rating` | Number | Average customer rating on the App Store five-star scale. |
| `rating_count` | Integer | Number of customer ratings recorded for the app. |
| `release_date` | String | App release date reported by the storefront, in ISO 8601 format. |
| `version` | String | Current app version listed by the storefront. |
| `content_rating` | String | Age guidance or content rating assigned to the app. |
| `seller` | String | Seller name printed on the App Store product page. |
| `input_query` | String | Search phrase used to find this App Store listing. |
| `scraped_at` | String | UTC date and time when this app listing was collected. |

## Use cases

- App discovery teams can build localized catalogs of available apps.
- Product marketers can compare competitor ratings, prices, and release history.
- App publishers can monitor how listings appear across storefronts.

## How to use

1. Add one search phrase to `queries` for each App Store search.
2. Set `store`, `country`, and `limit` to focus collection.
3. Run the Actor and export result rows from its dataset.

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

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `queries` | Array\<object\> | Yes | App Store search phrases. Add one object per phrase. |
| `queries[].query` | string | Yes per entry | Search term \(q is accepted as an alias\) |
| `queries[].store` | string | No | Store vertical: apps, mac, books, audiobooks, movies, tv, music, or podcasts |
| `queries[].country` | string | No | Two-letter Apple storefront country code \(default: us\) |
| `queries[].limit` | integer | No | Maximum number of results |
| `store` | string | No | Store vertical: apps, mac, books, audiobooks, movies, tv, music, or podcasts |
| `country` | string | No | Two-letter Apple storefront country code \(default: us\) |
| `limit` | integer | No | Maximum number of results |
| `maxResults` | integer | No | Maximum dataset items to save across this entire run. |

## Output example

```json
{
  "id": "1234567890",
  "type": "apps",
  "title": "Pocket Notes: Daily Planner",
  "artist": "Northstar Mobile Studio",
  "artist_id": "12345",
  "description": "Organize daily tasks, quick notes, and reminders in one simple planner.",
  "url": "https://apps.apple.com/us/app/pocket-notes-daily-planner/id1234567890",
  "artwork": "https://cdn.northstarmobile.dev/apps/pocket-notes/icon-512.png",
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
  "seller": "Northstar Mobile Studio",
  "input_query": "notion",
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

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~apple-app-store-search-scraper/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### Can I search storefronts outside the United States?

Yes. Set `country` to the two-letter App Store country code and choose a supported store type.

## Related Scrappa Actors

- [Apple App Store App Details Scraper](https://apify.com/thescrappa/apple-app-store-app-details-scraper)
- [Google Shopping Scraper](https://apify.com/thescrappa/google-shopping-scraper)
- [Google Lens Visual Search Scraper](https://apify.com/thescrappa/google-lens-scraper)
