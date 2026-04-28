# Google Maps Reviews Scraper

Extract customer reviews from a Google Maps business. Get review text, ratings, reviewer metadata, owner responses, review links, images, and pagination tokens.

## Features

- **Review Content** - Review text, ratings, review IDs, and review links
- **Metadata** - Timestamps, likes, language tags, images, and owner responses
- **Reviewer Profiles** - Author name, profile link, photo, review count, and local guide level when available
- **Search** - Search within reviews for a specific business
- **Sorting** - Sort by most relevant, newest, highest rating, or lowest rating
- **Pagination** - Use the returned page token to retrieve additional review pages
- **Caching Controls** - Use cached results or request fresh data by maximum cache age

## Input

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `business_id` | string | Yes | Google Maps business/place ID in `0x...:0x...` format |
| `sort` | integer | Yes | Sort order: `1` most relevant, `2` newest, `3` highest rating, `4` lowest rating |
| `limit` | integer | No | Number of reviews per page, 1-20 (default: 10) |
| `page` | string | No | Pagination token from a previous response |
| `search` | string | No | Search query to match within reviews |
| `debug` | boolean | No | Enable debug logging |
| `use_cache` | boolean | No | Use cached results when available (default: true) |
| `maximum_cache_age` | integer | No | Maximum cache age in seconds (default: 3600, set 0 for fresh data) |

## Output

### Dataset (Review Records)

Each review is saved as a record in the dataset:

```json
{
  "review_id": "ChdDSUhNMG9nS0VJQ0FnSUN...",
  "author_name": "John D.",
  "rating": 5,
  "review_text": ["Amazing pizza and friendly staff! Highly recommend."],
  "timestamp": 1705314600,
  "review_likes": 23,
  "review_language": ["en"],
  "owner_response_text": "Thank you for the great review!",
  "owner_response_timestamp": 1705333500,
  "author_profile_photo": "https://...",
  "author_review_count": 156,
  "author_local_guide_level": 6,
  "author_link": "https://www.google.com/maps/contrib/...",
  "review_link": "https://www.google.com/maps/reviews/...",
  "images": ["https://..."]
}
```

### Key-Value Store (Full Response)

The complete response is saved to the `OUTPUT` key:

```json
{
  "items": [
    {
      "review_id": "ChdDSUhNMG9nS0VJQ0FnSUN...",
      "author_name": "John D.",
      "rating": 5,
      "review_text": ["Amazing pizza and friendly staff! Highly recommend."]
    }
  ],
  "nextPage": "CAESBkVnSUl..."
}
```

## Example Input

```json
{
  "business_id": "0x808fba02425dad8f:0x6c296c66619367e0",
  "sort": 2,
  "limit": 10,
  "search": "service",
  "use_cache": true,
  "maximum_cache_age": 3600
}
```

## Pricing

**Standard Pricing:**
- $0.30 per 1,000 reviews extracted

**Volume Discounts (For Gold Members & Above):**
- $0.25 per 1,000 reviews (at 100k+ monthly usage)
- $0.20 per 1,000 reviews (at 500k+ monthly usage)

No API keys required - just use the actor and pay as you go.

## Support

For issues or questions, contact us through Apify.
