# Google Maps Reviews Scraper

Extract Google Maps reviews for a specific business by `business_id`. Use it to monitor new reviews, analyze customer sentiment, audit competitor locations, enrich local lead lists, and build review datasets without maintaining Google Maps scraping infrastructure.

## What this actor does

- Looks up one Google Maps business by `business_id`.
- Returns review text, rating, reviewer metadata, review links, images, likes, language, owner responses, and pagination.
- Supports review sorting by most relevant, newest, highest rating, or lowest rating.
- Supports keyword search inside reviews for targeted reputation and support workflows.
- Saves each review as a dataset row and stores the full Scrappa response in the `OUTPUT` key-value store record.

## Common use cases

- Track new reviews for your own locations.
- Monitor competitor reviews by market, category, or city.
- Find complaint themes such as service, price, delivery, warranty, wait time, or staff.
- Join reviews with Google Maps Search, Business Details, and Photos data using the same `business_id`.
- Export review records to CSV, Google Sheets, Make, n8n, Zapier, BI tools, or your own API pipeline.

## How to get a `business_id`

This actor starts from a Google Maps `business_id`, not a search query. The fastest path is:

1. Run the Scrappa Google Maps Search actor or Google Maps Advanced Search actor with a query such as `coffee shops in Austin` or `dentists near Miami`.
2. Open the dataset for that run.
3. Copy the `business_id` value for the business you want.
4. Paste that value into this actor.

The ID usually looks like this:

```text
0x808fba02425dad8f:0x6c296c66619367e0
```

Keep the `business_id` in your downstream system so you can join search results, business details, photos, and reviews later.

## Input

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `business_id` | string | Yes | Google Maps business identifier in `0x...:0x...` format. Copy it from Google Maps Search, Advanced Search, Autocomplete, or Business Details results. |
| `sort` | integer | Yes | `1` most relevant, `2` newest, `3` highest rating, `4` lowest rating. Default: `2` newest. |
| `limit` | integer | No | Reviews to return on this page, from 1 to 20. Default: `10`. |
| `page` | string | No | Pagination token from the previous response's `nextPage` field. |
| `search` | string | No | Keyword to search inside reviews. Leave blank to return all matching reviews for the selected sort. |
| `debug` | boolean | No | Enable Scrappa debug output for troubleshooting. |
| `use_cache` | boolean | No | Use cached results when available. Default: `true`. |
| `maximum_cache_age` | integer | No | Maximum cache age in seconds. Default: `3600`; set `0` to request fresh data. |

## Tested first-run example

This input was tested against the live Apify actor on May 2, 2026. The run succeeded and returned review dataset rows for the sample business.

```json
{
  "business_id": "0x808fba02425dad8f:0x6c296c66619367e0",
  "sort": 2,
  "limit": 5,
  "search": "service",
  "use_cache": true,
  "maximum_cache_age": 3600
}
```

Use `limit: 5` for a quick first run. For production collection, use `limit: 20` and pass the returned `nextPage` token into the next run.

## Output

### Dataset rows

Each review is saved as one dataset item:

```json
{
  "review_id": "Ci9DQUlRQUNvZENodHljRjlvT21GTFFsSXpZbXRNTkRaYWRqUkljREl6T1d4RmNFRRAB",
  "rating": 1,
  "timestamp": 1775965549283,
  "author_name": "Michael Carlton",
  "author_profile_photo": "https://lh3.googleusercontent.com/...",
  "author_link": "https://www.google.com/maps/contrib/107018891077092762969?hl=en",
  "author_review_count": 48,
  "author_local_guide_level": 1,
  "review_language": ["en"],
  "review_text": ["This review is for a warranty request from Google..."],
  "owner_response_text": null,
  "owner_response_timestamp": null,
  "review_link": "https://www.google.com/maps/reviews/data=...",
  "review_likes": null,
  "images": ["https://lh3.googleusercontent.com/..."]
}
```

Field availability depends on what Google exposes for the review. Some reviews do not include images, owner responses, likes, or local guide metadata.

### Full response

The complete response is also stored in the key-value store under `OUTPUT`:

```json
{
  "items": [
    {
      "review_id": "Ci9DQUlRQUNvZENodHljRjlvT21GTFFsSXpZbXRNTkRaYWRqUkljREl6T1d4RmNFRRAB",
      "author_name": "Michael Carlton",
      "rating": 1,
      "review_text": ["This review is for a warranty request from Google..."]
    }
  ],
  "nextPage": "CAESBkVnSUl..."
}
```

Use `nextPage` when you need the next page of reviews for the same business and sort/search settings.

## Production tips

- Start with `sort: 2` and a small `limit` to confirm the business is correct.
- Use `search` for complaint mining and QA workflows; leave it blank for full review collection.
- Store `business_id`, `review_id`, `timestamp`, `rating`, and `review_link` in your database for deduplication.
- Keep `use_cache` enabled for testing and repeated QA runs. Set `maximum_cache_age` to `0` only when freshness matters more than cost and speed.
- Schedule repeated runs with `sort: 2` to monitor new review activity.

## Pricing and scale

Standard price: `$0.30 per 1,000 reviews extracted`.

For higher-volume review monitoring, competitor intelligence, local SEO audits, or direct API access outside Apify, use Scrappa directly at `https://scrappa.co`. Scrappa can support larger review pipelines while keeping the same Google Maps data model used by this actor.

## Support

For Apify run issues, open an issue on this actor with the run ID, input JSON, and expected result. For direct Scrappa API access or larger usage, contact Scrappa through `https://scrappa.co`.
