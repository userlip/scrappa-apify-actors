# TrustedShops Shop Profile Scraper for Marketing

The TrustedShops Shop Profile Scraper for Marketing collects public profile fields and account details from Trusted Shops. Provide one or more public URLs; the actor saves source fields such as `tsid`, `requested_tsid`, `name`, and `url` to an Apify dataset.

## What data can you extract?

The dataset contains fields returned by Trusted Shops. The field names below match the Actor output schema. A source may leave optional values empty or omit fields when they are not available for a result.

| Field | Type | Description |
| --- | --- | --- |
| `tsid` | text | TSID returned for this result. |
| `requested_tsid` | text | Requested TSID returned for this result. |
| `name` | text | Shop Name returned for this result. |
| `url` | link | Merchant URL returned for this result. |
| `profile_url` | link | TrustedShops Profile returned for this result. |
| `language` | text | Language returned for this result. |
| `target_market` | text | Target Market returned for this result. |
| `rating` | number | Rating returned for this result. |
| `review_count` | number | Reviews returned for this result. |
| `certified` | boolean | Certified returned for this result. |
| `category_names` | text | Categories returned for this result. |
| `category_ids` | text | Category IDs returned for this result. |
| `source_url` | link | Source URL returned for this result. |
| `profile_metadata` | object | Profile Metadata returned for this result. |

## Use cases

- Collect public profile fields and account details to support reputation research.
- Compare records across the input queries or entities you provide.
- Export structured results to research and reporting workflows.

## How to use

1. Open the **Input** tab and use the example JSON below.
2. Change the query, URL, identifier, or other fields you need. For multi-target work, use `tsids` and start with a short list.
3. Start the Actor. Open the run's default dataset to inspect, download, or export the returned records.

```json
{
  "tsids": [
    "XFB15FFBDE1DEE7A55D292A7D48598A6A"
  ],
  "urls": [
    "https://www.trustedshops.de/bewertung/info_XFB15FFBDE1DEE7A55D292A7D48598A6A.html"
  ]
}
```

Apify stores the run output in a dataset. You can download the dataset in JSON, CSV, Excel, XML, or other available formats from the run page.

## Output example

This illustrative record uses synthetic values. It shows the real output field names; optional source values may be null or absent.

```json
{
  "tsid": "example-123",
  "requested_tsid": "example-123",
  "name": "Example value",
  "url": "https://example.com/result/1",
  "profile_url": "https://example.com/result/1",
  "language": "en",
  "target_market": "Example value"
}
```

## Input fields

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `tsids` | array of string | No | Recommended. Process many TrustedShops IDs in one Apify run. Each successful TSID produces one charged dataset item. |
| `urls` | array of string | No | TrustedShops profile URLs. The actor extracts TSIDs from URLs when possible, then fetches the shop profile. |
| `tsid` | string | No | Single TrustedShops ID. Prefer TSIDs for batch usage. |
| `url` | string | No | Single TrustedShops profile URL. Prefer URLs for batch usage. |
| `include_raw_response` | boolean | No | Include the full Scrappa TrustedShops shop profile response in each dataset item for debugging or custom field mapping. |

## Pricing

**Current live price:** $0.20 per 1,000 results.

The price is based on the latest live Apify pricing entry. Per-result charges depend on the number of billed results returned. If the price line lists subscription tiers, the rate shown for each tier applies to that Apify subscription level.

## FAQ

### Is it legal to scrape this data?

This Actor is intended for data that is publicly available from Trusted Shops. You are responsible for following the source site's terms, privacy and copyright rules, and the laws that apply to your use of the data. Only collect information you have a lawful basis to use.

### What limits should I expect?

Use the field constraints and limits in the input table. The source controls which records are available, so a narrow query or unavailable page can return fewer results or none. Keep batch lists small when you need a quick first run.

### Can I run it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/actors/thescrappa~trustedshops-shop-profile-scraper/runs`. The response includes `defaultDatasetId`; use it to fetch the run's dataset items. See the [Apify Run Actor API](https://docs.apify.com/api/v2/actors-runs-post) and [Actor runs guide](https://docs.apify.com/api/v2/actors-actor-runs). Apify integrations for [Make](https://docs.apify.com/integrations/make), [Zapier](https://docs.apify.com/integrations/zapier), and [n8n](https://docs.apify.com/integrations/n8n) can trigger runs and pass results to the next workflow step. Send dataset rows to Google Sheets through a Sheets step or a workflow integration.

### What happens if a request fails?

Apify reports input validation and source request errors in the run details. Review the error, correct the input, and retry after a temporary source problem. Depending on when a request stops, the dataset may be empty or contain results collected before the failure.

## Related Scrappa Actors

- [Google Maps Reviews Scraper for Local Reputation](https://apify.com/thescrappa/google-maps-reviews-scraper)
- [Jameda Reviews Scraper for Lead Research](https://apify.com/thescrappa/jameda-reviews-scraper)
- [Kununu Reviews Scraper for Campaign Research](https://apify.com/thescrappa/kununu-reviews-scraper)
- [TrustedShops Reviews Scraper for Campaign Research](https://apify.com/thescrappa/trustedshops-reviews-scraper)
- [Trusted Shops Search Scraper for Campaign Research](https://apify.com/thescrappa/trustedshops-search-scraper)
