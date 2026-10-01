# TikTok Profile Scraper

Look up a public TikTok profile with display name, username and follower counts. Enter a public TikTok username to retrieve profile details without searching for posts.

## What data can you extract?

Profile names and counts follow public TikTok account details; private or unavailable values can be null.

| Field | Type | Description |
| --- | --- | --- |
| `unique_id` | text | TikTok username for this profile; null when the account does not expose a username. |
| `nickname` | text | Nickname shown for the TikTok profile by TikTok, in the format used by the source; null when it is omitted. |
| `user_id` | text | user ID for the TikTok profile, assigned by TikTok; null when the source does not expose it. |
| `follower_count` | number | Number of followers shown by TikTok, as a whole number; zero is possible, and null means no count was reported. |
| `following_count` | number | Number of followings shown by TikTok, as a whole number; zero is possible, and null means no count was reported. |
| `heart_count` | number | Number of likes shown by TikTok, as a whole number; zero is possible, and null means no count was reported. |
| `video_count` | number | Number of videos shown by TikTok, as a whole number; zero is possible, and null means no count was reported. |
| `verified` | boolean | Whether the source marks the profile or review as verified; false is a reported value, while null means TikTok provided no flag. |
| `private_account` | boolean | Whether the account is private; false is a reported value, while null means TikTok provided no flag. |
| `region` | text | Region shown for the TikTok profile by TikTok; null when TikTok does not provide the value. |
| `avatar` | link | Profile image url for this TikTok profile on TikTok; null when the source does not provide a URL. |

## Use cases

- Social researchers can discover TikTok videos matching a phrase or creator.
- Creators can compare captions and engagement before choosing a topic.
- Campaign teams can monitor posts linked to a product or event.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Provide a public TikTok identifier or URL in `profile`; optional fields control the lookup where supported.
3. Start the run and open its default dataset to inspect or download the rows.

```json
{
  "profile": "@tiktok"
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `profile` | string | Yes | TikTok username with or without @, full HTTPS TikTok profile URL, or numeric user ID up to 30 digits. Bare numeric values are treated as user IDs; prefix numeric usernames with @. |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "follower_count": 12600,
  "unique_id": "morgan.creates",
  "nickname": "Morgan Creates",
  "user_id": "7210458831",
  "following_count": 418,
  "heart_count": 26400,
  "video_count": 246,
  "verified": true
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

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~tiktok-profile-scraper/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### Should the TikTok profile input include an @ sign?

Enter the public username in `profile` as accepted by the Input tab. This Actor returns profile details for that account; it does not search by display name.

## Related Scrappa Actors

- [TikTok Ads Scraper](https://apify.com/thescrappa/tiktok-ads-scraper)
- [TikTok Hashtag Details Scraper](https://apify.com/thescrappa/tiktok-challenge-details-scraper)
- [TikTok Hashtag Videos Scraper](https://apify.com/thescrappa/tiktok-challenge-posts-scraper)
- [TikTok Challenge Search Scraper](https://apify.com/thescrappa/tiktok-challenge-search-scraper)
- [TikTok Comments Scraper](https://apify.com/thescrappa/tiktok-comments-scraper)
