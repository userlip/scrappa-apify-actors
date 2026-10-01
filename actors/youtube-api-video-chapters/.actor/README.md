# YouTube Video Chapters Scraper

Get YouTube video chapter titles and their start times in seconds. Enter one or more YouTube video IDs to retrieve their chapter lists.

## What data can you extract?

Chapter titles and start or end times come from the chapter markers YouTube provides. A video without chapter markers can still have a dataset row with an empty `chapters` array.

| Field | Type | Description |
| --- | --- | --- |
| `videoId` | string | video ID for the video chapter, assigned by YouTube; null when the source does not expose it. |
| `chapters` | array | Chapter titles with start times in seconds from the video beginning from YouTube; an empty list when no entries are available. |
| `title` | string | Title of the video chapter, as shown by YouTube; null when no title is published. |
| `startTime` | number | Start time in seconds from the start of the YouTube video; null when timing is not available. |
| `endTime` | number | End time in seconds from the start of the YouTube video; null when timing is not available. |

## Use cases

- Video editors can review chapter titles and times while preparing a description.
- Content teams can compare section lengths before reorganizing a tutorial.
- Researchers can jump to relevant sections when coding long-form video.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Enter one or more comma-separated YouTube video IDs in `ids`, or use `id` for a single video. The Actor reads chapter data for those IDs.
3. Start the run and open its default dataset to inspect or download the rows.

```json
{
  "ids": "dQw4w9WgXcQ,aqz-KE-bpKQ"
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `ids` | string | No | Comma-separated YouTube video IDs. Prefer this batch field to process multiple videos in one Apify run. |
| `id` | string | No | Single YouTube video ID. Use ids for batch runs. |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "videoId": "aB3dE5fG7hJ",
  "title": "Choosing Containers and Soil",
  "chapters": [
    {
      "title": "Market entrance",
      "time": 0
    },
    {
      "title": "Coffee stalls",
      "time": 94
    }
  ],
  "startTime": 0,
  "endTime": 94
}
```

## Pricing

**Current live price:** Apify Free tier: $0.30 per 1,000 results; Bronze: $0.25 per 1,000 results; Silver: $0.22 per 1,000 results; Gold, Platinum, and Diamond: $0.20 per 1,000 results.

Each saved video chapter record counts as one result, including a video row whose `chapters` array is empty.

## FAQ

### Is it legal to collect public information?

This Actor is for information visible on the public source pages it reads. You are responsible for checking the source terms, privacy and copyright rules, and the laws that apply to your use.

### How many records will a run return?

The result count depends on the input limits, pagination settings and what the source makes available. A small query or unavailable page can return fewer records, including none.

### Can I call it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~youtube-api-video-chapters/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### What if a YouTube video has no chapters?

Submit a video ID through `id` or `ids`. If the video has no chapter markers, the dataset can still include its video record with an empty `chapters` array.

## Related Scrappa Actors

- [YouTube Batch Video Scraper](https://apify.com/thescrappa/youtube-api-batch-videos)
- [YouTube Channel Podcasts Scraper](https://apify.com/thescrappa/youtube-api-channel-podcasts)
- [YouTube Channel Video Scraper](https://apify.com/thescrappa/youtube-api-channel-videos)
- [YouTube Channel About Details Scraper](https://apify.com/thescrappa/youtube-api-get-channel-about-details)
- [YouTube Channel Community Posts Scraper](https://apify.com/thescrappa/youtube-api-get-channel-community)
