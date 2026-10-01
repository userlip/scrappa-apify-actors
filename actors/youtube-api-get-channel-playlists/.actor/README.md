# YouTube Channel Playlists Scraper

List playlists from a YouTube channel with titles, thumbnails and video counts. Submit a channel ID or a list of IDs to collect the public playlists published by each channel.

## What data can you extract?

Titles, publication details and engagement counts reflect public YouTube pages; some fields are hidden or unavailable for a video.

| Field | Type | Description |
| --- | --- | --- |
| `id` | string | source ID for the YouTube playlist, assigned by YouTube; null when the source does not expose it. |
| `title` | string | Title of the YouTube playlist, as shown by YouTube; null when no title is published. |
| `thumbnail` | string | Thumbnail url shown for the YouTube playlist by YouTube, in the format used by the source; null when it is omitted. |
| `videoCount` | string | Number of videos shown by YouTube, as a number or digit string; zero is possible, and null means no count was reported. |

## Use cases

- Content teams can catalog playlists and compare titles, owners and video counts.
- Researchers can locate collections around a topic before sampling videos.
- Channel managers can check playlist details while maintaining a public catalog.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Provide one or more YouTube channel IDs in `ids`, or one channel ID in `id`. The Actor lists playlists associated with those channels.
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
| `id` | string | No | Single YouTube channel ID. Use ids for batch runs. |
| `ids` | string | No | Comma-separated YouTube channel IDs. Prefer this for batch runs. |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "title": "Small Space Gardening Guides",
  "id": "aB3dE5fG7hJ",
  "thumbnail": "https://images.example.com/video/market-morning-thumb.jpg",
  "videoCount": "246"
}
```

## Pricing

**Current live price:** Apify Free tier: $0.30 per 1,000 results; Bronze: $0.25 per 1,000 results; Silver: $0.22 per 1,000 results; Gold, Platinum, and Diamond: $0.20 per 1,000 results.

Each saved playlist row counts as one result.

## FAQ

### Is it legal to collect public information?

This Actor is for information visible on the public source pages it reads. You are responsible for checking the source terms, privacy and copyright rules, and the laws that apply to your use.

### How many records will a run return?

The number of playlist rows depends on the playlists returned for each submitted channel ID. The input does not include a pagination field.

### Can I call it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~youtube-api-get-channel-playlists/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### Can I request playlists from several YouTube channels together?

Yes. Use `ids` for a comma-separated list of channel IDs, or `id` for one channel. The result rows identify playlists returned for each channel.

## Related Scrappa Actors

- [YouTube Batch Video Scraper](https://apify.com/thescrappa/youtube-api-batch-videos)
- [YouTube Channel Podcasts Scraper](https://apify.com/thescrappa/youtube-api-channel-podcasts)
- [YouTube Channel Video Scraper](https://apify.com/thescrappa/youtube-api-channel-videos)
- [YouTube Channel About Details Scraper](https://apify.com/thescrappa/youtube-api-get-channel-about-details)
- [YouTube Channel Community Posts Scraper](https://apify.com/thescrappa/youtube-api-get-channel-community)
