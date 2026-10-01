# YouTube Playlist Search Scraper

Find YouTube playlists by phrase, with titles, channel names and playlist links. Enter a playlist search phrase and choose a supported sort order.

## What data can you extract?

Titles, publication details and engagement counts reflect public YouTube pages; some fields are hidden or unavailable for a video.

| Field | Type | Description |
| --- | --- | --- |
| `playlistId` | string | YouTube playlist ID for the YouTube playlist, assigned by YouTube; null when the source does not expose it. |
| `title` | string | Title of the YouTube playlist, as shown by YouTube; null when no title is published. |
| `url` | string | URL for this YouTube playlist on YouTube; null when the source does not provide a link. |
| `thumbnail` | string | Thumbnail url shown for the YouTube playlist by YouTube, in the format used by the source; null when it is omitted. |
| `channelId` | string | YouTube channel ID for the YouTube playlist, assigned by YouTube; null when the source does not expose it. |
| `channelTitle` | string | Name of the YouTube playlist, as shown by YouTube; null when no name is published. |
| `videoCount` | number | Number of videos shown by YouTube, as a whole number; zero is possible, and null means no count was reported. |

## Use cases

- Content teams can catalog playlists and compare titles, owners and video counts.
- Researchers can locate collections around a topic before sampling videos.
- Channel managers can check playlist details while maintaining a public catalog.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Set `q` to the search term or source identifier you want to look up, then use the optional filters listed below.
3. Start the run and open its default dataset to inspect or download the rows.

```json
{
  "q": "music",
  "sort": "relevance",
  "limit": 5
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `q` | string | Yes | Search phrase used to find YouTube playlists. A broad term such as music is a good starting point. |
| `sort` | string | No | Sort by: relevance, rating, upload_date, view_count Constraints: allowed values: relevance, rating, upload_date, view_count. |
| `limit` | integer | No | Results per page (1-20, default: 20) Constraints: minimum 1; maximum 20. |
| `continuation` | string | No | Pagination token for next page |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "title": "Balcony Gardening for Beginners",
  "url": "https://www.youtube.com/watch?v=aB3dE5fG7hJ",
  "playlistId": "PL7nQX9kL2aB4cD6eF8gH0iJmNopQrStUvW",
  "thumbnail": "https://images.example.com/video/market-morning-thumb.jpg",
  "channelId": "UCaBcdEFghIJKlMNopQRSTuv",
  "channelTitle": "Harborlight Studio",
  "videoCount": 246
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

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~youtube-api-playlists/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### What should I enter in YouTube Playlist Search?

Enter a playlist topic or title in `q`. The Actor searches for matching public playlists; use `sort`, `limit` or `continuation` when needed.

## Related Scrappa Actors

- [YouTube Batch Video Scraper](https://apify.com/thescrappa/youtube-api-batch-videos)
- [YouTube Channel Podcasts Scraper](https://apify.com/thescrappa/youtube-api-channel-podcasts)
- [YouTube Channel Video Scraper](https://apify.com/thescrappa/youtube-api-channel-videos)
- [YouTube Channel About Details Scraper](https://apify.com/thescrappa/youtube-api-get-channel-about-details)
- [YouTube Channel Community Posts Scraper](https://apify.com/thescrappa/youtube-api-get-channel-community)
