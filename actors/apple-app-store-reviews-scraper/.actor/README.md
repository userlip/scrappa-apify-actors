# Apple App Store Reviews Scraper

Collect public Apple App Store reviews with ratings, titles, author names, comments, and posting dates. Search several app IDs in one run and page through the review history.

## What data can you extract?

| Field | Type | Description |
| --- | --- | --- |
| `id` | String | Stable source identifier for this record. |
| `title` | String | Title displayed by the source for this record. |
| `author` | String | Public display name supplied by the source. |
| `rating` | Integer | Rating value reported by the source. |
| `text` | String | Text value associated with this apple app store reviews record. |
| `date` | String | Date value associated with this apple app store reviews record. |
| `is_edited` | Boolean | Is Edited value associated with this apple app store reviews record. |
| `input_product_id` | String | Apple or Google Play product identifier supplied for this lookup. |
| `scraped_at` | String | UTC date and time when this record was collected. |

## Use cases

- App publishers can monitor recent customer feedback across product versions.
- Product analysts can compare rating distributions and review themes between apps.
- Market researchers can track public sentiment for competing mobile products.

## How to use

1. Add one Apple product ID per entry in `apps`.
2. Choose a storefront country and store type, then set review filters if needed.
3. Set `maxPages` and `maxResults` to bound the review collection.

```json
{
  "apps": [
    {
      "product_id": "1232780281"
    }
  ],
  "country": "us",
  "store": "apps",
  "maxResults": 10,
  "maxPages": 1
}
```

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `apps` | Array\<object\> | Yes | Apple App Store product IDs to collect public customer reviews for. |
| `apps[].product_id` | string | Yes per entry | Apple product id \(numeric adam id\) |
| `apps[].id` | string | No | Value accepted by the Apple App Store Reviews Scraper search. |
| `apps[].store` | string | No | Store vertical: apps, mac, books, audiobooks, movies, tv, music, or podcasts Accepted values: apps, mac, books, audiobooks, movies, tv, music, podcasts. |
| `apps[].country` | string | No | Kununu country market code, such as de, at, or ch. |
| `apps[].season_id` | string | No | TV season id; returns the season product instead of the show. |
| `apps[].num` | integer | No | Requested review result count per page |
| `apps[].offset` | integer | No | Number of company matches to skip before returning the next page. |
| `apps[].next_page_token` | string | No | Review pagination token from a previous response |
| `id` | string | No | Value accepted by the Apple App Store Reviews Scraper search. |
| `store` | string | No | Store vertical: apps, mac, books, audiobooks, movies, tv, music, or podcasts Accepted values: apps, mac, books, audiobooks, movies, tv, music, podcasts. |
| `country` | string | No | Kununu country market code, such as de, at, or ch. |
| `season_id` | string | No | TV season id; returns the season product instead of the show. |
| `num` | integer | No | Requested review result count per page |
| `offset` | integer | No | Number of company matches to skip before returning the next page. |
| `next_page_token` | string | No | Review pagination token from a previous response |
| `maxResults` | integer | No | Maximum dataset items to save across this entire run. |
| `maxPages` | integer | No | Maximum pages to request for each batch entry. |

## Output example

```json
{
  "id": "1846739204",
  "title": "A reliable trail companion",
  "author": "Maya Chen",
  "rating": 5,
  "text": "Clear maps and useful offline guidance made this a dependable part of weekend hikes.",
  "date": "2026-09-24T10:15:00-07:00",
  "is_edited": false,
  "input_product_id": "1232780281",
  "scraped_at": "2026-10-01T12:00:00Z"
}
```

## Pricing

$0.30 per 1,000 results. Apify saves one dataset item per result and applies the Actor’s per-result price to saved items.

Pagination follows the source response. Set **maxPages** per batch entry and **maxResults** across the run.

## FAQ

### Is scraping this information legal?

Rules depend on the source, location, data type, and intended use. Check applicable laws and source terms, and make sure your workflow follows privacy and data protection requirements.

### What limits apply?

Submit up to 100 batch entries per run. Use **maxResults** to cap saved items and **maxPages** to limit pages for each entry. Results also depend on source availability and your Scrappa API plan.

### Can I start runs through the Apify API?

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~apple-app-store-reviews-scraper/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### Where do I find an Apple product ID?

Copy the numeric ID from an App Store listing URL, or use Apple App Store Search Scraper to find a product first.

## Related Scrappa Actors

- [Apple App Store App Details Scraper](https://apify.com/thescrappa/apple-app-store-app-details-scraper)
- [Apple App Store Search Scraper](https://apify.com/thescrappa/apple-app-store-search-scraper)
- [Apple App Store Top Charts Scraper](https://apify.com/thescrappa/apple-app-store-charts-scraper)
