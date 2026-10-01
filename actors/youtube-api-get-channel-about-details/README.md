# YouTube Channel About Details Scraper

Collect a YouTube channel’s public About information, including its name, description, links and audience totals. Submit a channel ID to retrieve the channel details YouTube makes available.

## What data can you extract?

YouTube channel metadata is grouped under `stats` and `details`. Missing source values appear as null, and `links` is an empty array when the channel has no public links.

| Field | Type | Description |
| --- | --- | --- |
| `channelId` | string | YouTube channel ID from the About response; null only when the response has neither `channelId` nor `id`. |
| `stats` | object | Channel statistics: `joinDate` is the displayed join date, `viewCount` is the lifetime view total, and `country` is the country YouTube reports. Individual values can be null. |
| `links` | array of object | Public links from the channel About page, preserved as source objects with fields such as `title` and `url`; empty when no links are listed. |
| `details` | object | About details: `description`, `email`, `name`, `subscriberCount`, `videoCount` and `channelUrl`. Missing description, email, name or counts can be null; if YouTube omits `channelUrl`, the Actor builds it from the channel ID. Counts retain YouTube’s display format when provided. |

## Use cases

- Creator teams can review a channel profile or catalog before a partnership discussion.
- Researchers can compare public join dates and channel totals across creators.
- Agencies can verify channel names, descriptions and public links before adding a creator to a shortlist.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Enter one or more YouTube channel IDs in `ids`, or one channel ID in `id`. The Actor retrieves the public About details for each channel.
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
  "channelId": "UCaBcdEFghIJKlMNopQRSTuv",
  "stats": {
    "joinDate": "Joined June 18, 2018",
    "viewCount": "1845000",
    "country": "United States"
  },
  "links": [
    {
      "title": "Website",
      "url": "https://harborlight.example/"
    }
  ],
  "details": {
    "description": "Harborlight Studio shares practical gardening lessons for apartments and small homes.",
    "email": null,
    "name": "Harborlight Studio",
    "subscriberCount": "12.4K subscribers",
    "videoCount": "148 videos",
    "channelUrl": "https://www.youtube.com/channel/UCaBcdEFghIJKlMNopQRSTuv"
  }
}
```

## Pricing

**Current live price:** Apify Free tier: $0.30 per 1,000 results; Bronze: $0.25 per 1,000 results; Silver: $0.22 per 1,000 results; Gold, Platinum, and Diamond: $0.20 per 1,000 results.

Each saved channel About record counts as one result.

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
