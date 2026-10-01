# TikTok Hashtag Details Scraper

Look up TikTok hashtag challenges with names, descriptions and participant counts. Submit multiple challenge names in one run to retrieve records for each target.

## What data can you extract?

Captions, profile details and engagement counts reflect public TikTok pages; counts can be hidden or absent.

| Field | Type | Description |
| --- | --- | --- |
| `challenge_id` | text | TikTok challenge ID for the hashtag challenge, assigned by TikTok; null when the source does not expose it. |
| `challenge_name` | text | Tiktok challenge name shown for the hashtag challenge by TikTok, in the format used by the source; null when it is omitted. |
| `description` | text | Description text from TikTok for this hashtag challenge; null when the source has no text to show. |
| `user_count` | number | Number of participants shown by TikTok, as a whole number; zero is possible, and null means no count was reported. |
| `view_count` | number | Number of video views shown by TikTok, as a whole number; zero is possible, and null means no count was reported. |
| `video_count` | number | Number of videos shown by TikTok, as a whole number; zero is possible, and null means no count was reported. |
| `cover` | link | Cover for this hashtag challenge on TikTok; null when the source does not provide a URL. |
| `request_challenge_name` | text | Tiktok challenge name passed to TikTok. This input value is copied into the output row; null when it was not supplied. |
| `request_challenge_id` | text | Tiktok challenge id passed to TikTok. This input value is copied into the output row; null when it was not supplied. |
| `retrieved_at` | date | Time the page was retrieved shown by TikTok, in ISO 8601 date and time; null if the source omits the date. |

## Use cases

- Campaign managers can monitor public videos using a TikTok challenge or hashtag.
- Researchers can compare captions and engagement across posts under a tag.
- Creators can review examples before planning a themed video.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Put the supported targets in `challenge_names` and use the identifier or URL format required by TikTok.
3. Start the run and open its default dataset to inspect or download the rows.

```json
{
  "challenge_names": [
    "booktok",
    "fitness"
  ]
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `challenge_names` | array of string | No | Preferred batch input. Hashtag/challenge names; an optional leading # is removed. Combined names and IDs are capped at 100. Constraints: maximum 100 items. |
| `challenge_ids` | array of string | No | Preferred batch input for stable numeric IDs. Combined names and IDs are capped at 100. Constraints: maximum 100 items. |
| `challenge_name` | string | No | Optional backward-compatible single lookup; processed together with batch inputs. |
| `challenge_id` | string | No | Optional backward-compatible numeric ID; processed together with batch inputs. |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "description": "Three ways to style a vintage wool coat for fall. Which look is your favorite?",
  "view_count": 18400,
  "challenge_id": "7348291056172489012",
  "challenge_name": "#weekendmarket",
  "user_count": 12600,
  "video_count": 246,
  "cover": "https://images.example.com/challenges/weekend-market-cover.jpg",
  "retrieved_at": "2026-10-01T10:30:00Z"
}
```

## Pricing

**Current live price:** $0.25 per 1,000 results.

Each saved video, post or comment record counts as one result.

## FAQ

### Is it legal to collect public information?

This Actor is for information visible on the public source pages it reads. You are responsible for checking the source terms, privacy and copyright rules, and the laws that apply to your use.

### How many records will a run return?

The result count depends on the input limits, pagination settings and what the source makes available. A small query or unavailable page can return fewer records, including none.

### Can I call it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~tiktok-challenge-details-scraper/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### Can I look up a TikTok challenge by its name or ID?

Yes. Use `challenge_name` or `challenge_id` for one lookup, or pass names in `challenge_names` and IDs in `challenge_ids`. TikTok Challenge Search can help find a challenge first.

## Related Scrappa Actors

- [TikTok Ads Scraper](https://apify.com/thescrappa/tiktok-ads-scraper)
- [TikTok Hashtag Videos Scraper](https://apify.com/thescrappa/tiktok-challenge-posts-scraper)
- [TikTok Challenge Search Scraper](https://apify.com/thescrappa/tiktok-challenge-search-scraper)
- [TikTok Comments Scraper](https://apify.com/thescrappa/tiktok-comments-scraper)
- [TikTok Followers Scraper](https://apify.com/thescrappa/tiktok-followers-scraper)
