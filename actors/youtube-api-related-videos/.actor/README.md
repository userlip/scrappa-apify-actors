# YouTube Related Videos Scraper

Find YouTube videos related to a video ID, with titles, channels and view counts. Enter a YouTube video ID and pass a continuation value to request another page.

## What data can you extract?

Titles, publication details and engagement counts reflect public YouTube pages; some fields are hidden or unavailable for a video.

| Field | Type | Description |
| --- | --- | --- |
| `id` | string | source ID for the YouTube video, assigned by YouTube; null when the source does not expose it. |
| `videoId` | string | video ID for the YouTube video, assigned by YouTube; null when the source does not expose it. |
| `title` | string | Title of the YouTube video, as shown by YouTube; null when no title is published. |
| `url` | string | URL for this YouTube video on YouTube; null when the source does not provide a link. |
| `thumbnail` | string | Thumbnail url shown for the YouTube video by YouTube, in the format used by the source; null when it is omitted. |
| `duration` | string/number | Duration of this YouTube video, in the duration format shown by the source; null when YouTube provides no timing information. |
| `viewCount` | integer/number/string | Number of video views shown by YouTube, as a number or digit string; zero is possible, and null means no count was reported. |
| `publishedTimeText` | string | Publication age displayed by YouTube, such as “2 days ago”; null when YouTube does not show it. |
| `publishDate` | string | Date the video was published shown by YouTube, in the format displayed by the source; null if the source omits the date. |
| `channel` | object | YouTube channel details with channel ID, title, URL, thumbnail and subscriber count; null when the source provides no details. |

## Use cases

- Video researchers can collect public titles and channel details for a topic or video list.
- Creators can compare publishing details before planning follow-up content.
- Content teams can build a searchable catalog from YouTube videos.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Set `id` to the search term or source identifier you want to look up, then use the optional filters listed below.
3. Start the run and open its default dataset to inspect or download the rows.

```json
{
  "id": "dQw4w9WgXcQ"
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `id` | string | Yes | YouTube video ID |
| `continuation` | string | No | Optional continuation token from a previous related videos response for fetching the next page. |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "channel": {
    "id": "UCaBcdEFghIJKlMNopQRSTuv",
    "name": "Harborlight Studio",
    "url": "https://www.youtube.com/@HarborlightStudio",
    "thumbnail": "https://images.example.com/channels/harborlight-studio.jpg",
    "subscriberCount": 18600
  },
  "title": "Choosing the Right Pots for Herbs",
  "viewCount": 18400,
  "videoId": "aB3dE5fG7hJ",
  "url": "https://www.youtube.com/watch?v=aB3dE5fG7hJ",
  "id": "aB3dE5fG7hJ",
  "thumbnail": "https://images.example.com/video/market-morning-thumb.jpg",
  "duration": "4:18"
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

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~youtube-api-related-videos/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### How do I request another page of related YouTube videos?

Submit a video ID in `id`. For another page, reuse that ID with the `continuation` value returned by the previous response.

## Related Scrappa Actors

- [YouTube Batch Video Scraper](https://apify.com/thescrappa/youtube-api-batch-videos)
- [YouTube Channel Podcasts Scraper](https://apify.com/thescrappa/youtube-api-channel-podcasts)
- [YouTube Channel Video Scraper](https://apify.com/thescrappa/youtube-api-channel-videos)
- [YouTube Channel About Details Scraper](https://apify.com/thescrappa/youtube-api-get-channel-about-details)
- [YouTube Channel Community Posts Scraper](https://apify.com/thescrappa/youtube-api-get-channel-community)
