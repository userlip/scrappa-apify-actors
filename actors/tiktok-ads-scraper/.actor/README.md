# TikTok Ads Scraper

Review public TikTok ad creatives with advertiser names, copy, calls to action and landing pages. Provide a public ad page URL or a list of ad URLs to compare creatives and landing pages.

## What data can you extract?

Captions, profile details and engagement counts reflect public TikTok pages; counts can be hidden or absent.

| Field | Type | Description |
| --- | --- | --- |
| `ad_id` | text | ad ID for the TikTok ad, assigned by TikTok; null when the source does not expose it. |
| `advertiser_name` | text | Advertiser name shown for the TikTok ad by TikTok, in the format used by the source; null when it is omitted. |
| `brand_name` | text | Brand name shown for the TikTok ad by TikTok, in the format used by the source; null when it is omitted. |
| `advertiser_id` | text | advertiser id for the TikTok ad, assigned by TikTok; null when the source does not expose it. |
| `account_name` | text | Account name shown for the TikTok ad by TikTok, in the format used by the source; null when it is omitted. |
| `industry` | text | Industry category shown for the TikTok ad by TikTok; null when TikTok does not provide the value. |
| `objective` | text | Objective shown for the TikTok ad by TikTok, in the format used by the source; null when it is omitted. |
| `creative_text` | text | Creative text from TikTok for this TikTok ad; null when the source has no text to show. |
| `landing_page` | link | Landing page for this TikTok ad on TikTok; null when the source does not provide a URL. |
| `destination` | link | Destination for this TikTok ad on TikTok; null when the source does not provide a URL. |
| `cta` | text | Cta shown for the TikTok ad by TikTok, in the format used by the source; null when it is omitted. |
| `video_url` | link | Video url for this TikTok ad on TikTok; null when the source does not provide a URL. |
| `cover` | link | Cover for this TikTok ad on TikTok; null when the source does not provide a URL. |
| `media_urls` | array | Media URLs attached to the post from TikTok; an empty list when no entries are available. |
| `region` | text | Region shown for the TikTok ad by TikTok; null when TikTok does not provide the value. |
| `country` | text | Country shown for the TikTok ad by TikTok; null when TikTok does not provide the value. |
| `language` | text | Language code or language name used for this text; null when TikTok does not provide the value. |
| `category` | text | Category assigned to the TikTok ad by TikTok; null when TikTok does not provide the value. |
| `like_count` | number | Number of likes shown by TikTok, as a whole number; zero is possible, and null means no count was reported. |
| `comment_count` | number | Number of comments shown by TikTok, as a whole number; zero is possible, and null means no count was reported. |
| `share_count` | number | Number of shares shown by TikTok, as a whole number; zero is possible, and null means no count was reported. |
| `cached` | boolean | Whether the record came from cache; false is a reported value, while null means TikTok provided no flag. |
| `request_url` | link | Source page url passed to TikTok. This input value is copied into the output row; null when it was not supplied. |
| `request_ad_id` | text | Tiktok ad id passed to TikTok. This input value is copied into the output row; null when it was not supplied. |
| `request_index` | number | Zero-based position of this request in the submitted TikTok input batch; null for a single-item lookup. |
| `result_found` | boolean | Whether the requested result was found; false is a reported value, while null means TikTok provided no flag. |
| `error_message` | text | Diagnostic text for the TikTok lookup; null when the request completes without an error. |

## Use cases

- Paid-media teams can compare public ad creatives, calls to action and landing pages.
- Brand researchers can monitor advertisers promoting similar products on TikTok.
- Creative strategists can catalog media assets before writing a campaign brief.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Put the supported targets in `urls` and use the identifier or URL format required by TikTok.
3. Start the run and open its default dataset to inspect or download the rows.

```json
{
  "urls": [
    "https://ads.tiktok.com/business/creativecenter/topads/7213160569871581185/pc/en?countryCode=US&period=30"
  ]
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `urls` | array of string | No | One or more TikTok Creative Center ad URLs. The actor pushes one dataset item for each requested ad URL. Constraints: minimum 1 items; maximum 100 items. |
| `url` | string | No | Legacy single URL field for API callers. Ignored when TikTok Creative Center Ad URLs is provided. |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "media_urls": [
    "https://ads.example.com/creative/seasonal-catalog.mp4"
  ],
  "ad_id": "ad_962814",
  "advertiser_name": "Cedar & Pine Outfitters",
  "brand_name": "Cedar & Pine Outfitters",
  "advertiser_id": "advertiser_587214",
  "account_name": "Cedar & Pine Campaigns",
  "industry": "Retail technology",
  "objective": "website visits"
}
```

## Pricing

**Current live price:** $0.20 per 1,000 results.

Each saved video, post or comment record counts as one result.

## FAQ

### Is it legal to collect public information?

This Actor is for information visible on the public source pages it reads. You are responsible for checking the source terms, privacy and copyright rules, and the laws that apply to your use.

### How many records will a run return?

The result count depends on the input limits, pagination settings and what the source makes available. A small query or unavailable page can return fewer records, including none.

### Can I call it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~tiktok-ads-scraper/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### Can I search TikTok ads by advertiser name?

This Actor takes a public ad URL in `url` or several URLs in `urls`. Start with an ad page URL, then review its public creative, advertiser and landing-page details.

## Related Scrappa Actors

- [TikTok Hashtag Details Scraper](https://apify.com/thescrappa/tiktok-challenge-details-scraper)
- [TikTok Hashtag Videos Scraper](https://apify.com/thescrappa/tiktok-challenge-posts-scraper)
- [TikTok Challenge Search Scraper](https://apify.com/thescrappa/tiktok-challenge-search-scraper)
- [TikTok Comments Scraper](https://apify.com/thescrappa/tiktok-comments-scraper)
- [TikTok Followers Scraper](https://apify.com/thescrappa/tiktok-followers-scraper)
