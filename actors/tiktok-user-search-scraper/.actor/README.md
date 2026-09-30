# TikTok User Search Scraper

Search public TikTok accounts by keyword and collect profile, follower, and verification fields.

## Data you get

- **user_id**: TikTok user identifier.
- **unique_id**: Public TikTok handle.
- **nickname**: Public profile display name.
- **follower_count**: Follower count returned for the account.
- **verified**: Whether the public account is marked verified.

## Use cases

- Creator discovery
- Influencer research
- Social listening and brand monitoring

## How to use

Add one or more entries to **keywords**. Each entry maps its **keywords** value to the Scrappa **keywords** input. Shared endpoint options can be set at the top level.

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

## Output example

This synthetic example shows the response fields and the input value attached to each result.

```json
{
  "user": {
    "user_id": "1234567890123456789",
    "unique_id": "sample_creator",
    "nickname": "Sample Creator",
    "avatar": "https://example.com/avatar.jpg"
  },
  "stats": {
    "follower_count": 25000,
    "verified": false
  },
  "input_keywords": "cooking",
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

Yes. Read results from the Apify dataset, use the Apify API or client libraries, or connect the dataset to your existing data workflow. Each row includes **input_keywords** and **scraped_at** for traceability.

## Related Actors

- [Tiktok Profile Scraper](https://apify.com/thescrappa/tiktok-profile-scraper)
- [Tiktok Search Scraper](https://apify.com/thescrappa/tiktok-search-scraper)

## Search terms

`TikTok User Search Scraper`, `user_id`, `unique_id`, `nickname`, `/tiktok/user/search API`
