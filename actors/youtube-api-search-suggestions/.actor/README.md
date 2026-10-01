# YouTube Search Suggestions Scraper

Find YouTube search suggestions for a phrase and locale. Enter a phrase and select a language or country to request localized YouTube suggestions.

## What data can you extract?

Titles, publication details and engagement counts reflect public YouTube pages; some fields are hidden or unavailable for a video.

| Field | Type | Description |
| --- | --- | --- |
| `query` | string | Query shown for the YouTube video by YouTube, in the format used by the source; null when it is omitted. |
| `suggestion` | string | Suggested phrase shown for the YouTube video by YouTube, in the format used by the source; null when it is omitted. |
| `position` | integer | Result position in the YouTube YouTube video list, as a whole number; null when the source does not supply one. |
| `hl` | string | Hl shown for the YouTube video by YouTube, in the format used by the source; null when it is omitted. |
| `gl` | string | Gl shown for the YouTube video by YouTube, in the format used by the source; null when it is omitted. |

## Use cases

- Video researchers can collect public titles and channel details for a topic or video list.
- Creators can compare publishing details before planning follow-up content.
- Content teams can build a searchable catalog from YouTube videos.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Set `q` to the search term or source identifier you want to look up, then use the optional filters listed below.
3. Start the run and open its default dataset to inspect or download the rows.

```json
{
  "q": "javascript",
  "hl": "en",
  "gl": "US"
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `q` | string | Yes | Search query to get YouTube autocomplete suggestions for. |
| `hl` | string | No | YouTube interface language code. |
| `gl` | string | No | Two-letter country code used for localized suggestions. |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "query": "weekend markets in Seattle",
  "suggestion": "weekend farmers markets",
  "hl": "en",
  "gl": "us"
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

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~youtube-api-search-suggestions/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### Can YouTube Search Suggestions return suggestions in another language?

Set `q` to the phrase and use `hl` for a language code and `gl` for a country code. Suggestions depend on the selected locale and source response.

## Related Scrappa Actors

- [YouTube Batch Video Scraper](https://apify.com/thescrappa/youtube-api-batch-videos)
- [YouTube Channel Podcasts Scraper](https://apify.com/thescrappa/youtube-api-channel-podcasts)
- [YouTube Channel Video Scraper](https://apify.com/thescrappa/youtube-api-channel-videos)
- [YouTube Channel About Details Scraper](https://apify.com/thescrappa/youtube-api-get-channel-about-details)
- [YouTube Channel Community Posts Scraper](https://apify.com/thescrappa/youtube-api-get-channel-community)
