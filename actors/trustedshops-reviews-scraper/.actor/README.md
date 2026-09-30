# TrustedShops Reviews Scraper for Campaign Research

The TrustedShops Reviews Scraper for Campaign Research collects reviews, ratings, and comment details from Trusted Shops. Provide one or more public URLs; the actor saves source fields such as `tsid`, `shop_name`, `rating`, and `review_title` to an Apify dataset.

## What data can you extract?

The dataset contains fields returned by Trusted Shops. The field names below match the Actor output schema. A source may leave optional values empty or omit fields when they are not available for a result.

| Field | Type | Description |
| --- | --- | --- |
| `tsid` | text | TSID returned for this result. |
| `shop_name` | text | Shop returned for this result. |
| `rating` | number | Rating returned for this result. |
| `review_title` | text | Title returned for this result. |
| `review_text` | text | Review returned for this result. |
| `created_at` | date | Created returned for this result. |
| `verified` | boolean | Verified returned for this result. |
| `criteria` | object | Criteria returned for this result. |
| `review_id` | text | Review ID returned for this result. |
| `page` | number | Page returned for this result. |
| `source_url` | link | Source URL returned for this result. |
| `request_market` | text | Market returned for this result. |
| `page_total_reviews` | number | Total Reviews returned for this result. |
| `page_total_pages` | number | Total Pages returned for this result. |

## Use cases

- Collect reviews, ratings, and comment details to support reputation research.
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
  ],
  "page": 1
}
```

Apify stores the run output in a dataset. You can download the dataset in JSON, CSV, Excel, XML, or other available formats from the run page.

## Output example

This illustrative record uses synthetic values. It shows the real output field names; optional source values may be null or absent.

```json
{
  "tsid": "example-123",
  "shop_name": "Example value",
  "rating": 4.7,
  "review_title": "Example result",
  "review_text": "Example public text.",
  "created_at": "2026-09-30T10:00:00Z",
  "verified": true
}
```

## Input fields

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `tsids` | array of string | No | TrustedShops shop TSIDs. Up to 50 shops per run. Constraints: maximum 50 items. |
| `urls` | array of string | No | TrustedShops shop profile URLs containing info_<TSID>. Use this for batch monitoring from profile links. Constraints: maximum 50 items. |
| `tsid` | string | No | Single TSID compatibility input. Ignored when TSIDs or Profile URLs are provided. |
| `url` | string | No | Single TrustedShops profile URL compatibility input. |
| `market` | string | No | Optional TrustedShops market parameter when supported by the Scrappa endpoint. Constraints: allowed values: DEU, GBR, AUT, CHE, NLD, ESP, ITA, FRA, BEL, POL, PRT. |
| `page` | integer | No | First TrustedShops reviews page to fetch. Constraints: minimum 1; maximum 100. |
| `max_pages` | integer | No | Number of review pages to fetch per TSID. Constraints: minimum 1; maximum 25. |
| `size` | integer | No | Reviews to request per page from Scrappa. Constraints: minimum 1; maximum 100. |
| `include_raw_responses` | boolean | No | Include full Scrappa page responses in the OUTPUT key-value-store record. Leave disabled for large batch runs. |

## Pricing

**Current live price:** $0.25 per 1,000 results.

The price is based on the latest live Apify pricing entry. Per-result charges depend on the number of billed results returned. If the price line lists subscription tiers, the rate shown for each tier applies to that Apify subscription level.

## FAQ

### Is it legal to scrape this data?

This Actor is intended for data that is publicly available from Trusted Shops. You are responsible for following the source site's terms, privacy and copyright rules, and the laws that apply to your use of the data. Only collect information you have a lawful basis to use.

### What limits should I expect?

Use the field constraints and limits in the input table. The source controls which records are available, so a narrow query or unavailable page can return fewer results or none. Keep batch lists small when you need a quick first run.

### Can I run it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/actors/thescrappa~trustedshops-reviews-scraper/runs`. The response includes `defaultDatasetId`; use it to fetch the run's dataset items. See the [Apify Run Actor API](https://docs.apify.com/api/v2/actors-runs-post) and [Actor runs guide](https://docs.apify.com/api/v2/actors-actor-runs). Apify integrations for [Make](https://docs.apify.com/integrations/make), [Zapier](https://docs.apify.com/integrations/zapier), and [n8n](https://docs.apify.com/integrations/n8n) can trigger runs and pass results to the next workflow step. Send dataset rows to Google Sheets through a Sheets step or a workflow integration.

### What happens if a request fails?

Apify reports input validation and source request errors in the run details. Review the error, correct the input, and retry after a temporary source problem. Depending on when a request stops, the dataset may be empty or contain results collected before the failure.

## Related Scrappa Actors

- [Google Maps Reviews Scraper for Local Reputation](https://apify.com/thescrappa/google-maps-reviews-scraper)
- [Jameda Reviews Scraper for Lead Research](https://apify.com/thescrappa/jameda-reviews-scraper)
- [Kununu Reviews Scraper for Campaign Research](https://apify.com/thescrappa/kununu-reviews-scraper)
- [Trusted Shops Search Scraper for Campaign Research](https://apify.com/thescrappa/trustedshops-search-scraper)
- [TrustedShops Shop Profile Scraper for Marketing](https://apify.com/thescrappa/trustedshops-shop-profile-scraper)
