# TikTok Followers Scraper

Review public profiles following a TikTok account, with usernames and follower counts. Enter a public TikTok username and set the requested count to inspect that account’s followers.

## What data can you extract?

Follower profile details are limited to information TikTok makes visible for the requested account.

| Field | Type | Description |
| --- | --- | --- |
| `unique_id` | text | TikTok username for this profile; null when the account does not expose a username. |
| `user_id` | text | user ID for the follower profile, assigned by TikTok; null when the source does not expose it. |
| `nickname` | text | Nickname shown for the follower profile by TikTok, in the format used by the source; null when it is omitted. |
| `avatar` | image | Profile image url for this follower profile on TikTok; null when the source does not provide a URL. |
| `follower_count` | number | Number of followers shown by TikTok, as a whole number; zero is possible, and null means no count was reported. |
| `verified` | boolean | Whether the source marks the profile or review as verified; false is a reported value, while null means TikTok provided no flag. |
| `lookup_unique_id` | text | lookup unique id for the follower profile, assigned by TikTok; null when the source does not expose it. |
| `lookup_user_id` | text | lookup user id for the follower profile, assigned by TikTok; null when the source does not expose it. |

## Use cases

- Creator managers can review public account connections while mapping a niche community.
- Researchers can compare profiles and counts across a creator network.
- Partnership teams can shortlist public accounts for manual review.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Set `profile` to the search term or source identifier you want to look up, then use the optional filters listed below.
3. Start the run and open its default dataset to inspect or download the rows.

```json
{
  "profile": "@tiktok",
  "count": 10
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `profile` | string | Yes | TikTok username with or without @, full HTTPS TikTok profile URL, or numeric user ID up to 30 digits. Bare numeric values are treated as user IDs; prefix numeric usernames with @. |
| `count` | integer | No | Number of followers to return from this page. Scrappa accepts 1-50. Constraints: minimum 1; maximum 50. |
| `time` | integer | No | Follower pagination token/time marker from a previous run. Leave empty for the first page. The actor also accepts cursor as an alias when provided through API input. Constraints: minimum 0. |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "follower_count": 12600,
  "unique_id": "morgan.creates",
  "user_id": "7210458831",
  "nickname": "Morgan Creates",
  "avatar": "https://source.example.com/record/market-guide",
  "verified": true,
  "lookup_unique_id": "morgan.creates",
  "lookup_user_id": "7210458831"
}
```

## Pricing

**Current live price:** Apify Free tier: $0.30 per 1,000 results; Bronze: $0.25 per 1,000 results; Silver: $0.22 per 1,000 results; Gold, Platinum, and Diamond: $0.20 per 1,000 results.

Each saved video, post or comment record counts as one result.

## FAQ

### Is it legal to collect public information?

This Actor is for information visible on the public source pages it reads. You are responsible for checking the source terms, privacy and copyright rules, and the laws that apply to your use.

### How many records will a run return?

The result count depends on the input limits, pagination settings and what the source makes available. A small query or unavailable page can return fewer records, including none.

### Can I call it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~tiktok-followers-scraper/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### Which TikTok account does Followers use as its target?

Set `profile` to the public TikTok username whose followers you want to inspect. Use `count` to limit the requested number of profiles; the source may return fewer.

## Related Scrappa Actors

- [TikTok Ads Scraper](https://apify.com/thescrappa/tiktok-ads-scraper)
- [TikTok Hashtag Details Scraper](https://apify.com/thescrappa/tiktok-challenge-details-scraper)
- [TikTok Hashtag Videos Scraper](https://apify.com/thescrappa/tiktok-challenge-posts-scraper)
- [TikTok Challenge Search Scraper](https://apify.com/thescrappa/tiktok-challenge-search-scraper)
- [TikTok Comments Scraper](https://apify.com/thescrappa/tiktok-comments-scraper)
