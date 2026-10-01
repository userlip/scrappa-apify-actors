# TikTok User Search Scraper

Search public TikTok accounts by keyword and collect profile identifiers, display names, avatar URLs, follower counts, and verification state. Cursor pagination bounds collection.

## What data can you extract?

| Field | Type | Description |
| --- | --- | --- |
| `user` | Object | Public TikTok profile identity, including account ID, handle, display name, and avatar. |
| `stats` | Object | Public profile measures, including follower count and verified-account status. |
| `user_id` | String or null | TikTok account identifier returned for this public profile. |
| `unique_id` | String or null | Public TikTok handle shown on the profile. |
| `nickname` | String or null | Display name shown on the TikTok profile. |
| `avatar` | String or null | Public profile image URL returned for the account. |
| `follower_count` | String or null | Number of followers reported for the public account. |
| `verified` | String or null | True when TikTok marks the public account as verified. |
| `input_keywords` | String | Keyword submitted to find matching public TikTok profiles. |
| `scraped_at` | String | UTC date and time when this profile record was collected. |

## Use cases

- Creator partnership teams can find public profiles that match a topic or niche.
- Social analysts can compare follower counts and verification across searches.
- Brand teams can build lists of relevant public accounts before planning outreach.

## How to use

1. Add one keyword or creator niche to `keywords` per search.
2. Set `count` per page and use `maxPages` and `maxResults` to bound collection.
3. Read flattened profile fields alongside nested source objects.

```json
{
  "keywords": [
    {
      "keywords": "cooking"
    }
  ],
  "maxResults": 20,
  "maxPages": 2,
  "count": 10,
  "cursor": "0"
}
```

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `keywords` | Array\<object\> | Yes | Keywords, account fragments, or creator niches to search. |
| `keywords[].keywords` | string | Yes per entry | Search text for public TikTok users. Use a username fragment, brand name, creator niche, product category, or campaign keyword. |
| `keywords[].count` | integer | No | Number of user matches to return for the page, up to 50. |
| `keywords[].cursor` | string | No | Pagination cursor from the previous response. Pass this value with the same `keywords` query when `hasMore` is true. |
| `count` | integer | No | Number of user matches to return for the page, up to 50. |
| `cursor` | string | No | Pagination cursor from the previous response. Pass this value with the same `keywords` query when `hasMore` is true. |
| `maxResults` | integer | No | Maximum dataset items to save across this entire run. |
| `maxPages` | integer | No | Maximum pages to request for each batch entry. |

## Output example

```json
{
  "user": {
    "user_id": "1234567890123456789",
    "unique_id": "trailmixlab",
    "nickname": "Trail Mix Lab",
    "avatar": "https://cdn.trailmixlab.social/profile/avatar-512.jpg"
  },
  "stats": {
    "follower_count": 28700,
    "verified": false
  },
  "user_id": "1234567890123456789",
  "unique_id": "trailmixlab",
  "nickname": "Trail Mix Lab",
  "avatar": "https://cdn.trailmixlab.social/profile/avatar-512.jpg",
  "follower_count": 28700,
  "verified": false,
  "input_keywords": "cooking",
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

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~tiktok-user-search-scraper/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### Does the search include private accounts?

No. The Actor searches public user results and does not provide private account data.

## Related Scrappa Actors

- [TikTok Profile Scraper](https://apify.com/thescrappa/tiktok-profile-scraper)
- [TikTok Search Scraper](https://apify.com/thescrappa/tiktok-search-scraper)
- [TikTok Followers Scraper](https://apify.com/thescrappa/tiktok-followers-scraper)
