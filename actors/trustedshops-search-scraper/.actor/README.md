# Trusted Shops Search Scraper for Campaign Research

The Trusted Shops Search Scraper for Campaign Research collects search results, names, and source links from Trusted Shops. Provide a search phrase or a short list of phrases; the actor saves source fields such as `accountName`, `shopName`, `tsID`, and `averageRating` to an Apify dataset.

## What data can you extract?

The dataset contains fields returned by Trusted Shops. The field names below match the Actor output schema. A source may leave optional values empty or omit fields when they are not available for a result.

| Field | Type | Description |
| --- | --- | --- |
| `accountName` | text | Account Name returned for this result. |
| `shopName` | text | Shop Name returned for this result. |
| `tsID` | text | TSID returned for this result. |
| `averageRating` | number | Rating returned for this result. |
| `reviewCount` | number | Reviews returned for this result. |
| `certificationState` | boolean | Certified returned for this result. |
| `profile_url` | link | Trusted Shops Profile returned for this result. |
| `shop_url` | link | Shop URL returned for this result. |
| `profileType` | text | Profile Type returned for this result. |
| `category_names` | text | Categories returned for this result. |
| `shopDescription` | text | Description returned for this result. |
| `shopLogoUrl` | image | Logo returned for this result. |
| `contractStartDate` | number | Contract Start returned for this result. |
| `request_q` | text | Request Query returned for this result. |
| `request_market` | text | Market returned for this result. |
| `request_page` | number | Page returned for this result. |
| `total_shop_count` | number | Total Shops returned for this result. |
| `total_page_count` | number | Total Pages returned for this result. |

## Use cases

- Collect search results, names, and source links to support reputation research.
- Compare records across the input queries or entities you provide.
- Export structured results to research and reporting workflows.

## How to use

1. Open the **Input** tab and use the example JSON below.
2. Change the query, URL, identifier, or other fields you need. Set the lookup fields to match the query or identifier you want to collect.
3. Start the Actor. Open the run's default dataset to inspect, download, or export the returned records.

```json
{
  "q": "zalando",
  "page": 0
}
```

Apify stores the run output in a dataset. You can download the dataset in JSON, CSV, Excel, XML, or other available formats from the run page.

## Output example

This illustrative record uses synthetic values. It shows the real output field names; optional source values may be null or absent.

```json
{
  "accountName": "42",
  "shopName": "Example value",
  "tsID": "example-123",
  "averageRating": 4.7,
  "reviewCount": 42,
  "certificationState": true,
  "profile_url": "https://example.com/result/1"
}
```

## Input fields

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `q` | string | Yes | Shop, brand, or domain query to search on Trusted Shops. |
| `market` | string | No | Trusted Shops target market. Constraints: allowed values: DEU, GBR, AUT, CHE, NLD, ESP, ITA, FRA, BEL, POL, PRT. |
| `page` | integer | No | Zero-based Trusted Shops search results page. Constraints: minimum 0; maximum 100. |
| `max_pages` | integer | No | Number of result pages to fetch, starting from Start Page. Constraints: minimum 1; maximum 10. |

## Pricing

**Current live price:** $0.20 per 1,000 results.

The price is based on the latest live Apify pricing entry. Per-result charges depend on the number of billed results returned. If the price line lists subscription tiers, the rate shown for each tier applies to that Apify subscription level.

## FAQ

### Is it legal to scrape this data?

This Actor is intended for data that is publicly available from Trusted Shops. You are responsible for following the source site's terms, privacy and copyright rules, and the laws that apply to your use of the data. Only collect information you have a lawful basis to use.

### What limits should I expect?

Use the field constraints and limits in the input table. The source controls which records are available, so a narrow query or unavailable page can return fewer results or none. Keep batch lists small when you need a quick first run.

### Can I run it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/actors/thescrappa~trustedshops-search-scraper/runs`. The response includes `defaultDatasetId`; use it to fetch the run's dataset items. See the [Apify Run Actor API](https://docs.apify.com/api/v2/actors-runs-post) and [Actor runs guide](https://docs.apify.com/api/v2/actors-actor-runs). Apify integrations for [Make](https://docs.apify.com/integrations/make), [Zapier](https://docs.apify.com/integrations/zapier), and [n8n](https://docs.apify.com/integrations/n8n) can trigger runs and pass results to the next workflow step. Send dataset rows to Google Sheets through a Sheets step or a workflow integration.

### What happens if a request fails?

Apify reports input validation and source request errors in the run details. Review the error, correct the input, and retry after a temporary source problem. Depending on when a request stops, the dataset may be empty or contain results collected before the failure.

## Related Scrappa Actors

- [Google Maps Reviews Scraper for Local Reputation](https://apify.com/thescrappa/google-maps-reviews-scraper)
- [Jameda Reviews Scraper for Lead Research](https://apify.com/thescrappa/jameda-reviews-scraper)
- [Kununu Reviews Scraper for Campaign Research](https://apify.com/thescrappa/kununu-reviews-scraper)
- [TrustedShops Reviews Scraper for Campaign Research](https://apify.com/thescrappa/trustedshops-reviews-scraper)
- [TrustedShops Shop Profile Scraper for Marketing](https://apify.com/thescrappa/trustedshops-shop-profile-scraper)
