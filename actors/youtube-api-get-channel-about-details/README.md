# YouTube Channel About Details Scraper

Review a YouTube channel’s About details, including name, description, custom URL and country. Submit a YouTube channel ID to retrieve the public details shown on its About page.

## What data can you extract?

Titles, publication details and engagement counts reflect public YouTube pages; some fields are hidden or unavailable for a video.

| Field | Type | Description |
| --- | --- | --- |
| `channelId` | string | YouTube channel ID for the YouTube channel record, assigned by YouTube; null when the source does not expose it. |
| `title` | string | Title of the YouTube channel record, as shown by YouTube; null when no title is published. |
| `description` | string | Description text from YouTube for this YouTube channel record; null when the source has no text to show. |
| `customUrl` | string | Custom url shown for the YouTube channel record by YouTube, in the format used by the source; null when it is omitted. |
| `country` | string | Country shown for the YouTube channel record by YouTube; null when YouTube does not provide the value. |
| `joinedDate` | string | Date the channel joined youtube shown by YouTube, in the format displayed by the source; null if the source omits the date. |

## Use cases

- Creator teams can review a channel profile or catalog before a partnership discussion.
- Researchers can compare channel descriptions, subscriber counts and published videos.
- Analysts can maintain a directory of public YouTube channels.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Set `ids` to the search term or source identifier you want to look up, then use the optional filters listed below.
3. Start the run and open its default dataset to inspect or download the rows.

```json
{
  "ids": "UCJZv4d5rbIKd4QHMPkcABCw,UC_x5XG1OV2P6uZZ5FSM9Ttw"
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `ids` | string | No | Comma-separated YouTube channel IDs. Prefer this for batch runs. |
| `id` | string | No | Single YouTube channel ID. Use ids for batch runs. |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "title": "Harborlight Studio",
  "description": "Harborlight Studio shares practical gardening lessons for apartments and small homes.",
  "channelId": "UCaBcdEFghIJKlMNopQRSTuv",
  "customUrl": "@HarborlightStudio",
  "country": "United States",
  "joinedDate": "June 18, 2018"
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

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~youtube-api-get-channel-about-details/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### Can I use a YouTube handle for About details?

This Actor expects a channel ID in `id` or `ids`. A handle is a different identifier, so resolve it to a channel ID before making the request.

## Related Scrappa Actors

- [YouTube Batch Video Scraper](https://apify.com/thescrappa/youtube-api-batch-videos)
- [YouTube Channel Podcasts Scraper](https://apify.com/thescrappa/youtube-api-channel-podcasts)
- [YouTube Channel Video Scraper](https://apify.com/thescrappa/youtube-api-channel-videos)
- [YouTube Channel Community Posts Scraper](https://apify.com/thescrappa/youtube-api-get-channel-community)
- [YouTube Channel Details Scraper](https://apify.com/thescrappa/youtube-api-get-channel-details)
