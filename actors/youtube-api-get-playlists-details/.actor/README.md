# YouTube Playlist Details Scraper

Look up a YouTube playlist with its title, description, channel and video count. Paste a YouTube playlist ID to retrieve its details.

## What data can you extract?

Titles, publication details and engagement counts reflect public YouTube pages; some fields are hidden or unavailable for a video.

| Field | Type | Description |
| --- | --- | --- |
| `id` | string | source ID for the YouTube playlist, assigned by YouTube; null when the source does not expose it. |
| `playlistId` | string | YouTube playlist ID for the YouTube playlist, assigned by YouTube; null when the source does not expose it. |
| `title` | string | Title of the YouTube playlist, as shown by YouTube; null when no title is published. |
| `description` | string | Description text from YouTube for this YouTube playlist; null when the source has no text to show. |
| `thumbnail` | string | Thumbnail url shown for the YouTube playlist by YouTube, in the format used by the source; null when it is omitted. |
| `author` | object | Creator profile with ID, username, display name, avatar and follower count from YouTube; null when the source provides no details. |
| `videoCount` | integer/number/string | Number of videos shown by YouTube, as a number or digit string; zero is possible, and null means no count was reported. |
| `viewCount` | integer/number/string | Number of video views shown by YouTube, as a number or digit string; zero is possible, and null means no count was reported. |
| `lastUpdated` | string | Last update time shown by the source shown by YouTube, in ISO 8601 date and time; null if the source omits the date. |
| `videos` | array | Videos in the playlist with video ID, title and URL from YouTube; an empty list when no entries are available. |
| `continuation` | string | Continuation shown for the YouTube playlist by YouTube, in the format used by the source; null when it is omitted. |

## Use cases

- Content teams can catalog playlists and compare titles, owners and video counts.
- Researchers can locate collections around a topic before sampling videos.
- Channel managers can check playlist details while maintaining a public catalog.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Set `id` to the search term or source identifier you want to look up, then use the optional filters listed below.
3. Start the run and open its default dataset to inspect or download the rows.

```json
{
  "id": "PLrAXtmErZgOeiKm4sgNOknGvNjby9efdf"
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `id` | string | Yes | YouTube playlist ID. |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "playlistId": "PL7nQX9kL2aB4cD6eF8gH0iJmNopQrStUvW",
  "title": "Balcony Gardening for Beginners",
  "description": "A short collection of practical guides to containers, soil and herbs for small spaces.",
  "author": {
    "id": "UCaBcdEFghIJKlMNopQRSTuv",
    "name": "Harborlight Studio",
    "url": "https://www.youtube.com/@HarborlightStudio",
    "thumbnail": "https://images.example.com/channels/harborlight-studio.jpg",
    "subscriberCount": 18600
  },
  "videoCount": 246,
  "videos": [
    {
      "videoId": "aB3dE5fG7hJ",
      "title": "Market morning in Portland",
      "url": "https://www.youtube.com/watch?v=aB3dE5fG7hJ"
    }
  ],
  "viewCount": 18400,
  "id": "PL7nQX9kL2aB4cD6eF8gH0iJmNopQrStUvW"
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

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~youtube-api-get-playlists-details/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### Where do I find the YouTube playlist ID?

Copy the playlist ID from a public playlist URL, or use YouTube Playlist Search to find a playlist first. Submit that ID in `id`.

## Related Scrappa Actors

- [YouTube Batch Video Scraper](https://apify.com/thescrappa/youtube-api-batch-videos)
- [YouTube Channel Podcasts Scraper](https://apify.com/thescrappa/youtube-api-channel-podcasts)
- [YouTube Channel Video Scraper](https://apify.com/thescrappa/youtube-api-channel-videos)
- [YouTube Channel About Details Scraper](https://apify.com/thescrappa/youtube-api-get-channel-about-details)
- [YouTube Channel Community Posts Scraper](https://apify.com/thescrappa/youtube-api-get-channel-community)
