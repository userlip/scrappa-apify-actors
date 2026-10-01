# Instagram User Info | Cheapest $0.20/1k results

Review a public Instagram profile with its biography, follower count and account details. Submit one public username or several usernames to retrieve the profiles available to Instagram.

## What data can you extract?

Post and profile details reflect information visible on public Instagram pages; optional fields can be absent.

| Field | Type | Description |
| --- | --- | --- |
| `username` | text | Username shown for the Instagram profile by Instagram, in the format used by the source; null when it is omitted. |
| `full_name` | text | Name of the Instagram profile, as shown by Instagram; null when no name is published. |
| `biography` | text | Profile biography from Instagram for this Instagram profile; null when the source has no text to show. |
| `follower_count` | number | Number of followers shown by Instagram, as a whole number; zero is possible, and null means no count was reported. |
| `following_count` | number | Number of followings shown by Instagram, as a whole number; zero is possible, and null means no count was reported. |
| `media_count` | number | Number of medias shown by Instagram, as a whole number; zero is possible, and null means no count was reported. |
| `is_verified` | boolean | Whether the source marks the profile as verified; false is a reported value, while null means Instagram provided no flag. |
| `is_private` | boolean | Whether the account is private; false is a reported value, while null means Instagram provided no flag. |

## Use cases

- Social teams can review public post captions and engagement details.
- Brand researchers can compare post data while monitoring a campaign.
- Creators can archive source-linked posts for a recurring content review.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Put the supported targets in `usernames` and use the identifier or URL format required by Instagram.
3. Start the run and open its default dataset to inspect or download the rows.

```json
{
  "usernames": [
    "natgeo",
    "instagram"
  ]
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `usernames` | array of string | No | Recommended. Process multiple usernames in one Actor run so startup and storage overhead are shared across results. Duplicates are fetched once. Constraints: minimum 1 items; maximum 100 items. |
| `username` | string | No | Backward-compatible single username. Prefer usernames for normal usage, especially when processing more than one profile. |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "follower_count": 12600,
  "username": "morgan.creates",
  "full_name": "Morgan Lee",
  "biography": "Sharing neighborhood markets, simple recipes and weekend walks.",
  "following_count": 418,
  "media_count": 18,
  "is_verified": true,
  "is_private": false
}
```

## Pricing

**Current live price:** $0.20 per 1,000 results.

Each saved dataset record counts as one result.

## FAQ

### Is it legal to collect public information?

This Actor is for information visible on the public source pages it reads. You are responsible for checking the source terms, privacy and copyright rules, and the laws that apply to your use.

### How many records will a run return?

The result count depends on the input limits, pagination settings and what the source makes available. A small query or unavailable page can return fewer records, including none.

### Can I call it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~instagram-user-info-cheapest-0-20-1000-results/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### Can Instagram User Info process several usernames together?

Yes. Pass usernames in `usernames` or use `username` for one public profile. Fields such as biography and follower count may be absent from a restricted profile.

## Related Scrappa Actors

- [Instagram Post Info | Cheapest $0.20/1k results](https://apify.com/thescrappa/instagram-post-info-cheapest-0-20-1000-results)
- [Instagram User Posts Scraper](https://apify.com/thescrappa/instagram-user-posts-cheapest-0-20-1000-results)
- [Pinterest Search Scraper](https://apify.com/thescrappa/pinterest-search-scraper)
- [TikTok Search Scraper](https://apify.com/thescrappa/tiktok-search-scraper)
