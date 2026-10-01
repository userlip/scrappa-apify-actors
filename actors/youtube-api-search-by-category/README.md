# YouTube Category Search Scraper

Find YouTube videos in selected categories with titles, descriptions and channel details. Choose one or more video categories and supported sort, length or upload-time filters.

## What data can you extract?

Titles, publication details and engagement counts reflect public YouTube pages; some fields are hidden or unavailable for a video.

| Field | Type | Description |
| --- | --- | --- |
| `type` | string | Category assigned to the YouTube video by YouTube; null when YouTube does not provide the value. |
| `id` | string | source ID for the YouTube video, assigned by YouTube; null when the source does not expose it. |
| `title` | string | Title of the YouTube video, as shown by YouTube; null when no title is published. |
| `description` | string | Description text from YouTube for this YouTube video; null when the source has no text to show. |
| `thumbnail` | string | Thumbnail url shown for the YouTube video by YouTube, in the format used by the source; null when it is omitted. |
| `duration` | string | Duration of this YouTube video, in the duration format shown by the source; null when YouTube provides no timing information. |
| `viewCount` | string | Number of video views shown by YouTube, as a number or digit string; zero is possible, and null means no count was reported. |
| `publishedTime` | string | Publication age displayed by YouTube, such as “2 days ago”; null when YouTube does not show it. |
| `channel` | object | YouTube channel details with channel ID, title, URL, thumbnail and subscriber count; null when the source provides no details. |
| `badges` | array | Source labels attached to this video, such as live or premium status from YouTube; an empty list when no entries are available. |
| `isLive` | boolean | Whether the video is a livestream; false is a reported value, while null means YouTube provided no flag. |
| `isShort` | boolean | Whether YouTube classifies the video as a Short; false is a reported value, while null means YouTube provided no flag. |
| `isPremium` | boolean | Whether YouTube marks the video as premium; false is a reported value, while null means YouTube provided no flag. |
| `expandableMetadata` | object | Video search metadata with source labels and related search details from YouTube; null when the source provides no details. |

## Use cases

- Video researchers can collect public titles and channel details for a topic or video list.
- Creators can compare publishing details before planning follow-up content.
- Content teams can build a searchable catalog from YouTube videos.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Set `category` to the search term or source identifier you want to look up, then use the optional filters listed below.
3. Start the run and open its default dataset to inspect or download the rows.

```json
{
  "category": [
    "education"
  ],
  "sort": [
    "relevance"
  ],
  "duration": [
    "short"
  ],
  "upload_date": [
    "hour"
  ],
  "limit": 10
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `category` | array of string | Yes | Category to search in Constraints: minimum 1 items; maximum 1 items. |
| `sort` | array of string | No | Sort by: relevance, rating, upload_date, view_count Constraints: minimum 0 items; maximum 1 items. |
| `duration` | array of string | No | Video duration: short, medium, long Constraints: minimum 0 items; maximum 1 items. |
| `upload_date` | array of string | No | Upload date filter Constraints: minimum 0 items; maximum 1 items. |
| `limit` | integer | No | Maximum number of results to request. The upstream API can return a smaller page with a continuation token. Constraints: minimum 1; maximum 1024. |
| `continuation` | string | No | Pagination token for next page |
| `contentType` | array of string | No | Filter by content type Constraints: minimum 0 items; maximum 1 items. |
| `features` | string | No | Comma-separated: live,4k,hd,subtitles,cc,360,vr180,3d,hdr |

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
  "title": "A Simple Guide to Container Gardening",
  "description": "Simple container choices and care tips for growing herbs in a small space.",
  "viewCount": "18400",
  "type": "Video",
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

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~youtube-api-search-by-category/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### Which video categories can YouTube Category Search use?

Choose a category supported by the input selector, then optionally set `sort`, `duration`, `upload_date` or other listed filters. Category results depend on YouTube availability.

## Related Scrappa Actors

- [YouTube Batch Video Scraper](https://apify.com/thescrappa/youtube-api-batch-videos)
- [YouTube Channel Podcasts Scraper](https://apify.com/thescrappa/youtube-api-channel-podcasts)
- [YouTube Channel Video Scraper](https://apify.com/thescrappa/youtube-api-channel-videos)
- [YouTube Channel About Details Scraper](https://apify.com/thescrappa/youtube-api-get-channel-about-details)
- [YouTube Channel Community Posts Scraper](https://apify.com/thescrappa/youtube-api-get-channel-community)
