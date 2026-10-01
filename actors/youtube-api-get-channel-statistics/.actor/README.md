# YouTube Channel Statistics Scraper

Review public YouTube channel totals for views, subscribers and published videos. Provide a channel ID to retrieve the public subscriber, view and video totals currently available.

## What data can you extract?

The dataset contains channel-level totals for views, subscribers and videos rather than individual video records. YouTube may omit subscriber totals or return abbreviated public counts.

| Field | Type | Description |
| --- | --- | --- |
| `channelId` | string | YouTube channel ID for the YouTube channel record, assigned by YouTube; null when the source does not expose it. |
| `viewCount` | number | Total views across the channel as a whole number when YouTube makes the count public; null when the count is omitted. |
| `subscriberCount` | number | Public channel subscriber total when YouTube exposes it; this value may be rounded or null when the count is hidden. |
| `videoCount` | number | Total videos on the channel as a whole number when YouTube provides the count; null when omitted. |

## Use cases

- Creator teams can compare channel-level view and subscriber totals before a partnership discussion.
- Researchers can compare public view and video totals across channels.
- Analysts can refresh a spreadsheet of public YouTube channel statistics.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Enter one or more YouTube channel IDs in `ids`, or a single channel ID in `id`. The Actor returns the public totals available for each channel.
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
  "viewCount": 18400,
  "channelId": "UCaBcdEFghIJKlMNopQRSTuv",
  "subscriberCount": 18600,
  "videoCount": 246
}
```

## Pricing

**Current live price:** Apify Free tier: $0.30 per 1,000 results; Bronze: $0.25 per 1,000 results; Silver: $0.22 per 1,000 results; Gold, Platinum, and Diamond: $0.20 per 1,000 results.

Each saved channel statistics row counts as one result.

## FAQ

### Is it legal to collect public information?

This Actor is for information visible on the public source pages it reads. You are responsible for checking the source terms, privacy and copyright rules, and the laws that apply to your use.

### How many records will a run return?

The result count depends on the input limits, pagination settings and what the source makes available. A small query or unavailable page can return fewer records, including none.

### Can I call it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~youtube-api-get-channel-statistics/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### Are YouTube channel statistics lifetime totals or recent counts?

This Actor returns the public totals supplied for the channel, such as subscriber, view and video counts. The source can hide or omit individual totals.

## Related Scrappa Actors

- [YouTube Batch Video Scraper](https://apify.com/thescrappa/youtube-api-batch-videos)
- [YouTube Channel Podcasts Scraper](https://apify.com/thescrappa/youtube-api-channel-podcasts)
- [YouTube Channel Video Scraper](https://apify.com/thescrappa/youtube-api-channel-videos)
- [YouTube Channel About Details Scraper](https://apify.com/thescrappa/youtube-api-get-channel-about-details)
- [YouTube Channel Community Posts Scraper](https://apify.com/thescrappa/youtube-api-get-channel-community)
