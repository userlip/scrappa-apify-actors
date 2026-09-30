# YouTube Trending Video Scraper for Video Analysis

The YouTube Trending Video Scraper for Video Analysis collects video, channel, and search result data from YouTube. Provide the fields listed below; the actor saves source fields such as `type`, `id`, `title`, and `description` to an Apify dataset.

## What data can you extract?

The dataset contains fields returned by YouTube. The field names below match the Actor output schema. A source may leave optional values empty or omit fields when they are not available for a result.

| Field | Type | Description |
| --- | --- | --- |
| `type` | string | The result type returned by YouTube. |
| `id` | string | The YouTube video identifier. |
| `title` | string | The video title. |
| `description` | string | The video description or snippet. |
| `thumbnail` | string | The video thumbnail URL. |
| `duration` | string | The display duration for the video. |
| `viewCount` | string | The display view count returned by YouTube. |
| `publishedTime` | string | The relative publish time returned by YouTube. |
| `channel` | object | Channel metadata for the video. |
| `badges` | array | Labels returned by YouTube. |
| `isLive` | boolean | Is live value returned for this record. |
| `isShort` | boolean | Is Short value returned for this record. |
| `isPremium` | boolean | Is Premium value returned for this record. |
| `expandableMetadata` | object | Expandable metadata value returned for this record. |

## Use cases

- Collect video, channel, and search result data for video and creator research.
- Review returned titles, channels, timestamps, or engagement fields.
- Export video records to a content planning or analysis workflow.

## How to use

1. Open the **Input** tab and use the example JSON below.
2. Change the query, URL, identifier, or other fields you need. Set the lookup fields to match the query or identifier you want to collect.
3. Start the Actor. Open the run's default dataset to inspect, download, or export the returned records.

```json
{
  "type": [
    "now"
  ]
}
```

Apify stores the run output in a dataset. You can download the dataset in JSON, CSV, Excel, XML, or other available formats from the run page.

## Output example

This illustrative record uses synthetic values. It shows the real output field names; optional source values may be null or absent.

```json
{
  "type": "Example value",
  "id": "example-123",
  "title": "Example result",
  "description": "Example public text.",
  "thumbnail": "https://example.com/image.jpg",
  "duration": "3:21",
  "viewCount": "42"
}
```

## Input fields

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `category` | array of string | No | Optional category filter. Leave this empty to use YouTube’s default trending feed. |
| `type` | array of string | No | Trending period or mode. |

## Pricing

**Current live price:** Apify Free tier: $0.30 per 1,000 results; Bronze: $0.25 per 1,000 results; Silver: $0.22 per 1,000 results; Gold, Platinum, and Diamond: $0.20 per 1,000 results.

The price is based on the latest live Apify pricing entry. Per-result charges depend on the number of billed results returned. If the price line lists subscription tiers, the rate shown for each tier applies to that Apify subscription level.

## FAQ

### Is it legal to scrape this data?

This Actor is intended for data that is publicly available from YouTube. You are responsible for following the source site's terms, privacy and copyright rules, and the laws that apply to your use of the data. Only collect information you have a lawful basis to use.

### What limits should I expect?

Use the field constraints and limits in the input table. The source controls which records are available, so a narrow query or unavailable page can return fewer results or none. Keep batch lists small when you need a quick first run.

### Can I run it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/actors/thescrappa~youtube-api-trending-videos/runs`. The response includes `defaultDatasetId`; use it to fetch the run's dataset items. See the [Apify Run Actor API](https://docs.apify.com/api/v2/actors-runs-post) and [Actor runs guide](https://docs.apify.com/api/v2/actors-actor-runs). Apify integrations for [Make](https://docs.apify.com/integrations/make), [Zapier](https://docs.apify.com/integrations/zapier), and [n8n](https://docs.apify.com/integrations/n8n) can trigger runs and pass results to the next workflow step. Send dataset rows to Google Sheets through a Sheets step or a workflow integration.

### What happens if a request fails?

Apify reports input validation and source request errors in the run details. Review the error, correct the input, and retry after a temporary source problem. Depending on when a request stops, the dataset may be empty or contain results collected before the failure.

## Related Scrappa Actors

- [YouTube Batch Video Scraper for Creator Research](https://apify.com/thescrappa/youtube-api-batch-videos)
- [YouTube Channel Podcast Scraper for Video Analysis](https://apify.com/thescrappa/youtube-api-channel-podcasts)
- [YouTube Channel Video Scraper for Creator Research](https://apify.com/thescrappa/youtube-api-channel-videos)
- [YouTube Channel Profile Scraper for Creators](https://apify.com/thescrappa/youtube-api-get-channel-about-details)
- [YouTube Channel Community Posts Scraper for Brands](https://apify.com/thescrappa/youtube-api-get-channel-community)
