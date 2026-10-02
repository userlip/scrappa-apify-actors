# Apple App Store Developer Apps Scraper

List public apps associated with Apple developer accounts and collect product IDs, names, genres, prices, ratings, versions, and links. Batch developer IDs to review publisher catalogs.

## What data can you extract?

| Field | Type | Description |
| --- | --- | --- |
| `id` | String | Stable source identifier for this record. |
| `type` | String | Type value associated with this apple app store developer apps record. |
| `title` | String | Title displayed by the source for this record. |
| `artist` | String | Artist value associated with this apple app store developer apps record. |
| `artist_id` | String | Artist Id value associated with this apple app store developer apps record. |
| `url` | String | Public source or listing URL associated with this record. |
| `artwork` | String | Artwork value associated with this apple app store developer apps record. |
| `genres` | Array\<String\> | Apple category labels assigned to the app. |
| `price` | Integer | Quoted price in the currency shown by the source. |
| `formatted_price` | String | Formatted Price value associated with this apple app store developer apps record. |
| `currency` | String | Three-letter currency code for the quoted amount. |
| `rating` | Number | Rating value reported by the source. |
| `rating_count` | Integer | Rating Count value associated with this apple app store developer apps record. |
| `release_date` | String | Release Date value associated with this apple app store developer apps record. |
| `version` | String | Version value associated with this apple app store developer apps record. |
| `content_rating` | String | Content Rating value associated with this apple app store developer apps record. |
| `seller` | String | Seller value associated with this apple app store developer apps record. |
| `input_artist_id` | String | Apple developer account identifier supplied for this app catalog request. |
| `scraped_at` | String | UTC date and time when this record was collected. |

## Use cases

- App publishers can audit public apps attached to developer accounts.
- Market analysts can compare product portfolios, prices, and ratings by publisher.
- Mobile researchers can discover related apps from a developer catalog.

## How to use

1. Add an Apple artist or developer ID to each entry in `developers`.
2. Set a storefront country for the app metadata lookup.
3. Export app rows or connect the dataset to a catalog workflow.

```json
{
  "developers": [
    {
      "artist_id": "284882218"
    }
  ],
  "country": "us",
  "maxResults": 10
}
```

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `developers` | Array\<object\> | Yes | Apple developer or artist IDs whose App Store apps you want to list. |
| `developers[].artist_id` | string | Yes per entry | Apple artist/developer id |
| `developers[].id` | string | No | Value accepted by the Apple App Store Developer Apps Scraper search. |
| `developers[].country` | string | No | Kununu country market code, such as de, at, or ch. |
| `id` | string | No | Value accepted by the Apple App Store Developer Apps Scraper search. |
| `country` | string | No | Kununu country market code, such as de, at, or ch. |
| `maxResults` | integer | No | Maximum dataset items to save across this entire run. |

## Output example

```json
{
  "id": "1234567890",
  "type": "apps",
  "title": "Trail Notes: Hiking Planner",
  "artist": "Northstar Mobile, Inc.",
  "artist_id": "284882218",
  "url": "https://apps.apple.com/us/app/trail-notes-hiking-planner/id1234567890",
  "artwork": "https://is1-ssl.mzstatic.com/image/thumb/app-icon/512x512bb.jpg",
  "genres": [
    "Navigation"
  ],
  "price": 0,
  "formatted_price": "Free",
  "currency": "USD",
  "rating": 4.7,
  "rating_count": 18420,
  "release_date": "2023-05-18",
  "version": "4.8.1",
  "content_rating": "4+",
  "seller": "Northstar Mobile, Inc.",
  "input_artist_id": "284882218",
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

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~apple-app-store-developer-scraper/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### Is artist\_id the same as an app product ID?

No. The artist ID identifies a developer account; each returned app has its own product ID.

## Related Scrappa Actors

- [Apple App Store App Details Scraper](https://apify.com/thescrappa/apple-app-store-app-details-scraper)
- [Apple App Store Search Scraper](https://apify.com/thescrappa/apple-app-store-search-scraper)
- [Apple App Store Reviews Scraper](https://apify.com/thescrappa/apple-app-store-reviews-scraper)
