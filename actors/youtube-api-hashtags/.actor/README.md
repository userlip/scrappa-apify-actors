# YouTube Hashtag Videos Scraper

Find public YouTube videos for a hashtag, with titles, channels and view counts. Enter a hashtag and optionally set video length, upload time or sort order.

## What data can you extract?

Titles, publication details and engagement counts reflect public YouTube pages; some fields are hidden or unavailable for a video.

| Field | Type | Description |
| --- | --- | --- |
| `videoId` | string | video ID for the YouTube video, assigned by YouTube; null when the source does not expose it. |
| `title` | string | Title of the YouTube video, as shown by YouTube; null when no title is published. |
| `url` | string | URL for this YouTube video on YouTube; null when the source does not provide a link. |
| `thumbnail` | string | Thumbnail url shown for the YouTube video by YouTube, in the format used by the source; null when it is omitted. |
| `duration` | string | Duration of this YouTube video, in the duration format shown by the source; null when YouTube provides no timing information. |
| `viewCount` | number | Number of video views shown by YouTube, as a whole number; zero is possible, and null means no count was reported. |
| `publishedTimeText` | string | Publication age displayed by YouTube, such as “2 days ago”; null when YouTube does not show it. |
| `channelId` | string | YouTube channel ID for the YouTube video, assigned by YouTube; null when the source does not expose it. |

## Use cases

- Video researchers can collect public titles and channel details for a topic or video list.
- Creators can compare publishing details before planning follow-up content.
- Content teams can build a searchable catalog from YouTube videos.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Set `hashtag` to the search term or source identifier you want to look up, then use the optional filters listed below.
3. Start the run and open its default dataset to inspect or download the rows.

```json
{
  "hashtag": "music",
  "sort": "relevance",
  "duration": "short",
  "upload_date": "hour",
  "limit": 5
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `hashtag` | string | Yes | Public YouTube hashtag to search. Include or omit the # prefix. |
| `sort` | string | No | Sort by: relevance, rating, upload_date, view_count Constraints: allowed values: relevance, rating, upload_date, view_count. |
| `duration` | string | No | Video duration: short, medium, long Constraints: allowed values: short, medium, long. |
| `upload_date` | string | No | Upload date Constraints: allowed values: hour, today, week, month, year. |
| `limit` | integer | No | Results per page (1-20, default: 20) Constraints: minimum 1; maximum 20. |
| `continuation` | string | No | Pagination token for next page |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "title": "Small Space Gardening Tips",
  "viewCount": 18400,
  "videoId": "aB3dE5fG7hJ",
  "url": "https://www.youtube.com/watch?v=aB3dE5fG7hJ",
  "thumbnail": "https://images.example.com/video/market-morning-thumb.jpg",
  "duration": "4:18",
  "publishedTimeText": "2 days ago",
  "channelId": "UCaBcdEFghIJKlMNopQRSTuv"
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

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~youtube-api-hashtags/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### Should the YouTube hashtag include its leading #?

Use the hashtag format shown in the Input tab. The prefilled value includes the leading hash; supported `duration`, `upload_date` and `sort` filters are optional.

## Related Scrappa Actors

- [YouTube Batch Video Scraper](https://apify.com/thescrappa/youtube-api-batch-videos)
- [YouTube Channel Podcasts Scraper](https://apify.com/thescrappa/youtube-api-channel-podcasts)
- [YouTube Channel Video Scraper](https://apify.com/thescrappa/youtube-api-channel-videos)
- [YouTube Channel About Details Scraper](https://apify.com/thescrappa/youtube-api-get-channel-about-details)
- [YouTube Channel Community Posts Scraper](https://apify.com/thescrappa/youtube-api-get-channel-community)
