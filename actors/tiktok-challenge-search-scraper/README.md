# TikTok Challenge Search Scraper

TikTok Challenge Search Scraper finds TikTok challenge and hashtag metadata from keywords through Scrappa. Use it to discover challenge IDs before running hashtag-post workflows, monitor topic demand, or build TikTok trend research pipelines.

## Features

- Search one or more keywords in a single Apify run
- Return one dataset item per TikTok challenge result
- Include challenge IDs and names for downstream TikTok hashtag post scraping
- Preserve raw TikTok challenge fields alongside normalized columns
- Charge per saved `challenge-result` event when published with pay-per-event pricing
- Compact `OUTPUT` summary for compatibility

## Input

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `keywords` | array of strings | No | Preferred batch input. Keyword, topic, niche, product, brand, or hashtag terms to search. |
| `keyword` | string | No | Legacy single keyword input. Ignored when `keywords` contains valid values. |
| `count` | integer | No | Number of challenge results to request for each keyword. Scrappa accepts `1-50`. |

Provide at least one value in `keywords` or `keyword`.

## Example Input

```json
{
  "keywords": ["cosplay", "fitness"],
  "count": 10
}
```

## Output

Each TikTok challenge is saved as one dataset item:

```json
{
  "challenge_id": "1234567890123456789",
  "challenge_name": "cosplay",
  "description": "Example TikTok challenge description",
  "view_count": 123456789,
  "video_count": 12345,
  "user_count": 6789,
  "request_keyword": "cosplay",
  "request_count": 10
}
```

The dataset item also keeps the raw fields returned by Scrappa/TikTok so downstream workflows can use fields that are not shown in the default table view.

The `OUTPUT` key-value-store record contains a compact summary with processed keywords, saved challenge count, and charge-limit status. Dataset output is the primary result channel.

## Pricing

Publish this actor with Apify pay-per-event pricing using the `challenge-result` event. Suggested starting price: `$0.00025` per saved challenge result (`$0.25/1k results`). Confirm active paid pricing or the earliest allowed scheduled paid pricing in Apify before public launch.

## Support

For higher-volume usage or direct API access, use Scrappa at https://scrappa.co.
