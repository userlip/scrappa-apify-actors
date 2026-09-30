# Vinted Search Scraper for Product Research

The Vinted Search Scraper for Product Research collects marketplace listings, prices, and item details from Vinted. Provide a search phrase or a short list of phrases; the actor saves source fields such as `id`, `title`, `price_amount`, and `price_currency` to an Apify dataset.

## What data can you extract?

The dataset contains fields returned by Vinted. The field names below match the Actor output schema. A source may leave optional values empty or omit fields when they are not available for a result.

| Field | Type | Description |
| --- | --- | --- |
| `id` | text | Item ID returned for this result. |
| `title` | text | Title returned for this result. |
| `price_amount` | text | Price returned for this result. |
| `price_currency` | text | Currency returned for this result. |
| `total_item_price` | text | Total Price returned for this result. |
| `shipping_price` | text | Shipping returned for this result. |
| `brand_name` | text | Brand returned for this result. |
| `category_name` | text | Category returned for this result. |
| `size_name` | text | Size returned for this result. |
| `condition` | text | Condition returned for this result. |
| `url` | link | Listing URL returned for this result. |
| `image_url` | image | Image returned for this result. |
| `seller_login` | text | Seller returned for this result. |
| `seller_feedback_reputation` | number | Seller Rating returned for this result. |
| `favourite_count` | number | Favorites returned for this result. |
| `view_count` | number | Views returned for this result. |
| `request_query` | text | Search returned for this result. |
| `request_country` | text | Country returned for this result. |
| `request_page` | number | Page returned for this result. |
| `request_order` | text | Sort returned for this result. |
| `total_entries` | number | Total Listings returned for this result. |
| `total_pages` | number | Total Pages returned for this result. |

## Use cases

- Collect marketplace listings, prices, and item details to research product availability and pricing.
- Compare item, seller, and listing details across a small search batch.
- Export marketplace records for catalog or resale analysis.

## How to use

1. Open the **Input** tab and use the example JSON below.
2. Change the query, URL, identifier, or other fields you need. Set the lookup fields to match the query or identifier you want to collect.
3. Start the Actor. Open the run's default dataset to inspect, download, or export the returned records.

```json
{
  "query": "nike shoes",
  "country": "DE",
  "page": 1,
  "per_page": 24,
  "order": "newest_first"
}
```

Apify stores the run output in a dataset. You can download the dataset in JSON, CSV, Excel, XML, or other available formats from the run page.

## Output example

This illustrative record uses synthetic values. It shows the real output field names; optional source values may be null or absent.

```json
{
  "id": "example-123",
  "title": "Example result",
  "price_amount": "129.99",
  "price_currency": "USD",
  "total_item_price": "129.99",
  "shipping_price": "129.99",
  "brand_name": "Example value"
}
```

## Input fields

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `query` | string | No | Vinted search text, for example nike shoes, zara dress, or vintage jacket. Leave empty to browse listings by filters. |
| `country` | string | No | Vinted country market. Constraints: allowed values: FR, DE, ES, IT, NL, BE, AT, PL, CZ, LT, LU, SK, HU, RO, PT, SE, DK, FI, US. |
| `page` | integer | No | One-based Vinted search results page. Constraints: minimum 1; maximum 999. |
| `per_page` | integer | No | Number of listings to request per page. Scrappa allows up to 100. Constraints: minimum 1; maximum 100. |
| `max_pages` | integer | No | Number of result pages to fetch, starting from Start Page. Constraints: minimum 1; maximum 20. |
| `order` | string | No | Vinted result sort order. Constraints: allowed values: relevance, newest_first, price_low_to_high, price_high_to_low. |
| `catalog_ids` | string | No | Comma-separated Vinted category IDs. |
| `brand_ids` | string | No | Comma-separated Vinted brand IDs. |
| `size_ids` | string | No | Comma-separated Vinted size IDs. |
| `color_ids` | string | No | Comma-separated Vinted color IDs. |
| `material_ids` | string | No | Comma-separated Vinted material IDs. |
| `status_ids` | string | No | Comma-separated Vinted item condition/status IDs. |
| `price_from` | number | No | Minimum listing price. Constraints: minimum 0. |
| `price_to` | number | No | Maximum listing price. Constraints: minimum 0. |

## Pricing

**Current live price:** $0.20 per 1,000 results.

The price is based on the latest live Apify pricing entry. Per-result charges depend on the number of billed results returned. If the price line lists subscription tiers, the rate shown for each tier applies to that Apify subscription level.

## FAQ

### Is it legal to scrape this data?

This Actor is intended for data that is publicly available from Vinted. You are responsible for following the source site's terms, privacy and copyright rules, and the laws that apply to your use of the data. Only collect information you have a lawful basis to use.

### What limits should I expect?

Use the field constraints and limits in the input table. The source controls which records are available, so a narrow query or unavailable page can return fewer results or none. Keep batch lists small when you need a quick first run.

### Can I run it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/actors/thescrappa~vinted-search-scraper/runs`. The response includes `defaultDatasetId`; use it to fetch the run's dataset items. See the [Apify Run Actor API](https://docs.apify.com/api/v2/actors-runs-post) and [Actor runs guide](https://docs.apify.com/api/v2/actors-actor-runs). Apify integrations for [Make](https://docs.apify.com/integrations/make), [Zapier](https://docs.apify.com/integrations/zapier), and [n8n](https://docs.apify.com/integrations/n8n) can trigger runs and pass results to the next workflow step. Send dataset rows to Google Sheets through a Sheets step or a workflow integration.

### What happens if a request fails?

Apify reports input validation and source request errors in the run details. Review the error, correct the input, and retry after a temporary source problem. Depending on when a request stops, the dataset may be empty or contain results collected before the failure.

## Related Scrappa Actors

- [Kleinanzeigen Listing Details Scraper for Buyers](https://apify.com/thescrappa/kleinanzeigen-listing-details-scraper)
- [Kleinanzeigen Search Scraper for Product Research](https://apify.com/thescrappa/kleinanzeigen-search-scraper)
- [Vinted Item Details Scraper for Product Research](https://apify.com/thescrappa/vinted-item-details-scraper)
- [Vinted User Items Scraper for Product Research](https://apify.com/thescrappa/vinted-user-items-scraper)
- [Vinted User Profile Scraper for Seller Research](https://apify.com/thescrappa/vinted-user-profile-scraper)
